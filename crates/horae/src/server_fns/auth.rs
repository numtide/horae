//! Auth server functions.

use super::*;

// ── Auth ─────────────────────────────────────────────────────────────────────
// Login and logout are not server fns — they go through the Axum `/auth/login`
// (OIDC / dev-login) and `POST /auth/logout` routes; see `src/auth/`.

/// Return the current session identity without financial or provider metadata.
#[server]
pub async fn get_me() -> Result<crate::models::user::CurrentUser, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    current_user(&state.db, user).await
}

#[cfg(feature = "server")]
pub(super) async fn current_user(
    db: &sqlx::PgPool,
    user: User,
) -> Result<crate::models::user::CurrentUser, ServerFnError> {
    let zones = sqlx::query!(
        "SELECT u.timezone, o.timezone AS workspace
         FROM users u JOIN organizations o ON o.id = u.org_id WHERE u.id = $1",
        user.id
    )
    .fetch_one(db)
    .await
    .map_err(server_err)?;
    let effective = zones.timezone.as_deref().unwrap_or(&zones.workspace);
    Ok(crate::models::user::CurrentUser {
        id: user.id,
        org_id: user.org_id,
        email: user.email,
        name: user.name,
        utc_offset_seconds: crate::timezone::utc_offset_seconds(effective, chrono::Utc::now()),
        timezone: zones.timezone,
        workspace_timezone: zones.workspace,
    })
}

/// Browse bounded permission history under current explicit Administrator authority.
#[server]
pub async fn list_permission_audit(
    filter: crate::models::permission_audit::AuditFilter,
    after: Option<crate::models::permission_audit::AuditCursor>,
    expected_requester: Option<crate::models::permission_editor::PermissionRequester>,
) -> Result<crate::models::permission_audit::AuditPage, ServerFnError> {
    use crate::ops::permissions::audit::HISTORY_UNAVAILABLE;
    let caller = unchecked_session_caller_or(HISTORY_UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    audit_page(
        &state.db,
        caller,
        &filter,
        after.as_ref(),
        expected_requester,
    )
    .await
    .map_err(op_error)
}

#[cfg(feature = "server")]
pub(crate) async fn audit_page(
    db: &sqlx::PgPool,
    caller: crate::ops::Caller,
    filter: &crate::models::permission_audit::AuditFilter,
    after: Option<&crate::models::permission_audit::AuditCursor>,
    expected_requester: Option<crate::models::permission_editor::PermissionRequester>,
) -> Result<crate::models::permission_audit::AuditPage, crate::ops::OpError> {
    use crate::ops;
    let mut read = authenticated(
        ops::begin_read(db, caller).await,
        ops::permissions::audit::HISTORY_UNAVAILABLE,
    )?;
    let result =
        ops::permissions::audit::page(&mut read.tx, &read.actor, filter, after, expected_requester)
            .await;
    read.finish(result).await
}

/// Explain the session person's stored permissions. This display snapshot
/// cannot authorize later requests or select another user.
#[server]
pub async fn get_my_permissions()
-> Result<crate::models::own_permissions::OwnPermissions, ServerFnError> {
    use crate::ops;
    const UNAVAILABLE: &str = "Permission state is unavailable";
    let caller = unchecked_session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    let mut read =
        authenticated(ops::begin_read(&state.db, caller).await, UNAVAILABLE).map_err(op_error)?;
    let result = ops::profile::permissions(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

/// The running server's version label, for administrators only.
#[server]
pub async fn get_server_version() -> Result<String, ServerFnError> {
    use crate::ops;
    let caller = unchecked_session_caller().await?;
    let state = crate::state::global_state().await;
    let read = ops::begin_read(&state.db, caller).await.map_err(op_error)?;
    let result = ops::profile::server_version(&read.actor, state);
    read.finish(result).await.map_err(op_error)
}
