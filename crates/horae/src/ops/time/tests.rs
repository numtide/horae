//! The timer operations through `ops::begin`, as the session adapters run them.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

use super::*;
use crate::ops::testing::Recorder;
use crate::ops::{self, Caller};
use crate::server_fns::test_seed::{SeedIds, seed};

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
            "state" => crate::server_fns::test_seed::clear_state(&pool, ids.user_id).await,
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
