use super::*;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use std::time::Duration;

pub(super) async fn editable_entry(pool: &PgPool, billable: bool) -> TimeEntry {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, $3)",
        ids.project_id,
        ids.task_id,
        billable,
    )
    .execute(pool)
    .await
    .unwrap();
    insert_time_entry(
        pool,
        ids.user_id,
        NewTimeEntry {
            project_id: ids.project_id,
            task_id: ids.task_id,
            spent_date: "2026-09-07".parse().unwrap(),
            minutes: 60,
            notes: Some("Kept"),
            billable,
            start_minute: Some(540),
            is_running: false,
        },
    )
    .await
    .unwrap()
}

async fn restrict_task(pool: &PgPool, entry: &TimeEntry) {
    sqlx::query!(
        "INSERT INTO project_task_settings (id, org_id, project_id, task_id, restricted)
         VALUES ($1, $2, $3, $4, true)",
        uuid::Uuid::now_v7(),
        entry.org_id,
        entry.project_id,
        entry.task_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn restricted_empty_task_rejects_picker_manual_and_timer_starts(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    restrict_task(&pool, &entry).await;
    assert!(
        fetch_time_entry_contexts(&pool, entry.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    for is_running in [false, true] {
        let result = insert_time_entry(
            &pool,
            entry.user_id,
            NewTimeEntry {
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: 15,
                notes: None,
                billable: true,
                start_minute: None,
                is_running,
            },
        )
        .await;
        assert!(matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn restriction_changes_are_rechecked_before_manual_or_timer_insertion(pool: PgPool) {
    for is_running in [false, true] {
        for revoke_grant in [false, true] {
            let entry = editable_entry(&pool, true).await;
            restrict_task(&pool, &entry).await;
            if revoke_grant {
                grant_task(&pool, &entry).await;
            } else {
                sqlx::query!("UPDATE project_task_settings SET restricted = false WHERE project_id = $1 AND task_id = $2",
                    entry.project_id, entry.task_id).execute(&pool).await.unwrap();
            }
            let mut revoke = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *revoke)
                .await
                .unwrap()
                .unwrap();
            if revoke_grant {
                sqlx::query!("DELETE FROM project_task_members WHERE project_id = $1 AND task_id = $2 AND user_id = $3",
                    entry.project_id, entry.task_id, entry.user_id).execute(&mut *revoke).await.unwrap();
            } else {
                sqlx::query!("UPDATE project_task_settings SET restricted = true WHERE project_id = $1 AND task_id = $2",
                    entry.project_id, entry.task_id).execute(&mut *revoke).await.unwrap();
            }
            let run_pool = pool.clone();
            let mut runs = tokio::task::JoinSet::new();
            runs.spawn(async move {
                insert_time_entry(
                    &run_pool,
                    entry.user_id,
                    NewTimeEntry {
                        project_id: entry.project_id,
                        task_id: entry.task_id,
                        spent_date: entry.spent_date,
                        minutes: 15,
                        notes: None,
                        billable: true,
                        start_minute: None,
                        is_running,
                    },
                )
                .await
            });
            wait_for_blocked(&pool, blocker).await;
            revoke.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), runs.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(matches!(
                result,
                Err(ServerFnError::ServerError { code: CONFLICT, .. })
            ));
            assert_eq!(
                sqlx::query_scalar!(
                    "SELECT count(*) FROM time_entries WHERE user_id = $1",
                    entry.user_id
                )
                .fetch_one(&pool)
                .await
                .unwrap(),
                Some(1)
            );
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoked_timer_stop_preserves_the_open_state_guard(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    sqlx::query!("UPDATE time_entries SET is_running = true, started_at = now(), state = 'submitted' WHERE id = $1", entry.id)
        .execute(&pool).await.unwrap();
    restrict_task(&pool, &entry).await;
    let result = stop_entry_timer(&pool, entry.user_id, entry.id).await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let row = sqlx::query!(
        "SELECT is_running, minutes FROM time_entries WHERE id = $1",
        entry.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.is_running);
    assert_eq!(row.minutes, 60);
}

async fn grant_task(pool: &PgPool, entry: &TimeEntry) {
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        uuid::Uuid::now_v7(),
        entry.project_id,
        entry.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_task_members (id, org_id, project_id, task_id, user_id)
         VALUES ($1, $2, $3, $4, $5)",
        uuid::Uuid::now_v7(),
        entry.org_id,
        entry.project_id,
        entry.task_id,
        entry.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn assignment_cascade_allows_an_inflight_entry_project_fk_to_finish(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    restrict_task(&pool, &entry).await;
    grant_task(&pool, &entry).await;
    let actor = sqlx::query_as!(User,
        r#"SELECT id, org_id, email, name, oidc_subject,
        cost_rate_cents, billable_rate_cents, active, created_at AS "created_at: chrono::DateTime<chrono::Utc>"
        FROM users WHERE id=$1"#, entry.user_id,
    ).fetch_one(&pool).await.unwrap();
    let assignment = sqlx::query_scalar!(
        "SELECT id FROM assignments WHERE project_id=$1 AND user_id=$2",
        entry.project_id,
        entry.user_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut barrier = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *barrier)
        .await
        .unwrap();
    sqlx::query!("LOCK TABLE time_entries IN SHARE MODE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let mut requests = tokio::task::JoinSet::new();
    let db = pool.clone();
    requests.spawn(async move {
        insert_time_entry(
            &db,
            entry.user_id,
            NewTimeEntry {
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: 15,
                notes: None,
                billable: true,
                is_running: false,
                start_minute: None,
            },
        )
        .await
        .map(|_| ())
    });
    wait_for_blocked(&pool, blocker).await;
    let writer = sqlx::query_scalar!(
        "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker,
    ).fetch_one(&pool).await.unwrap();
    let db = pool.clone();
    requests.spawn(async move {
        crate::server_fns::projects::remove_assignment(&db, &actor, assignment)
            .await
            .map(|removed| assert!(removed.is_some()))
    });
    wait_for_blocked(&pool, writer).await;
    barrier.commit().await.unwrap();
    while let Some(result) = tokio::time::timeout(Duration::from_secs(10), requests.join_next())
        .await
        .unwrap()
    {
        result.unwrap().unwrap();
    }
    let remaining = sqlx::query!(
        "SELECT (SELECT count(*) FROM time_entries WHERE project_id=$1) AS entries,
                (SELECT count(*) FROM assignments WHERE project_id=$1) AS assignments,
                (SELECT count(*) FROM project_task_members WHERE project_id=$1) AS members",
        entry.project_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (remaining.entries, remaining.assignments, remaining.members),
        (Some(2), Some(0), Some(0))
    );
    let denied = insert_time_entry(
        &pool,
        entry.user_id,
        NewTimeEntry {
            project_id: entry.project_id,
            task_id: entry.task_id,
            spent_date: entry.spent_date,
            minutes: 15,
            notes: None,
            billable: true,
            is_running: false,
            start_minute: None,
        },
    )
    .await;
    assert!(
        denied.is_err(),
        "removed task allowance must deny the next entry"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn task_grants_allow_tracking_with_or_without_administrator_state(pool: PgPool) {
    for administrator in [false, true] {
        let entry = editable_entry(&pool, true).await;
        restrict_task(&pool, &entry).await;
        grant_task(&pool, &entry).await;
        sqlx::query!(
            "UPDATE person_permission_states SET is_administrator = $2 WHERE user_id = $1",
            entry.user_id,
            administrator
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            fetch_time_entry_contexts(&pool, entry.user_id)
                .await
                .unwrap()
                .len(),
            1
        );
        let running = insert_time_entry(
            &pool,
            entry.user_id,
            NewTimeEntry {
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: 0,
                notes: None,
                billable: true,
                start_minute: None,
                is_running: true,
            },
        )
        .await
        .unwrap();
        stop_entry_timer(&pool, entry.user_id, running.id)
            .await
            .unwrap();
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn revoked_grant_does_not_trap_an_own_running_timer(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let running = insert_time_entry(
        &pool,
        entry.user_id,
        NewTimeEntry {
            project_id: entry.project_id,
            task_id: entry.task_id,
            spent_date: entry.spent_date,
            minutes: 0,
            notes: None,
            billable: true,
            start_minute: None,
            is_running: true,
        },
    )
    .await
    .unwrap();
    restrict_task(&pool, &entry).await;
    let foreign_user = seed(&pool, BuiltInProfile::Member).await;
    let foreign = stop_entry_timer(&pool, foreign_user.user_id, running.id).await;
    assert!(matches!(
        foreign,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    let (stopped, _) = stop_entry_timer(&pool, entry.user_id, running.id)
        .await
        .unwrap();
    assert!(!stopped.is_running);
    assert!(stopped.started_at.is_none());
    assert_eq!(stopped.state, EntryState::Open);
}
