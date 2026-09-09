use super::*;
use crate::server_fns::test_seed::{seed, time_entry, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;

#[sqlx::test(migrations = "./migrations")]
async fn submission_freezes_the_minutes_after_a_competing_edit(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15, round_dir = 'up' WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut edit = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *edit)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("UPDATE time_entries SET minutes = 121 WHERE id = $1", entry)
        .execute(&mut *edit)
        .await
        .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        submit_user_week(
            &run_pool,
            ids.user_id,
            ids.org_id,
            "2026-09-07".parse().unwrap(),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    edit.commit().await.unwrap();
    let (_, total) = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    let stored = sqlx::query!(
        "SELECT minutes, rounded_minutes FROM time_entries WHERE id = $1",
        entry
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (stored.minutes, stored.rounded_minutes, total),
        (121, Some(135), 121)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_uses_one_pool_connection(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let single = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(200))
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let result = submit_user_week(
        &single,
        ids.user_id,
        ids.org_id,
        "2026-09-07".parse().unwrap(),
    )
    .await;
    assert!(result.is_ok(), "{result:?}");
    single.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_rejects_a_start_that_is_not_the_organizations_week_start(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2026-09-08' WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = submit_user_week(
        &pool,
        ids.user_id,
        ids.org_id,
        "2026-09-08".parse().unwrap(),
    )
    .await;
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ),
        "{result:?}"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_waits_for_a_new_timer_before_checking_the_week(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let mut writer = crate::db::begin_time_entry_write(&pool, ids.user_id)
        .await
        .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        submit_user_week(
            &run_pool,
            ids.user_id,
            ids.org_id,
            "2026-09-07".parse().unwrap(),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, is_running, started_at, state)
         SELECT $1, org_id, user_id, project_id, task_id, spent_date, 0, billable, true, now(), 'open'
         FROM time_entries WHERE id = $2",
        uuid::Uuid::now_v7(),
        entry
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    let state = sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM time_entries WHERE id = $1"#,
        entry
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, EntryState::Open);
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelling_a_submission_releases_its_write_barrier(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let mut editor = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *editor)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("UPDATE time_entries SET minutes = 121 WHERE id = $1", entry)
        .execute(&mut *editor)
        .await
        .unwrap();
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move {
        submit_user_week(
            &run_pool,
            ids.user_id,
            ids.org_id,
            "2026-09-07".parse().unwrap(),
        )
        .await
    });
    wait_for_blocked(&pool, blocker).await;
    run.abort_all();
    assert!(run.join_next().await.unwrap().unwrap_err().is_cancelled());
    editor.rollback().await.unwrap();
    let writer = tokio::time::timeout(
        Duration::from_secs(5),
        crate::db::begin_time_entry_write(&pool, ids.user_id),
    )
    .await
    .expect("cancelled submission must release its lock")
    .unwrap();
    writer.rollback().await.unwrap();
    let (_, total) = submit_user_week(
        &pool,
        ids.user_id,
        ids.org_id,
        "2026-09-07".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(total, 60);
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_accepts_the_organizations_sunday_start(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE organizations SET week_start = 7 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (approval, total) = submit_user_week(
        &pool,
        ids.user_id,
        ids.org_id,
        "2026-09-06".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        (approval.period_start, approval.period_end, total),
        (
            "2026-09-06".parse().unwrap(),
            "2026-09-12".parse().unwrap(),
            60
        )
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_does_not_wait_for_another_users_writer(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let writer = crate::db::begin_time_entry_write(&pool, uuid::Uuid::now_v7())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        submit_user_week(
            &pool,
            ids.user_id,
            ids.org_id,
            "2026-09-07".parse().unwrap(),
        ),
    )
    .await
    .unwrap();
    assert!(result.is_ok(), "{result:?}");
    writer.rollback().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn submission_barrier_allows_shared_writers_for_the_same_user(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let first = crate::db::begin_time_entry_write(&pool, ids.user_id)
        .await
        .unwrap();
    let second = tokio::time::timeout(
        Duration::from_secs(5),
        crate::db::begin_time_entry_write(&pool, ids.user_id),
    )
    .await
    .expect("entry writers must not serialize each other")
    .unwrap();
    second.rollback().await.unwrap();
    first.rollback().await.unwrap();
}
