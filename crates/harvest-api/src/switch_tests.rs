use super::*;
use crate::pagination_tests::request;
use crate::testing::seed;
use axum::{Extension, http::header::LOCATION};
use horae_core::permissions::catalog::BuiltInProfile;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session};

/// What the binary serves around the switch: `mount` beside the importer's
/// OAuth callback (`horae-api`), under a session signed in as `user_id`.
async fn app(pool: &PgPool, enabled: bool, user_id: Uuid) -> Router {
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    horae_api::session::set_session_user_id(&session, user_id)
        .await
        .unwrap();
    mount(enabled, pool.clone(), crate::pagination_tests::limiter())
        .merge(horae_api::harvest_oauth::callback_router())
        .layer(Extension(session))
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn switched_off_the_harvest_paths_are_an_empty_404(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let app = app(&pool, false, ids.user_id).await;

    for uri in [
        "/harvest/v2/users/me",
        "/v2/users/me",
        "/harvest/v2/time_entries.json",
        "/v2/projects/1",
        "/harvest/v2/no/such/path",
    ] {
        let (status, body) = request(&app, uri).await;
        assert_eq!(
            (status, body.as_str()),
            (StatusCode::NOT_FOUND, ""),
            "{uri}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn switched_on_the_harvest_paths_are_served(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let app = app(&pool, true, ids.user_id).await;

    for uri in ["/harvest/v2/users/me", "/v2/users/me"] {
        let (status, body) = request(&app, uri).await;
        assert_eq!(status, StatusCode::OK, "{uri}: {body}");
        let me: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(me["id"].is_i64(), "{uri}: {body}");
    }
}

/// FR-063: the importer's OAuth callback is not part of the switch.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_importer_oauth_callback_answers_in_both_settings(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    for enabled in [false, true] {
        let response = app(&pool, enabled, ids.user_id)
            .await
            .oneshot(
                axum::http::Request::builder()
                    .uri("/auth/harvest/callback?code=c&state=s")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // No stored attempt, so the callback refuses the state and sends the
        // admin back to the importer: the route answered.
        assert_eq!(
            response.status(),
            StatusCode::SEE_OTHER,
            "enabled={enabled}"
        );
        let location = response.headers()[LOCATION].to_str().unwrap();
        assert!(
            location.starts_with("/admin/importers/harvest?"),
            "enabled={enabled}: {location}"
        );
    }
}
