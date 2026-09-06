//! User server functions.

use super::*;

// ── Users ─────────────────────────────────────────────────────────────────────

/// Blank out the pay-sensitive fields for callers below manager: pickers and
/// name lookups only need identities, while rates are manager/admin material
/// (SPEC §6 keeps members away from other users' money).
#[cfg(feature = "server")]
fn hide_rates(users: &mut [User]) {
    for u in users {
        u.cost_rate_cents = None;
        u.billable_rate_cents = None;
    }
}

/// List users. Any authenticated user can list active users (for pickers), but
/// only managers and admins see the rate fields; pass `include_inactive = true`
/// to also see deactivated accounts (admin only).
#[server]
pub async fn list_users(include_inactive: bool) -> Result<Vec<User>, ServerFnError> {
    let viewer = if include_inactive {
        require_admin().await?
    } else {
        require_user().await?
    };
    let state = crate::state::global_state().await;

    let mut users = sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    if !viewer.is_manager_or_above() {
        hide_rates(&mut users);
    }
    Ok(users)
}

/// Create a new user account. Requires admin role.
#[server]
pub async fn create_user(email: String, name: String, role: String) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let org_role: OrgRole = parse_enum(&role, "role (use admin, manager, or member)")?;

    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (id, org_id, email, name, org_role)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        admin.org_id,
        email,
        name,
        org_role as OrgRole,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("users_email_key") {
            conflict("A user with this email already exists")
        } else {
            server_err(e)
        }
    })?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserCreated {
            occurred_at: chrono::Utc::now(),
            org_id: admin.org_id,
            user: user_payload(&user),
        });
    Ok(user)
}

/// The target user's current `(active, org_role)` within the org, if they exist.
/// Read before a role/active change to drive both the plugin event and the
/// last-admin guard.
#[cfg(feature = "server")]
async fn user_active_role(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<(bool, OrgRole)>, ServerFnError> {
    let row = sqlx::query!(
        r#"SELECT active, org_role as "org_role: OrgRole"
             FROM users WHERE id = $1 AND org_id = $2"#,
        user_id,
        org_id,
    )
    .fetch_optional(db)
    .await
    .map_err(server_err)?;
    Ok(row.map(|r| (r.active, r.org_role)))
}

/// Guard against removing the org's last active admin. `exclude` is the user
/// being changed (deactivated or demoted), so they are not counted among the
/// admins that must remain — otherwise a lone admin could lock everyone out and
/// force a re-seed (FR-002).
#[cfg(feature = "server")]
async fn ensure_other_active_admin(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    exclude: uuid::Uuid,
) -> Result<(), ServerFnError> {
    let others = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64"
             FROM users
            WHERE org_id = $1 AND id <> $2 AND active = true AND org_role = $3"#,
        org_id,
        exclude,
        OrgRole::Admin as OrgRole,
    )
    .fetch_one(db)
    .await
    .map_err(server_err)?;

    if others == 0 {
        return Err(conflict(
            "The organization must keep at least one active admin.",
        ));
    }
    Ok(())
}

/// Change a user's organization role. Requires admin role.
#[server]
pub async fn set_user_role(user_id: String, role: String) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let org_role: OrgRole = parse_enum(&role, "role (use admin, manager, or member)")?;

    // Read the prior role/active so the event reports the transition (a no-op
    // role change emits nothing, FR-012) and so we can refuse demoting the last
    // active admin — including an admin dropping their own role.
    let current = user_active_role(&state.db, user_id, admin.org_id).await?;
    let previous: Option<OrgRole> = current.map(|(_, role)| role);

    let demoting_active_admin = current.is_some_and(|(active, role)| {
        active && role == OrgRole::Admin && org_role != OrgRole::Admin
    });
    if demoting_active_admin {
        ensure_other_active_admin(&state.db, admin.org_id, user_id).await?;
    }

    let user = sqlx::query_as!(
        User,
        r#"UPDATE users SET org_role = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        user_id,
        admin.org_id,
        org_role as OrgRole,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("User not found"))?;

    if let Some(prev) = previous.filter(|p| *p != user.org_role) {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::UserRoleChanged {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                user: user_payload(&user),
                previous_role: prev.to_string(),
            });
    }
    Ok(user)
}

/// Activate or deactivate a user account. Deactivated users cannot sign in
/// but their historical time entries are preserved (FR-002).
#[server]
pub async fn set_user_active(user_id: String, active: bool) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let user_id = parse_uuid(&user_id, "user_id")?;

    let current = user_active_role(&state.db, user_id, admin.org_id).await?;
    let was_active: Option<bool> = current.map(|(active, _)| active);

    // Deactivating the last active admin would lock the org out.
    let deactivating_active_admin =
        !active && current.is_some_and(|(a, role)| a && role == OrgRole::Admin);
    if deactivating_active_admin {
        ensure_other_active_admin(&state.db, admin.org_id, user_id).await?;
    }

    let user = sqlx::query_as!(
        User,
        r#"UPDATE users SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        user_id,
        admin.org_id,
        active,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("User not found"))?;

    // FR-005 defines only a deactivation event (no user_reactivated).
    if crate::plugin::event::active_transition(was_active, active)
        == Some(crate::plugin::event::ActiveTransition::Deactivated)
    {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::UserDeactivated {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                user: user_payload(&user),
            });
    }
    Ok(user)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn hide_rates_clears_both_rate_fields() {
        let mut users = vec![User {
            id: uuid::Uuid::now_v7(),
            org_id: uuid::Uuid::now_v7(),
            email: "a@test.com".into(),
            name: "A".into(),
            oidc_subject: None,
            org_role: OrgRole::Member,
            cost_rate_cents: Some(6000),
            billable_rate_cents: Some(10000),
            active: true,
            created_at: chrono::Utc::now(),
        }];
        hide_rates(&mut users);
        assert_eq!(users[0].cost_rate_cents, None);
        assert_eq!(users[0].billable_rate_cents, None);
    }
}
