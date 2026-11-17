//! The timer operations through `ops::begin`, as the session adapters run them.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

use super::*;
use crate::ops::testing::Recorder;
use crate::ops::{self, Caller};
use crate::testing::{SeedIds, seed};

mod activity;
mod entries;
mod timesheet;
mod update;

/// The timer operations run as the session adapters run them, for tests that
/// name only the person. An unknown person gets an organization of their own,
/// so the actor carries no authority.
async fn session_write(pool: &PgPool, user_id: Uuid) -> ops::WriteTx {
    let org_id = sqlx::query_scalar!("SELECT org_id FROM users WHERE id=$1", user_id)
        .fetch_optional(pool)
        .await
        .unwrap()
        .unwrap_or_else(Uuid::now_v7);
    ops::begin(pool, Caller::session(org_id, user_id))
        .await
        .unwrap()
}

/// New own time for tests: a running entry starts the timer (which files it on
/// today, with no minutes, start or billability request); any other is created
/// on the person's own Timesheet.
struct NewTimeEntry<'a> {
    project_id: Uuid,
    task_id: Uuid,
    spent_date: NaiveDate,
    minutes: i32,
    notes: Option<&'a str>,
    billable: bool,
    start_minute: Option<i32>,
    is_running: bool,
}

async fn insert_time_entry(
    pool: &PgPool,
    user_id: Uuid,
    input: NewTimeEntry<'_>,
) -> Result<TimeEntry, OpError> {
    use crate::models::permission_editor::PermissionRequester;

    if input.is_running {
        return start_timer_record(pool, user_id, input.project_id, input.task_id, input.notes)
            .await;
    }
    let mut write = session_write(pool, user_id).await;
    let context = TimesheetWriteContext {
        expected_requester: PermissionRequester {
            org_id: write.actor.org_id(),
            user_id,
        },
        subject_id: user_id,
    };
    let command = TimesheetCommand::Create {
        entry: TimesheetEntryInput {
            project_id: input.project_id,
            task_id: input.task_id,
            spent_date: input.spent_date,
            minutes: input.minutes,
            notes: input.notes.map(str::to_owned),
            billable: input.billable,
            start_minute: input.start_minute,
        },
    };
    let result = apply(&mut write.tx, &write.actor, &context, command).await;
    let events = Recorder::default();
    write.commit(result, &events).await?;
    let [AppEvent::TimeEntryCreated { time_entry, .. }] = &events.take()[..] else {
        panic!("a created entry announces itself once");
    };
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
           FROM time_entries WHERE id = $1"#,
        time_entry.id,
    )
    .fetch_one(pool)
    .await
    .unwrap())
}

async fn start_timer_record(
    pool: &PgPool,
    user_id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
    notes: Option<&str>,
) -> Result<TimeEntry, OpError> {
    let mut write = session_write(pool, user_id).await;
    let result = start_timer(&mut write.tx, &write.actor, project_id, task_id, notes).await;
    write.commit(result, &Recorder::default()).await
}

/// The timer widget's stop, by the person who owns the timer.
async fn stop_entry_timer(
    pool: &PgPool,
    user_id: Uuid,
    entry_id: Uuid,
) -> Result<(TimeEntry, Option<String>), OpError> {
    let mut write = session_write(pool, user_id).await;
    let result = stop_timer(&mut write.tx, &write.actor, entry_id).await;
    write.commit(result, &Recorder::default()).await
}

