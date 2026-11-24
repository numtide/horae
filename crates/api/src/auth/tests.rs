//! Sign-in coming back to an OAuth request (spec 024 FR-023, research R-3):
//! only `/oauth/` paths are remembered, and only safe ones are followed.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::routing::{get, post};
use axum::{Extension, Router};
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session};

use super::{RETURN_TO_KEY, login_redirect_guard, safe_return_to, take_return_to};

fn session() -> Session {
    Session::new(None, Arc::new(MemoryStore::default()), None)
}

/// A page behind the guard, as the server layers it.
fn guarded(session: &Session) -> Router {
    Router::new()
        .route("/{*path}", get(|| async { "page" }))
        .layer(axum::middleware::from_fn(login_redirect_guard))
        .layer(Extension(session.clone()))
}

async fn load(router: &Router, uri: &str) -> StatusCode {
    let request = Request::get(uri)
        .header(header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();
    router.clone().oneshot(request).await.unwrap().status()
}

#[test]
fn only_plain_oauth_paths_are_safe() {
    for safe in [
        "/oauth/consent?request=abc",
        "/oauth/authorize?redirect_uri=http://127.0.0.1:5000/cb&state=a",
    ] {
        assert_eq!(safe_return_to(safe), Some(safe));
    }
    for unsafe_ in [
        "/",
        "/time",
        "//evil.example/oauth/",
        "https://evil.example/oauth/consent",
        "/oauth/../admin",
        "/oauth/%2e%2e/admin",
        "/oauth//evil.example",
        "/oauth/\\evil",
        "/oauth/consent\n",
        "oauth/consent",
    ] {
        assert_eq!(safe_return_to(unsafe_), None, "{unsafe_:?}");
    }
}

#[tokio::test]
async fn a_signed_out_oauth_page_is_remembered() {
    let session = session();
    let router = guarded(&session);
    let target = "/oauth/consent?request=abc";
    assert_eq!(load(&router, target).await, StatusCode::SEE_OTHER);
    assert_eq!(take_return_to(&session).await, target);
    // Used once.
    assert_eq!(take_return_to(&session).await, "/");
}

#[tokio::test]
async fn other_pages_still_land_on_the_home_page() {
    let session = session();
    let router = guarded(&session);
    assert_eq!(
        load(&router, "/time?week=2026-W48").await,
        StatusCode::SEE_OTHER
    );
    assert_eq!(take_return_to(&session).await, "/");
}

#[tokio::test]
async fn an_unsafe_stored_value_is_ignored() {
    let session = session();
    for stored in [
        "//evil.example",
        "/oauth/../x",
        "https://evil.example/oauth/x",
    ] {
        session.insert(RETURN_TO_KEY, stored).await.unwrap();
        assert_eq!(take_return_to(&session).await, "/", "{stored}");
    }
}

#[tokio::test]
async fn the_metadata_document_is_public() {
    let session = session();
    let router = guarded(&session);
    let status = load(&router, "/.well-known/oauth-authorization-server").await;
    assert_eq!(status, StatusCode::OK);
}

/// "Not you? Sign out" keeps the request for the next person to sign in.
#[tokio::test]
async fn signing_out_from_the_approval_page_keeps_the_request() {
    let session = session();
    // No person in the session: announcing a sign-out needs the server's
    // global state, which a unit test does not have.
    session.insert("other", 1).await.unwrap();
    session
        .insert(crate::oauth::pending::KEY, serde_json::json!([["abc", {}]]))
        .await
        .unwrap();
    let router = Router::new()
        .route("/auth/logout", post(super::dev::logout_post))
        .layer(Extension(session.clone()));
    let logout = |body: &'static str| {
        Request::post("/auth/logout")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap()
    };
    let response = router
        .clone()
        .oneshot(logout("return_to=%2Foauth%2Fconsent%3Frequest%3Dabc"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let other: Option<i32> = session.get("other").await.unwrap();
    assert!(other.is_none());
    let kept: Option<serde_json::Value> = session.get(crate::oauth::pending::KEY).await.unwrap();
    assert!(kept.is_some());
    assert_eq!(take_return_to(&session).await, "/oauth/consent?request=abc");

    // A plain sign-out keeps nothing.
    router.oneshot(logout("")).await.unwrap();
    let kept: Option<serde_json::Value> = session.get(crate::oauth::pending::KEY).await.unwrap();
    assert!(kept.is_none());
}
