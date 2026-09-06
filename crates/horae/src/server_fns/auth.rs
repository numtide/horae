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
