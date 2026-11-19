//! `Authorization: Bearer horae_pat_…`: the personal access token a public API
//! request authenticates with (spec 022 FR-010). The session cookie is never
//! read here.

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
        match ops::api_tokens::authenticate(&PgPool::from_ref(state), token).await {
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
