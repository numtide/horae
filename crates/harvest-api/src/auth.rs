//! Who a Harvest API request acts as (spec 021 R-6, spec 022 FR-062, spec 024
//! FR-071): a personal access token from `Authorization: Bearer` or the
//! `access_token` query parameter, which Harvest clients send, an app's OAuth
//! access token from the header only (D-5), else the web session cookie.
//! Tokens are the ones `/api/v1` takes, and their requests count against the
//! same limit there and here.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::{FromRef, FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{Method, header};
use horae_api::rate_limit::{RateLimiter, retry_after_seconds};
use horae_core::permissions::oauth::Scope;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::errors::ApiError;
use crate::ops::{self, Caller, Channel};

const ACCOUNT_HEADER: &str = "harvest-account-id";

/// What an OAuth token needs to read a route (spec 024
/// `contracts/api-changes.md`); every other method needs `time:write`. Each
/// entry of the route table names one, as a request extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteScope {
    /// Who the token acts as: any scope will do (FR-061).
    Any,
    Time,
    Catalog,
}

impl RouteScope {
    /// The scope a request with `method` needs; `None` for any.
    fn needed(self, method: &Method) -> Option<Scope> {
        match self {
            _ if !matches!(*method, Method::GET | Method::HEAD) => Some(Scope::TimeWrite),
            Self::Any => None,
            Self::Time => Some(Scope::TimeRead),
            Self::Catalog => Some(Scope::CatalogRead),
        }
    }
}

/// Extractor that authenticates a Harvest API request.
///
/// It carries only the caller's identity, and for an OAuth token the scope
/// that unlocks its actor. Each reader loads the caller's stored permission
/// state inside its own transaction (see [`super::begin_read`]), so the
/// grants it decides with cannot go stale between this extractor and the
/// rows it returns.
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
        let query_token = query.access_token.as_deref().filter(|t| !t.is_empty());
        let token = match (header_token, query_token) {
            (Some(token), _) => token,
            // D-5: an OAuth token in a URL ends up in logs and histories.
            (None, Some(token)) if token.starts_with(ops::oauth::ACCESS_TOKEN_PREFIX) => {
                return Err(ApiError::invalid_token(
                    "OAuth access tokens go in the Authorization header",
                ));
            }
            (None, Some(token)) => token,
            (None, None) => return by_session(parts, &db).await,
        };
        let account = parts
            .headers
            .get(ACCOUNT_HEADER)
            .and_then(|value| value.to_str().ok())
            .or(query.account_id.as_deref());
        let caller = by_token(&db, &Arc::<RateLimiter>::from_ref(state), token, account).await?;
        let route = parts.extensions.get::<RouteScope>().copied();
        let caller = unlock(caller, route, &parts.method)?;
        Ok(AuthUser {
            user_id: caller.user_id,
            org_id: caller.org_id,
            channel: caller.channel,
        })
    }
}

async fn by_token(
    db: &PgPool,
    limiter: &RateLimiter,
    token: &str,
    account: Option<&str>,
) -> Result<Caller, ApiError> {
    // Each authenticator answers `None` for the other's prefix.
    let found = if token.starts_with(ops::oauth::ACCESS_TOKEN_PREFIX) {
        ops::oauth::tokens::authenticate(db, token).await
    } else {
        ops::api_tokens::authenticate(db, token).await
    };
    let caller = found
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::invalid_token("The token is unknown, revoked or expired"))?;
    // An app's access tokens count against their grant, so refreshing does
    // not start a new window (spec 024 R-14).
    let key = match caller.channel {
        Channel::Token { token_id } => token_id,
        Channel::OAuth { grant_id, .. } => grant_id,
        _ => {
            return Err(ApiError::internal(
                "a token authenticated as another channel",
            ));
        }
    };
    // Counted before the account check, so a client that keeps naming the
    // wrong account is limited like any other.
    limiter
        .check(key, Instant::now())
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
    Ok(caller)
}

/// An OAuth caller reaches the handler only if its token holds the scope the
/// route needs for `method`; its actor then loads unlocked (`Caller::unlock`)
/// before any operation runs. Other callers pass unchanged.
fn unlock(caller: Caller, route: Option<RouteScope>, method: &Method) -> Result<Caller, ApiError> {
    let Channel::OAuth { scopes, .. } = caller.channel else {
        return Ok(caller);
    };
    let route = route.ok_or_else(|| ApiError::internal("a Harvest route names no scope"))?;
    // A grant always holds a scope; the fallback only names one to ask for.
    let scope = route
        .needed(method)
        .or_else(|| scopes.normalize().iter().next())
        .unwrap_or(Scope::TimeRead);
    caller
        .unlock(scope)
        .ok_or_else(|| ApiError::insufficient_scope(scope))
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
