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
    after: Option<crate::models::permission_audit::AuditCursor>,
    expected_requester: Option<crate::models::permission_editor::PermissionRequester>,
) -> Result<crate::models::permission_audit::AuditPage, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate permission history request");
            server_err("Permission history is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    super::permissions::audit::page(
        &state.db,
        user.org_id,
        user.id,
        after.as_ref(),
        expected_requester,
    )
    .await
    .map_err(|error| match error {
        super::permissions::audit::AuditReadError::Forbidden => {
            forbidden("Current administrator authority is required")
        }
        error => {
            tracing::error!(%error,"Unable to load permission history");
            server_err("Permission history is unavailable")
        }
    })
}

/// Explain the session person's stored permissions. This display snapshot
/// cannot authorize later requests or select another user.
#[server]
pub async fn get_my_permissions()
-> Result<crate::models::own_permissions::OwnPermissions, ServerFnError> {
    use crate::ops::{self, Caller};
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate own permission request");
            server_err("Permission state is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::profile::permissions(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// The running server's version label, for administrators only.
#[server]
pub async fn get_server_version() -> Result<String, ServerFnError> {
    use crate::ops::{self, Caller};
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let read = ops::begin_read(&state.db, Caller::session(user.org_id, user.id)).await?;
    let result = ops::profile::server_version(&read.actor);
    Ok(read.finish(result).await?)
}
