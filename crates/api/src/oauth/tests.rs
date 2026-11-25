//! The authorization server as a whole: metadata (FR-095), off without a
//! public URL, and no credential in the logs across a full flow (FR-051).

use std::io::Write;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use horae_core::permissions::oauth::ClientKind;
use sqlx::PgPool;
use tower::ServiceExt;

use super::testing::{Harness, form_request, param};
use crate::ops::oauth::testing::{
    CHALLENGE, REDIRECT_URI, VERIFIER, all_scopes, approve, register,
};

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_metadata_document_lists_only_s256(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let answer = h.get("/.well-known/oauth-authorization-server").await;
    assert_eq!(answer.status, StatusCode::OK);
    assert_eq!(answer.header(header::CACHE_CONTROL), Some("max-age=3600"));
    assert_eq!(
        answer.json(),
        serde_json::json!({
            "issuer": "https://horae.example.com",
            "authorization_endpoint": "https://horae.example.com/oauth/authorize",
            "token_endpoint": "https://horae.example.com/oauth/token",
            "revocation_endpoint": "https://horae.example.com/oauth/revoke",
            "response_types_supported": ["code"],
            "grant_types_supported": ["authorization_code", "refresh_token"],
            "code_challenge_methods_supported": ["S256"],
            "token_endpoint_auth_methods_supported":
                ["none", "client_secret_basic", "client_secret_post"],
            "revocation_endpoint_auth_methods_supported":
                ["none", "client_secret_basic", "client_secret_post"],
            "scopes_supported": ["time:read", "time:write", "catalog:read"],
            "authorization_response_iss_parameter_supported": true,
            "service_documentation": "https://horae.example.com/api/v1/openapi.json"
        })
    );
}

#[tokio::test]
async fn every_path_answers_404_without_a_public_url() {
    let router = super::router(None);
    for (method, path) in [
        ("GET", "/oauth/authorize?client_id=x"),
        ("POST", "/oauth/token"),
        ("POST", "/oauth/revoke"),
        ("GET", "/.well-known/oauth-authorization-server"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
    }
}

/// A `tracing` writer into a shared buffer.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// FR-051: across a whole flow, right and wrong, every credential that
/// appears in the logs stops at its 8-character display prefix.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn no_credential_reaches_the_logs(pool: PgPool) {
    let captured = Captured::default();
    let writer = captured.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let h = Harness::new(&pool).await;
    let confidential = register(
        &pool,
        &h.ids,
        "Server app",
        ClientKind::Confidential,
        all_scopes(),
    )
    .await;
    let secret = confidential.secret.clone().unwrap();

    let query: String = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs([
            ("response_type", "code"),
            ("client_id", h.app.client_id.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("code_challenge", CHALLENGE),
            ("code_challenge_method", "S256"),
            ("state", "s"),
        ])
        .finish();
    let answer = h.get(&format!("/oauth/authorize?{query}")).await;
    let id = param(&answer.location(), "request").unwrap();
    let request = super::pending::take(&h.session, &id)
        .await
        .unwrap()
        .unwrap();
    let (_, code) = approve(&pool, &h.ids, &request).await;

    let exchange = [
        ("grant_type", "authorization_code"),
        ("client_id", h.app.client_id.as_str()),
        ("code", code.as_str()),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", VERIFIER),
    ];
    let tokens = h.post("/oauth/token", &exchange).await.json();
    let refresh = tokens["refresh_token"].as_str().unwrap();
    let access = tokens["access_token"].as_str().unwrap();
    let refresh_form = [
        ("grant_type", "refresh_token"),
        ("client_id", h.app.client_id.as_str()),
        ("refresh_token", refresh),
    ];
    h.post("/oauth/token", &refresh_form).await;
    // Reuse of the code and of the refresh token, and a wrong secret.
    h.post("/oauth/token", &exchange).await;
    h.post("/oauth/token", &refresh_form).await;
    h.send(form_request(
        "/oauth/token",
        &refresh_form,
        Some((&confidential.client_id, &format!("{secret}x"))),
    ))
    .await;
    h.post(
        "/oauth/revoke",
        &[("client_id", h.app.client_id.as_str()), ("token", access)],
    )
    .await;
    drop(_guard);

    let logs = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(!logs.is_empty(), "the flow logged nothing to check");
    for secret in [code.as_str(), access, refresh, secret.as_str()] {
        assert!(!logs.contains(secret), "a credential was logged");
    }
    for prefix in ["horae_oac_", "horae_oat_", "horae_ort_", "horae_ocs_"] {
        for (at, _) in logs.match_indices(prefix) {
            let rest = &logs[at + prefix.len()..];
            let run = rest
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(byte))
                .count();
            assert!(run <= 8, "{prefix} followed by {run} characters");
        }
    }
}
