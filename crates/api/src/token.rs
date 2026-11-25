//! `Authorization: Bearer …`: the token a public API request authenticates
//! with, a personal access token (`horae_pat_…`, spec 022 FR-010) or an
//! app's OAuth access token (`horae_oat_…`, spec 024 FR-070). The session
//! cookie is never read here.

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use sqlx::PgPool;

use crate::ops::{self, Caller};

/// The caller a valid token stands for. Handlers pass it to `ops::begin` or
/// `ops::begin_read`, whose `Actor::load` checks the person again.
#[derive(Debug, Clone, Copy)]
pub struct ApiCaller(pub Caller);

impl<S: Send + Sync> FromRequestParts<S> for ApiCaller
where
    PgPool: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // The header is the secret: it is never logged, and a refusal never echoes it.
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(bearer)
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let pool = PgPool::from_ref(state);
        // Each authenticator answers `None` for the other's prefix, and for
        // any other credential.
        let found = if token.starts_with(ops::oauth::ACCESS_TOKEN_PREFIX) {
            ops::oauth::tokens::authenticate(&pool, token).await
        } else {
            ops::api_tokens::authenticate(&pool, token).await
        };
        match found {
            Ok(Some(caller)) => Ok(Self(caller)),
            Ok(None) => Err(StatusCode::UNAUTHORIZED),
            Err(error) => {
                tracing::error!(%error, "Could not check an API token");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// The credential of a `Bearer` authorization; the scheme ignores case (RFC 7235).
pub fn bearer(value: &str) -> Option<&str> {
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}

#[cfg(test)]
mod tests;
