//! Which scope an OAuth access token needs for each operation (research R-7).
//! The map is read from the published document, so the scopes a client sees
//! there are the ones enforced: an operation's `{"oauth2": ["<scope>"]}`
//! requirement names its scope, `{"oauth2": []}` takes any scope, and an
//! operation with no `oauth2` requirement refuses OAuth tokens (FR-062).

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::extract::{MatchedPath, Request};
use axum::http::{HeaderValue, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use horae_core::permissions::oauth::Scope;
use serde_json::Value;

use crate::ops::{Caller, Channel};
use crate::problem::Problem;

/// What an operation's `oauth2` requirement asks of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Required {
    /// Any granted scope, for who the token acts as (FR-061).
    Any,
    Scope(Scope),
}

/// The scope requirement of every operation that accepts OAuth tokens, by
/// method and path template as the document spells them (`/me`,
/// `/time_entries/{id}`).
pub(crate) fn requirements() -> &'static HashMap<(Method, String), Required> {
    static MAP: OnceLock<HashMap<(Method, String), Required>> = OnceLock::new();
    MAP.get_or_init(|| {
        let document = serde_json::to_value(super::document()).expect("the document serializes");
        let mut map = HashMap::new();
        for (path, methods) in document["paths"].as_object().into_iter().flatten() {
            for (method, operation) in methods.as_object().into_iter().flatten() {
                let Some(scopes) = operation["security"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find_map(|requirement| requirement.get("oauth2"))
                    .and_then(Value::as_array)
                else {
                    continue;
                };
                let required = match &scopes[..] {
                    [] => Required::Any,
                    [scope] => Required::Scope(
                        scope
                            .as_str()
                            .and_then(Scope::parse)
                            .unwrap_or_else(|| panic!("{method} {path}: unknown scope {scope}")),
                    ),
                    _ => panic!("{method} {path} names more than one scope"),
                };
                let method = Method::from_bytes(method.to_uppercase().as_bytes())
                    .unwrap_or_else(|_| panic!("{method} {path}: not a method"));
                map.insert((method, path.clone()), required);
            }
        }
        map
    })
}

/// Middleware after [`super::authenticate`]: an OAuth caller reaches the
/// operation only if its token holds the scope the operation declares, and
/// then with its actor unlocked for it (`Caller::unlock`). Otherwise the
/// answer is 403 `insufficient_scope`. Personal access tokens pass.
pub(crate) async fn require(mut request: Request, next: Next) -> Response {
    let Some(caller) = request.extensions().get::<Caller>().copied() else {
        return next.run(request).await;
    };
    let Channel::OAuth { scopes, .. } = caller.channel else {
        return next.run(request).await;
    };
    // A GET route also answers HEAD.
    let method = match request.method() {
        &Method::HEAD => Method::GET,
        method => method.clone(),
    };
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().trim_start_matches("/api/v1").to_owned())
        .unwrap_or_default();
    let required = requirements().get(&(method, path)).copied();
    let scope = match required {
        Some(Required::Scope(scope)) => Some(scope),
        Some(Required::Any) => scopes.normalize().iter().next(),
        None => None,
    };
    match scope.and_then(|scope| caller.unlock(scope)) {
        Some(unlocked) => {
            request.extensions_mut().insert(unlocked);
            next.run(request).await
        }
        None => insufficient_scope(match required {
            Some(Required::Scope(scope)) => Some(scope),
            _ => None,
        }),
    }
}

/// 403 `insufficient_scope`, naming the scope that would do when there is
/// one (RFC 6750 §3.1).
fn insufficient_scope(needed: Option<Scope>) -> Response {
    let problem = Problem::new(
        StatusCode::FORBIDDEN,
        "insufficient_scope",
        Some("The token's scopes do not cover this request".into()),
    );
    let challenge = match needed {
        Some(scope) => format!(
            r#"Bearer error="insufficient_scope", scope="{}""#,
            scope.as_str()
        ),
        None => r#"Bearer error="insufficient_scope""#.to_owned(),
    };
    let mut response = problem.into_response();
    if let Ok(value) = HeaderValue::from_str(&challenge) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    response
}
