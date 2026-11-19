//! Time-entry writes through the router (spec 021 step 3): each mode of
//! create, update, delete, stop and restart, with the web app's rules and
//! refusals, and never with the session cookie alone.

use axum::body::Body;
use axum::http::{Request, header};
use horae_server::ops::testing::Recorder;
use horae_server::plugin::AppEvent;
use tower::ServiceExt;

use super::tokens::token;
use super::*;
use crate::testing::{SeedIds, add_person};

/// An administrator and a member, both able to track the seeded pair, each
/// with a token.
struct Team {
    admin: SeedIds,
    member: SeedIds,
    admin_token: String,
    member_token: String,
}

async fn team(pool: &PgPool) -> Team {
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
    let (admin_token, _) = token(pool, &admin).await;
    let (member_token, _) = token(pool, &member).await;
    Team {
        admin,
        member,
        admin_token,
        member_token,
    }
}

/// The seeded pair on a day with no other entries, plus `fields`.
async fn entry(pool: &PgPool, ids: &SeedIds, fields: Value) -> Value {
    let mut body = json!({
        "project_id": api_id(pool, "project", ids.project_id).await,
        "task_id": api_id(pool, "task", ids.task_id).await,
        "spent_date": "2026-09-08",
    });
    body.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    body
}

fn json_request(method: &str, uri: &str, token: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn bare(method: &str, uri: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// The status and body; an empty body reads as `null`.
async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (status, body)
}

async fn create(app: &Router, token: &str, body: &Value) -> (StatusCode, Value) {
    send(
        app,
        json_request("POST", "/harvest/v2/time_entries", token, body),
    )
    .await
}

async fn stored_minutes(pool: &PgPool, harvest_id: &Value) -> i32 {
    sqlx::query_scalar!(
        "SELECT minutes FROM time_entries WHERE harvest_id = $1",
        harvest_id.as_i64().unwrap()
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn entry_count(pool: &PgPool) -> Option<i64> {
    sqlx::query_scalar!("SELECT count(*) FROM time_entries")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn hours_create_an_entry_of_that_length(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());

    let body = entry(
        &pool,
        &team.member,
        json!({ "hours": 1.5, "notes": "Review" }),
    )
    .await;
    let (status, created) = create(&app, &team.member_token, &body).await;

    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(stored_minutes(&pool, &created["id"]).await, 90);
    assert_eq!(created["hours"].to_string(), "1.50");
    assert_eq!(created["is_running"], false);
    assert_eq!(created["notes"], "Review");
    assert_eq!(created["spent_date"], "2026-09-08");
    assert_eq!(
        created["user"]["id"],
        api_id(&pool, "user", team.member.user_id).await
    );
    assert_eq!(
        (&created["started_time"], &created["ended_time"]),
        (&Value::Null, &Value::Null),
        "an entry without a start has no time of day"
    );
    assert!(created.get("horae_notice").is_none(), "{created}");
}

/// Hours are read as exact decimals and rounded half up to whole minutes
/// (FR-020), from a JSON number, a JSON string, a form body or the query.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn hours_round_half_up_to_whole_minutes_from_any_encoding(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let project = api_id(&pool, "project", team.member.project_id).await;
    let task = api_id(&pool, "task", team.member.task_id).await;
    let fields = format!("project_id={project}&task_id={task}&spent_date=2026-09-08");
    let form = |hours: &str| {
        Request::builder()
            .method("POST")
            .uri("/harvest/v2/time_entries")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", team.member_token),
            )
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(format!("{fields}&hours={hours}")))
            .unwrap()
    };
    let query = |hours: &str| {
        bare(
            "POST",
            &format!("/harvest/v2/time_entries?{fields}&hours={hours}"),
            &team.member_token,
        )
    };
    let number = |hours: &str| {
        let body: Value = serde_json::from_str(&format!(
            r#"{{"project_id":{project},"task_id":{task},"spent_date":"2026-09-08","hours":{hours}}}"#
        ))
        .unwrap();
        json_request(
            "POST",
            "/harvest/v2/time_entries",
            &team.member_token,
            &body,
        )
    };

    // Written back with two decimals: 1 minute is 0.02 hours.
    for (request, minutes, hours) in [
        (number("0.01"), 1, "0.02"),
        (number("1.255"), 75, "1.25"),
        (number("0.333"), 20, "0.33"),
        (form("0.01"), 1, "0.02"),
        (query("1.255"), 75, "1.25"),
        (
            json_request(
                "POST",
                "/harvest/v2/time_entries",
                &team.member_token,
                &entry(&pool, &team.member, json!({ "hours": "0.333" })).await,
            ),
            20,
            "0.33",
        ),
    ] {
        let (status, created) = send(&app, request).await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        assert_eq!(
            stored_minutes(&pool, &created["id"]).await,
            minutes,
            "{created}"
        );
        assert_eq!(created["hours"].to_string(), hours);
    }

    for hours in [json!(-1), json!("abc"), json!(true)] {
        let body = entry(&pool, &team.member, json!({ "hours": hours })).await;
        let (status, refused) = create(&app, &team.member_token, &body).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{hours}: {refused}"
        );
        assert!(refused["message"].is_string(), "{refused}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_start_and_an_end_create_an_entry_between_them(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());

    let body = entry(
        &pool,
        &team.member,
        json!({ "started_time": "9:00am", "ended_time": "10:30am", "hours": 5 }),
    )
    .await;
    let (status, created) = create(&app, &team.member_token, &body).await;

    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(stored_minutes(&pool, &created["id"]).await, 90, "times win");
    assert_eq!(
        (&created["started_time"], &created["ended_time"]),
        (&json!("9:00am"), &json!("10:30am"))
    );
    let start = sqlx::query_scalar!(
        "SELECT start_minute FROM time_entries WHERE harvest_id = $1",
        created["id"].as_i64().unwrap()
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(start, Some(540));

    // 24-hour times are read too, and a start with hours keeps the hours.
    let body = entry(
        &pool,
        &team.member,
        json!({ "started_time": "13:00", "hours": 0.5 }),
    )
    .await;
    let (status, created) = create(&app, &team.member_token, &body).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(
        (&created["started_time"], &created["ended_time"]),
        (&json!("1:00pm"), &json!("1:30pm"))
    );
}

/// An end before the start, which is how a span past midnight reads on one
/// day, is the Timesheet's negative-minutes refusal.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_span_ending_before_it_starts_or_past_midnight_is_refused(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());

    for (started, ended) in [("10:00am", "9:00am"), ("11:00pm", "1:00am")] {
        let body = entry(
            &pool,
            &team.member,
            json!({ "started_time": started, "ended_time": ended }),
        )
        .await;
        let (status, refused) = create(&app, &team.member_token, &body).await;
        assert_eq!(
            (status, refused),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "message": "Minutes cannot be negative" })
            ),
            "{started}–{ended}"
        );
    }
    for fields in [
        json!({ "ended_time": "9:00am" }),
        json!({ "started_time": "9:00am" }),
        json!({ "started_time": "25:00", "ended_time": "26:00" }),
    ] {
        let body = entry(&pool, &team.member, fields.clone()).await;
        let (status, refused) = create(&app, &team.member_token, &body).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{fields}: {refused}"
        );
    }
    assert_eq!(entry_count(&pool).await, Some(0));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn without_hours_or_times_a_timer_starts_and_stops_and_restarts(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let token = &team.member_token;
    let timer = entry(&pool, &team.member, json!({ "notes": "Call" })).await;

    let (status, first) = create(&app, token, &timer).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["is_running"], true);
    assert!(first["timer_started_at"].is_string(), "{first}");
    assert_eq!(first["notes"], "Call");

    // D-6: a second timer stops the first.
    let (status, second) = create(&app, token, &timer).await;
    assert_eq!(status, StatusCode::CREATED, "{second}");
    let first_uri = format!("/harvest/v2/time_entries/{}", first["id"]);
    let (_, first_now) = send(&app, bare("GET", &first_uri, token)).await;
    assert_eq!(
        first_now["is_running"], false,
        "starting stopped the running timer"
    );

    let second_uri = format!("/harvest/v2/time_entries/{}", second["id"]);
    let (status, stopped) = send(&app, bare("PATCH", &format!("{second_uri}/stop"), token)).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(
        (&stopped["is_running"], &stopped["timer_started_at"]),
        (&json!(false), &Value::Null)
    );
    let (status, again) = send(&app, bare("PATCH", &format!("{second_uri}/stop"), token)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{again}");
    assert!(again["message"].is_string(), "{again}");

    // D-6 again: restarting the first stops nothing else (none runs) and
    // runs it; restarting the second then stops the first.
    let (status, restarted) =
        send(&app, bare("PATCH", &format!("{first_uri}/restart"), token)).await;
    assert_eq!(status, StatusCode::OK, "{restarted}");
    assert_eq!(restarted["is_running"], true);
    let (status, _) = send(&app, bare("PATCH", &format!("{second_uri}/restart"), token)).await;
    assert_eq!(status, StatusCode::OK);
    let (_, first_now) = send(&app, bare("GET", &first_uri, token)).await;
    assert_eq!(
        first_now["is_running"], false,
        "restarting stopped the running timer"
    );
    let running = sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE is_running")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(running, Some(1));

    let yesterday = time_entry(&pool, &team.member, EntryState::Open).await;
    let uri = format!(
        "/harvest/v2/time_entries/{}/restart",
        api_id(&pool, "time_entry", yesterday).await
    );
    let (status, refused) = send(&app, bare("PATCH", &uri, token)).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "another day's entry: {refused}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_update_changes_only_the_fields_it_names(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let token = &team.member_token;
    let body = entry(
        &pool,
        &team.member,
        json!({ "started_time": "9:00am", "ended_time": "10:00am", "notes": "Draft" }),
    )
    .await;
    let (_, created) = create(&app, token, &body).await;
    let uri = format!("/harvest/v2/time_entries/{}", created["id"]);

    let (status, updated) = send(
        &app,
        json_request("PATCH", &uri, token, &json!({ "notes": "Final" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        (&updated["notes"], &updated["started_time"]),
        (&json!("Final"), &json!("9:00am"))
    );
    assert_eq!(updated["hours"].to_string(), "1.00");

    // An end alone keeps the start.
    let (status, updated) = send(
        &app,
        json_request("PATCH", &uri, token, &json!({ "ended_time": "11:30am" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        (updated["hours"].to_string(), &updated["ended_time"]),
        ("2.50".to_owned(), &json!("11:30am"))
    );

    let (status, updated) = send(
        &app,
        json_request(
            "PATCH",
            &uri,
            token,
            &json!({ "hours": 0.25, "notes": null }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{updated}");
    assert_eq!(
        (
            updated["hours"].to_string(),
            &updated["notes"],
            &updated["started_time"]
        ),
        ("0.25".to_owned(), &Value::Null, &json!("9:00am"))
    );
    assert_eq!(stored_minutes(&pool, &created["id"]).await, 15);

    let (status, refused) = send(
        &app,
        json_request("PATCH", &uri, token, &json!({ "ended_time": "8:00am" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_delete_answers_200_and_the_entry_is_gone(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let token = &team.member_token;
    let (_, created) = create(
        &app,
        token,
        &entry(&pool, &team.member, json!({ "hours": 1 })).await,
    )
    .await;
    let uri = format!("/harvest/v2/time_entries/{}", created["id"]);

    let (status, body) = send(&app, bare("DELETE", &uri, token)).await;

    assert_eq!((status, body), (StatusCode::OK, Value::Null));
    let (status, _) = send(&app, bare("GET", &uri, token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = send(&app, bare("DELETE", &uri, token)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Principle IV: a request with only the session cookie never writes, even
/// though the same cookie reads.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_session_cookie_alone_cannot_write(pool: PgPool) {
    let team = team(&pool).await;
    let existing = time_entry(&pool, &team.member, EntryState::Open).await;
    let app = signed_in(&pool, team.member.user_id).await;
    let entry_uri = format!(
        "/harvest/v2/time_entries/{}",
        api_id(&pool, "time_entry", existing).await
    );
    let body = entry(&pool, &team.member, json!({ "hours": 1 })).await;
    let cookie_only = |method: &str, uri: &str| {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    };

    for (method, uri) in [
        ("POST", "/harvest/v2/time_entries".to_owned()),
        ("PATCH", entry_uri.clone()),
        ("DELETE", entry_uri.clone()),
        ("PATCH", format!("{entry_uri}/stop")),
        ("PATCH", format!("{entry_uri}/restart")),
    ] {
        let (status, refused) = send(&app, cookie_only(method, &uri)).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri}: {refused}"
        );
        assert_eq!(refused["error"], "invalid_token");
    }
    assert_eq!(entry_count(&pool).await, Some(1));
    let (status, _) = send(&app, cookie_only("GET", &entry_uri)).await;
    assert_eq!(status, StatusCode::OK, "the cookie still reads");
}

/// A token can never do more than its owner: a member writes only their own
/// time, and another person's entry is not even found.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_token_writes_only_what_its_owner_may(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let admin_number = api_id(&pool, "user", team.admin.user_id).await;
    let member_number = api_id(&pool, "user", team.member.user_id).await;

    let for_admin = entry(
        &pool,
        &team.admin,
        json!({ "hours": 1, "user_id": admin_number }),
    )
    .await;
    let (status, refused) = create(&app, &team.member_token, &for_admin).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"], "forbidden");
    let timer = entry(&pool, &team.admin, json!({ "user_id": admin_number })).await;
    let (status, _) = create(&app, &team.member_token, &timer).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let admins = time_entry(&pool, &team.admin, EntryState::Open).await;
    let uri = format!(
        "/harvest/v2/time_entries/{}",
        api_id(&pool, "time_entry", admins).await
    );
    for request in [
        json_request("PATCH", &uri, &team.member_token, &json!({ "hours": 2 })),
        bare("DELETE", &uri, &team.member_token),
        bare("PATCH", &format!("{uri}/restart"), &team.member_token),
    ] {
        let (status, refused) = send(&app, request).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    }
    assert_eq!(entry_count(&pool).await, Some(1));

    // The administrator may write for the member.
    let for_member = entry(
        &pool,
        &team.member,
        json!({ "hours": 1, "user_id": member_number }),
    )
    .await;
    let (status, created) = create(&app, &team.admin_token, &for_member).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["user"]["id"], member_number);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn submitted_approved_and_invoiced_entries_are_locked(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    for state in [
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ] {
        let locked = time_entry(&pool, &team.member, state).await;
        let uri = format!(
            "/harvest/v2/time_entries/{}",
            api_id(&pool, "time_entry", locked).await
        );
        for request in [
            json_request("PATCH", &uri, &team.member_token, &json!({ "hours": 2 })),
            bare("DELETE", &uri, &team.member_token),
        ] {
            let (status, refused) = send(&app, request).await;
            assert_eq!(status, StatusCode::CONFLICT, "{state:?}: {refused}");
            assert_eq!(
                refused["message"],
                "Approval or billing coverage requires the approval editing integration"
            );
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn missing_and_malformed_fields_are_422(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let token = &team.member_token;
    let complete = entry(&pool, &team.member, json!({ "hours": 1 })).await;

    for (key, message) in [
        ("project_id", "project_id is required"),
        ("task_id", "task_id is required"),
        ("spent_date", "spent_date is required"),
    ] {
        let mut body = complete.clone();
        body.as_object_mut().unwrap().remove(key);
        let (status, refused) = create(&app, token, &body).await;
        assert_eq!(
            (status, refused),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "message": message })
            )
        );
    }
    let mut body = complete.clone();
    body["spent_date"] = json!("08/09/2026");
    let (status, _) = create(&app, token, &body).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, refused) = send(
        &app,
        Request::builder()
            .method("POST")
            .uri("/harvest/v2/time_entries")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("[1]"))
            .unwrap(),
    )
    .await;
    assert_eq!(
        (status, refused),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "message": "The body must be a JSON object" })
        )
    );
}

/// FR-030: a write reaches the plugins as the web app's does, once committed,
/// and a refusal reaches them not at all.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn plugins_hear_committed_writes_only(pool: PgPool) {
    let team = team(&pool).await;
    let events = Arc::new(Recorder::default());
    let app = serve(V1State {
        events: events.clone(),
        ..test_state(pool.clone(), limiter())
    });

    let body = entry(&pool, &team.member, json!({ "hours": 1 })).await;
    let (status, created) = create(&app, &team.member_token, &body).await;
    assert_eq!(status, StatusCode::CREATED);
    let created_id = sqlx::query_scalar!(
        "SELECT id FROM time_entries WHERE harvest_id = $1",
        created["id"].as_i64().unwrap()
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let heard = events.take();
    assert!(
        matches!(&heard[..], [AppEvent::TimeEntryCreated { time_entry, .. }] if time_entry.id == created_id),
        "{heard:?}"
    );

    let refused = entry(&pool, &team.member, json!({ "hours": -1 })).await;
    let (status, _) = create(&app, &team.member_token, &refused).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(events.take().is_empty(), "a refusal is never heard");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_daily_limit_is_the_web_apps_409(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());
    let token = &team.member_token;
    let full = entry(&pool, &team.member, json!({ "hours": 240 })).await;
    let (status, _) = create(&app, token, &full).await;
    assert_eq!(status, StatusCode::CREATED);

    let more = entry(&pool, &team.member, json!({ "hours": 0.02 })).await;
    let (status, refused) = create(&app, token, &more).await;

    assert_eq!(
        (status, refused),
        (
            StatusCode::CONFLICT,
            json!({ "message": "Daily hours cannot exceed 240." })
        )
    );
}

/// A number no record has is handed to the operation, which refuses it as
/// the web app refuses an unknown pair; nothing is written.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unknown_project_or_task_number_writes_nothing(pool: PgPool) {
    let team = team(&pool).await;
    let app = router(pool.clone(), limiter());

    for key in ["project_id", "task_id"] {
        let mut body = entry(&pool, &team.member, json!({ "hours": 1 })).await;
        body[key] = json!(999_999_999);
        let (status, refused) = create(&app, &team.member_token, &body).await;
        assert!(status.is_client_error(), "{key}: {status} {refused}");
        assert!(refused.is_object(), "{refused}");
    }
    assert_eq!(entry_count(&pool).await, Some(0));
}
