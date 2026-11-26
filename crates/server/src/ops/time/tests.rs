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
/// name only the person. An unknown person gets an organization of their own;
/// a session of an inactive or unknown person is signed out here, before the
/// operation runs.
async fn session_write(pool: &PgPool, user_id: Uuid) -> Result<ops::WriteTx, OpError> {
    let org_id = sqlx::query_scalar!("SELECT org_id FROM users WHERE id=$1", user_id)
        .fetch_optional(pool)
        .await
        .unwrap()
        .unwrap_or_else(Uuid::now_v7);
    ops::begin(pool, Caller::session(org_id, user_id)).await
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
    let mut write = session_write(pool, user_id).await?;
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
    let mut write = session_write(pool, user_id).await?;
    let result = start_timer(&mut write.tx, &write.actor, project_id, task_id, notes).await;
    let started = write.commit(result, &Recorder::default()).await?;
    Ok(started.entry)
}

/// The timer widget's stop, by the person who owns the timer.
async fn stop_entry_timer(
    pool: &PgPool,
    user_id: Uuid,
    entry_id: Uuid,
) -> Result<(TimeEntry, Option<String>), OpError> {
    let mut write = session_write(pool, user_id).await?;
    let result = stop_timer(&mut write.tx, &write.actor, entry_id).await;
    write.commit(result, &Recorder::default()).await
}

async fn fetch_time_entry_contexts(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<TimeEntryContext>, OpError> {
    let mut write = session_write(pool, user_id).await?;
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

async fn start(pool: &PgPool, ids: &SeedIds, events: &Recorder) -> Result<Started, OpError> {
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

    let running = start(&pool, &ids, &events).await.unwrap().entry;
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

/// Stopping needs only an active person, so an OAuth token unlocked for a
/// read scope must not reach it; `time:write` does.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_oauth_token_stops_a_timer_only_with_time_write(pool: PgPool) {
    use horae_core::permissions::oauth::ScopeSet;
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let running = start(&pool, &ids, &Recorder::default())
        .await
        .unwrap()
        .entry;
    let stop_with = async |scopes: &[Scope]| {
        let scopes = ScopeSet::of(scopes);
        let caller = Caller::oauth(
            ids.org_id,
            ids.user_id,
            Uuid::now_v7(),
            Uuid::now_v7(),
            scopes,
        );
        let mut write = ops::begin(&pool, caller).await.unwrap();
        write.actor.unlock_scope(Scope::TimeRead).unwrap();
        let result = stop_timer(&mut write.tx, &write.actor, running.id).await;
        write.commit(result, &Recorder::default()).await
    };

    let refused = stop_with(&[Scope::TimeRead, Scope::CatalogRead]).await;
    assert!(matches!(refused, Err(OpError::Forbidden(_))), "{refused:?}");
    let (stopped, _) = stop_with(&[Scope::TimeWrite]).await.unwrap();
    assert!(!stopped.is_running);
}

/// An Administrator tracks on every project, but an OAuth token never carries
/// that identity: through one, only assigned projects are trackable.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrators_oauth_token_tracks_only_assigned_projects(pool: PgPool) {
    use horae_core::permissions::oauth::ScopeSet;
    let ids = trackable(&pool, BuiltInProfile::Administrator).await;
    sqlx::query!("DELETE FROM assignments WHERE user_id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let options = async |caller: Caller| {
        let mut read = ops::begin_read(&pool, caller).await.unwrap();
        let context = TimesheetWriteContext {
            expected_requester: read.actor.requester(),
            subject_id: ids.user_id,
        };
        let result = tracking(&mut read.tx, &read.actor, &context).await;
        read.finish(result).await.unwrap()
    };

    let session = options(Caller::session(ids.org_id, ids.user_id)).await;
    assert_eq!(session.len(), 1);
    let oauth = Caller::oauth(
        ids.org_id,
        ids.user_id,
        Uuid::now_v7(),
        Uuid::now_v7(),
        ScopeSet::of(&Scope::ALL),
    );
    assert!(
        options(oauth.unlock(Scope::CatalogRead).unwrap())
            .await
            .is_empty()
    );
}

/// `catalog:read` lists what the person may track (FR-060) though it carries
/// no time write. `TimeWriteOwn` is in the Member floor, so a state without
/// it is invalid, and the actor refuses to load rather than list anything.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_oauth_token_without_time_write_lists_what_the_person_may_track(pool: PgPool) {
    use horae_core::permissions::oauth::ScopeSet;
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let caller = Caller::oauth(
        ids.org_id,
        ids.user_id,
        Uuid::now_v7(),
        Uuid::now_v7(),
        ScopeSet::of(&[Scope::CatalogRead]),
    )
    .unlock(Scope::CatalogRead)
    .unwrap();
    let mut read = ops::begin_read(&pool, caller).await.unwrap();
    let context = TimesheetWriteContext {
        expected_requester: read.actor.requester(),
        subject_id: ids.user_id,
    };
    let result = tracking(&mut read.tx, &read.actor, &context).await;
    assert_eq!(read.finish(result).await.unwrap().len(), 1);

    crate::ops::testing::drop_grant(&pool, &ids, Permission::TimeWriteOwn).await;
    let refused = ops::begin_read(&pool, caller).await.err();
    assert!(
        matches!(refused, Some(OpError::Forbidden(_))),
        "{refused:?}"
    );
}

/// A start while a timer runs stops it first, as its stop would (021 D-6),
/// and announces the stop before the new timer.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_start_stops_the_running_timer_first(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let events = Recorder::default();
    let first = start(&pool, &ids, &events).await.unwrap().entry;
    sqlx::query!(
        "UPDATE time_entries SET started_at = now() - interval '20 minutes' WHERE id = $1",
        first.id
    )
    .execute(&pool)
    .await
    .unwrap();
    events.take();

    let second = start(&pool, &ids, &events).await.unwrap();

    let (stopped, notice) = second.stopped.expect("the running timer was stopped");
    assert_eq!(
        (stopped.id, stopped.minutes, stopped.is_running, notice),
        (first.id, 20, false, None)
    );
    assert!(second.entry.is_running);
    assert!(
        matches!(
            events.take().as_slice(),
            [
                AppEvent::TimeEntryStopped { time_entry: a, .. },
                AppEvent::TimeEntryCreated { time_entry: b, .. },
            ] if a.id == first.id && b.id == second.entry.id
        ),
        "the stop is announced before the start"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refused_start_announces_nothing(pool: PgPool) {
    let ids = trackable(&pool, BuiltInProfile::Member).await;
    let events = Recorder::default();
    start(&pool, &ids, &events).await.unwrap();
    events.take();
    sqlx::query!(
        "UPDATE project_tasks SET active = false WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let second = start(&pool, &ids, &events).await;
    assert!(matches!(second, Err(OpError::Conflict(_))), "{second:?}");
    assert!(events.take().is_empty());
    let running = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM time_entries WHERE user_id = $1 AND is_running"#,
        ids.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(running, 1, "a refused start leaves the running timer alone");
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
    let running = start(&pool, &ids, &events).await.unwrap().entry;
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
