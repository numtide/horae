//! `POST /oauth/revoke` (RFC 7009): an app ends the grant of one of its
//! tokens. Unknown, expired, revoked and other apps' tokens answer the same
//! 200, so the endpoint reveals nothing (§2.2).

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use super::OAuthState;
use super::client_auth::{self, error, no_cache};
use crate::ops::oauth::tokens::{self, TokenError};

pub async fn revoke(State(state): State<OAuthState>, headers: HeaderMap, body: Bytes) -> Response {
    let (form, client, basic) = match client_auth::read(&headers, &body) {
        Ok(read) => read,
        Err(refusal) => return refusal.into_response(),
    };
    let Some(token) = form.get("token") else {
        return error(TokenError::InvalidRequest("token is required"), basic);
    };
    let mut tx = match tokens::begin(&state.pool).await {
        Ok(tx) => tx,
        Err(storage) => return error(TokenError::Storage(storage), basic),
    };
    let result = tokens::revoke_token(&mut tx, &client, token).await;
    match tokens::finish(tx, result).await {
        Ok(()) => {
            let mut response = StatusCode::OK.into_response();
            no_cache(&mut response);
            response
        }
        Err(refused) => error(refused, basic),
    }
}

#[cfg(test)]
mod tests;
