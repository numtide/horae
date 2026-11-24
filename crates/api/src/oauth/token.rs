//! `POST /oauth/token`: the `authorization_code` and `refresh_token` grants
//! (RFC 6749 §4.1.3 and §6).

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::OAuthState;
use super::client_auth::{self, error, no_cache, unsupported_grant_type};
use crate::ops::oauth::tokens::{self, TokenError};

pub async fn token(State(state): State<OAuthState>, headers: HeaderMap, body: Bytes) -> Response {
    let (form, client, basic) = match client_auth::read(&headers, &body) {
        Ok(read) => read,
        Err(refusal) => return refusal.into_response(),
    };
    let missing = |what| error(TokenError::InvalidRequest(what), basic);
    let mut tx = match tokens::begin(&state.pool).await {
        Ok(tx) => tx,
        Err(storage) => return error(TokenError::Storage(storage), basic),
    };
    let result = match form.get("grant_type") {
        Some("authorization_code") => {
            let (Some(code), Some(redirect_uri), Some(verifier)) = (
                form.get("code"),
                form.get("redirect_uri"),
                form.get("code_verifier"),
            ) else {
                return missing("code, redirect_uri and code_verifier are required");
            };
            tokens::exchange_code(
                &mut tx,
                &state.config,
                &client,
                code,
                redirect_uri,
                verifier,
            )
            .await
        }
        Some("refresh_token") => {
            let Some(refresh_token) = form.get("refresh_token") else {
                return missing("refresh_token is required");
            };
            tokens::refresh(
                &mut tx,
                &state.config,
                &client,
                refresh_token,
                form.get("scope"),
            )
            .await
        }
        Some(_) => return unsupported_grant_type(),
        None => return missing("grant_type is required"),
    };
    match tokens::finish(tx, result).await {
        Ok(set) => {
            let mut response = Json(json!({
                "access_token": set.access_token.expose(),
                "token_type": "Bearer",
                "expires_in": set.expires_in,
                "refresh_token": set.refresh_token.expose(),
                "scope": set.scopes.display(),
            }))
            .into_response();
            no_cache(&mut response);
            response
        }
        Err(refused) => error(refused, basic),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod refresh_tests;
