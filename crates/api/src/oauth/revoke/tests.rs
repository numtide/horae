//! `POST /oauth/revoke` (RFC 7009, FR-090).

use axum::http::StatusCode;
use horae_core::permissions::oauth::ClientKind;
use sqlx::PgPool;
use uuid::Uuid;

use crate::oauth::testing::{Answer, Harness};
use crate::ops::oauth::testing::{all_scopes, audit_actions, register, signed_in};
use crate::ops::oauth::tokens::authenticate;

async fn revoke(h: &Harness, client_id: &str, token: &str) -> Answer {
    h.post(
        "/oauth/revoke",
        &[("client_id", client_id), ("token", token)],
    )
    .await
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

fn assert_ok_and_empty(answer: &Answer) {
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert!(answer.body.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoking_an_access_token_ends_its_grant(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&pool, &h.ids, &h.app).await;
    assert_ok_and_empty(&revoke(&h, &h.app.client_id, tokens.access_token.expose()).await);
    assert_eq!(
        revoke_reason(&pool, grant_id).await.as_deref(),
        Some("client")
    );
    assert!(
        authenticate(&pool, tokens.access_token.expose())
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        audit_actions(&pool, grant_id).await,
        ["grant_approved", "grant_revoked"]
    );
    // Twice answers the same and records nothing more.
    assert_ok_and_empty(&revoke(&h, &h.app.client_id, tokens.access_token.expose()).await);
    assert_eq!(audit_actions(&pool, grant_id).await.len(), 2);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoking_a_refresh_token_ends_its_grant(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let (grant_id, tokens) = signed_in(&pool, &h.ids, &h.app).await;
    assert_ok_and_empty(&revoke(&h, &h.app.client_id, tokens.refresh_token.expose()).await);
    assert_eq!(
        revoke_reason(&pool, grant_id).await.as_deref(),
        Some("client")
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unknown_token_answers_the_same(pool: PgPool) {
    let h = Harness::new(&pool).await;
    assert_ok_and_empty(&revoke(&h, &h.app.client_id, "horae_oat_unknown").await);
}

/// Another app cannot end a grant that is not its own, and cannot tell.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn another_apps_token_is_left_alone(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let other = register(&pool, &h.ids, "Other", ClientKind::Public, all_scopes()).await;
    let (grant_id, tokens) = signed_in(&pool, &h.ids, &h.app).await;
    assert_ok_and_empty(&revoke(&h, &other.client_id, tokens.access_token.expose()).await);
    assert_eq!(revoke_reason(&pool, grant_id).await, None);
    assert!(
        authenticate(&pool, tokens.access_token.expose())
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_client_that_fails_to_authenticate_is_refused(pool: PgPool) {
    let h = Harness::new(&pool).await;
    let answer = revoke(&h, "horae_app_unknown", "horae_oat_unknown").await;
    assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
    assert_eq!(answer.error(), "invalid_client");
    let missing = h
        .post("/oauth/revoke", &[("client_id", h.app.client_id.as_str())])
        .await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    assert_eq!(missing.error(), "invalid_request");
}
