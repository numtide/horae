use super::*;
use crate::importers::harvest::{ApiImportError, account_switch};
use crate::server_fns::test_seed::{seed, wait_for_blocked};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use std::time::Duration;

const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[derive(Clone, Copy, Debug)]
enum Change {
    Connect,
    Disconnect,
    Switch,
}

const CHANGES: [Change; 3] = [Change::Connect, Change::Disconnect, Change::Switch];

async fn run(pool: &PgPool, org: Uuid, actor: Uuid, change: Change) -> anyhow::Result<()> {
    match change {
        Change::Connect => {
            store_for_attempt(
                pool,
                org,
                actor,
                KEY,
                "original",
                "replacement-access",
                "replacement-refresh",
                None,
                None,
                account_switch::Version {
                    account_generation: 0,
                    connection_revision: 1,
                },
            )
            .await
        }
        Change::Disconnect => disconnect(pool, org, actor)
            .await
            .map_err(|error| match error {
                ApiImportError::Other(inner) => inner,
                other => other.into(),
            }),
        Change::Switch => account_switch::change(pool, org, actor, "original", 0, 1).await,
    }
}

async fn connected(pool: &PgPool) -> crate::server_fns::test_seed::SeedIds {
    let ids = seed(pool, OrgRole::Admin).await;
    store(
        pool,
        ids.org_id,
        ids.user_id,
        KEY,
        "original",
        "access",
        "refresh",
        None,
        None,
    )
    .await
    .unwrap();
    ids
}

