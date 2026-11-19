//! What `/api/v1` router tests share. [`send`] checks every answer against
//! the published document with [`assert_conforms`] (research R-2), so a
//! response the document does not describe fails the test that produced it.

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

use super::{V1State, document};
use crate::config::{ApiConfig, CursorKey, RateLimit};
use crate::ops::{self, Caller};
use crate::testing::SeedIds;

/// The v1 router with a fixed cursor key and `requests` per minute.
pub fn router(pool: &PgPool, requests: u32) -> Router {
    super::router(state(pool, requests))
}

/// [`router`] whose writes dispatch their events to `events`.
pub fn router_hearing(pool: &PgPool, events: std::sync::Arc<ops::testing::Recorder>) -> Router {
    super::router(V1State {
        events,
        ..state(pool, 1000)
    })
}

fn state(pool: &PgPool, requests: u32) -> V1State {
    V1State::new(
        pool.clone(),
        &ApiConfig {
            cursor_key: CursorKey::new([1; 32]),
            rate_limit: RateLimit {
                requests,
                window: std::time::Duration::from_secs(60),
            },
        },
    )
}

/// A request with a JSON body.
pub fn json(method: &str, uri: &str, token: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

/// A request without a body.
pub fn call(method: &str, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// A new token of the seeded person, by its whole secret.
pub async fn token(pool: &PgPool, ids: &SeedIds, name: &str) -> (String, uuid::Uuid) {
    let mut write = ops::begin(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::create(&mut write.tx, &write.actor, name, None).await;
    let (secret, row) = write.commit(result, &ops::NoEvents).await.unwrap();
    (secret.expose().to_owned(), row.id)
}

pub fn get(uri: &str, token: Option<&str>) -> Request<Body> {
    let mut request = Request::builder().uri(uri);
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    request.body(Body::empty()).unwrap()
}

/// Sends `request` to the operation `operation_id` and checks the answer
/// against the document. The body is `null` when the answer has none.
pub async fn send(
    router: &Router,
    operation_id: &str,
    request: Request<Body>,
) -> (StatusCode, HeaderMap, Value) {
    let response = router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    let document = serde_json::to_value(document()).unwrap();
    assert_conforms(&document, operation_id, status.as_u16(), &body);
    (status, headers, body)
}

/// Fails unless `document` describes `body` as the answer of `operation_id`
/// with `status`: the response for that status, else the default one, and
/// its schema validated as JSON Schema 2020-12 (OpenAPI 3.1), formats
/// included.
pub fn assert_conforms(document: &Value, operation_id: &str, status: u16, body: &Value) {
    let operation = document["paths"]
        .as_object()
        .into_iter()
        .flat_map(|paths| paths.values())
        .filter_map(Value::as_object)
        .flat_map(|methods| methods.values())
        .find(|operation| operation["operationId"] == operation_id)
        .unwrap_or_else(|| panic!("no operation {operation_id} in the document"));
    let responses = &operation["responses"];
    let mut response = responses
        .get(status.to_string())
        .or_else(|| responses.get("default"))
        .unwrap_or_else(|| panic!("{operation_id} documents no {status} answer"));
    if let Some(reference) = response["$ref"].as_str() {
        response = document
            .pointer(reference.trim_start_matches('#'))
            .unwrap_or_else(|| panic!("{reference} does not resolve"));
    }
    let Some(content) = response["content"].as_object() else {
        assert!(
            body.is_null(),
            "{operation_id} {status} documents no body, but answered {body}"
        );
        return;
    };
    let [(_, media)] = content.iter().collect::<Vec<_>>()[..] else {
        panic!("{operation_id} {status} documents several media types");
    };
    let Some(schema) = media.get("schema") else {
        return;
    };
    let root = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "allOf": [schema],
        "components": document["components"],
    });
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&root)
        .unwrap_or_else(|error| panic!("{operation_id} {status}: bad schema: {error}"));
    let errors: Vec<String> = validator
        .iter_errors(body)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{operation_id} {status} does not conform:\n{}\nbody: {body}",
        errors.join("\n")
    );
}
