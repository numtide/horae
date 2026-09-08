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
