use super::update_tests::editable_entry;
use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
enum Operation {
    Create,
    Start,
    Stop,
    Update,
    Delete,
    Move,
    Reorder,
}

async fn attempt(
    pool: &PgPool,
    actor: Uuid,
    entry: &TimeEntry,
    operation: Operation,
) -> Result<(), ServerFnError> {
    match operation {
        Operation::Create | Operation::Start => insert_time_entry(
            pool,
            actor,
            NewTimeEntry {
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: 15,
                notes: None,
                billable: true,
                start_minute: None,
                is_running: matches!(operation, Operation::Start),
            },
        )
        .await
        .map(|_| ()),
        Operation::Stop => stop_entry_timer(pool, actor, entry.id).await.map(|_| ()),
        Operation::Update => update_entry(pool, actor, entry.id, 90, None, true, None)
            .await
            .map(|_| ()),
        Operation::Delete => delete_entry(pool, actor, entry.id).await.map(|_| ()),
        Operation::Move => reschedule_entry(pool, actor, entry.id, entry.spent_date, 600, 90)
            .await
            .map(|_| ()),
        Operation::Reorder => reorder_entries(pool, actor, entry.spent_date, &[entry.id]).await,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn every_time_mutation_denies_inactive_and_missing_owners_without_changes(pool: PgPool) {
    for role in [OrgRole::Member, OrgRole::Manager, OrgRole::Admin] {
        for operation in [
            Operation::Create,
            Operation::Start,
            Operation::Stop,
            Operation::Update,
            Operation::Delete,
            Operation::Move,
            Operation::Reorder,
        ] {
            let entry = editable_entry(&pool, true).await;
            sqlx::query!(
                "UPDATE time_entries SET start_minute=NULL, is_running=$2,
                started_at=CASE WHEN $2 THEN now() ELSE NULL END WHERE id=$1",
                entry.id,
                matches!(operation, Operation::Stop)
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE users SET active=false,org_role=$2 WHERE id=$1",
                entry.user_id,
                role as OrgRole
            )
            .execute(&pool)
            .await
            .unwrap();
            let before = sqlx::query_scalar!(
                "SELECT jsonb_agg(to_jsonb(e) ORDER BY e.id) FROM time_entries e WHERE user_id=$1",
                entry.user_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            for actor in [entry.user_id, Uuid::now_v7()] {
                let result = attempt(&pool, actor, &entry, operation).await;
                assert!(
                    matches!(
                        result,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{role:?} {operation:?}: {result:?}"
                );
                let after = sqlx::query_scalar!("SELECT jsonb_agg(to_jsonb(e) ORDER BY e.id) FROM time_entries e WHERE user_id=$1", entry.user_id)
                    .fetch_one(&pool).await.unwrap();
                assert_eq!(after, before);
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn waiting_time_writer_rechecks_committed_or_rolled_back_deactivation(pool: PgPool) {
    for organization_gate in [false, true] {
        for commit in [false, true] {
            let entry = editable_entry(&pool, true).await;
            let mut revoke = pool.begin().await.unwrap();
            if organization_gate {
                crate::db::lock_organization(
                    &mut revoke,
                    entry.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
            }
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", entry.user_id)
                .execute(&mut *revoke)
                .await
                .unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *revoke)
                .await
                .unwrap();
            let db = pool.clone();
            let input = entry.clone();
            let mut operations = tokio::task::JoinSet::new();
            operations
                .spawn(async move { attempt(&db, input.user_id, &input, Operation::Update).await });
            wait_for_blocked(&pool, blocker).await;
            if commit {
                revoke.commit().await.unwrap();
            } else {
                revoke.rollback().await.unwrap();
            }
            let result = operations.join_next().await.unwrap().unwrap();
            if commit {
                assert!(
                    matches!(
                        result,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{result:?}"
                );
            } else {
                result.unwrap();
            }
            let minutes =
                sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", entry.id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert_eq!(minutes, if commit { 60 } else { 90 });
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn admitted_time_writer_finishes_before_account_deactivation(pool: PgPool) {
    for organization_gate in [false, true] {
        let entry = editable_entry(&pool, true).await;
        let mut barrier = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM time_entries WHERE id=$1 FOR UPDATE",
            entry.id
        )
        .fetch_one(&mut *barrier)
        .await
        .unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *barrier)
            .await
            .unwrap();
        let db = pool.clone();
        let input = entry.clone();
        let mut operations = tokio::task::JoinSet::new();
        operations
            .spawn(async move { attempt(&db, input.user_id, &input, Operation::Update).await });
        wait_for_blocked(&pool, blocker).await;
        let writer = sqlx::query_scalar!("SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker)
            .fetch_one(&pool).await.unwrap();
        let db = pool.clone();
        let org_id = entry.org_id;
        let user_id = entry.user_id;
        operations.spawn(async move {
            let mut revoke = db.begin().await.map_err(server_err)?;
            if organization_gate {
                crate::db::lock_organization(
                    &mut revoke,
                    org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .map_err(server_err)?;
            }
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", user_id)
                .execute(&mut *revoke)
                .await
                .map_err(server_err)?;
            revoke.commit().await.map_err(server_err)
        });
        wait_for_blocked(&pool, writer).await;
        barrier.rollback().await.unwrap();
        while let Some(result) = operations.join_next().await {
            result.unwrap().unwrap();
        }
        assert_eq!(
            sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", entry.id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            90
        );
        assert!(matches!(
            attempt(&pool, entry.user_id, &entry, Operation::Update).await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelled_time_writer_releases_fences_and_preserves_pool_defaults(pool: PgPool) {
    let entry = editable_entry(&pool, true).await;
    let writer = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let mut barrier = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM time_entries WHERE id=$1 FOR UPDATE",
        entry.id
    )
    .fetch_one(&mut *barrier)
    .await
    .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *barrier)
        .await
        .unwrap();
    let db = writer.clone();
    let input = entry.clone();
    let mut operations = tokio::task::JoinSet::new();
    operations.spawn(async move { attempt(&db, input.user_id, &input, Operation::Update).await });
    wait_for_blocked(&pool, blocker).await;
    operations.abort_all();
    assert!(
        operations
            .join_next()
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    barrier.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", entry.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
    tokio::time::timeout(
        std::time::Duration::from_secs(6),
        attempt(&writer, entry.user_id, &entry, Operation::Update),
    )
    .await
    .unwrap()
    .unwrap();
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&writer).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    writer.close().await;
}
