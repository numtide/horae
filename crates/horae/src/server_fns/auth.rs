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

/// Return the current session identity without financial or provider metadata.
#[server]
pub async fn get_me() -> Result<crate::models::user::CurrentUser, ServerFnError> {
    let user = require_user().await?;
    Ok(crate::models::user::CurrentUser {
        id: user.id,
        org_id: user.org_id,
        email: user.email,
        name: user.name,
        org_role: user.org_role,
    })
}
