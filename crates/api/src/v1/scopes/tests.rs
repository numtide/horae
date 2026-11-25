//! The scope map's shape rules and the layer's default deny, without a
//! database: a test router puts an OAuth caller in the extensions, as
//! `v1::authenticate` does, and answers 200 when the layer lets it through.

use axum::Router;
use axum::body::Body;
use axum::middleware::from_fn;
use axum::routing::get;
use horae_core::permissions::oauth::ScopeSet;
use tower::ServiceExt;
use uuid::Uuid;

use super::*;

#[test]
fn reads_name_a_read_scope_and_writes_time_write() {
    use Scope::{CatalogRead, TimeRead, TimeWrite};
    let ok = [
        (Method::GET, "/me", Required::Any),
        (Method::GET, "/tasks", Required::Scope(CatalogRead)),
        (Method::HEAD, "/time_entries", Required::Scope(TimeRead)),
        (Method::POST, "/time_entries", Required::Scope(TimeWrite)),
        (
            Method::DELETE,
            "/time_entries/{id}",
            Required::Scope(TimeWrite),
        ),
    ];
    for (method, path, required) in ok {
        assert_eq!(fits(&method, path, required), Ok(()), "{method} {path}");
    }
    let refused = [
        (Method::GET, "/tasks", Required::Any),
        (Method::POST, "/me", Required::Any),
        (Method::GET, "/time_entries", Required::Scope(TimeWrite)),
        (
            Method::POST,
            "/projects/{id}/archive",
            Required::Scope(CatalogRead),
        ),
        (
            Method::PATCH,
            "/time_entries/{id}",
            Required::Scope(TimeRead),
        ),
    ];
    for (method, path, required) in refused {
        assert!(fits(&method, path, required).is_err(), "{method} {path}");
    }
}

#[test]
fn the_published_document_fits_the_rules() {
    // Panics if an operation's scope does not fit its method.
    assert!(!requirements().is_empty());
}

/// `/api/v1/me/timer` (`time:read`) and a route the document does not
/// describe, behind the layer, for an OAuth caller holding `scopes`.
fn app(scopes: ScopeSet) -> Router {
    let caller = Caller::oauth(Uuid::nil(), Uuid::nil(), Uuid::nil(), Uuid::nil(), scopes);
    Router::new()
        .route("/api/v1/me/timer", get(|| async { "through" }))
        .route("/api/v1/undescribed", get(|| async { "through" }))
        .route_layer(from_fn(require))
        .layer(from_fn(
            move |mut request: Request, next: Next| async move {
                request.extensions_mut().insert(caller);
                next.run(request).await
            },
        ))
}

async fn answer(scopes: ScopeSet, method: Method, uri: &str) -> (StatusCode, Option<String>) {
    let request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let response = app(scopes).oneshot(request).await.unwrap();
    let challenge = response
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .map(|value| value.to_str().unwrap().to_owned());
    (response.status(), challenge)
}

#[tokio::test]
async fn a_route_missing_from_the_map_refuses_every_oauth_token() {
    let all = ScopeSet::of(&Scope::ALL);
    assert_eq!(
        answer(all, Method::GET, "/api/v1/undescribed").await,
        (
            StatusCode::FORBIDDEN,
            Some(r#"Bearer error="insufficient_scope""#.into())
        )
    );
}

#[tokio::test]
async fn head_needs_the_scope_of_get() {
    let catalog = ScopeSet::of(&[Scope::CatalogRead]);
    assert_eq!(
        answer(catalog, Method::HEAD, "/api/v1/me/timer").await,
        (
            StatusCode::FORBIDDEN,
            Some(r#"Bearer error="insufficient_scope", scope="time:read""#.into())
        )
    );
    let time = ScopeSet::of(&[Scope::TimeRead]);
    assert_eq!(
        answer(time, Method::HEAD, "/api/v1/me/timer").await,
        (StatusCode::OK, None)
    );
}
