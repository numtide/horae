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

/// Money-looking properties that are deliberately neither a `Money` nor a
/// `Rate`, each with why. Empty: every amount and price uses its one
/// convention.
const NOT_MONEY: &[&str] = &[];

/// The schemas a property named `name`, described as `description`, must
/// refer to: `Rate` for a price per hour, `Money` for an amount, either for a
/// description that only mentions money, and none when it is not money.
fn money_schemas(name: &str, description: &str) -> &'static [&'static str] {
    const RATE_WORDS: &[&str] = &["rate", "price"];
    const AMOUNT_WORDS: &[&str] = &[
        "amount", "cost", "total", "cents", "minor", "fee", "tax", "discount", "subtotal",
        "balance", "money",
    ];
    // An identifier (`tax_id`) is never an amount, whatever it names.
    let identifier = name == "id" || name.ends_with("_id");
    let has = |words: &[&str]| !identifier && name.split('_').any(|word| words.contains(&word));
    if has(RATE_WORDS) {
        &["Rate"]
    } else if has(AMOUNT_WORDS) {
        &["Money"]
    } else if ["money", "cents", "minor unit", "per hour"]
        .iter()
        .any(|phrase| description.to_lowercase().contains(phrase))
    {
        &["Money", "Rate"]
    } else {
        &[]
    }
}

fn refers_to(schema: &Value, target: &str) -> bool {
    match schema {
        Value::Object(fields) => {
            fields.get("$ref") == Some(&json!(format!("#/components/schemas/{target}")))
                || fields.values().any(|child| refers_to(child, target))
        }
        Value::Array(items) => items.iter().any(|child| refers_to(child, target)),
        _ => false,
    }
}

/// Every property under `node` that looks like money, with the path to it,
/// and whether it refers to a schema it must.
fn money_properties(node: &Value, path: &str, found: &mut Vec<(String, bool)>) {
    match node {
        Value::Object(fields) => {
            if let Some(Value::Object(properties)) = fields.get("properties") {
                for (name, schema) in properties {
                    let description = schema["description"].as_str().unwrap_or_default();
                    let allowed = money_schemas(name, description);
                    if !allowed.is_empty() {
                        let conforms = allowed.iter().any(|target| refers_to(schema, target));
                        found.push((format!("{path}.{name}"), conforms));
                    }
                }
            }
            for (key, child) in fields {
                money_properties(child, &format!("{path}/{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                money_properties(child, &format!("{path}/{index}"), found);
            }
        }
        _ => {}
    }
}

/// No endpoint can add a second money convention: in the document, request
/// or response, every price per hour is a `$ref` to `Rate` and every other
/// amount a `$ref` to `Money`.
#[test]
fn every_amount_in_the_document_is_money_or_a_rate() {
    let mut document = serde_json::to_value(document()).unwrap();
    // The two schemas are the one place their fields are spelled out.
    let schemas = document["components"]["schemas"].as_object_mut().unwrap();
    for own in ["Money", "Rate"] {
        schemas.remove(own).expect("the document defines it");
    }
    let mut found = Vec::new();
    money_properties(&document, "", &mut found);
    let stray: Vec<&String> = found
        .iter()
        .filter(|(path, conforms)| {
            !conforms && !NOT_MONEY.iter().any(|allowed| path.ends_with(allowed))
        })
        .map(|(path, _)| path)
        .collect();
    assert!(
        stray.is_empty(),
        "amounts or rates in another convention: {stray:?}"
    );
    for property in [".hourly_rate", ".budget_amount"] {
        assert!(
            found.iter().any(|(path, _)| path.ends_with(property)),
            "the walk finds {property}: {found:?}"
        );
    }
}

#[test]
fn rate_words_ask_for_a_rate_and_amount_words_for_money() {
    assert_eq!(money_schemas("hourly_rate", ""), ["Rate"]);
    assert_eq!(money_schemas("unit_price", ""), ["Rate"]);
    assert_eq!(money_schemas("budget_amount", ""), ["Money"]);
    assert_eq!(money_schemas("subtotal", ""), ["Money"]);
    assert_eq!(money_schemas("tax_id", ""), [] as [&str; 0]);
    assert_eq!(money_schemas("note", "In cents"), ["Money", "Rate"]);
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
fn the_document_offers_oauth_with_relative_urls_and_the_three_scopes() {
    let document = serde_json::to_value(document()).unwrap();
    let scheme = &document["components"]["securitySchemes"]["oauth2"];
    assert_eq!(scheme["type"], "oauth2");
    let flow = &scheme["flows"]["authorizationCode"];
    assert_eq!(flow["authorizationUrl"], "/oauth/authorize");
    assert_eq!(flow["tokenUrl"], "/oauth/token");
    assert_eq!(flow["refreshUrl"], "/oauth/token");
    let scopes: Vec<&String> = flow["scopes"].as_object().unwrap().keys().collect();
    assert_eq!(scopes, ["catalog:read", "time:read", "time:write"]);
}

/// Every token operation takes a personal token, or an OAuth token with the
/// one scope it names (FR-062). Only the document itself is public, and only
/// `/me` takes any scope (FR-061).
#[test]
fn every_token_operation_declares_its_oauth_scope() {
    let document = serde_json::to_value(document()).unwrap();
    for (path, methods) in document["paths"].as_object().unwrap() {
        for (method, operation) in methods.as_object().unwrap() {
            if path == "/openapi.json" {
                continue;
            }
            let security = operation["security"].as_array().unwrap();
            let [token, oauth] = &security[..] else {
                panic!("{method} {path}: {security:?}");
            };
            assert_eq!(token, &json!({ "token": [] }), "{method} {path}");
            let scopes = oauth["oauth2"].as_array().unwrap();
            if path == "/me" {
                assert!(scopes.is_empty(), "/me takes any scope");
            } else {
                assert_eq!(scopes.len(), 1, "{method} {path}: {scopes:?}");
            }
        }
    }
    assert_eq!(super::scopes::requirements().len(), 19);
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
    assert_eq!(operations, 20);
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
        oauth: None,
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
