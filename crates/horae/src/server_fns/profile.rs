//! The signed-in person's own profile, and the workspace time zone.

use super::*;
use crate::models::user::ProfileOptions;

/// The sign-in binding and the zone names the Profile and Workspace settings
/// offer. The person's other values come from `get_me`.
#[server]
pub async fn get_profile_options() -> Result<ProfileOptions, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let row = sqlx::query!(
        "SELECT u.oidc_issuer, u.timezone, o.timezone AS workspace_timezone
         FROM users u JOIN organizations o ON o.id = u.org_id WHERE u.id = $1",
        user.id
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;
    Ok(ProfileOptions {
        sign_in: row.oidc_issuer.as_deref().map(issuer_host),
        zones: zone_choices(row.timezone.into_iter().chain([row.workspace_timezone])),
    })
}

/// The picker's zones plus any `current` one it leaves out (an alias saved
/// before, or imported), so the field can still show it.
#[cfg(feature = "server")]
fn zone_choices(current: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut zones = crate::timezone::names();
    for zone in current {
        if !zones.contains(&zone) {
            zones.push(zone);
        }
    }
    zones
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
/// follows. Needs permission to manage the company.
#[server]
pub async fn set_workspace_timezone(timezone: String) -> Result<(), ServerFnError> {
    use crate::ops::{self, Caller};
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::organization::set_timezone(&mut write.tx, &write.actor, &timezone).await;
    Ok(write.commit(result, &*state.plugins).await?)
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
    if !crate::models::user::profile_name_is_valid(name) {
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

#[cfg(all(test, feature = "server"))]
mod tests;
