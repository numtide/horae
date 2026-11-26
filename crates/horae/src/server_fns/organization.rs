//! Organization server functions: week start, name, invoice branding and
//! invoice rounding.

use super::*;
#[cfg(feature = "server")]
use crate::ops;
use horae_core::types::RoundingMethod;

/// First weekday of the signed-in user's organization, shared by the timesheet
/// and its date picker. Invalid stored configuration must not silently default.
#[server]
pub async fn get_week_start() -> Result<chrono::Weekday, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let day = sqlx::query_scalar!(
        "SELECT week_start FROM organizations WHERE id = $1",
        caller.org_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;
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
    let user_id = session_caller().await?.user_id;
    let state = crate::state::global_state().await;

    let name = sqlx::query_scalar!(
        r#"SELECT o.name
             FROM organizations o
             JOIN users u ON u.org_id = o.id
            WHERE u.id = $1"#,
        user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(name)
}

// ── Organization branding ─────────────────────────────────────────────────────

#[server]
pub async fn get_org_branding() -> Result<OrgBranding, ServerFnError> {
    let caller = session_caller().await?;
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
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await.map_err(op_error)?;
    let result =
        ops::organization::update_branding(&mut write.tx, &write.actor, &expected, &branding).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}

#[server]
pub async fn get_invoice_rounding() -> Result<RoundingMethod, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await.map_err(op_error)?;
    let result = ops::organization::invoice_rounding(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn set_invoice_rounding(method: RoundingMethod) -> Result<RoundingMethod, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await.map_err(op_error)?;
    let result = ops::organization::set_invoice_rounding(&mut write.tx, &write.actor, method).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}
