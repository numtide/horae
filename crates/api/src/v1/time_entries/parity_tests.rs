//! Every write the web app refuses is refused through the API with the same
//! reason (SC-004): the problem's `detail` is the message the Timesheet's
//! server function answers for the same command.

use axum::http::StatusCode;
use horae_core::types::EntryState;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::tests::{day, new_entry, team};
use crate::models::scoped_time::{TimesheetCommand, TimesheetEntryInput, TimesheetWriteContext};
use crate::ops::{self, Caller};
use crate::testing::{SeedIds, time_entry};
use crate::v1::testing::{call, json, router, send, token};

/// The web app's refusal of `command` by `actor` on `subject`'s time.
async fn web_refusal(
    pool: &PgPool,
    actor: &SeedIds,
    subject: Uuid,
    command: TimesheetCommand,
) -> String {
    let mut write = ops::begin(pool, Caller::session(actor.org_id, actor.user_id))
        .await
        .unwrap();
    let context = TimesheetWriteContext {
        expected_requester: write.actor.requester(),
        subject_id: subject,
    };
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    write
        .commit(result, &ops::NoEvents)
        .await
        .err()
        .expect("the web app refuses it")
        .to_string()
}

fn input(ids: &SeedIds, day: chrono::NaiveDate, minutes: i32) -> TimesheetEntryInput {
    TimesheetEntryInput {
        project_id: ids.project_id,
        task_id: ids.task_id,
        spent_date: day,
        minutes,
        notes: Some("Review".into()),
        billable: true,
        start_minute: None,
    }
}

fn assert_same(api: (StatusCode, Value), web: &str, status: StatusCode) {
    let (got, body) = api;
    assert_eq!(got, status, "{body}");
    assert_eq!(body["detail"], json!(web), "{body}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn each_refusal_has_the_web_apps_reason(pool: PgPool) {
    let team = team(&pool).await;
    let (member, _) = token(&pool, &team.member, "member").await;
    let router = router(&pool, 1000);
    let day = day();
    let me = team.member.user_id;

    // A submitted entry is locked.
    let locked = time_entry(&pool, &team.member, EntryState::Submitted).await;
    let api = send(
        &router,
        "updateTimeEntry",
        json(
            "PATCH",
            &format!("/api/v1/time_entries/{locked}"),
            &member,
            &json!({ "minutes": 5 }),
        ),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        me,
        TimesheetCommand::Update {
            entry_id: locked,
            entry: input(&team.member, "2026-09-07".parse().unwrap(), 5),
        },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::CONFLICT);

    // Another day's entry does not restart.
    let earlier = time_entry(&pool, &team.member, EntryState::Open).await;
    let api = send(
        &router,
        "restartTimer",
        call(
            "POST",
            &format!("/api/v1/time_entries/{earlier}/restart"),
            &member,
        ),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        me,
        TimesheetCommand::RestartTimer { entry_id: earlier },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::CONFLICT);

    // A stopped entry does not stop.
    let api = send(
        &router,
        "stopTimer",
        call(
            "POST",
            &format!("/api/v1/time_entries/{earlier}/stop"),
            &member,
        ),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        me,
        TimesheetCommand::StopTimer { entry_id: earlier },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::CONFLICT);

    // A project the person is not assigned to cannot be tracked.
    let elsewhere = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Elsewhere','EUR')",
        elsewhere,
        team.admin.org_id,
        team.admin.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let api = send(
        &router,
        "startTimer",
        json(
            "POST",
            "/api/v1/timers",
            &member,
            &json!({ "project_id": elsewhere, "task_id": team.member.task_id }),
        ),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        me,
        TimesheetCommand::StartTimer {
            project_id: elsewhere,
            task_id: team.member.task_id,
            notes: None,
        },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::CONFLICT);

    // A member writes no one else's time.
    let mut theirs = new_entry(&team.admin, day);
    theirs["person_id"] = json!(team.admin.user_id);
    let api = send(
        &router,
        "createTimeEntry",
        json("POST", "/api/v1/time_entries", &member, &theirs),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        team.admin.user_id,
        TimesheetCommand::Create {
            entry: input(&team.admin, day, 90),
        },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::FORBIDDEN);

    // Minutes cannot be negative.
    let mut negative = new_entry(&team.member, day);
    negative["minutes"] = json!(-1);
    let api = send(
        &router,
        "createTimeEntry",
        json("POST", "/api/v1/time_entries", &member, &negative),
    )
    .await;
    let web = web_refusal(
        &pool,
        &team.member,
        me,
        TimesheetCommand::Create {
            entry: input(&team.member, day, -1),
        },
    )
    .await;
    assert_same((api.0, api.2), &web, StatusCode::UNPROCESSABLE_ENTITY);
}
