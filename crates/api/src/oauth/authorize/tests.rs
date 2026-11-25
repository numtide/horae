//! `GET /oauth/authorize`: the checks of contracts/oauth-endpoints.md in
//! order, which errors stay on Horae's page and which go back to the app,
//! the headers, and the request kept in the session.

use axum::http::{StatusCode, header};
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use sqlx::PgPool;

use crate::oauth::pending;
use crate::oauth::testing::{Answer, Harness, param, query};
use crate::ops::oauth::testing::{CHALLENGE, ISSUER, REDIRECT_URI, register};

/// An authorization request of `client_id` with the parameters a correct
/// app sends, `overrides` replacing or (with an empty value) removing them.
fn uri(client_id: &str, overrides: &[(&str, &str)]) -> String {
    let mut params = vec![
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", REDIRECT_URI),
        ("code_challenge", CHALLENGE),
        ("code_challenge_method", "S256"),
        ("state", "af0ifjsldkj"),
    ];
    for (name, value) in overrides {
        params.retain(|(key, _)| key != name);
        if !value.is_empty() {
            params.push((name, value));
        }
    }
    let query: String = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params)
        .finish();
    format!("/oauth/authorize?{query}")
}

fn assert_page_error(answer: &Answer, reason: &str) {
    assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{}", answer.body);
    assert!(answer.header(header::LOCATION).is_none());
    assert!(
        answer.body.contains("This app can’t connect to Horae"),
        "{}",
        answer.body
    );
    assert!(answer.body.contains(reason), "{}", answer.body);
}

