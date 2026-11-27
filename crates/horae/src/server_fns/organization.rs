//! Organization server functions: week start, name and invoice branding.

use super::*;
#[cfg(feature = "server")]
use crate::ops;

/// First weekday of the signed-in user's organization, shared by the timesheet
/// and its date picker. Invalid stored configuration must not silently default.
#[server]
pub async fn get_week_start() -> Result<chrono::Weekday, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let state = crate::state::global_state().await;
    // No transaction checks the account here, so the read does.
    let day = sqlx::query_scalar!(
        "SELECT o.week_start FROM organizations o
           JOIN users u ON u.org_id = o.id AND u.id = $2 AND u.active
          WHERE o.id = $1",
        caller.org_id,
        caller.user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| unauthorized(crate::ops::ACCOUNT_NOT_ACTIVE))?;
    match day {
        1 => Ok(chrono::Weekday::Mon),
        2 => Ok(chrono::Weekday::Tue),
        3 => Ok(chrono::Weekday::Wed),
        4 => Ok(chrono::Weekday::Thu),
        5 => Ok(chrono::Weekday::Fri),
        6 => Ok(chrono::Weekday::Sat),
        7 => Ok(chrono::Weekday::Sun),
        _ => Err(server_err(
            "The organization's week start must be between 1 and 7",
        )),
    }
}

/// The organization's display name — for the admin shell's workspace header.
/// Any signed-in user may read it (it is not sensitive).
#[server]
pub async fn get_org_name() -> Result<String, ServerFnError> {
    let user_id = unchecked_session_caller().await?.user_id;
    let state = crate::state::global_state().await;

    // No transaction checks the account here, so the read does.
    let name = sqlx::query_scalar!(
        r#"SELECT o.name
             FROM organizations o
             JOIN users u ON u.org_id = o.id
            WHERE u.id = $1 AND u.active"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| unauthorized(crate::ops::ACCOUNT_NOT_ACTIVE))?;

    Ok(name)
}

// ── Organization branding ─────────────────────────────────────────────────────

#[server]
pub async fn get_org_branding() -> Result<OrgBranding, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await.map_err(op_error)?;
    let result = ops::organization::branding(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn update_org_branding(
    branding: OrgBranding,
    expected: OrgBranding,
) -> Result<OrgBranding, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await.map_err(op_error)?;
    let result =
        ops::organization::update_branding(&mut write.tx, &write.actor, &expected, &branding).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}
