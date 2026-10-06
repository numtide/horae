//! Auth server functions.

use super::*;

// ── Auth ─────────────────────────────────────────────────────────────────────
// Login is not a server fn — the real flow goes through the Axum `/auth/login`
// route (OIDC / dev-login); see `src/auth/`.

/// Destroy the current session (logout).
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::clear_session(&session)
        .await
        .map_err(server_err)
}

/// Return the currently authenticated user, or 401 if not logged in.
#[server]
pub async fn get_me() -> Result<User, ServerFnError> {
    require_user().await
}

/// Explain the session person's scoped permissions, or None before activation.
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
