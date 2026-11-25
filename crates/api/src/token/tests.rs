//! The bearer extractor behind a test route that answers with its caller.

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use horae_core::permissions::catalog::BuiltInProfile;
use tower::ServiceExt;
use uuid::Uuid;

use super::*;
use crate::ops::{Channel, NoEvents};
use crate::testing::{SeedIds, seed};

fn whoami(pool: PgPool) -> Router {
    Router::new()
        .route(
            "/whoami",
            get(|ApiCaller(caller): ApiCaller| async move {
                let token_id = match caller.channel {
                    Channel::Token { token_id } | Channel::OAuth { token_id, .. } => token_id,
                    _ => panic!("not a token caller: {caller:?}"),
                };
                format!("{} {} {token_id}", caller.org_id, caller.user_id)
            }),
        )
        .with_state(pool)
}

async fn call(pool: &PgPool, authorization: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::get("/whoami");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = whoami(pool.clone())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn issue(pool: &PgPool, ids: &SeedIds) -> (String, Uuid) {
    let mut write = ops::begin(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::create(&mut write.tx, &write.actor, "script", None).await;
    let (secret, row) = write.commit(result, &NoEvents).await.unwrap();
    (secret.expose().to_owned(), row.id)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_valid_token_answers_as_its_owner(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (token, token_id) = issue(&pool, &ids).await;
    let expected = format!("{} {} {token_id}", ids.org_id, ids.user_id);
    for scheme in ["Bearer", "bearer"] {
        let (status, body) = call(&pool, Some(&format!("{scheme} {token}"))).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, expected.as_str()));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_missing_malformed_or_unknown_credential_is_unauthorized(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (token, _) = issue(&pool, &ids).await;
    for authorization in [
        None,
        Some(token.clone()),
        Some(format!("Basic {token}")),
        Some("Bearer ".into()),
        Some("Bearer horae_pat_unknown".into()),
    ] {
        let (status, body) = call(&pool, authorization.as_deref()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{authorization:?}");
        assert!(!body.contains(&token), "the refusal echoes the token");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_revoked_token_is_unauthorized(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (token, token_id) = issue(&pool, &ids).await;
    let mut write = ops::begin(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::revoke(&mut write.tx, &write.actor, token_id).await;
    write.commit(result, &NoEvents).await.unwrap();
    let (status, _) = call(&pool, Some(&format!("Bearer {token}"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_oauth_access_token_answers_as_its_person(pool: PgPool) {
    use crate::ops::oauth::testing::{public_app, signed_in};
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let admin = crate::testing::add_person(&pool, &ids, BuiltInProfile::Administrator).await;
    let app = public_app(&pool, &admin).await;
    let (_, tokens) = signed_in(&pool, &ids, &app).await;
    let (status, body) = call(
        &pool,
        Some(&format!("Bearer {}", tokens.access_token.expose())),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with(&format!("{} {} ", ids.org_id, ids.user_id)));

    // A refresh token is no bearer credential.
    let refresh = format!("Bearer {}", tokens.refresh_token.expose());
    assert_eq!(
        call(&pool, Some(&refresh)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn only_a_bearer_scheme_with_a_credential_is_read() {
    assert_eq!(bearer("Bearer abc"), Some("abc"));
    assert_eq!(bearer("BEARER  abc "), Some("abc"));
    assert_eq!(bearer("Bearer"), None);
    assert_eq!(bearer("Bearer  "), None);
    assert_eq!(bearer("Token abc"), None);
}
