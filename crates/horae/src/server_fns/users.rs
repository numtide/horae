//! User server functions.

use super::*;
use crate::models::people::{PeopleActivity, PeopleCursor, PeoplePage};
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
#[cfg(feature = "server")]
use permissions::provisioning::{self, profile_for_role};

mod rates;
pub use rates::{load_person_rates, update_person_rates};

// ── Users ─────────────────────────────────────────────────────────────────────

/// Read a bounded people-directory page under current canonical permissions.
/// This is not a report/approval identity resolver or project assignment picker.
#[server]
pub async fn list_people(
    activity: PeopleActivity,
    after: Option<PeopleCursor>,
) -> Result<PeoplePage, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate directory request");
            server_err("People directory is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::directory::read(&state.db, user.org_id, user.id, activity, after.as_ref())
        .await
        .map_err(|error| match error {
            permissions::directory::DirectoryError::Forbidden => {
                forbidden("Current people-read authority is required")
            }
            permissions::directory::DirectoryError::InvalidCursor => {
                err(BAD_REQUEST, "Invalid directory cursor")
            }
            error => {
                tracing::error!(%error, "People directory read failed");
                server_err("People directory is unavailable")
            }
        })
}

/// List identities for the current directory and workflow consumers, without
/// financial or authentication metadata. Any active user can list active people;
/// including deactivated accounts requires legacy administrator access.
#[server]
pub async fn list_users(include_inactive: bool) -> Result<Vec<UserListItem>, ServerFnError> {
    let viewer = if include_inactive {
        require_admin().await?
    } else {
        require_user().await?
    };
    let state = crate::state::global_state().await;

    sqlx::query_as!(
        UserListItem,
        r#"SELECT id, email, name, org_role as "org_role: OrgRole", active
         FROM users
         WHERE org_id = $2 AND ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
        viewer.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)
}

/// Create a new user account. Requires admin role.
#[server]
pub async fn create_user(email: String, name: String, role: String) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let org_role: OrgRole = parse_enum(&role, "role (use admin, manager, or member)")?;
    let user = insert_user(&state.db, admin.org_id, admin.id, &email, &name, org_role).await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserCreated {
            occurred_at: chrono::Utc::now(),
            org_id: admin.org_id,
            user: user_payload(&user),
        });
    Ok(user)
}

/// The most people one request may add, matching the importer's bounded list
/// of missing people.
#[cfg(feature = "server")]
const MAX_NEW_PEOPLE: usize = 500;

/// Add members, deactivated unless the administrator chose otherwise. The
/// Harvest run page uses this for people a run could not match. An email that
/// already has an account (added meanwhile, or in another tab) is reported as
/// present rather than failing the rest.
#[server]
pub async fn add_harvest_people(
    people: Vec<NewPerson>,
    activation: Activation,
) -> Result<AddedPeople, ServerFnError> {
    let admin = require_admin().await?;
    if people.is_empty() || people.len() > MAX_NEW_PEOPLE {
        return Err(err(
            BAD_REQUEST,
            format!("Add between 1 and {MAX_NEW_PEOPLE} people at a time"),
        ));
    }
    let people = people
        .into_iter()
        .map(|person| {
            person
                .normalized()
                .ok_or_else(|| err(BAD_REQUEST, "Every person needs an email address"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let state = crate::state::global_state().await;
    let result = insert_users(
        &state.db,
        admin.org_id,
        admin.id,
        &people,
        OrgRole::Member,
        activation,
        Existing::Skip,
    )
    .await?;
    for user in &result.added {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::UserCreated {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                user: user_payload(user),
            });
    }
    Ok(result)
}

#[cfg(feature = "server")]
pub(crate) async fn insert_user(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    email: &str,
    name: &str,
    org_role: OrgRole,
) -> Result<User, ServerFnError> {
    let person = NewPerson {
        email: email.to_owned(),
        name: name.to_owned(),
    };
    let mut created = insert_users(
        db,
        org_id,
        actor_id,
        &[person],
        org_role,
        Activation::Active,
        Existing::Refuse,
    )
    .await?;
    Ok(created.added.remove(0))
}

#[cfg(feature = "server")]
async fn insert_users(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    people: &[NewPerson],
    org_role: OrgRole,
    activation: Activation,
    existing: Existing,
) -> Result<AddedPeople, ServerFnError> {
    let mut tx = begin_user_access_change(db, org_id, actor_id).await?;
    // Under policy 1 privileges are given through the permission editor, which
    // audits them; a role chosen here would bypass it.
    if access_policy(&mut tx, org_id).await? != 0 && org_role != OrgRole::Member {
        return Err(conflict(
            "Add the person as a member, then choose their profile in the permission editor",
        ));
    }
    // The organization row is locked first, then each email, in one order so
    // two batches cannot deadlock. Emails compare trimmed and ignoring case.
    let key = |person: &NewPerson| person.email.trim().to_lowercase();
    let mut order: Vec<_> = people.iter().collect();
    order.sort_by_key(|person| key(person));
    for pair in order.windows(2) {
        if key(pair[0]) == key(pair[1]) {
            return Err(err(
                BAD_REQUEST,
                format!("The email {} is listed twice", pair[1].email),
            ));
        }
    }
    let mut fresh = Vec::with_capacity(order.len());
    let mut present = Vec::new();
    for person in order {
        let taken = crate::db::lock_new_user_email(&mut tx, &person.email)
            .await
            .map_err(server_err)?;
        match (taken, existing) {
            (false, _) => fresh.push(person),
            (true, Existing::Skip) => present.push(person.email.clone()),
            (true, Existing::Refuse) => {
                return Err(conflict(format!(
                    "A user with the email {} already exists",
                    person.email
                )));
            }
        }
    }
    let mut users = Vec::with_capacity(fresh.len());
    for person in fresh {
        let user = sqlx::query_as!(
            User,
            r#"INSERT INTO users (id, org_id, email, name, org_role, active)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, org_id, email, name, oidc_subject,
                       org_role as "org_role: OrgRole",
                       cost_rate_cents, billable_rate_cents, active,
                       created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
            uuid::Uuid::now_v7(),
            org_id,
            person.email,
            person.name,
            org_role as OrgRole,
            activation == Activation::Active,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("users_email_key") {
                conflict(format!(
                    "A user with the email {} already exists",
                    person.email
                ))
            } else {
                server_err(e)
            }
        })?;
        provisioning::provision(&mut tx, org_id, user.id, profile_for_role(org_role))
            .await
            .map_err(server_err)?;
        users.push(user);
    }
    tx.commit().await.map_err(server_err)?;
    Ok(AddedPeople {
        added: users,
        present,
    })
}

/// What adding people does with an email an account already uses.
#[cfg(feature = "server")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Existing {
    /// Refuse the request: the person was meant to be new.
    Refuse,
    /// Leave the account alone and report the email as already present.
    Skip,
}

#[cfg(feature = "server")]
async fn begin_user_access_change(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    // The last-admin count must include changes committed while the
    // organization lock was awaited, regardless of the connection default.
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    // Both role and activation changes must serialize before counting admins;
    // locking only the target user lets two admins remove each other.
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Organization not found"))?;
    // Recheck after the organization lock: authority may have been revoked
    // while this request waited. Keep the actor locked through commit.
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND org_id = $2
           AND active AND org_role = $3 FOR SHARE",
        actor_id,
        org_id,
        OrgRole::Admin as OrgRole,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| forbidden("Active administrator access required"))?;
    Ok(tx)
}

