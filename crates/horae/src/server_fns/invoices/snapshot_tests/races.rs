use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use std::{sync::Arc, time::Duration};

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_deny_winning_actor_revocation_without_revision_change(pool: PgPool) {
    for reader in [Reader::Invoice, Reader::Fees] {
        for gated in [false, true] {
            for deactivate in [false, true] {
                let ids = Arc::new(fee_tests::single_fee(&pool).await);
                let mut writer = pool.begin().await.unwrap();
                let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
                    .fetch_one(&mut *writer)
                    .await
                    .unwrap();
                if gated {
                    crate::db::lock_organization(
                        &mut writer,
                        ids.org_id,
                        crate::db::OrganizationLock::AccessChange,
                    )
                    .await
                    .unwrap();
                }
                if deactivate {
                    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                        .execute(&mut *writer)
                        .await
                        .unwrap();
                } else {
                    sqlx::query!(
                        "UPDATE users SET org_role='member' WHERE id=$1",
                        ids.user_id
                    )
                    .execute(&mut *writer)
                    .await
                    .unwrap();
                }
                let pending = {
                    let pool = pool.clone();
                    let ids = Arc::clone(&ids);
                    tokio::spawn(async move {
                        read(&pool, (ids.org_id, ids.user_id), &ids, reader).await
                    })
                };
                wait_for_blocked(&pool, pid).await;
                writer.commit().await.unwrap();
                assert_denied(
                    tokio::time::timeout(Duration::from_secs(5), pending)
                        .await
                        .unwrap()
                        .unwrap(),
                );
                assert_eq!(
                    sqlx::query_scalar!(
                        "SELECT access_revision FROM organizations WHERE id=$1",
                        ids.org_id
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                    0
                );
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_retry_revision_change_and_refresh_amounts(pool: PgPool) {
    for reader in [Reader::Invoice, Reader::Fees] {
        let ids = Arc::new(fee_tests::single_fee(&pool).await);
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let ids = Arc::clone(&ids);
            tokio::spawn(async move { read(&pool, (ids.org_id, ids.user_id), &ids, reader).await })
        };
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "UPDATE project_settings SET fee_amount_cents=25000 WHERE project_id=$1",
            ids.project_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            25000
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_hold_authority_until_materialization(pool: PgPool) {
    for reader in [Reader::Invoice, Reader::Fees] {
        let ids = Arc::new(fee_tests::single_fee(&pool).await);
        let mut blocker = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let blocker_pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let ids = Arc::clone(&ids);
            tokio::spawn(async move { read(&pool, (ids.org_id, ids.user_id), &ids, reader).await })
        };
        wait_for_blocked(&pool, blocker_pid).await;
        let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker_pid).fetch_one(&pool).await.unwrap().unwrap();
        let revoke = {
            let pool = pool.clone();
            let ids = Arc::clone(&ids);
            tokio::spawn(async move {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            })
        };
        wait_for_blocked(&pool, reader_pid).await;
        blocker.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            12500
        );
        tokio::time::timeout(Duration::from_secs(5), revoke)
            .await
            .unwrap()
            .unwrap();
        assert_denied(read(&pool, (ids.org_id, ids.user_id), &ids, reader).await);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_keep_fee_values_from_their_initial_snapshot(pool: PgPool) {
    for reader in [Reader::Invoice, Reader::Fees] {
        let ids = Arc::new(fee_tests::single_fee(&pool).await);
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *writer)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let ids = Arc::clone(&ids);
            tokio::spawn(async move { read(&pool, (ids.org_id, ids.user_id), &ids, reader).await })
        };
        wait_for_blocked(&pool, pid).await;
        sqlx::query!(
            "UPDATE project_settings SET fee_amount_cents=25000 WHERE project_id=$1",
            ids.project_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(5), pending)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            12500
        );
        assert_eq!(
            read(&pool, (ids.org_id, ids.user_id), &ids, reader)
                .await
                .unwrap(),
            25000
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn financial_snapshots_release_cancelled_reads_for_one_connection(pool: PgPool) {
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for reader in [Reader::Invoice, Reader::Fees] {
        let ids = Arc::new(fee_tests::single_fee(&pool).await);
        let mut blocker = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE project_settings IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        let pending = {
            let pool = reader_pool.clone();
            let ids = Arc::clone(&ids);
            tokio::spawn(async move { read(&pool, (ids.org_id, ids.user_id), &ids, reader).await })
        };
        wait_for_blocked(&pool, pid).await;
        pending.abort();
        assert!(pending.await.unwrap_err().is_cancelled());
        blocker.rollback().await.unwrap();
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(5),
                read(&reader_pool, (ids.org_id, ids.user_id), &ids, reader)
            )
            .await
            .unwrap()
            .unwrap(),
            12500
        );
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            ids.user_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        writer.rollback().await.unwrap();
    }
    reader_pool.close().await;
}
