//! Personal access tokens on the Harvest API (spec 022 FR-008, FR-062): the
//! tokens `/api/v1` takes, in Harvest's places, sharing its request limit.

use axum::body::Body;
use axum::http::{HeaderMap, Request, header};
use horae_server::config::{ApiConfig, CursorKey, RateLimit};
use tower::ServiceExt;

use super::*;
use crate::testing::SeedIds;

/// A new token of the seeded person: its whole secret and its id.
pub(crate) async fn token(pool: &PgPool, ids: &SeedIds) -> (String, Uuid) {
    let mut write = ops::begin(pool, ops::Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::create(&mut write.tx, &write.actor, "Harvest client", None).await;
    let (secret, row) = write.commit(result, &ops::NoEvents).await.unwrap();
    (secret.expose().to_owned(), row.id)
}

async fn account_id(pool: &PgPool, ids: &SeedIds) -> i64 {
    ops::api_tokens::fetch_harvest_account_id(pool, ids.org_id)
        .await
        .unwrap()
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, headers, serde_json::from_slice(&body).unwrap())
}

fn get(uri: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut request = Request::builder().uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    request.body(Body::empty()).unwrap()
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_reads_from_the_header_or_the_query(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids).await;
    let app = router(pool.clone(), limiter());
    let me = api_id(&pool, "user", ids.user_id).await;

    for request in [
        get(
            "/harvest/v2/users/me",
            &[("authorization", &bearer(&secret))],
        ),
        get("/v2/users/me.json", &[("authorization", &bearer(&secret))]),
        get(&format!("/harvest/v2/users/me?access_token={secret}"), &[]),
    ] {
        let uri = request.uri().to_string();
        let (status, _, body) = send(&app, request).await;
        assert_eq!((status, &body["id"]), (StatusCode::OK, &json!(me)), "{uri}");
    }
}

/// A token acts with its owner's grants, as the session does: a member's
/// token reads only their own time.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_reads_with_its_owners_permissions(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids).await;
    let app = router(pool.clone(), limiter());
    let auth = bearer(&secret);

    let (status, _, body) = send(&app, get("/harvest/v2/users", &[("authorization", &auth)])).await;

    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "forbidden");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_account_id_must_name_the_tokens_organization(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids).await;
    let app = router(pool.clone(), limiter());
    let account = account_id(&pool, &ids).await.to_string();
    let other = account_id(&pool, &seed(&pool, BuiltInProfile::Member).await)
        .await
        .to_string();
    let auth = bearer(&secret);

    for (uri, header_account) in [
        ("/harvest/v2/users/me".to_owned(), Some(account.as_str())),
        (format!("/harvest/v2/users/me?account_id={account}"), None),
    ] {
        let mut headers = vec![("authorization", auth.as_str())];
        headers.extend(header_account.map(|a| ("Harvest-Account-Id", a)));
        let (status, _, body) = send(&app, get(&uri, &headers)).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    }
    for (uri, header_account) in [
        ("/harvest/v2/users/me".to_owned(), Some(other.as_str())),
        ("/harvest/v2/users/me".to_owned(), Some("not-a-number")),
        (format!("/harvest/v2/users/me?account_id={other}"), None),
    ] {
        let mut headers = vec![("authorization", auth.as_str())];
        headers.extend(header_account.map(|a| ("Harvest-Account-Id", a)));
        let (status, _, body) = send(&app, get(&uri, &headers)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{uri} {header_account:?}");
        assert_eq!(
            body,
            json!({
                "error": "forbidden",
                "error_description": "Harvest-Account-Id does not name this token's account",
            })
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoked_expired_and_unknown_tokens_are_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (revoked, revoked_id) = token(&pool, &ids).await;
    let (expired, expired_id) = token(&pool, &ids).await;
    sqlx::query!(
        "UPDATE api_tokens SET revoked_at = now() WHERE id = $1",
        revoked_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE api_tokens SET expires_at = now() - interval '1 second' WHERE id = $1",
        expired_id
    )
    .execute(&pool)
    .await
    .unwrap();
    // A refused token is not rescued by a valid session cookie.
    let app = signed_in(&pool, ids.user_id).await;

    for secret in [revoked.as_str(), expired.as_str(), "horae_pat_unknown"] {
        for request in [
            get(
                "/harvest/v2/users/me",
                &[("authorization", &bearer(secret))],
            ),
            get(&format!("/harvest/v2/users/me?access_token={secret}"), &[]),
        ] {
            let (status, _, body) = send(&app, request).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(
                body,
                json!({
                    "error": "invalid_token",
                    "error_description": "The token is unknown, revoked or expired",
                })
            );
        }
    }
}

/// One limiter behind both APIs, as the binary builds it: a token's requests
/// to `/api/v1` and `/harvest/v2` count against one limit.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn both_apis_count_a_tokens_requests_against_one_limit(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids).await;
    let v1 = horae_api::v1::V1State::new(
        pool.clone(),
        &ApiConfig {
            cursor_key: CursorKey::new([1; 32]),
            rate_limit: RateLimit {
                requests: 3,
                window: std::time::Duration::from_secs(60),
            },
        },
    );
    let app = mount(true, v1.clone()).merge(horae_api::v1::router(v1));
    let auth = bearer(&secret);
    let harvest = || get("/harvest/v2/users/me", &[("authorization", &auth)]);
    let horae = || get("/api/v1/me", &[("authorization", &auth)]);

    assert_eq!(send(&app, horae()).await.0, StatusCode::OK);
    assert_eq!(send(&app, harvest()).await.0, StatusCode::OK);
    assert_eq!(send(&app, horae()).await.0, StatusCode::OK);

    let (status, headers, body) = send(&app, harvest()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let wait = headers[header::RETRY_AFTER].to_str().unwrap().to_owned();
    assert_eq!(body["retry_after"].to_string(), wait);
    assert!(body["message"].is_string(), "{body}");
    assert_eq!(send(&app, horae()).await.0, StatusCode::TOO_MANY_REQUESTS);
}

/// The session cookie still reads (spec 021 D-7), and no token limit applies
/// to it.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn cookie_reads_are_unchanged_and_unlimited(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let session = tower_sessions::Session::new(
        None,
        std::sync::Arc::new(tower_sessions::MemoryStore::default()),
        None,
    );
    horae_api::session::set_session_user_id(&session, ids.user_id)
        .await
        .unwrap();
    let one_request = Arc::new(RateLimiter::new(RateLimit {
        requests: 1,
        window: std::time::Duration::from_secs(60),
    }));
    let app = router(pool.clone(), one_request).layer(axum::Extension(session));

    for _ in 0..3 {
        let (status, _, body) = send(&app, get("/harvest/v2/users/me", &[])).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
}
