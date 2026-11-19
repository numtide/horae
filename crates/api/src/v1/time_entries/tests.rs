//! The eight time-entry operations over HTTP. Every answer is checked against
//! the published document by `send`.

use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use chrono::NaiveDate;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ops::testing::Recorder;
use crate::plugin::AppEvent;
use crate::testing::{SeedIds, add_person, seed, time_entry};
use crate::v1::testing::{call, get, json, router, router_hearing, send, token};

/// An administrator and a member, both able to track the seeded pair.
pub(super) struct Team {
    pub admin: SeedIds,
    pub member: SeedIds,
}

pub(super) async fn team(pool: &PgPool) -> Team {
    let admin = seed(pool, BuiltInProfile::Administrator).await;
    let member = add_person(pool, &admin, BuiltInProfile::Member).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        admin.project_id,
        admin.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    for person in [admin.user_id, member.user_id] {
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            admin.project_id,
            person
        )
        .execute(pool)
        .await
        .unwrap();
    }
    Team { admin, member }
}

/// A day for entries with a duration: the seed's entries are on 2026-09-07.
pub(super) fn day() -> NaiveDate {
    "2026-09-08".parse().unwrap()
}

pub(super) fn new_entry(ids: &SeedIds, day: NaiveDate) -> Value {
    json!({
        "project_id": ids.project_id,
        "task_id": ids.task_id,
        "spent_date": day,
        "minutes": 90,
        "notes": "Review",
    })
}