/// A redirect back to the app with `error`, the app's `state` and `iss`.
fn assert_redirect_error(answer: &Answer, error: &str) {
    assert_eq!(answer.status, StatusCode::FOUND, "{}", answer.body);
    let location = answer.location();
    assert!(location.as_str().starts_with(REDIRECT_URI), "{location}");
    assert_eq!(
        param(&location, "error").as_deref(),
        Some(error),
        "{location}"
    );
    assert!(param(&location, "error_description").is_some());
    assert_eq!(param(&location, "state").as_deref(), Some("af0ifjsldkj"));
    assert_eq!(param(&location, "iss").as_deref(), Some(ISSUER));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_valid_request_waits_in_the_session_for_the_approval_page(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let answer = h.get(&uri(&h.app.client_id, &[])).await;
    assert_eq!(answer.status, StatusCode::FOUND);
    let location = answer.location();
    assert_eq!(location.path(), "/oauth/consent");
    let id = param(&location, "request").unwrap();
    let request = pending::get(&h.session, &id).await.unwrap().unwrap();
    assert_eq!(request.app_id, h.app.id);
    assert_eq!(request.redirect_uri, REDIRECT_URI);
    assert_eq!(request.code_challenge, CHALLENGE);
    assert_eq!(request.state.as_deref(), Some("af0ifjsldkj"));
    // No scope asks for every scope the app is registered for.
    assert_eq!(request.scope, "time:read time:write catalog:read");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_answer_is_uncacheable_and_cannot_be_framed(pool: PgPool) {
    let h = Harness::new(&pool).await;
    for answer in [
        h.get(&uri(&h.app.client_id, &[])).await,
        h.get(&uri("horae_app_unknown", &[])).await,
        h.get(&uri(&h.app.client_id, &[("response_type", "token")]))
            .await,
    ] {
        assert_eq!(answer.header(header::CACHE_CONTROL), Some("no-store"));
        assert_eq!(answer.header(header::X_FRAME_OPTIONS), Some("DENY"));
        assert_eq!(
            answer.header(header::CONTENT_SECURITY_POLICY),
            Some("frame-ancestors 'none'")
        );
        assert_eq!(answer.header(header::REFERRER_POLICY), Some("no-referrer"));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_untrusted_client_or_redirect_uri_is_shown_on_horaes_page(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let id = h.app.client_id.as_str();
    assert_page_error(
        &h.get(&uri("", &[("client_id", "")])).await,
        "doesn’t name an app",
    );
    assert_page_error(
        &h.get(&uri("horae_app_unknown", &[])).await,
        "isn’t registered",
    );
    // The built-in Android client starts disabled (D-1).
    assert_page_error(
        &h.get(&uri(
            "horae-android",
            &[("redirect_uri", "dev.horae.android:/oauth/callback")],
        ))
        .await,
        "turned this app off",
    );
    assert_page_error(
        &h.get(&uri(id, &[("redirect_uri", "")])).await,
        "doesn’t say where",
    );
    for unregistered in [
        "com.example.app:/callback/more",
        "com.example.app:/call",
        "COM.EXAMPLE.APP:/callback",
        "com.example.app:/callback?x=1",
        "com.example.app:/callback#x",
        "https://evil.example/callback",
        "http://localhost/callback",
        "http://127.0.0.1/other",
    ] {
        assert_page_error(
            &h.get(&uri(id, &[("redirect_uri", unregistered)])).await,
            "didn’t register",
        );
    }
    // The client cannot be trusted when its id is repeated.
    let repeated = format!("{}&client_id={id}", uri(id, &[]));
    assert_page_error(&h.get(&repeated).await, "doesn’t name an app");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_loopback_redirect_may_use_any_port(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let answer = h
        .get(&uri(
            &h.app.client_id,
            &[("redirect_uri", "http://127.0.0.1:53682/callback")],
        ))
        .await;
    assert_eq!(answer.status, StatusCode::FOUND);
    assert_eq!(answer.location().path(), "/oauth/consent");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn other_errors_go_back_to_the_app_with_state_and_issuer(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let id = h.app.client_id.as_str();
    for (overrides, error) in [
        (vec![("response_type", "")], "invalid_request"),
        (
            vec![("response_type", "token")],
            "unsupported_response_type",
        ),
        (vec![("code_challenge", "")], "invalid_request"),
        (vec![("code_challenge", "too-short")], "invalid_request"),
        (vec![("code_challenge_method", "")], "invalid_request"),
        (vec![("code_challenge_method", "plain")], "invalid_request"),
        (vec![("scope", "time:read admin")], "invalid_scope"),
    ] {
        assert_redirect_error(&h.get(&uri(id, &overrides)).await, error);
    }
    let repeated = format!("{}&scope=time:read&scope=time:read", uri(id, &[]));
    assert_redirect_error(&h.get(&repeated).await, "invalid_request");
}

/// PKCE is required of a confidential app as of a public one (D-7).
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_confidential_app_needs_an_s256_challenge_too(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let app = register(
        &pool,
        &h.ids,
        "Server app",
        ClientKind::Confidential,
        ScopeSet::of(&[Scope::TimeRead]),
    )
    .await;
    let redirect = "https://app.example.com/callback";
    for overrides in [
        [("code_challenge", ""), ("redirect_uri", redirect)],
        [
            ("code_challenge_method", "plain"),
            ("redirect_uri", redirect),
        ],
    ] {
        let answer = h.get(&uri(&app.client_id, &overrides)).await;
        assert_eq!(answer.status, StatusCode::FOUND);
        let location = answer.location();
        assert!(location.as_str().starts_with(redirect));
        assert_eq!(
            param(&location, "error").as_deref(),
            Some("invalid_request")
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_scope_the_app_is_not_registered_for_is_refused(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let app = register(
        &pool,
        &h.ids,
        "Reader",
        ClientKind::Public,
        ScopeSet::of(&[Scope::TimeRead]),
    )
    .await;
    let answer = h
        .get(&uri(&app.client_id, &[("scope", "time:write")]))
        .await;
    assert_redirect_error(&answer, "invalid_scope");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_state_too_long_is_refused_without_being_echoed(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let long = "s".repeat(1025);
    let answer = h.get(&uri(&h.app.client_id, &[("state", &long)])).await;
    assert_eq!(answer.status, StatusCode::FOUND);
    let location = answer.location();
    assert_eq!(
        param(&location, "error").as_deref(),
        Some("invalid_request")
    );
    assert!(
        param(&location, "state").is_none(),
        "{:?}",
        query(&location)
    );
    assert_eq!(param(&location, "iss").as_deref(), Some(ISSUER));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_session_keeps_the_five_newest_requests(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let mut ids = Vec::new();
    for _ in 0..6 {
        let answer = h.get(&uri(&h.app.client_id, &[])).await;
        ids.push(param(&answer.location(), "request").unwrap());
    }
    assert!(pending::get(&h.session, &ids[0]).await.unwrap().is_none());
    for id in &ids[1..] {
        assert!(pending::get(&h.session, id).await.unwrap().is_some());
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_request_expires_after_ten_minutes(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let answer = h.get(&uri(&h.app.client_id, &[])).await;
    let id = param(&answer.location(), "request").unwrap();
    let mut stored: Vec<(String, crate::ops::oauth::authorize::PendingRequest)> =
        h.session.get(pending::KEY).await.unwrap().unwrap();
    stored[0].1.created_at -= chrono::Duration::minutes(10);
    h.session.insert(pending::KEY, stored).await.unwrap();
    assert!(pending::get(&h.session, &id).await.unwrap().is_none());
    assert!(pending::take(&h.session, &id).await.unwrap().is_none());
}
