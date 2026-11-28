//! The approval page's server functions, through the adapters their
//! `#[server]` bodies call after reading the session. The operations' own
//! rules are tested in `ops::oauth::authorize`.

use std::sync::Arc;

use horae_core::permissions::catalog::BuiltInProfile;
use horae_server::ops::oauth::testing::{
    ISSUER, REDIRECT_URI, all_scopes, pending as pending_request, public_app,
};
use sqlx::PgPool;
use tower_sessions::MemoryStore;

use super::*;
use crate::testing::{SeedIds, seed};

fn caller(ids: &SeedIds) -> Caller {
    Caller::session(ids.org_id, ids.user_id)
}

fn new_session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

fn code(error: &ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => *code,
        other => panic!("not a server error: {other:?}"),
    }
}

/// A seeded person with a request of a public app waiting in `session`.
async fn waiting(pool: &PgPool, session: &Session) -> (SeedIds, String) {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    let app = public_app(pool, &ids).await;
    let request = pending_request(&app, REDIRECT_URI, all_scopes());
    let id = horae_api::oauth::pending::store(session, request)
        .await
        .unwrap();
    (ids, id)
}

async fn grants(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM oauth_grants"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_details_name_the_app_and_the_person(pool: PgPool) {
    let session = new_session();
    let (ids, id) = waiting(&pool, &session).await;
    let details = details(&pool, &session, caller(&ids), &id).await.unwrap();
    assert_eq!(details.request_id, id);
    assert_eq!(details.app_name, "Widget");
    assert!(!details.built_in);
    assert_eq!(details.scopes, ["time:read", "time:write", "catalog:read"]);
    assert_eq!(details.return_to_display, "com.example.app");
    assert_eq!(details.organization_name, "Test Org");
    // Reading does not use the request up.
    assert!(details_ok(&pool, &session, &ids, &id).await);
}

async fn details_ok(pool: &PgPool, session: &Session, ids: &SeedIds, id: &str) -> bool {
    details(pool, session, caller(ids), id).await.is_ok()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn allow_creates_a_grant_and_code_for_the_sessions_request_once(pool: PgPool) {
    let session = new_session();
    let (ids, id) = waiting(&pool, &session).await;
    let config = crate::config::OAuthConfig::default();
    let url = approve(&pool, &session, caller(&ids), &id, Some(ISSUER), &config)
        .await
        .unwrap();
    assert!(
        url.starts_with("com.example.app:/callback?code=horae_oac_"),
        "{url}"
    );
    assert!(
        url.ends_with("&state=xyz&iss=https%3A%2F%2Fhorae.example.com"),
        "{url}"
    );
    assert_eq!(grants(&pool).await, 1);
    let code_count = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM oauth_codes WHERE redirect_uri = $1"#,
        REDIRECT_URI
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(code_count, 1);

    let again = approve(&pool, &session, caller(&ids), &id, Some(ISSUER), &config)
        .await
        .unwrap_err();
    assert_eq!(code(&again), NOT_FOUND);
    assert_eq!(grants(&pool).await, 1);
    assert!(!details_ok(&pool, &session, &ids, &id).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_request_of_another_session_finds_nothing(pool: PgPool) {
    let session = new_session();
    let (ids, id) = waiting(&pool, &session).await;
    let other = new_session();
    let config = crate::config::OAuthConfig::default();
    let error = approve(&pool, &other, caller(&ids), &id, Some(ISSUER), &config)
        .await
        .unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);
    let error = details(&pool, &other, caller(&ids), &id).await.unwrap_err();
    assert_eq!(code(&error), NOT_FOUND);
    assert_eq!(
        code(&deny(&other, &id, Some(ISSUER)).await.unwrap_err()),
        NOT_FOUND
    );
    assert_eq!(grants(&pool).await, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn deny_answers_access_denied_once(pool: PgPool) {
    let session = new_session();
    let (_, id) = waiting(&pool, &session).await;
    let url = deny(&session, &id, Some(ISSUER)).await.unwrap();
    assert_eq!(
        url,
        "com.example.app:/callback?error=access_denied&state=xyz&iss=https%3A%2F%2Fhorae.example.com"
    );
    assert_eq!(
        code(&deny(&session, &id, Some(ISSUER)).await.unwrap_err()),
        NOT_FOUND
    );
    assert_eq!(grants(&pool).await, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_inactive_person_cannot_allow(pool: PgPool) {
    let session = new_session();
    let (ids, id) = waiting(&pool, &session).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let config = crate::config::OAuthConfig::default();
    // The deactivated session is signed out, and nothing is granted.
    let result = approve(&pool, &session, caller(&ids), &id, Some(ISSUER), &config).await;
    assert!(crate::server_fns::signed_out(&result), "{result:?}");
    assert_eq!(grants(&pool).await, 0);
}