async fn create(router: &Router, token: &str, body: &Value) -> (StatusCode, Value) {
    let (status, _, body) = send(
        router,
        "createTimeEntry",
        json("POST", "/api/v1/time_entries", token, body),
    )
    .await;
    (status, body)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_entry_is_created_read_listed_changed_and_deleted(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    let day = day();

    let (status, headers, created) = send(
        &router,
        "createTimeEntry",
        json(
            "POST",
            "/api/v1/time_entries",
            &secret,
            &new_entry(&team.member, day),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().unwrap().to_owned();
    assert_eq!(headers["location"], format!("/api/v1/time_entries/{id}"));
    assert_eq!(created["person_id"], json!(team.member.user_id));
    assert_eq!(
        (
            &created["minutes"],
            &created["notes"],
            &created["billable"],
            &created["state"]
        ),
        (&json!(90), &json!("Review"), &json!(true), &json!("open"))
    );

    let (status, _, read) = send(
        &router,
        "getTimeEntry",
        get(&format!("/api/v1/time_entries/{id}"), Some(&secret)),
    )
    .await;
    assert_eq!((status, &read), (StatusCode::OK, &created));

    let (status, _, page) = send(
        &router,
        "listTimeEntries",
        get("/api/v1/time_entries", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page, json!({ "data": [created], "next_cursor": null }));

    let uri = format!("/api/v1/time_entries/{id}");
    let (status, _, patched) = send(
        &router,
        "updateTimeEntry",
        json(
            "PATCH",
            &uri,
            &secret,
            &json!({ "minutes": 30, "notes": null }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{patched}");
    assert_eq!(
        (&patched["minutes"], &patched["notes"]),
        (&json!(30), &Value::Null)
    );
    assert_eq!(patched["spent_date"], json!(day), "other fields are kept");

    let (status, _, body) = send(&router, "deleteTimeEntry", call("DELETE", &uri, &secret)).await;
    assert_eq!((status, body), (StatusCode::NO_CONTENT, Value::Null));
    let (status, _, body) = send(&router, "getTimeEntry", get(&uri, Some(&secret))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "not_found");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn plugin_events_are_heard_after_the_commit_and_never_for_a_refusal(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let events = Arc::new(Recorder::default());
    let router = router_hearing(&pool, events.clone());
    let day = day();

    let (status, created) = create(&router, &secret, &new_entry(&team.member, day)).await;
    assert_eq!(status, StatusCode::CREATED);
    let heard = events.take();
    assert!(
        matches!(&heard[..], [AppEvent::TimeEntryCreated { time_entry, .. }]
            if json!(time_entry.id) == created["id"]),
        "{heard:?}"
    );
    let stored = sqlx::query_scalar!("SELECT count(*) FROM time_entries")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, Some(1), "heard only once committed");

    let mut refused = new_entry(&team.member, day);
    refused["minutes"] = json!(-1);
    let (status, _) = create(&router, &secret, &refused).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(events.take().is_empty(), "a refusal is never heard");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_entry_outside_the_tokens_scope_is_not_found(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    let theirs = time_entry(&pool, &team.admin, EntryState::Open).await;
    let uri = format!("/api/v1/time_entries/{theirs}");
    for (operation, request) in [
        ("getTimeEntry", get(&uri, Some(&secret))),
        (
            "updateTimeEntry",
            json("PATCH", &uri, &secret, &json!({ "minutes": 1 })),
        ),
        ("deleteTimeEntry", call("DELETE", &uri, &secret)),
        ("stopTimer", call("POST", &format!("{uri}/stop"), &secret)),
        (
            "restartTimer",
            call("POST", &format!("{uri}/restart"), &secret),
        ),
    ] {
        let (status, _, body) = send(&router, operation, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{operation}: {body}");
        assert_eq!(body["code"], "not_found");
    }
    let (_, _, page) = send(
        &router,
        "listTimeEntries",
        get("/api/v1/time_entries", Some(&secret)),
    )
    .await;
    assert_eq!(
        page["data"],
        json!([]),
        "the list holds only the member's own time"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn writing_for_someone_else_needs_the_owners_write_scope(pool: PgPool) {
    let team = team(&pool).await;
    let (admin, _) = token(&pool, &team.admin, "admin").await;
    let (member, _) = token(&pool, &team.member, "member").await;
    let router = router(&pool, 1000);
    let day = day();

    let mut for_member = new_entry(&team.member, day);
    for_member["person_id"] = json!(team.member.user_id);
    let (status, body) = create(&router, &admin, &for_member).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["person_id"], json!(team.member.user_id));

    let mut for_admin = new_entry(&team.admin, day);
    for_admin["person_id"] = json!(team.admin.user_id);
    let (status, body) = create(&router, &member, &for_admin).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "forbidden");

    let timer = json!({
        "person_id": team.admin.user_id,
        "project_id": team.admin.project_id,
        "task_id": team.admin.task_id,
    });
    let (status, _, body) = send(
        &router,
        "startTimer",
        json("POST", "/api/v1/timers", &member, &timer),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn submitted_approved_and_invoiced_entries_are_locked(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    for state in [
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ] {
        let entry = time_entry(&pool, &team.member, state).await;
        let uri = format!("/api/v1/time_entries/{entry}");
        for (operation, request) in [
            (
                "updateTimeEntry",
                json("PATCH", &uri, &secret, &json!({ "minutes": 1 })),
            ),
            ("deleteTimeEntry", call("DELETE", &uri, &secret)),
        ] {
            let (status, _, body) = send(&router, operation, request).await;
            assert_eq!(
                status,
                StatusCode::CONFLICT,
                "{state:?} {operation}: {body}"
            );
            assert_eq!(body["code"], "conflict");
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invalid_input_names_its_field(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    let day = day();
    for (change, code, field) in [
        (json!({ "minutes": -1 }), "invalid", "minutes"),
        (json!({ "start_minute": 1440 }), "invalid", "start_minute"),
        (json!({ "hours": 1.5 }), "unknown_field", "hours"),
    ] {
        let mut body = new_entry(&team.member, day);
        body.as_object_mut()
            .unwrap()
            .extend(change.as_object().unwrap().clone());
        let (status, problem) = create(&router, &secret, &body).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{change}: {problem}"
        );
        assert_eq!(
            (&problem["code"], &problem["field"]),
            (&json!(code), &json!(field))
        );
    }
    let (status, problem) = create(&router, &secret, &json!({ "minutes": 5 })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{problem}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_timer_starts_stops_and_restarts_and_a_start_stops_the_running_one(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    let timer = json!({ "project_id": team.member.project_id, "task_id": team.member.task_id });

    let start = || json("POST", "/api/v1/timers", &secret, &timer);
    let (status, headers, first) = send(&router, "startTimer", start()).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["is_running"], true);
    assert_ne!(first["spent_date"], json!("2026-09-07"), "dated today");
    let first_id = first["id"].as_str().unwrap().to_owned();
    assert_eq!(
        headers["location"],
        format!("/api/v1/time_entries/{first_id}")
    );

    let (status, _, second) = send(&router, "startTimer", start()).await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let first_uri = format!("/api/v1/time_entries/{first_id}");
    let (_, _, first) = send(&router, "getTimeEntry", get(&first_uri, Some(&secret))).await;
    assert_eq!(
        first["is_running"], false,
        "starting stopped the running timer"
    );

    let second_uri = format!("/api/v1/time_entries/{}", second["id"].as_str().unwrap());
    let (status, _, stopped) = send(
        &router,
        "stopTimer",
        call("POST", &format!("{second_uri}/stop"), &secret),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(
        (&stopped["is_running"], &stopped["timer_started_at"]),
        (&json!(false), &Value::Null)
    );
    let (status, _, again) = send(
        &router,
        "stopTimer",
        call("POST", &format!("{second_uri}/stop"), &secret),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");

    let (status, _, restarted) = send(
        &router,
        "restartTimer",
        call("POST", &format!("{first_uri}/restart"), &secret),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{restarted}");
    assert_eq!(restarted["is_running"], true);
    let (_, _, running) = send(
        &router,
        "getRunningTimer",
        get("/api/v1/me/timer", Some(&secret)),
    )
    .await;
    assert_eq!(running["id"], json!(first_id));

    let yesterday = time_entry(&pool, &team.member, EntryState::Open).await;
    let (status, _, body) = send(
        &router,
        "restartTimer",
        call(
            "POST",
            &format!("/api/v1/time_entries/{yesterday}/restart"),
            &secret,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "another day's entry: {body}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_starts_leave_one_running_timer(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.member, "script").await;
    let router = router(&pool, 1000);
    let timer = json!({ "project_id": team.member.project_id, "task_id": team.member.task_id });
    let starts = (0..4).map(|_| {
        let (router, request) = (
            router.clone(),
            json("POST", "/api/v1/timers", &secret, &timer),
        );
        tokio::spawn(async move { send(&router, "startTimer", request).await })
    });
    let mut created = 0;
    for start in starts {
        let (status, _, body) = start.await.unwrap();
        match status {
            StatusCode::CREATED => created += 1,
            StatusCode::CONFLICT => assert_eq!(body["code"], "conflict"),
            other => panic!("{other}: {body}"),
        }
    }
    assert!(created >= 1);
    let running = sqlx::query_scalar!(
        "SELECT count(*) FROM time_entries WHERE user_id = $1 AND is_running",
        team.member.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(running, Some(1));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_list_walks_its_pages_with_signed_cursors(pool: PgPool) {
    let team = team(&pool).await;
    let (secret, _) = token(&pool, &team.admin, "script").await;
    let router = router(&pool, 1000);
    let mut ids = Vec::new();
    for owner in [&team.admin, &team.member, &team.admin] {
        ids.push(json!(time_entry(&pool, owner, EntryState::Open).await));
    }

    let (status, _, first) = send(
        &router,
        "listTimeEntries",
        get("/api/v1/time_entries?limit=2", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let listed: Vec<_> = first["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].clone())
        .collect();
    assert_eq!(listed, ids[..2]);
    let cursor = first["next_cursor"].as_str().unwrap();

    let (_, _, second) = send(
        &router,
        "listTimeEntries",
        get(
            &format!("/api/v1/time_entries?limit=2&cursor={cursor}"),
            Some(&secret),
        ),
    )
    .await;
    assert_eq!(second["data"][0]["id"], ids[2]);
    assert_eq!(second["next_cursor"], Value::Null);

    let other_filters = format!(
        "/api/v1/time_entries?limit=2&cursor={cursor}&person_id={}",
        team.admin.user_id
    );
    let (status, _, body) = send(
        &router,
        "listTimeEntries",
        get(&other_filters, Some(&secret)),
    )
    .await;
    assert_eq!(
        (status, &body["code"]),
        (StatusCode::BAD_REQUEST, &json!("invalid_cursor"))
    );

    let mine = format!("/api/v1/time_entries?person_id={}", team.member.user_id);
    let (_, _, page) = send(&router, "listTimeEntries", get(&mine, Some(&secret))).await;
    assert_eq!(page["data"].as_array().unwrap().len(), 1);
    assert_eq!(page["data"][0]["id"], ids[1]);

    for bad in ["limit=0", "limit=1001", "from=2026-09-08&to=2026-09-07"] {
        let (status, _, body) = send(
            &router,
            "listTimeEntries",
            get(&format!("/api/v1/time_entries?{bad}"), Some(&secret)),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}: {body}");
    }
    let (status, _, _) = send(
        &router,
        "listTimeEntries",
        get("/api/v1/time_entries?from=yesterday", Some(&secret)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
