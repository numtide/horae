//! `/me`, `/me/trackable` and `/me/timer`, and how every token route
//! authenticates: a bearer token only, revoked tokens refused, and the
//! per-token limit.

use std::sync::Arc;

use axum::Extension;
use axum::http::StatusCode;
use horae_core::permissions::catalog::BuiltInProfile;
use serde_json::json;
use sqlx::PgPool;
use tower_sessions::{MemoryStore, Session};
use uuid::Uuid;

use crate::models::scoped_time::{TimesheetCommand, TimesheetWriteContext};
use crate::ops::{self, Caller};
use crate::testing::{SeedIds, add_person, seed};
use crate::v1::testing::{get, router, send, token};

/// The seeded person may track the seeded project's task.
async fn trackable(pool: &PgPool, profile: BuiltInProfile) -> SeedIds {
    let ids = seed(pool, profile).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        ids.project_id,
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn me_is_the_tokens_owner_organization_and_token(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, token_id) = token(&pool, &ids, "calendar sync").await;
    let (status, _, body) = send(
        &router(&pool, 100),
        "getMe",
        get("/api/v1/me", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "person": {
                "id": ids.user_id,
                "name": "Test User",
                "email": format!("{}@test.com", ids.user_id),
            },
            "organization": { "id": ids.org_id, "name": "Test Org" },
            "administrator": false,
            "profile": "Member",
            "token": {
                "id": token_id,
                "name": "calendar sync",
                "expires_at": null,
                "kind": "personal",
                "oauth": null,
            },
        })
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrators_token_says_so(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (secret, _) = token(&pool, &ids, "admin script").await;
    let (status, _, body) = send(
        &router(&pool, 100),
        "getMe",
        get("/api/v1/me", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["administrator"], true);
}

fn assert_unauthenticated(status: StatusCode, body: &serde_json::Value, secret: Option<&str>) {
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["code"], "unauthenticated");
    if let Some(secret) = secret {
        assert!(
            !body.to_string().contains(secret),
            "the refusal echoes the token"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_token_route_needs_a_bearer_token(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids, "script").await;
    let router = router(&pool, 100);
    for (operation, uri) in [
        ("getMe", "/api/v1/me"),
        ("listTrackable", "/api/v1/me/trackable"),
        ("getRunningTimer", "/api/v1/me/timer"),
    ] {
        let (status, headers, body) = send(&router, operation, get(uri, None)).await;
        assert_unauthenticated(status, &body, Some(&secret));
        assert_eq!(headers["content-type"], "application/problem+json");

        let query = format!("{uri}?access_token={secret}");
        let (status, _, body) = send(&router, operation, get(&query, None)).await;
        assert_unauthenticated(status, &body, Some(&secret));

        let mut basic = get(uri, None);
        basic
            .headers_mut()
            .insert("authorization", format!("Basic {secret}").parse().unwrap());
        let (status, _, body) = send(&router, operation, basic).await;
        assert_unauthenticated(status, &body, Some(&secret));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_session_cookie_is_not_a_credential(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    crate::session::set_session_user_id(&session, ids.user_id)
        .await
        .unwrap();
    let signed_in = router(&pool, 100).layer(Extension(session));
    let (status, _, body) = send(&signed_in, "getMe", get("/api/v1/me", None)).await;
    assert_unauthenticated(status, &body, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_revoked_token_is_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (secret, token_id) = token(&pool, &ids, "script").await;
    let router = router(&pool, 100);
    let (status, _, _) = send(&router, "getMe", get("/api/v1/me", Some(&secret))).await;
    assert_eq!(status, StatusCode::OK);

    let mut write = ops::begin(&pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::revoke(&mut write.tx, &write.actor, token_id).await;
    write.commit(result, &ops::NoEvents).await.unwrap();

    let (status, _, body) = send(&router, "getMe", get("/api/v1/me", Some(&secret))).await;
    assert_unauthenticated(status, &body, Some(&secret));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn over_its_limit_a_token_waits_and_another_token_does_not(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let (first, _) = token(&pool, &ids, "busy").await;
    let (second, _) = token(&pool, &ids, "quiet").await;
    let router = router(&pool, 2);
    for _ in 0..2 {
        let (status, _, _) = send(&router, "getMe", get("/api/v1/me", Some(&first))).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (status, headers, body) = send(
        &router,
        "getRunningTimer",
        get("/api/v1/me/timer", Some(&first)),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
    assert_eq!(body["code"], "rate_limited");
    let retry_after: u64 = headers["retry-after"].to_str().unwrap().parse().unwrap();
    assert!((1..=60).contains(&retry_after), "{retry_after}");

    let (status, _, _) = send(&router, "getMe", get("/api/v1/me", Some(&second))).await;
    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn trackable_lists_what_the_timesheet_offers(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids, "script").await;
    let (status, _, body) = send(
        &router(&pool, 100),
        "listTrackable",
        get("/api/v1/me/trackable", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({
            "data": [{
                "project_id": ids.project_id,
                "project_name": "Widget",
                "task_id": ids.task_id,
                "task_name": "Dev",
                "billable": true,
            }],
            "next_cursor": null,
        })
    );
}

/// What the timesheet's tracking operation answers a session of `ids`' person
/// for `subject`.
async fn timesheet_tracking(pool: &PgPool, ids: &SeedIds, subject: Uuid) -> serde_json::Value {
    let mut read = ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let context = TimesheetWriteContext {
        expected_requester: read.actor.requester(),
        subject_id: subject,
    };
    let result = ops::time::tracking(&mut read.tx, &read.actor, &context).await;
    let options = read.finish(result).await.unwrap();
    let data: Vec<_> = options
        .into_iter()
        .map(crate::v1::types::Trackable::from)
        .collect();
    json!({ "data": data, "next_cursor": null })
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn trackable_for_someone_else_is_what_the_timesheet_answers(pool: PgPool) {
    let admin = trackable(&pool, BuiltInProfile::Administrator).await;
    let member = add_person(&pool, &admin, BuiltInProfile::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        member.project_id,
        member.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let router = router(&pool, 100);
    let uri = format!("/api/v1/me/trackable?person_id={}", member.user_id);
    for (asking, subject) in [(&admin, member.user_id), (&member, admin.user_id)] {
        let (secret, _) = token(&pool, asking, "script").await;
        let uri = format!("/api/v1/me/trackable?person_id={subject}");
        let (status, _, body) = send(&router, "listTrackable", get(&uri, Some(&secret))).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body, timesheet_tracking(&pool, asking, subject).await);
    }
    let (secret, _) = token(&pool, &admin, "script").await;
    let (_, _, body) = send(&router, "listTrackable", get(&uri, Some(&secret))).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1, "{body}");

    let (status, _, body) = send(
        &router,
        "listTrackable",
        get("/api/v1/me/trackable?person_id=nobody", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "bad_request");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_timer_is_the_running_entry_or_no_content(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let (secret, _) = token(&pool, &ids, "script").await;
    let router = router(&pool, 100);
    let (status, _, body) = send(
        &router,
        "getRunningTimer",
        get("/api/v1/me/timer", Some(&secret)),
    )
    .await;
    assert_eq!(
        (status, body),
        (StatusCode::NO_CONTENT, serde_json::Value::Null)
    );

    let caller = Caller::session(ids.org_id, ids.user_id);
    let mut write = ops::begin(&pool, caller).await.unwrap();
    let context = TimesheetWriteContext {
        expected_requester: write.actor.requester(),
        subject_id: ids.user_id,
    };
    let command = TimesheetCommand::StartTimer {
        project_id: ids.project_id,
        task_id: ids.task_id,
        notes: Some("Review".into()),
    };
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    write.commit(result, &ops::NoEvents).await.unwrap();

    let (status, _, body) = send(
        &router,
        "getRunningTimer",
        get("/api/v1/me/timer", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["person_id"], json!(ids.user_id));
    assert_eq!(body["project_id"], json!(ids.project_id));
    assert_eq!(body["notes"], "Review");
    assert_eq!(body["is_running"], true);
    assert_eq!(body["state"], "open");
    assert!(body["timer_started_at"].is_string(), "{body}");
}