async fn fetch_time_entry_contexts(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<TimeEntryContext>, OpError> {
    let mut write = session_write(pool, user_id).await;
    let result = contexts(&mut write.tx, &write.actor).await.map(Done::new);
    write.commit(result, &Recorder::default()).await
}

#[test]
fn snaps_unaligned_start_to_the_grid() {
    // A drag-move can hand in a sub-grid start (e.g. 4:04); every write path
    // must snap it (FR-008). 244 → 240 (4:00), duration unchanged.
    assert_eq!(normalize_start(210, Some(244)), Ok((210, Some(240))));
}

#[test]
fn clamps_duration_against_the_snapped_start() {
    // Snap first (1430 → 1425), then clamp so start + minutes never crosses
    // midnight (FR-012): 1425 + 60 would exceed the day, clamped to 15.
    assert_eq!(normalize_start(60, Some(1430)), Ok((15, Some(1425))));
}

#[test]
fn untimed_start_is_left_untouched() {
    assert_eq!(normalize_start(90, None), Ok((90, None)));
}

#[test]
fn rejects_out_of_range_start_as_invalid_input() {
    for start in [-1, 1440] {
        assert!(
            matches!(
                normalize_start(60, Some(start)),
                Err(OpError::Invalid {
                    field: Some("start_minute"),
                    ..
                })
            ),
            "{start}"
        );
    }
}

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

async fn start(pool: &PgPool, ids: &SeedIds, events: &Recorder) -> Result<TimeEntry, OpError> {
    let mut write = ops::begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = start_timer(
        &mut write.tx,
        &write.actor,
        ids.project_id,
        ids.task_id,
        None,
    )
    .await;
    write.commit(result, events).await
}

async fn stop(
    pool: &PgPool,
    ids: &SeedIds,
    entry_id: Uuid,
    events: &Recorder,
) -> Result<(TimeEntry, Option<String>), OpError> {
    let mut write = ops::begin(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = stop_timer(&mut write.tx, &write.actor, entry_id).await;
    write.commit(result, events).await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn starting_and_stopping_announce_the_entry_after_commit(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let events = Recorder::default();

    let running = start(&pool, &ids, &events).await.unwrap();
    let started = events.take();
    assert!(
        matches!(
            started.as_slice(),
            [AppEvent::TimeEntryCreated { time_entry, .. }]
                if time_entry.id == running.id && time_entry.is_running
        ),
        "{started:?}"
    );

    let (stopped, _) = stop(&pool, &ids, running.id, &events).await.unwrap();
    let after = events.take();
    assert!(
        matches!(
            after.as_slice(),
            [AppEvent::TimeEntryStopped { time_entry, .. }]
                if time_entry.id == stopped.id && !time_entry.is_running
        ),
        "{after:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refused_start_announces_nothing(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let events = Recorder::default();
    start(&pool, &ids, &events).await.unwrap();
    events.take();

    let second = start(&pool, &ids, &events).await;
    assert!(matches!(second, Err(OpError::Conflict(_))), "{second:?}");
    assert!(events.take().is_empty());
}

/// Starting a timer writes own time, so it needs the time-write grant like the
/// Timesheet does; being active and assigned is not enough. `TimeWriteOwn` is
/// part of the Member floor, so a stored state without it is invalid and the
/// actor fails closed before the grant check runs.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_start_without_time_write_is_refused(pool: PgPool) {
    for missing in ["state", "grant"] {
        let ids = trackable(&pool, BuiltInProfile::Member).await;
        match missing {
            "state" => crate::testing::clear_state(&pool, ids.user_id).await,
            _ => {
                sqlx::query!(
                    "UPDATE person_permission_states
                     SET grants = array_remove(grants, 'time_write_own') WHERE user_id = $1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }
        let events = Recorder::default();

        let refused = start(&pool, &ids, &events).await;

        assert!(
            matches!(refused, Err(OpError::Forbidden(_))),
            "{missing}: {refused:?}"
        );
        let entries = sqlx::query_scalar!(
            r#"SELECT count(*) as "count!" FROM time_entries WHERE user_id = $1"#,
            ids.user_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(entries, 0, "{missing}");
        assert!(events.take().is_empty(), "{missing}");
    }
}

/// Revoking task access must not trap a running timer: the stop skips the
/// task check on purpose.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_timer_stops_after_its_task_is_restricted(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let events = Recorder::default();
    let running = start(&pool, &ids, &events).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_task_settings (id, org_id, project_id, task_id, restricted)
         VALUES ($1, $2, $3, $4, true)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let (stopped, _) = stop(&pool, &ids, running.id, &events).await.unwrap();
    assert!(!stopped.is_running);
}
