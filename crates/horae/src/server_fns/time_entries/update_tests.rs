use super::*;
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

async fn editable_entry(pool: &PgPool, billable: bool) -> TimeEntry {
    let ids = seed(pool, OrgRole::Admin).await;
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
async fn restricted_empty_task_rejects_changes_and_noop_edits(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    restrict_task(&pool, &entry).await;
    for minutes in [60, 90] {
        let result = update_entry(
            &pool,
            entry.user_id,
            entry.id,
            minutes,
            Some("Kept"),
            true,
            Some(540),
        )
        .await;
        assert!(matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ));
    }
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id = $1", entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn restricted_reordering_rejects_the_whole_batch(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let allowed_task = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Allowed')",
        allowed_task,
        entry.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        entry.project_id,
        allowed_task
    )
    .execute(&pool)
    .await
    .unwrap();
    let allowed = insert_time_entry(
        &pool,
        entry.user_id,
        NewTimeEntry {
            project_id: entry.project_id,
            task_id: allowed_task,
            spent_date: entry.spent_date,
            minutes: 15,
            notes: None,
            billable: true,
            start_minute: None,
            is_running: false,
        },
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET start_minute = NULL WHERE id = $1",
        entry.id
    )
    .execute(&pool)
    .await
    .unwrap();
    restrict_task(&pool, &entry).await;
    let result = reorder_entries(
        &pool,
        entry.user_id,
        "2026-09-08".parse().unwrap(),
        &[allowed.id, entry.id],
    )
    .await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let rows = sqlx::query!(
        r#"SELECT spent_date as "spent_date: chrono::NaiveDate", sort_order FROM time_entries WHERE id = ANY($1)"#,
        &[allowed.id, entry.id]
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        rows.iter()
            .all(|row| row.spent_date == entry.spent_date && row.sort_order == 0)
    );
}

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
async fn restricted_task_rejects_reschedule_and_delete(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    restrict_task(&pool, &entry).await;
    let result = reschedule_entry(&pool, entry.user_id, entry.id, entry.spent_date, 600, 90).await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let result = delete_entry(&pool, entry.user_id, entry.id).await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id = $1", entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn task_grants_allow_tracking_and_archived_history_for_every_role(pool: PgPool) {
    for role in [OrgRole::Member, OrgRole::Manager, OrgRole::Admin] {
        let entry = editable_entry(&pool, true).await;
        restrict_task(&pool, &entry).await;
        grant_task(&pool, &entry).await;
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            entry.user_id,
            role as OrgRole
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
        sqlx::query!(
            "UPDATE projects SET active = false WHERE id = $1",
            entry.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE tasks SET active = false WHERE id = $1",
            entry.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            fetch_time_entry_contexts(&pool, entry.user_id)
                .await
                .unwrap()
                .is_empty()
        );
        let (edited, changed) = update_entry(&pool, entry.user_id, entry.id, 90, None, true, None)
            .await
            .unwrap();
        assert!(changed);
        reorder_entries(&pool, entry.user_id, entry.spent_date, &[entry.id])
            .await
            .unwrap();
        assert_eq!(
            reschedule_entry(&pool, entry.user_id, entry.id, entry.spent_date, 600, 90)
                .await
                .unwrap()
                .start_minute,
            Some(600)
        );
        assert_eq!(
            delete_entry(&pool, entry.user_id, entry.id)
                .await
                .unwrap()
                .minutes,
            edited.minutes
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
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
    let foreign = stop_entry_timer(&pool, uuid::Uuid::now_v7(), running.id).await;
    assert!(matches!(
        foreign,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    let stopped = stop_entry_timer(&pool, entry.user_id, running.id)
        .await
        .unwrap();
    assert!(!stopped.is_running);
    assert!(stopped.started_at.is_none());
    assert_eq!(stopped.state, EntryState::Open);
    assert!(
        update_entry(&pool, entry.user_id, stopped.id, 90, None, true, None)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn task_grant_revocation_committed_during_an_edit_is_rechecked(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    restrict_task(&pool, &entry).await;
    grant_task(&pool, &entry).await;
    let mut revoke = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "DELETE FROM project_task_members WHERE project_id = $1 AND task_id = $2 AND user_id = $3",
        entry.project_id,
        entry.task_id,
        entry.user_id
    )
    .execute(&mut *revoke)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut runs = tokio::task::JoinSet::new();
    runs.spawn(async move {
        update_entry(&run_pool, entry.user_id, entry.id, 90, None, true, None).await
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
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id = $1", entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn normalized_noop_preserves_the_row_and_updated_at(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let before = sqlx::query_scalar!(
        "SELECT xmin::text FROM time_entries WHERE id = $1",
        entry.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let (returned, changed) = update_entry(
        &pool,
        entry.user_id,
        entry.id,
        60,
        Some("Kept"),
        true,
        Some(541),
    )
    .await
    .unwrap();
    let after = sqlx::query_scalar!(
        "SELECT xmin::text FROM time_entries WHERE id = $1",
        entry.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((changed, returned, after), (false, entry, before));
}

#[sqlx::test(migrations = "./migrations")]
async fn effective_billability_noop_preserves_the_row(pool: PgPool) {
    let entry = editable_entry(&pool, false).await;
    let (returned, changed) = update_entry(
        &pool,
        entry.user_id,
        entry.id,
        60,
        Some("Kept"),
        true,
        Some(540),
    )
    .await
    .unwrap();
    assert_eq!((changed, returned), (false, entry));
}

#[sqlx::test(migrations = "./migrations")]
async fn competing_identical_edit_is_a_noop_after_the_first_commit(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    let expected_version = sqlx::query_scalar!(
        "UPDATE time_entries SET minutes = 121 WHERE id = $1 RETURNING xmin::text",
        entry.id
    )
    .fetch_one(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_entry(
            &run_pool,
            entry.user_id,
            entry.id,
            121,
            Some("Kept"),
            true,
            Some(540),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    first.commit().await.unwrap();
    let (returned, changed) = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    let version = sqlx::query_scalar!(
        "SELECT xmin::text FROM time_entries WHERE id = $1",
        entry.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (changed, returned.minutes, version),
        (false, 121, expected_version)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn competing_edit_that_restores_old_values_still_reports_a_change(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET minutes = 121 WHERE id = $1",
        entry.id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        update_entry(
            &run_pool,
            entry.user_id,
            entry.id,
            60,
            Some("Kept"),
            true,
            Some(540),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    first.commit().await.unwrap();
    let (returned, changed) = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!((changed, returned.minutes), (true, 60));
}

#[sqlx::test(migrations = "./migrations")]
async fn each_editable_field_changes_once_including_nulls(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    for (minutes, notes, billable, start) in [
        (61, Some("Kept"), true, Some(540)),
        (61, None, true, Some(540)),
        (61, Some(""), true, Some(540)),
        (61, Some(""), false, Some(540)),
        (61, Some(""), false, None),
        (61, Some(""), false, Some(600)),
    ] {
        let (updated, changed) = update_entry(
            &pool,
            entry.user_id,
            entry.id,
            minutes,
            notes,
            billable,
            start,
        )
        .await
        .unwrap();
        assert_eq!(
            (
                changed,
                updated.minutes,
                updated.notes.as_deref(),
                updated.billable,
                updated.start_minute
            ),
            (true, minutes, notes, billable, start)
        );
        let (repeated, changed) = update_entry(
            &pool,
            entry.user_id,
            entry.id,
            minutes,
            notes,
            billable,
            start,
        )
        .await
        .unwrap();
        assert_eq!((changed, repeated), (false, updated));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn missing_foreign_and_locked_entries_are_not_noops(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    for (user, id) in [
        (entry.user_id, uuid::Uuid::now_v7()),
        (uuid::Uuid::now_v7(), entry.id),
    ] {
        let error = update_entry(&pool, user, id, 60, Some("Kept"), true, Some(540))
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
    }
    for state in [
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ] {
        let before = sqlx::query_scalar!(
            "UPDATE time_entries SET state = $2 WHERE id = $1 RETURNING xmin::text",
            entry.id,
            state as EntryState,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let error = update_entry(
            &pool,
            entry.user_id,
            entry.id,
            60,
            Some("Kept"),
            true,
            Some(540),
        )
        .await
        .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
        let after = sqlx::query_scalar!(
            "SELECT xmin::text FROM time_entries WHERE id = $1",
            entry.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_lock_or_delete_is_rechecked_before_editing(pool: PgPool) {
    for delete in [false, true] {
        let entry = editable_entry(&pool, true).await;
        let mut first = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *first)
            .await
            .unwrap()
            .unwrap();
        if delete {
            sqlx::query!("DELETE FROM time_entries WHERE id = $1", entry.id)
                .execute(&mut *first)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "UPDATE time_entries SET state = 'submitted' WHERE id = $1",
                entry.id
            )
            .execute(&mut *first)
            .await
            .unwrap();
        }
        let run_pool = pool.clone();
        let mut run = tokio::task::JoinSet::new();
        run.spawn(async move {
            update_entry(&run_pool, entry.user_id, entry.id, 90, None, true, None).await
        });
        wait_for_blocked(&pool, blocker).await;
        first.commit().await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), run.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError { code: CONFLICT, .. }
        ));
        let minutes =
            sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id = $1", entry.id)
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert_eq!(minutes, if delete { None } else { Some(60) });
    }
}
