//! What the token and revocation endpoints share: the form body, client
//! authentication (HTTP Basic or the body, research R-12), the per-client
//! limit, and RFC 6749 §5.2 error bodies.

use std::time::Instant;

use axum::Json;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use super::{OAuthState, no_store, pairs};
use crate::ops::oauth::tokens::{ClientAuth, TokenError};
use crate::rate_limit::retry_after_seconds;

/// A form body whose parameters each appear once.
pub(super) struct Form(Vec<(String, String)>);

impl Form {
    pub(super) fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
            .filter(|value| !value.is_empty())
    }
}

/// A request the endpoint refuses before any operation runs.
pub(super) struct Refusal(Box<Response>);

impl From<Response> for Refusal {
    fn from(response: Response) -> Self {
        Self(Box::new(response))
    }
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        *self.0
    }
}

/// Parses the body, authenticates nothing yet but names the client, and
/// counts the request against that client's limit.
pub(super) fn read(
    state: &OAuthState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(Form, ClientAuth, bool), Refusal> {
    let form_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(';').next().is_some_and(|mime| {
                mime.trim()
                    .eq_ignore_ascii_case("application/x-www-form-urlencoded")
            })
        });
    if !form_type {
        return Err(Refusal::from(error(
            TokenError::InvalidRequest("Send the parameters as application/x-www-form-urlencoded"),
            false,
        )));
    }
    let pairs = pairs(body);
    let repeated = pairs
        .iter()
        .enumerate()
        .any(|(i, (key, _))| pairs[..i].iter().any(|(earlier, _)| earlier == key));
    if repeated {
        return Err(Refusal::from(error(
            TokenError::InvalidRequest("A parameter was sent more than once"),
            false,
        )));
    }
    let form = Form(pairs);
    let basic = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            value
                .split_once(' ')
                .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case("basic"))
        });
    let client = match basic {
        Some(value) => {
            // One way to authenticate per request (RFC 6749 §2.3).
            if form.get("client_secret").is_some() {
                return Err(Refusal::from(error(
                    TokenError::InvalidRequest("Authenticate with Basic or the body, not both"),
                    true,
                )));
            }
            let client = parse_basic(value)
                .ok_or_else(|| Refusal::from(error(TokenError::InvalidClient, true)))?;
            if form
                .get("client_id")
                .is_some_and(|id| id != client.client_id)
            {
                return Err(Refusal::from(error(TokenError::InvalidClient, true)));
            }
            client
        }
        None => ClientAuth {
            client_id: form
                .get("client_id")
                .ok_or_else(|| Refusal::from(error(TokenError::InvalidClient, false)))?
                .to_owned(),
            secret: form.get("client_secret").map(str::to_owned),
        },
    };
    if let Err(wait) = state
        .limiter
        .check(client.client_id.clone(), Instant::now())
    {
        return Err(Refusal::from(too_many_requests(wait)));
    }
    Ok((form, client, basic.is_some()))
}

/// `Basic base64(urlencode(id) ":" urlencode(secret))` (RFC 6749 §2.3.1).
fn parse_basic(value: &str) -> Option<ClientAuth> {
    let (_, encoded) = value.split_once(' ')?;
    let decoded = String::from_utf8(STANDARD.decode(encoded.trim()).ok()?).ok()?;
    let (id, secret) = decoded.split_once(':')?;
    let decode = |part: &str| {
        url::form_urlencoded::parse(format!("v={part}").as_bytes())
            .next()
            .map(|(_, value)| value.into_owned())
    };
    let client_id = decode(id).filter(|id| !id.is_empty())?;
    let secret = decode(secret)?;
    Some(ClientAuth {
        client_id,
        // A public app authenticating with Basic sends an empty secret.
        secret: (!secret.is_empty()).then_some(secret),
    })
}

/// The JSON error of RFC 6749 §5.2. A failed Basic authentication asks for
/// Basic again; nothing in the body comes from the request.
pub(super) fn error(error: TokenError, basic: bool) -> Response {
    let (status, code, description) = match error {
        TokenError::InvalidRequest(description) => (
            StatusCode::BAD_REQUEST,
            "invalid_request",
            Some(description),
        ),
        TokenError::InvalidClient => (
            StatusCode::UNAUTHORIZED,
            "invalid_client",
            Some("Client authentication failed"),
        ),
        TokenError::InvalidGrant | TokenError::Reused => (
            StatusCode::BAD_REQUEST,
            "invalid_grant",
            Some("The code or refresh token is invalid, expired or already used"),
        ),
        TokenError::InvalidScope => (
            StatusCode::BAD_REQUEST,
            "invalid_scope",
            Some("The requested scope exceeds what was granted"),
        ),
        TokenError::Storage(error) => {
            tracing::error!(%error, "OAuth token endpoint failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "server_error", None)
        }
    };
    let mut body = json!({ "error": code });
    if let Some(description) = description {
        body["error_description"] = description.into();
    }
    let mut response = (status, Json(body)).into_response();
    if status == StatusCode::UNAUTHORIZED && basic {
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static(r#"Basic realm="horae""#),
        );
    }
    no_cache(&mut response);
    response
}

/// `unsupported_grant_type`, which no operation decides.
pub(super) fn unsupported_grant_type() -> Response {
    let mut response = (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": "unsupported_grant_type",
            "error_description": "Use authorization_code or refresh_token",
        })),
    )
        .into_response();
    no_cache(&mut response);
    response
}

/// `Cache-Control: no-store` and `Pragma: no-cache` (RFC 6749 §5.1).
pub(super) fn no_cache(response: &mut Response) {
    no_store(response);
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
}

fn too_many_requests(wait: std::time::Duration) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({
            "error": "invalid_request",
            "error_description": "Too many requests",
        })),
    )
        .into_response();
    response.headers_mut().insert(
        header::RETRY_AFTER,
        HeaderValue::from(retry_after_seconds(wait)),
    );
    no_cache(&mut response);
    response
}
