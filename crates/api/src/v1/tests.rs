//! The published document: its snapshot, what it promises about every
//! operation, and the routes around `/api/v1`.

use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;

use super::document;
use super::testing::{assert_conforms, get, router, send};

const SNAPSHOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi/v1.json");

/// `UPDATE_OPENAPI=1 cargo test -p horae-api openapi` rewrites the snapshot.
#[test]
fn openapi_snapshot_is_current() {
    let generated = serde_json::to_value(document()).unwrap();
    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        let text = serde_json::to_string_pretty(&generated).unwrap();
        std::fs::write(SNAPSHOT, text + "\n").unwrap();
        return;
    }
    let committed: Value = serde_json::from_str(
        &std::fs::read_to_string(SNAPSHOT).expect("crates/api/openapi/v1.json exists"),
    )
    .unwrap();
    assert!(
        committed == generated,
        "crates/api/openapi/v1.json is stale; refresh it with \
         `UPDATE_OPENAPI=1 cargo test -p horae-api openapi` and review the diff"
    );
}

#[test]
fn the_document_is_openapi_3_1_with_bearer_tokens() {
    let document = serde_json::to_value(document()).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["info"]["version"], "1");
    assert_eq!(document["servers"], json!([{ "url": "/api/v1" }]));
    assert_eq!(document["security"], json!([{ "token": [] }]));
    assert_eq!(
        document["components"]["securitySchemes"]["token"],
        json!({
            "type": "http",
            "scheme": "bearer",
            "description": "A personal access token (horae_pat_…) created in personal settings.",
        })
    );
}

#[test]
fn every_operation_is_named_and_answers_errors_as_problems() {
    let document = serde_json::to_value(document()).unwrap();
    let mut operations = 0;
    for (path, methods) in document["paths"].as_object().unwrap() {
        for (method, operation) in methods.as_object().unwrap() {
            operations += 1;
            assert!(
                operation["operationId"].is_string(),
                "{method} {path} has no operationId"
            );
            if path == "/openapi.json" {
                assert_eq!(operation["security"], json!([{}]), "the document is public");
                continue;
            }
            assert_eq!(
                operation["responses"]["default"],
                json!({ "$ref": "#/components/responses/Problem" }),
                "{method} {path}"
            );
        }
    }
    assert_eq!(operations, 12);
}

#[test]
fn assert_conforms_refuses_a_body_the_document_does_not_describe() {
    let document = serde_json::to_value(document()).unwrap();
    let problem = json!({
        "type": "about:blank", "title": "Not Found", "status": 404,
        "detail": "Gone", "code": "not_found", "field": null,
    });
    assert_conforms(&document, "getMe", 404, &problem);
    for field in [json!("minutes"), json!({ "entry": "e1" })] {
        let problem = json!({
            "type": "about:blank", "title": "Unprocessable Entity", "status": 422,
            "detail": "Bad", "code": "invalid", "field": field,
        });
        assert_conforms(&document, "getMe", 422, &problem);
    }
    for wrong in [
        json!({ "type": "about:blank", "title": "Not Found", "status": 404 }),
        json!({ "person": {} }),
        json!(null),
    ] {
        let refused = std::panic::catch_unwind(|| {
            assert_conforms(&document, "getMe", 404, &wrong);
        });
        assert!(refused.is_err(), "accepted {wrong}");
    }
    let refused = std::panic::catch_unwind(|| {
        assert_conforms(&document, "getRunningTimer", 204, &json!({}));
    });
    assert!(refused.is_err(), "a 204 with a body");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_document_is_served_without_a_token(pool: PgPool) {
    let router = router(&pool, 100);
    let (status, headers, body) =
        send(&router, "getOpenApi", get("/api/v1/openapi.json", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "application/json");
    assert_eq!(body, serde_json::to_value(document()).unwrap());
}

fn whole_router(pool: &PgPool) -> axum::Router {
    crate::router(crate::RouterOptions {
        dev_login: false,
        login_assets: crate::LoginAssets {
            favicon_svg: String::new(),
            apple_touch_icon: String::new(),
        },
        v1: super::V1State::new(pool.clone(), &Default::default()),
    })
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn unknown_api_paths_answer_404_with_an_empty_body(pool: PgPool) {
    use tower::ServiceExt;
    let router = whole_router(&pool);
    for (method, uri) in [
        ("GET", "/api/v2/users/me"),
        ("GET", "/api/v2/users/me.json"),
        ("POST", "/api/nothing"),
        ("GET", "/api/v1/nothing"),
        ("GET", "/api/v1"),
    ] {
        let request = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {uri}");
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(body.is_empty(), "{method} {uri}: {body:?}");
    }
    let (status, _, _) = send(&router, "getOpenApi", get("/api/v1/openapi.json", None)).await;
    assert_eq!(status, StatusCode::OK, "v1 is mounted under the fallback");
}
