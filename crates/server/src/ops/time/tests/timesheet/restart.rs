//! Restarting a stopped entry, and starting or restarting while a timer runs
//! (021 D-6).

use super::*;

/// The subject's today, as the operation dates timers.
async fn today(pool: &PgPool, f: &Fixture) -> NaiveDate {
    crate::timezone::today_for_person(pool, f.context.subject_id)
        .await
        .unwrap()
}

/// A stopped entry of `day` holding `minutes`.
async fn stopped(pool: &PgPool, f: &Fixture, day: NaiveDate, minutes: i32) -> Uuid {
    let entry = f.entry(pool).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date=$2, minutes=$3 WHERE id=$1",
        entry,
        day as NaiveDate,
        minutes
    )
    .execute(pool)
    .await
    .unwrap();
    entry
}

/// Backdates the subject's running timer by `minutes`.
async fn ran_for(pool: &PgPool, f: &Fixture, minutes: i32) -> Uuid {
    sqlx::query_scalar!(
        "UPDATE time_entries SET started_at = now() - make_interval(mins => $2)
         WHERE user_id=$1 AND is_running RETURNING id",
        f.context.subject_id,
        minutes
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn start(pool: &PgPool, f: &Fixture) -> Result<Done<TimesheetChange>, OpError> {
    apply(
        pool,
        f.ids.org_id,
        f.ids.user_id,
        &f.context,
        TimesheetCommand::StartTimer {
            project_id: f.ids.project_id,
            task_id: f.ids.task_id,
            notes: None,
        },
    )
    .await
}

async fn restart(
    pool: &PgPool,
    f: &Fixture,
    entry_id: Uuid,
) -> Result<Done<TimesheetChange>, OpError> {
    apply(
        pool,
        f.ids.org_id,
        f.ids.user_id,
        &f.context,
        TimesheetCommand::RestartTimer { entry_id },
    )
    .await
}

struct Row {
    minutes: i32,
    start_minute: Option<i32>,
    is_running: bool,
}

async fn row(pool: &PgPool, id: Uuid) -> Row {
    sqlx::query_as!(
        Row,
        "SELECT minutes, start_minute, is_running FROM time_entries WHERE id=$1",
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[track_caller]
fn assert_conflict(result: Result<Done<TimesheetChange>, OpError>, expected: &str) {
    match result {
        Err(OpError::Conflict(message)) => assert_eq!(message, expected),
        Err(other) => panic!("expected a conflict, got {other:?}"),
        Ok(_) => panic!("expected a conflict, got success"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_restarted_entry_keeps_its_minutes_and_start(pool: PgPool) {
    let f = fixture(&pool).await;
    let entry = stopped(&pool, &f, today(&pool, &f).await, 45).await;
    sqlx::query!(
        "UPDATE time_entries SET start_minute=540 WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();

    let restarted = restart(&pool, &f, entry).await.unwrap();
    assert!(
        matches!(
            restarted.events.as_slice(),
            [AppEvent::TimeEntryUpdated { time_entry, .. }] if time_entry.id == entry && time_entry.is_running
        ),
        "{:?}",
        restarted.events
    );
    let running = row(&pool, entry).await;
    assert_eq!((running.minutes, running.is_running), (45, true));

    ran_for(&pool, &f, 20).await;
    f.execute(&pool, TimesheetCommand::StopTimer { entry_id: entry })
        .await
        .unwrap();
    let stopped = row(&pool, entry).await;
    assert_eq!(
        (stopped.minutes, stopped.start_minute, stopped.is_running),
        (65, Some(540), false)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_start_stops_the_running_timer_first(pool: PgPool) {
    let f = fixture(&pool).await;
    start(&pool, &f).await.unwrap();
    let first = ran_for(&pool, &f, 30).await;

    let started = start(&pool, &f).await.unwrap();

    assert_eq!(row(&pool, first).await.minutes, 30);
    assert!(!row(&pool, first).await.is_running);
    assert!(
        matches!(
            started.events.as_slice(),
            [
                AppEvent::TimeEntryStopped { time_entry: a, .. },
                AppEvent::TimeEntryCreated { time_entry: b, .. },
            ] if a.id == first && b.id != first && b.is_running
        ),
        "{:?}",
        started.events
    );
    assert!(started.value.projects.contains(&f.ids.project_id));
}

/// The running timer is stopped with the stop's daily-limit clamp, and the
/// restart reports what the limit cut.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_restart_stops_the_running_timer_with_the_stop_clamp(pool: PgPool) {
    let f = fixture(&pool).await;
    let day = today(&pool, &f).await;
    let entry = stopped(&pool, &f, day, 14_350).await;
    start(&pool, &f).await.unwrap();
    let timer = ran_for(&pool, &f, 90).await;

    let restarted = restart(&pool, &f, entry).await.unwrap();

    assert_eq!(
        restarted.value.notice.as_deref(),
        Some(
            "Daily hours cannot exceed 240. The timer was stopped at 0:50; 0:40 over the limit was not recorded."
        )
    );
    let timer = row(&pool, timer).await;
    assert_eq!((timer.minutes, timer.is_running), (50, false));
    let entry = row(&pool, entry).await;
    assert_eq!((entry.minutes, entry.is_running), (14_350, true));
}

/// Refusals leave everything as it was, the running timer included.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn refused_restarts_answer_conflict_and_change_nothing(pool: PgPool) {
    let f = fixture(&pool).await;
    let day = today(&pool, &f).await;
    let yesterday = stopped(&pool, &f, day.pred_opt().unwrap(), 30).await;
    let submitted = stopped(&pool, &f, day, 30).await;
    sqlx::query!(
        "UPDATE time_entries SET state='submitted' WHERE id=$1",
        submitted
    )
    .execute(&pool)
    .await
    .unwrap();
    start(&pool, &f).await.unwrap();
    let running = ran_for(&pool, &f, 10).await;
    let snapshot = || async {
        sqlx::query_scalar!(
            "SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM time_entries e WHERE user_id=$1",
            f.context.subject_id
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let before = snapshot().await;

    assert_conflict(
        restart(&pool, &f, running).await,
        "The timer is already running",
    );
    assert_conflict(
        restart(&pool, &f, yesterday).await,
        "Only today's entries can be restarted",
    );
    // A locked entry is refused as every Timesheet write on it is.
    assert_conflict(
        restart(&pool, &f, submitted).await,
        "Approval or billing coverage requires the approval editing integration",
    );
    assert_eq!(snapshot().await, before);
}
