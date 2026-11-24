//! `POST /oauth/token` with `grant_type=authorization_code` (FR-030 to
//! FR-042), client authentication (research R-12) and the per-client limit
//! (FR-081).

use axum::http::{StatusCode, header};
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use sqlx::PgPool;
use uuid::Uuid;

use crate::oauth::testing::{Answer, Harness, form_request};
use crate::ops::oauth::testing::{
    App, CHALLENGE, REDIRECT_URI, VERIFIER, all_scopes, approve, audit_actions, pending, register,
};

const CONFIDENTIAL_REDIRECT: &str = "https://app.example.com/callback";

/// `ids`' person allows a request of `app` to `redirect_uri`.
async fn code_for(h: &Harness, app: &App, redirect_uri: &str, scopes: ScopeSet) -> (Uuid, String) {
    approve(&h.pool, &h.ids, &pending(app, redirect_uri, scopes)).await
}

fn exchange_form<'a>(client_id: &'a str, code: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", VERIFIER),
    ]
}

fn confidential_form<'a>(client_id: &'a str, code: &'a str) -> Vec<(&'a str, &'a str)> {
    with(
        exchange_form(client_id, code),
        "redirect_uri",
        CONFIDENTIAL_REDIRECT,
    )
}

fn with<'a>(
    mut form: Vec<(&'a str, &'a str)>,
    name: &'a str,
    value: &'a str,
) -> Vec<(&'a str, &'a str)> {
    form.retain(|(key, _)| *key != name);
    if !value.is_empty() {
        form.push((name, value));
    }
    form
}

fn assert_error(answer: &Answer, status: StatusCode, error: &str) {
    assert_eq!(answer.status, status, "{}", answer.body);
    assert_eq!(answer.error(), error, "{}", answer.body);
    assert_eq!(answer.header(header::CACHE_CONTROL), Some("no-store"));
}

