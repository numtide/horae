//! `GET /.well-known/oauth-authorization-server` (RFC 8414, FR-095).

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use horae_core::permissions::oauth::Scope;
use serde_json::{Value, json};

use super::{AUTHORIZE, OAuthState, REVOKE, TOKEN};

pub async fn metadata(State(state): State<OAuthState>) -> Response {
    let mut response = Json(document(&state.issuer)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("max-age=3600"),
    );
    response
}

/// The same for every request: built from the issuer and the scope list.
pub(super) fn document(issuer: &str) -> Value {
    let auth_methods = ["none", "client_secret_basic", "client_secret_post"];
    json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}{AUTHORIZE}"),
        "token_endpoint": format!("{issuer}{TOKEN}"),
        "revocation_endpoint": format!("{issuer}{REVOKE}"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        // PKCE S256 only: a client never sees `plain` offered (FR-021).
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": auth_methods,
        "revocation_endpoint_auth_methods_supported": auth_methods,
        "scopes_supported": Scope::ALL.map(Scope::as_str),
        "authorization_response_iss_parameter_supported": true,
        "service_documentation": format!("{issuer}/api/v1/openapi.json"),
    })
}
