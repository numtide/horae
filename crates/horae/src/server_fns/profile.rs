//! The signed-in person's own profile, and the workspace time zone.

use super::*;
use crate::models::user::ProfileOptions;

/// The sign-in binding and the zone names the Profile and Workspace settings
/// offer. The person's other values come from `get_me`.
#[server]
pub async fn get_profile_options() -> Result<ProfileOptions, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let issuer = sqlx::query_scalar!("SELECT oidc_issuer FROM users WHERE id = $1", user.id)
        .fetch_one(&state.db)
        .await
        .map_err(server_err)?;
    Ok(ProfileOptions {
        sign_in: issuer.as_deref().map(issuer_host),
        zones: crate::timezone::names(),
    })
}

/// Change the signed-in person's display name and time zone (`None` follows the
/// workspace). Nobody else's account is reachable from here.
#[server]
pub async fn update_own_profile(
    name: String,
    timezone: Option<String>,
) -> Result<(), ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    update_profile_record(&state.db, user.org_id, user.id, &name, timezone.as_deref()).await
}

/// Change the organization's time zone, which every person without their own
/// follows. Administrators only.
#[server]
pub async fn set_workspace_timezone(timezone: String) -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    set_workspace_timezone_record(&state.db, admin.org_id, admin.id, &timezone).await
}

/// The host of an issuer URL, which is how people recognise their provider.
#[cfg(feature = "server")]
fn issuer_host(issuer: &str) -> String {
    let rest = issuer.split_once("://").map_or(issuer, |(_, rest)| rest);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

#[cfg(feature = "server")]
fn zone(name: &str) -> Result<&'static str, ServerFnError> {
    crate::timezone::parse(name)
        .map(|zone| zone.name())
        .ok_or_else(|| err(BAD_REQUEST, "Choose a time zone from the list"))
}

#[cfg(feature = "server")]
pub(super) async fn update_profile_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    user_id: uuid::Uuid,
    name: &str,
    timezone: Option<&str>,
) -> Result<(), ServerFnError> {
    let name = name.trim();
    if !(1..=200).contains(&name.chars().count()) || name.contains('\0') {
        return Err(err(BAD_REQUEST, "Enter a name of 1 to 200 characters"));
    }
    let timezone = timezone.map(zone).transpose()?;
    let updated = sqlx::query!(
        "UPDATE users SET name = $3, timezone = $4 WHERE id = $1 AND org_id = $2 AND active",
        user_id,
        org_id,
        name,
        timezone,
    )
    .execute(db)
    .await
    .map_err(server_err)?;
    if updated.rows_affected() == 0 {
        return Err(unauthorized("Account is not active"));
    }
    Ok(())
}

#[cfg(feature = "server")]
pub(super) async fn set_workspace_timezone_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    timezone: &str,
) -> Result<(), ServerFnError> {
    let timezone = zone(timezone)?;
    // Authority is checked in the same statement, so a role revoked meanwhile
    // cannot slip a change through.
    let updated = sqlx::query!(
        "UPDATE organizations SET timezone = $3 WHERE id = $1 AND EXISTS (
           SELECT 1 FROM users WHERE id = $2 AND org_id = $1 AND active AND org_role = $4)",
        org_id,
        actor_id,
        timezone,
        OrgRole::Admin as OrgRole,
    )
    .execute(db)
    .await
    .map_err(server_err)?;
    if updated.rows_affected() == 0 {
        return Err(forbidden("Admin access required"));
    }
    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests;