async fn snapshot(pool: &PgPool, org: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_array(
        (SELECT to_jsonb(c) FROM harvest_credentials c WHERE org_id = $1),
        (SELECT to_jsonb(b) FROM harvest_account_bindings b WHERE org_id = $1),
        (SELECT to_jsonb(g) FROM harvest_connection_generations g WHERE org_id = $1)
    ) AS "snapshot!""#,
        org
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn assert_denied(result: anyhow::Result<()>, change: Change) {
    assert!(
        matches!(
            result
                .as_ref()
                .err()
                .and_then(|e| e.downcast_ref::<ConnectionError>()),
            Some(ConnectionError::Unauthorized)
        ),
        "{change:?} must deny stale authority, got {result:?}",
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn connection_changes_require_current_tenant_bound_administrator(pool: PgPool) {
    for change in CHANGES {
        let ids = connected(&pool).await;
        let before = snapshot(&pool, ids.org_id).await;
        for (role, active) in [
            (OrgRole::Member, true),
            (OrgRole::Manager, true),
            (OrgRole::Admin, false),
        ] {
            sqlx::query!(
                "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                ids.user_id,
                role as OrgRole,
                active
            )
            .execute(&pool)
            .await
            .unwrap();
            assert_denied(run(&pool, ids.org_id, ids.user_id, change).await, change);
            assert_eq!(snapshot(&pool, ids.org_id).await, before);
        }
        let foreign = seed(&pool, OrgRole::Admin).await;
        for actor in [foreign.user_id, Uuid::now_v7()] {
            assert_denied(run(&pool, ids.org_id, actor, change).await, change);
            assert_eq!(snapshot(&pool, ids.org_id).await, before);
        }
        sqlx::query!(
            "UPDATE users SET org_role = 'admin', active = true WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        run(&pool, ids.org_id, ids.user_id, change).await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn disconnected_noop_still_requires_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let before = snapshot(&pool, ids.org_id).await;
    assert_denied(
        run(&pool, ids.org_id, ids.user_id, Change::Disconnect).await,
        Change::Disconnect,
    );
    assert_eq!(snapshot(&pool, ids.org_id).await, before);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn denied_first_connection_creates_no_binding_credentials_or_generation(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let before = snapshot(&pool, ids.org_id).await;
    let result = store_for_attempt(
        &pool,
        ids.org_id,
        ids.user_id,
        KEY,
        "original",
        "access",
        "refresh",
        None,
        None,
        account_switch::Version {
            account_generation: 0,
            connection_revision: 0,
        },
    )
    .await;
    assert_denied(result, Change::Connect);
    assert_eq!(snapshot(&pool, ids.org_id).await, before);
    for change in CHANGES {
        assert_denied(
            run(&pool, Uuid::now_v7(), ids.user_id, change).await,
            change,
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn connection_changes_recheck_authority_after_organization_wait(pool: PgPool) {
    let run_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!(
                    "SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL REPEATABLE READ"
                )
                .execute(connection)
                .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for change in CHANGES {
        for deactivate in [false, true] {
            let ids = connected(&pool).await;
            let before = snapshot(&pool, ids.org_id).await;
            let mut revoke = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *revoke)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            let db = run_pool.clone();
            let mut pending = tokio::task::JoinSet::new();
            pending.spawn(async move { run(&db, ids.org_id, ids.user_id, change).await });
            tokio::select! {
                result = pending.join_next() => panic!("{change:?} bypassed organization gate: {result:?}"),
                () = wait_for_blocked(&pool, blocker) => {},
            }
            sqlx::query!(
                "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                ids.user_id,
                if deactivate {
                    OrgRole::Admin
                } else {
                    OrgRole::Member
                } as OrgRole,
                !deactivate
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
            revoke.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), pending.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_denied(result, change);
            assert_eq!(snapshot(&pool, ids.org_id).await, before);
        }
    }
    run_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn connection_changes_recheck_authority_after_actor_only_wait(pool: PgPool) {
    for change in CHANGES {
        for deactivate in [false, true] {
            let ids = connected(&pool).await;
            let before = snapshot(&pool, ids.org_id).await;
            let mut revoke = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *revoke)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                "UPDATE users SET org_role = $2, active = $3 WHERE id = $1",
                ids.user_id,
                if deactivate {
                    OrgRole::Admin
                } else {
                    OrgRole::Member
                } as OrgRole,
                !deactivate
            )
            .execute(&mut *revoke)
            .await
            .unwrap();
            let db = pool.clone();
            let mut pending = tokio::task::JoinSet::new();
            pending.spawn(async move { run(&db, ids.org_id, ids.user_id, change).await });
            tokio::select! {
                result = pending.join_next() => panic!("{change:?} bypassed actor lock: {result:?}"),
                () = wait_for_blocked(&pool, blocker) => {},
            }
            revoke.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), pending.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_denied(result, change);
            assert_eq!(snapshot(&pool, ids.org_id).await, before);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn connection_changes_hold_authority_until_commit(pool: PgPool) {
    for change in CHANGES {
        let ids = connected(&pool).await;
        let before = snapshot(&pool, ids.org_id).await;
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *barrier)
            .await
            .unwrap()
            .unwrap();
        // Pause after authorization, before any credential change.
        sqlx::query!(
            "SELECT org_id FROM harvest_connection_generations WHERE org_id = $1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *barrier)
        .await
        .unwrap();
        let db = pool.clone();
        let mut write = tokio::task::JoinSet::new();
        write.spawn(async move { run(&db, ids.org_id, ids.user_id, change).await });
        wait_for_blocked(&pool, blocker).await;
        let writer = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            blocker,
        ).fetch_one(&pool).await.unwrap();
        let db = pool.clone();
        let mut revoke = tokio::task::JoinSet::new();
        revoke.spawn(async move {
            sqlx::query!(
                "UPDATE users SET org_role = 'member' WHERE id = $1",
                ids.user_id
            )
            .execute(&db)
            .await
        });
        tokio::select! {
            result = revoke.join_next() => panic!("{change:?} lost authority before commit: {result:?}"),
            () = wait_for_blocked(&pool, writer) => {},
        }
        barrier.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), write.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), revoke.join_next())
            .await
            .unwrap()
            .unwrap()
            .unwrap()
            .unwrap();
        let after = snapshot(&pool, ids.org_id).await;
        assert_ne!(after, before);
        assert_denied(run(&pool, ids.org_id, ids.user_id, change).await, change);
        assert_eq!(snapshot(&pool, ids.org_id).await, after);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn failed_connection_changes_roll_back_and_release_authority_and_reservation(pool: PgPool) {
    for change in CHANGES {
        let ids = connected(&pool).await;
        let before = snapshot(&pool, ids.org_id).await;
        sqlx::query!("ALTER TABLE harvest_connection_generations ADD CONSTRAINT reject_connection_change CHECK (connection_revision <= 1) NOT VALID")
            .execute(&pool).await.unwrap();
        assert!(run(&pool, ids.org_id, ids.user_id, change).await.is_err());
        assert_eq!(snapshot(&pool, ids.org_id).await, before);
        tokio::time::timeout(Duration::from_secs(5), async {
            let mut probe = pool.begin().await.unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *probe)
            .await
            .unwrap();
            sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
                .fetch_one(&mut *probe)
                .await
                .unwrap();
            probe.rollback().await.unwrap();
        })
        .await
        .expect("failure must release authorization locks");
        sqlx::query!(
            "ALTER TABLE harvest_connection_generations DROP CONSTRAINT reject_connection_change"
        )
        .execute(&pool)
        .await
        .unwrap();
        run(&pool, ids.org_id, ids.user_id, change).await.unwrap();
    }
}
