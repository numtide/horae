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
        org_role: user.org_role,
        utc_offset_seconds: crate::timezone::utc_offset_seconds(effective, chrono::Utc::now()),
        timezone: zones.timezone,
        workspace_timezone: zones.workspace,
    })
}

/// Read one permission-change record under the session's current Administrator authority.
/// Missing and foreign receipt IDs are indistinguishable; snapshots confer no authority.
#[server]
pub async fn get_permission_audit(
    receipt_id: uuid::Uuid,
) -> Result<Option<crate::models::permission_audit::AuditEntry>, ServerFnError> {
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
    super::permissions::audit::read(&state.db, user.org_id, user.id, receipt_id)
        .await
        .map_err(|error| match error {
            super::permissions::audit::AuditReadError::Forbidden => {
                forbidden("Current administrator authority is required")
            }
            error => {
                tracing::error!(%error, "Unable to load permission history");
                server_err("Permission history is unavailable")
            }
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

/// Explain the session person's scoped permissions. Always `Some`; the option
/// goes when the pages that read it stop handling the legacy policy.
/// This display snapshot cannot authorize later requests or select another user.
#[server]
pub async fn get_my_permissions()
-> Result<Option<crate::models::own_permissions::OwnPermissions>, ServerFnError> {
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
    super::permissions::own::read(&state.db, user.org_id, user.id)
        .await
        .map(Some)
        .map_err(|error| match error {
            super::permissions::own::OwnPermissionsError::Forbidden => {
                forbidden("Current active identity is required")
            }
            error => {
                tracing::error!(%error, "Unable to load own permission state");
                server_err("Permission state is unavailable")
            }
        })
}

/// The running server's version label, for administrators only.
#[server]
pub async fn get_server_version() -> Result<String, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    server_version(&state.db, &user).await
}

/// [`get_server_version`] for the session's active user.
#[cfg(feature = "server")]
pub(crate) async fn server_version(
    pool: &sqlx::PgPool,
    user: &crate::models::User,
) -> Result<String, ServerFnError> {
    let administrator = super::permissions::own::is_administrator(pool, user.org_id, user.id)
        .await
        .map_err(|error| match error {
            super::permissions::own::OwnPermissionsError::Forbidden => {
                forbidden("Current active identity is required")
            }
            error => {
                tracing::error!(%error, "Unable to check administrator authority");
                server_err("Version is unavailable")
            }
        })?;
    if !administrator {
        return Err(forbidden("Administrator authority is required"));
    }
    Ok(crate::version::current())
}
