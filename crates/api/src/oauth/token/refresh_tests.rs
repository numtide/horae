//! `POST /oauth/token` with `grant_type=refresh_token`: rotation and reuse
//! (research R-8, FR-043), expiry (FR-044), narrowing (FR-045) and what ends
//! a grant (FR-052).

use axum::http::StatusCode;
use horae_core::permissions::oauth::{Scope, ScopeSet};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::oauth::testing::{Answer, Harness, form_request};
use crate::ops::oauth::testing::{
    App, REDIRECT_URI, all_scopes, approve, audit_actions, exchange, pending,
};
use crate::ops::oauth::tokens::{TokenSet, authenticate};

async fn signed_in(h: &Harness, scopes: ScopeSet) -> (Uuid, TokenSet) {
    let (grant_id, code) = approve(&h.pool, &h.ids, &pending(&h.app, REDIRECT_URI, scopes)).await;
    (
        grant_id,
        exchange(&h.pool, &h.app, &code, REDIRECT_URI)
            .await
            .unwrap(),
    )
}

fn refresh_form<'a>(app: &'a App, token: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("grant_type", "refresh_token"),
        ("client_id", app.client_id.as_str()),
        ("refresh_token", token),
    ]
}

async fn refresh(h: &Harness, token: &str) -> Answer {
    h.post("/oauth/token", &refresh_form(&h.app, token)).await
}

fn assert_invalid_grant(answer: &Answer) {
    assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{}", answer.body);
    assert_eq!(answer.error(), "invalid_grant");
}

async fn revoke_reason(pool: &PgPool, grant_id: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT revoke_reason::text FROM oauth_grants WHERE id = $1",
        grant_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn works(pool: &PgPool, access_token: &str) -> bool {
    authenticate(pool, access_token).await.unwrap().is_some()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refresh_token_rotates(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    let answer = refresh(&h, tokens.refresh_token.expose()).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let body = answer.json();
    let next = body["refresh_token"].as_str().unwrap();
    assert_ne!(next, tokens.refresh_token.expose());
    assert_eq!(body["scope"], "time:read time:write catalog:read");
    assert!(works(&pool, body["access_token"].as_str().unwrap()).await);
    // The access token it replaced still works until it expires.
    assert!(works(&pool, tokens.access_token.expose()).await);
    assert_eq!(refresh(&h, next).await.status, StatusCode::OK);
    assert_eq!(revoke_reason(&pool, grant_id).await, None);
}

/// FR-043: presenting a used refresh token ends the whole grant.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_replayed_refresh_token_revokes_the_grant(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    let next = refresh(&h, tokens.refresh_token.expose()).await.json();
    assert_invalid_grant(&refresh(&h, tokens.refresh_token.expose()).await);
    assert_eq!(
        revoke_reason(&pool, grant_id).await.as_deref(),
        Some("refresh_reuse")
    );
    assert_eq!(
        audit_actions(&pool, grant_id).await,
        ["grant_approved", "refresh_reused"]
    );
    assert!(!works(&pool, next["access_token"].as_str().unwrap()).await);
    assert!(!works(&pool, tokens.access_token.expose()).await);
    assert_invalid_grant(&refresh(&h, next["refresh_token"].as_str().unwrap()).await);
}

/// Two uses racing: the row lock lets one through; the other is a reuse.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_uses_of_one_token_succeed_once_and_revoke(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    let form = refresh_form(&h.app, tokens.refresh_token.expose());
    let (first, second) = tokio::join!(
        h.router
            .clone()
            .oneshot(form_request("/oauth/token", &form, None)),
        h.router
            .clone()
            .oneshot(form_request("/oauth/token", &form, None)),
    );
    let mut statuses = [first.unwrap().status(), second.unwrap().status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::BAD_REQUEST]);
    assert_eq!(
        revoke_reason(&pool, grant_id).await.as_deref(),
        Some("refresh_reuse")
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_idle_refresh_token_expires(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    sqlx::query!(
        "UPDATE oauth_refresh_tokens SET idle_expires_at = now() WHERE grant_id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_invalid_grant(&refresh(&h, tokens.refresh_token.expose()).await);
    // An expired token is not a reuse.
    assert_eq!(revoke_reason(&pool, grant_id).await, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn no_refresh_outlives_the_grants_absolute_limit(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    sqlx::query!(
        "UPDATE oauth_grants SET expires_at = now() + interval '1 hour' WHERE id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let next = refresh(&h, tokens.refresh_token.expose()).await.json();
    // The new token's idle expiry stops at the grant's.
    let capped = sqlx::query_scalar!(
        r#"SELECT bool_and(r.idle_expires_at <= g.expires_at) AS "capped!"
           FROM oauth_refresh_tokens r JOIN oauth_grants g ON g.id = r.grant_id
           WHERE g.id = $1 AND r.used_at IS NULL"#,
        grant_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(capped);
    sqlx::query!(
        "UPDATE oauth_grants SET expires_at = now() WHERE id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_invalid_grant(&refresh(&h, next["refresh_token"].as_str().unwrap()).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refresh_may_narrow_the_scopes_but_not_widen_them(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (_, tokens) = signed_in(&h, ScopeSet::of(&[Scope::TimeWrite, Scope::TimeRead])).await;
    let mut form = refresh_form(&h.app, tokens.refresh_token.expose());
    form.push(("scope", "catalog:read"));
    let wider = h.post("/oauth/token", &form).await;
    assert_eq!(wider.status, StatusCode::BAD_REQUEST);
    assert_eq!(wider.error(), "invalid_scope");
    // A refused scope uses nothing up.
    form.pop();
    form.push(("scope", "time:read"));
    let narrower = h.post("/oauth/token", &form).await.json();
    assert_eq!(narrower["scope"], "time:read");
    // Narrowed for good: the next refresh carries only what this one had.
    let next = refresh(&h, narrower["refresh_token"].as_str().unwrap())
        .await
        .json();
    assert_eq!(next["scope"], "time:read");
}

/// FR-052: a revoked grant, a disabled app or a deactivated person stops
/// refreshing too.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refresh_stops_with_its_grant_app_or_person(pool: PgPool) {
    let h = Harness::new(&pool).await;

    let (grant_id, tokens) = signed_in(&h, all_scopes()).await;
    sqlx::query!(
        "UPDATE oauth_grants SET revoked_at = now(), revoke_reason = 'person' WHERE id = $1",
        grant_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_invalid_grant(&refresh(&h, tokens.refresh_token.expose()).await);

    let (_, tokens) = signed_in(&h, all_scopes()).await;
    sqlx::query!(
        "UPDATE users SET active = false WHERE id = $1",
        h.ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_invalid_grant(&refresh(&h, tokens.refresh_token.expose()).await);
    sqlx::query!(
        "UPDATE users SET active = true WHERE id = $1",
        h.ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE oauth_clients SET enabled = false WHERE id = $1",
        h.app.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let answer = refresh(&h, tokens.refresh_token.expose()).await;
    assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
    assert_eq!(answer.error(), "invalid_client");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_access_token_or_an_unknown_token_does_not_refresh(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (_, tokens) = signed_in(&h, all_scopes()).await;
    assert_invalid_grant(&refresh(&h, tokens.access_token.expose()).await);
    assert_invalid_grant(&refresh(&h, "horae_ort_unknown").await);
}
