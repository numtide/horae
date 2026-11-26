//! The signed-in person's own profile, and the workspace time zone.

use super::*;
use crate::models::user::ProfileOptions;

/// The sign-in binding and the zone names the Profile and Workspace settings
/// offer. The person's other values come from `get_me`.
#[server]
pub async fn get_profile_options() -> Result<ProfileOptions, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    // No transaction checks the account here, so the read does.
    let row = sqlx::query!(
        "SELECT u.oidc_issuer, u.timezone, o.timezone AS workspace_timezone
         FROM users u JOIN organizations o ON o.id = u.org_id
         WHERE u.id = $1 AND u.active",
        caller.user_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| unauthorized(crate::ops::ACCOUNT_NOT_ACTIVE))?;
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
    use crate::ops;
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await.map_err(op_error)?;
    let result =
        ops::profile::update(&mut write.tx, &write.actor, &name, timezone.as_deref()).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}

/// Change the organization's time zone, which every person without their own
/// follows. Needs permission to manage the company.
#[server]
pub async fn set_workspace_timezone(timezone: String) -> Result<(), ServerFnError> {
    use crate::ops;
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await.map_err(op_error)?;
    let result = ops::organization::set_timezone(&mut write.tx, &write.actor, &timezone).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}

/// The host of an issuer URL, which is how people recognise their provider.
#[cfg(feature = "server")]
fn issuer_host(issuer: &str) -> String {
    let rest = issuer.split_once("://").map_or(issuer, |(_, rest)| rest);
    rest.split('/').next().unwrap_or(rest).to_owned()
}

#[cfg(all(test, feature = "server"))]
mod tests;