async fn grant_revoked(pool: &PgPool, grant_id: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT revoke_reason::text FROM oauth_grants WHERE id = $1",
        grant_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_code_with_its_verifier_gives_the_first_tokens(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let scopes = ScopeSet::of(&[Scope::TimeRead, Scope::CatalogRead]);
    let (_, code) = code_for(&h, &h.app, REDIRECT_URI, scopes).await;
    let answer = h
        .post("/oauth/token", &exchange_form(&h.app.client_id, &code))
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.header(header::CACHE_CONTROL), Some("no-store"));
    assert_eq!(answer.header(header::PRAGMA), Some("no-cache"));
    let body = answer.json();
    assert!(
        body["access_token"]
            .as_str()
            .unwrap()
            .starts_with("horae_oat_")
    );
    assert!(
        body["refresh_token"]
            .as_str()
            .unwrap()
            .starts_with("horae_ort_")
    );
    assert_eq!(body["token_type"], "Bearer");
    assert_eq!(body["expires_in"], 3600);
    assert_eq!(body["scope"], "time:read catalog:read");
    let mut keys: Vec<&String> = body.as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "access_token",
            "expires_in",
            "refresh_token",
            "scope",
            "token_type"
        ]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_verifier_must_match_the_challenge(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, code) = code_for(&h, &h.app, REDIRECT_URI, all_scopes()).await;
    let form = exchange_form(&h.app.client_id, &code);
    // Well formed, but another flow's verifier.
    let other = "x".repeat(43);
    assert_error(
        &h.post("/oauth/token", &with(form.clone(), "code_verifier", &other))
            .await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
    assert_error(
        &h.post("/oauth/token", &with(form.clone(), "code_verifier", ""))
            .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    // Too short to be a verifier at all; the challenge itself is not one.
    assert_error(
        &h.post(
            "/oauth/token",
            &with(form.clone(), "code_verifier", "short"),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    assert_error(
        &h.post(
            "/oauth/token",
            &with(form.clone(), "code_verifier", CHALLENGE),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
    // Failed attempts do not use the code up, and revoke nothing.
    assert_eq!(h.post("/oauth/token", &form).await.status, StatusCode::OK);
    assert_eq!(grant_revoked(&pool, grant_id).await, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_code_is_bound_to_its_client_and_redirect_uri(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let other = register(&pool, &h.ids, "Other", ClientKind::Public, all_scopes()).await;
    let (_, code) = code_for(&h, &h.app, "http://127.0.0.1:5000/callback", all_scopes()).await;
    let form = with(
        exchange_form(&h.app.client_id, &code),
        "redirect_uri",
        "http://127.0.0.1:5000/callback",
    );
    assert_error(
        &h.post(
            "/oauth/token",
            &with(form.clone(), "client_id", &other.client_id),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
    // Another port of the same loopback URI is another redirect URI here.
    assert_error(
        &h.post(
            "/oauth/token",
            &with(
                form.clone(),
                "redirect_uri",
                "http://127.0.0.1:5001/callback",
            ),
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
    assert_eq!(h.post("/oauth/token", &form).await.status, StatusCode::OK);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_expired_code_is_refused(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, code) = code_for(&h, &h.app, REDIRECT_URI, all_scopes()).await;
    sqlx::query!(
        "UPDATE oauth_codes SET expires_at = now() WHERE grant_id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_error(
        &h.post("/oauth/token", &exchange_form(&h.app.client_id, &code))
            .await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
}

/// FR-031: the second exchange fails and ends every token of the first.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_replayed_code_revokes_its_grant(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, code) = code_for(&h, &h.app, REDIRECT_URI, all_scopes()).await;
    let form = exchange_form(&h.app.client_id, &code);
    let first = h.post("/oauth/token", &form).await.json();
    assert_error(
        &h.post("/oauth/token", &form).await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
    assert_eq!(
        grant_revoked(&pool, grant_id).await.as_deref(),
        Some("code_reuse")
    );
    assert_eq!(
        audit_actions(&pool, grant_id).await,
        ["grant_approved", "code_reused"]
    );
    let access = first["access_token"].as_str().unwrap();
    assert!(
        crate::ops::oauth::tokens::authenticate(&pool, access)
            .await
            .unwrap()
            .is_none()
    );
    let refresh = h
        .post(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", &h.app.client_id),
                ("refresh_token", first["refresh_token"].as_str().unwrap()),
            ],
        )
        .await;
    assert_error(&refresh, StatusCode::BAD_REQUEST, "invalid_grant");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_public_client_sends_no_secret(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (_, code) = code_for(&h, &h.app, REDIRECT_URI, all_scopes()).await;
    let form = exchange_form(&h.app.client_id, &code);
    assert_error(
        &h.post(
            "/oauth/token",
            &with(form.clone(), "client_secret", "horae_ocs_guess"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    );
    let basic = h
        .send(form_request(
            "/oauth/token",
            &form,
            Some((&h.app.client_id, "horae_ocs_guess")),
        ))
        .await;
    assert_error(&basic, StatusCode::UNAUTHORIZED, "invalid_client");
    assert_eq!(
        basic.header(header::WWW_AUTHENTICATE),
        Some(r#"Basic realm="horae""#)
    );
    assert_error(
        &h.post("/oauth/token", &with(form.clone(), "client_id", ""))
            .await,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    );
    assert_error(
        &h.post(
            "/oauth/token",
            &with(form.clone(), "client_id", "horae_app_unknown"),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    );
    // A public app may name itself through Basic with an empty secret.
    let answer = h
        .send(form_request(
            "/oauth/token",
            &form,
            Some((&h.app.client_id, "")),
        ))
        .await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_confidential_client_authenticates_with_its_secret(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let app = register(
        &pool,
        &h.ids,
        "Server app",
        ClientKind::Confidential,
        all_scopes(),
    )
    .await;
    let secret = app.secret.clone().unwrap();
    let wrong = format!("{}x", &secret[..secret.len() - 1]);
    let mut codes = Vec::new();
    for _ in 0..4 {
        codes.push(
            code_for(&h, &app, CONFIDENTIAL_REDIRECT, all_scopes())
                .await
                .1,
        );
    }
    let id = app.client_id.as_str();
    let exchange = |i: usize| confidential_form(id, &codes[i]);

    assert_error(
        &h.post("/oauth/token", &exchange(0)).await,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    );
    assert_error(
        &h.post("/oauth/token", &with(exchange(0), "client_secret", &wrong))
            .await,
        StatusCode::UNAUTHORIZED,
        "invalid_client",
    );
    let wrong_basic = h
        .send(form_request(
            "/oauth/token",
            &exchange(0),
            Some((&app.client_id, &wrong)),
        ))
        .await;
    assert_error(&wrong_basic, StatusCode::UNAUTHORIZED, "invalid_client");
    assert!(wrong_basic.header(header::WWW_AUTHENTICATE).is_some());

    let post = with(exchange(1), "client_secret", &secret);
    assert_eq!(h.post("/oauth/token", &post).await.status, StatusCode::OK);
    let basic = h
        .send(form_request(
            "/oauth/token",
            &exchange(2),
            Some((&app.client_id, &secret)),
        ))
        .await;
    assert_eq!(basic.status, StatusCode::OK, "{}", basic.body);
    // Basic and a secret in the body at once is one method too many.
    let both = with(exchange(3), "client_secret", &secret);
    let answer = h
        .send(form_request(
            "/oauth/token",
            &both,
            Some((&app.client_id, &secret)),
        ))
        .await;
    assert_error(&answer, StatusCode::BAD_REQUEST, "invalid_request");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn malformed_requests_are_invalid(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let id = h.app.client_id.as_str();
    assert_error(
        &h.post("/oauth/token", &[("client_id", id)]).await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    let answer = h
        .post(
            "/oauth/token",
            &[("grant_type", "password"), ("client_id", id)],
        )
        .await;
    assert_error(&answer, StatusCode::BAD_REQUEST, "unsupported_grant_type");
    assert_error(
        &h.post(
            "/oauth/token",
            &[("grant_type", "authorization_code"), ("client_id", id)],
        )
        .await,
        StatusCode::BAD_REQUEST,
        "invalid_request",
    );
    let repeated = h
        .post(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("grant_type", "refresh_token"),
                ("client_id", id),
            ],
        )
        .await;
    assert_error(&repeated, StatusCode::BAD_REQUEST, "invalid_request");
    let json = h
        .send(
            axum::http::Request::post("/oauth/token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(r#"{"grant_type":"refresh_token"}"#))
                .unwrap(),
        )
        .await;
    assert_error(&json, StatusCode::BAD_REQUEST, "invalid_request");
}

/// A client id longer than any Horae issues is refused before the limiter
/// sees it, so it neither counts nor adds a key.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_over_long_client_id_is_refused_before_counting(pool: PgPool) {
    let h = Harness::with_limit(&pool, 1).await;
    let long = "x".repeat(65);
    let form = [
        ("grant_type", "refresh_token"),
        ("client_id", long.as_str()),
        ("refresh_token", "horae_ort_guess"),
    ];
    for _ in 0..3 {
        let answer = h.post("/oauth/token", &form).await;
        assert_error(&answer, StatusCode::UNAUTHORIZED, "invalid_client");
    }
    let basic = h
        .send(form_request(
            "/oauth/token",
            &[("grant_type", "refresh_token"), ("refresh_token", "x")],
            Some((&long, "")),
        ))
        .await;
    assert_error(&basic, StatusCode::UNAUTHORIZED, "invalid_client");
    // The unknown-client window was never used: one request still fits.
    let unknown = h
        .post(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", "horae_app_unknown"),
                ("refresh_token", "x"),
            ],
        )
        .await;
    assert_error(&unknown, StatusCode::UNAUTHORIZED, "invalid_client");
}

/// Ids that name no app share one window, so made-up ids cannot each get
/// their own; a registered app keeps its own window.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn ids_that_name_no_app_share_one_window(pool: PgPool) {
    let h = Harness::with_limit(&pool, 2).await;
    let guess = |id: &'static str| {
        [
            ("grant_type", "refresh_token"),
            ("client_id", id),
            ("refresh_token", "x"),
        ]
    };
    assert_eq!(
        h.post("/oauth/token", &guess("horae_app_a")).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.post("/oauth/token", &guess("horae_app_b")).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        h.post("/oauth/token", &guess("horae_app_c")).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    let own = [
        ("grant_type", "refresh_token"),
        ("client_id", h.app.client_id.as_str()),
        ("refresh_token", "horae_ort_guess"),
    ];
    assert_error(
        &h.post("/oauth/token", &own).await,
        StatusCode::BAD_REQUEST,
        "invalid_grant",
    );
}

/// FR-081: the 31st request in a window is refused; another client is not.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_token_endpoint_is_limited_per_client(pool: PgPool) {
    let h = Harness::with_limit(&pool, 30).await;
    let form = [
        ("grant_type", "refresh_token"),
        ("client_id", h.app.client_id.as_str()),
        ("refresh_token", "horae_ort_guess"),
    ];
    for _ in 0..30 {
        assert_eq!(
            h.post("/oauth/token", &form).await.status,
            StatusCode::BAD_REQUEST
        );
    }
    let refused = h.post("/oauth/token", &form).await;
    assert_eq!(refused.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(refused.header(header::RETRY_AFTER).is_some());
    assert_eq!(
        refused.json(),
        serde_json::json!({"error": "invalid_request", "error_description": "Too many requests"})
    );
    // The revocation endpoint counts against the same window.
    let revoke = h
        .post(
            "/oauth/revoke",
            &[("client_id", h.app.client_id.as_str()), ("token", "x")],
        )
        .await;
    assert_eq!(revoke.status, StatusCode::TOO_MANY_REQUESTS);
    let other = [
        ("grant_type", "refresh_token"),
        ("client_id", "horae-android"),
        ("refresh_token", "horae_ort_guess"),
    ];
    assert_ne!(
        h.post("/oauth/token", &other).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
}