/// The organization's permission policy, read under the access-change lock.
#[cfg(feature = "server")]
async fn access_policy(
    db: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
) -> Result<i32, ServerFnError> {
    sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_one(db)
    .await
    .map_err(server_err)
}

/// Read the prior state under the organization's access-change lock, for both
/// the last-admin guard and the post-commit event.
#[cfg(feature = "server")]
async fn user_active_role(
    db: &mut sqlx::PgConnection,
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
    db: &mut sqlx::PgConnection,
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

    let (user, previous) =
        change_user_role(&state.db, admin.org_id, admin.id, user_id, org_role).await?;
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

#[cfg(feature = "server")]
pub(super) async fn change_user_role(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    user_id: uuid::Uuid,
    org_role: OrgRole,
) -> Result<(User, Option<OrgRole>), ServerFnError> {
    let mut tx = begin_user_access_change(db, org_id, actor_id).await?;
    // Under policy 1 roles no longer grant anything; profiles replace them.
    if access_policy(&mut tx, org_id).await? != 0 {
        return Err(conflict(
            "Roles no longer apply; change the person's profile in the permission editor",
        ));
    }
    // Read the prior role/active so the event reports the transition (a no-op
    // role change emits nothing, FR-012) and so we can refuse demoting the last
    // active admin — including an admin dropping their own role.
    let current = user_active_role(&mut tx, user_id, org_id).await?;
    let previous: Option<OrgRole> = current.map(|(_, role)| role);

    let demoting_active_admin = current.is_some_and(|(active, role)| {
        active && role == OrgRole::Admin && org_role != OrgRole::Admin
    });
    if demoting_active_admin {
        ensure_other_active_admin(&mut tx, org_id, user_id).await?;
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
        org_id,
        org_role as OrgRole,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("User not found"))?;
    provisioning::follow_role(&mut tx, org_id, user_id, org_role)
        .await
        .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;
    Ok((user, previous))
}

/// Activate or deactivate a user account. Deactivated users cannot sign in
/// but their historical time entries are preserved (FR-002).
#[server]
pub async fn set_user_active(user_id: String, active: bool) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let user_id = parse_uuid(&user_id, "user_id")?;

    let (user, was_active) =
        change_user_active(&state.db, admin.org_id, admin.id, user_id, active).await?;
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

#[cfg(feature = "server")]
async fn change_user_active(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    user_id: uuid::Uuid,
    active: bool,
) -> Result<(User, Option<bool>), ServerFnError> {
    let mut tx = begin_user_access_change(db, org_id, actor_id).await?;
    let current = user_active_role(&mut tx, user_id, org_id).await?;
    let was_active: Option<bool> = current.map(|(active, _)| active);

    // Deactivating the last active admin would lock the org out. Policy 1
    // counts Administrator states, as the permission editor does.
    if !active {
        if access_policy(&mut tx, org_id).await? == 0 {
            if current.is_some_and(|(a, role)| a && role == OrgRole::Admin) {
                ensure_other_active_admin(&mut tx, org_id, user_id).await?;
            }
        } else if provisioning::is_active_administrator(&mut tx, org_id, user_id)
            .await
            .map_err(server_err)?
            && !provisioning::other_active_administrator(&mut tx, org_id, user_id)
                .await
                .map_err(server_err)?
        {
            return Err(conflict(
                "The organization must keep at least one active administrator.",
            ));
        }
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
        org_id,
        active,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("User not found"))?;

    tx.commit().await.map_err(server_err)?;
    Ok((user, was_active))
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    mod authority;
    mod concurrency;
    mod people;
    mod permission_states;
    mod rates;
}
