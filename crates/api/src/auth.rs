//! The session and the `/auth/*` routes in front of [`horae_server::auth::oidc`].

pub mod dev;
pub mod oidc;
pub mod page;
pub mod session;

pub use page::LoginAssets;

use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use sqlx::PgPool;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, Session, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

/// Build the Axum sub-router for all `/auth/*` endpoints.
///
/// In dev mode (`DEV_LOGIN=1`) `/auth/login` serves the one-click admin page and
/// the `/auth/dev-login` bypass is registered. Otherwise `/auth/login` serves the
/// SSO landing page, `/auth/oidc/start` begins the OIDC flow, and
/// `/auth/callback` completes it — the passwordless dev bypass is **not**
/// registered, so it can never be reached in production.
///
/// `assets` are the icon URLs the login page links, resolved by the crate that
/// owns them.
pub fn router(dev_login: bool, assets: LoginAssets) -> Router {
    let mut router = Router::new().route("/auth/logout", post(dev::logout_post));

    if dev_login {
        router = router
            .route("/auth/login", get(dev::login_page))
            .route("/auth/dev-login", post(dev::dev_login_post));
    } else {
        router = router
            .route("/auth/login", get(oidc::login_page))
            .route("/auth/oidc/start", get(oidc::start))
            .route("/auth/callback", get(oidc::callback));
    }

    router.with_state(assets)
}

/// Redirect unauthenticated page navigations to the login screen.
///
/// Only plain page loads are guarded: `GET` requests that accept HTML and fall
/// outside the public paths (`/auth/*`, `/api/*`, `/harvest/*`, `/v2/*`,
/// `/.well-known/*`, `/health`, `/favicon.ico`).
/// Assets, server-function calls, and the Harvest API pass through untouched, and
/// excluding `/auth/*` keeps the login page reachable (no redirect loop). Data is
/// already protected by the server functions' own 401s; this only fixes the UX so
/// a signed-out visitor lands on login instead of a dead, empty app shell.
///
/// A signed-out load under `/oauth/` is remembered as the session's
/// `return_to`, so sign-in comes back to the app's authorization request
/// (spec 024 FR-023). Every other page still lands on `/` after sign-in.
///
/// Must be layered *inside* the session layer so the session is populated.
pub async fn login_redirect_guard(session: Session, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let public = path.starts_with("/auth/")
        || path.starts_with("/api/")
        || path.starts_with("/harvest/")
        || path.starts_with("/v2/")
        || path.starts_with("/.well-known/")
        || path == "/health"
        || path == "/favicon.ico";
    let is_page = request.method() == Method::GET
        && request
            .headers()
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|accept| accept.contains("text/html"));

    if !is_page || public {
        return next.run(request).await;
    }
    let signed_in = match session::get_session_user_id(&session).await {
        Ok(user_id) => user_id.is_some(),
        // Not known to be signed out: sending the person to sign in again
        // would end a session that is still valid.
        Err(error) => {
            tracing::error!(%error, "Could not read the session of a page load");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Horae is temporarily unavailable. Please reload the page.",
            )
                .into_response();
        }
    };
    if !signed_in {
        let target = request.uri().path_and_query().map(|target| target.as_str());
        if let Some(target) = target.and_then(safe_return_to)
            && let Err(error) = session.insert(RETURN_TO_KEY, target).await
        {
            tracing::warn!(%error, "Could not remember where sign-in returns to");
        }
        return Redirect::to("/auth/login").into_response();
    }
    next.run(request).await
}

/// Where sign-in sends the person back to, set only for `/oauth/` pages.
pub const RETURN_TO_KEY: &str = "return_to";

/// `value` if it is a path on this host under `/oauth/`: no scheme, no
/// `//`, no dot segments, plain or percent-encoded, and no backslash or
/// control character, so it can never redirect elsewhere (research R-3).
pub fn safe_return_to(value: &str) -> Option<&str> {
    let path = value.split('?').next().unwrap_or_default();
    let lower = path.to_ascii_lowercase();
    let safe = path.starts_with("/oauth/")
        && !path.contains("//")
        && !path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        && !["%2e", "%2f", "%5c"]
            .iter()
            .any(|encoded| lower.contains(encoded))
        && !value
            .chars()
            .any(|c| c == '\\' || c.is_whitespace() || c.is_control());
    safe.then_some(value)
}

/// The page to land on after a successful sign-in: the remembered `/oauth/`
/// page, checked again, else `/`. The value is used once.
pub async fn take_return_to(session: &Session) -> String {
    let stored: Option<String> = session.remove(RETURN_TO_KEY).await.ok().flatten();
    stored
        .as_deref()
        .and_then(safe_return_to)
        .unwrap_or("/")
        .to_owned()
}

#[cfg(test)]
mod tests;

/// Create and migrate the Postgres-backed session store, then return a
/// `SessionManagerLayer` ready to be applied to the Axum router. `secure` marks
/// cookies `Secure` (HTTPS-only) — set it in production.
pub async fn make_session_layer(
    pool: PgPool,
    secure: bool,
) -> anyhow::Result<SessionManagerLayer<PostgresStore>> {
    let store = PostgresStore::new(pool);
    store.migrate().await?;

    let layer = SessionManagerLayer::new(store)
        .with_secure(secure)
        // `Lax`, not the tower-sessions default `Strict`: the OIDC callback is a
        // cross-site top-level redirect from the provider, and `Strict` would
        // withhold the session cookie that carries the in-flight flow secrets,
        // breaking login. `Lax` still sends the cookie on that top-level GET.
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnSessionEnd); // expires when the browser session ends

    Ok(layer)
}
