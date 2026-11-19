//! Who a Harvest API request acts as (spec 021 R-6, spec 022 FR-062): a
//! personal access token from `Authorization: Bearer` or the `access_token`
//! query parameter, which Harvest clients send, else the web session cookie.
//! Tokens are the ones `/api/v1` takes, and their requests count against the
//! same limit there and here.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{FromRef, FromRequestParts, Query};
use axum::http::header;
use axum::http::request::Parts;
use horae_api::rate_limit::{RateLimiter, retry_after_seconds};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;
use crate::ops::{self, Caller, Channel};

const ACCOUNT_HEADER: &str = "harvest-account-id";

/// Extractor that authenticates a Harvest API request.
///
/// It carries only the caller's identity. Each reader loads the caller's
/// stored permission state inside its own transaction (see
/// [`super::begin_read`]), so the grants it decides with cannot go stale
/// between this extractor and the rows it returns.
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub channel: Channel,
}

impl AuthUser {
    pub fn caller(&self) -> Caller {
        Caller {
            org_id: self.org_id,
            user_id: self.user_id,
            channel: self.channel,
        }
    }
}

/// The query parameters Harvest clients may authenticate with.
#[derive(Deserialize, Default)]
struct Credentials {
    access_token: Option<String>,
    account_id: Option<String>,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser
where
    PgPool: FromRef<S>,
    Arc<RateLimiter>: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let db = PgPool::from_ref(state);
        // The token is a secret: it is never logged, and a refusal never echoes it.
        let query = Query::<Credentials>::try_from_uri(&parts.uri)
            .map(|Query(query)| query)
            .unwrap_or_default();
        let header_token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(horae_api::token::bearer);
        let token = header_token.or(query.access_token.as_deref().filter(|t| !t.is_empty()));
        if let Some(token) = token {
            let account = parts
                .headers
                .get(ACCOUNT_HEADER)
                .and_then(|value| value.to_str().ok())
                .or(query.account_id.as_deref());
            return by_token(&db, &Arc::<RateLimiter>::from_ref(state), token, account).await;
        }
        by_session(parts, &db).await
    }
}

async fn by_token(
    db: &PgPool,
    limiter: &RateLimiter,
    token: &str,
    account: Option<&str>,
) -> Result<AuthUser, ApiError> {
    let caller = ops::api_tokens::authenticate(db, token)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::unauthorized("The token is unknown, revoked or expired"))?;
    let Channel::Token { token_id } = caller.channel else {
        return Err(ApiError::internal(
            "a token authenticated as another channel",
        ));
    };
    // Counted before the account check, so a client that keeps naming the
    // wrong account is limited like any other.
    limiter
        .check(token_id, Instant::now())
        .map_err(|wait| ApiError::too_many_requests(retry_after_seconds(wait)))?;
    if let Some(account) = account {
        let expected = ops::api_tokens::fetch_harvest_account_id(db, caller.org_id)
            .await
            .map_err(ApiError::internal)?;
        if account.trim().parse::<i64>().ok() != Some(expected) {
            return Err(ApiError::forbidden(
                "Harvest-Account-Id does not name this token's account",
            ));
        }
    }
    Ok(AuthUser {
        user_id: caller.user_id,
        org_id: caller.org_id,
        channel: caller.channel,
    })
}

/// The web session's reads (spec 021 D-7). Requests without a token are not
/// rate limited.
async fn by_session(parts: &Parts, db: &PgPool) -> Result<AuthUser, ApiError> {
    // Extract the session from request extensions (set by SessionManagerLayer).
    let session = parts
        .extensions
        .get::<tower_sessions::Session>()
        .cloned()
        .ok_or_else(|| ApiError::unauthorized("No session"))?;

    let user_id = horae_api::session::get_session_user_id(&session)
        .await
        .ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

    // Look up the user's org. The `active` filter is what revokes a
    // deactivated user's still-live session (FR-002).
    let org_id = sqlx::query_scalar!(
        "SELECT org_id FROM users WHERE id = $1 AND active = true",
        user_id
    )
    .fetch_optional(db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::unauthorized("User not found"))?;

    Ok(AuthUser {
        user_id,
        org_id,
        channel: Channel::Session,
    })
}
