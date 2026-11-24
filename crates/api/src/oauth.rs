//! Horae's OAuth 2.0 authorization server (spec 024): `/oauth/authorize`,
//! `/oauth/token`, `/oauth/revoke` and the RFC 8414 metadata. The handlers
//! parse the request, call `ops::oauth` and render the answer; the rules live
//! in the operations (Principle IV). The approval page between authorize and
//! the app's redirect is the web app's `OAuthConsent` route.
//!
//! The server runs only with `HORAE_PUBLIC_URL`, its issuer; without it these
//! paths answer 404.

use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{any, get, post};
use sqlx::PgPool;

use crate::config::{OAuthConfig, PublicUrl};
use crate::rate_limit::RateLimiter;

pub mod authorize;
mod client_auth;
mod metadata;
pub mod pending;
mod revoke;
mod token;

const AUTHORIZE: &str = "/oauth/authorize";
const TOKEN: &str = "/oauth/token";
const REVOKE: &str = "/oauth/revoke";
const METADATA: &str = "/.well-known/oauth-authorization-server";

/// What the OAuth handlers share.
#[derive(Clone)]
pub struct OAuthState {
    pub pool: PgPool,
    /// `HORAE_PUBLIC_URL`, the issuer.
    pub issuer: Arc<str>,
    pub config: OAuthConfig,
    /// The token and revocation endpoints' limit, per client id (FR-081).
    pub limiter: Arc<RateLimiter<String>>,
}

impl OAuthState {
    pub fn new(pool: PgPool, issuer: &PublicUrl, config: OAuthConfig) -> Self {
        Self {
            pool,
            issuer: issuer.as_str().into(),
            config,
            limiter: Arc::new(RateLimiter::new(config.token_rate_limit)),
        }
    }
}

/// The authorization server's routes, or 404 on each of its paths when it
/// is off. The caller adds the session layer `/oauth/authorize` needs.
pub fn router(state: Option<OAuthState>) -> Router {
    let Some(state) = state else {
        let off = || any(|| async { StatusCode::NOT_FOUND });
        return Router::new()
            .route(AUTHORIZE, off())
            .route(TOKEN, off())
            .route(REVOKE, off())
            .route(METADATA, off());
    };
    Router::new()
        .route(AUTHORIZE, get(authorize::authorize))
        .route(TOKEN, post(token::token))
        .route(REVOKE, post(revoke::revoke))
        .route(METADATA, get(metadata::metadata))
        .with_state(state)
        .layer(axum::middleware::from_fn(security_headers))
}

/// Middleware for the whole server: responses under `/oauth/` cannot be
/// framed (clickjacking of the approval page, FR-024) and send no referrer,
/// so a code or a request id never leaves in a `Referer`.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let oauth = request.uri().path().starts_with("/oauth/");
    let mut response = next.run(request).await;
    if oauth {
        let headers = response.headers_mut();
        headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("frame-ancestors 'none'"),
        );
        headers.insert(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        );
    }
    response
}

/// `Cache-Control: no-store`: an answer that carries a credential or a
/// request must not be cached anywhere.
fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}

/// The name–value pairs of a query or form body, in order and repeats kept.
fn pairs(input: &[u8]) -> Vec<(String, String)> {
    url::form_urlencoded::parse(input).into_owned().collect()
}

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests;
