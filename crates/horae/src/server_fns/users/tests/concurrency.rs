use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;
use tokio::{task::JoinSet, time::timeout};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum Change {
    Demote,
    Deactivate,
}

async fn remove_admin(
    pool: &PgPool,
    org: Uuid,
    user: Uuid,
    change: Change,
) -> Result<User, ServerFnError> {
    match change {
        Change::Demote => change_user_role(pool, org, user, OrgRole::Member)
            .await
            .map(|(user, _)| user),
        Change::Deactivate => change_user_active(pool, org, user, false)
            .await
            .map(|(user, _)| user),
    }
}

async fn blocked_connections(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM pg_stat_activity
           WHERE datname = current_database() AND cardinality(pg_blocking_pids(pid)) > 0"#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn concurrent_changes_keep_an_admin(pool: &PgPool, first: Change, second: Change) {
    let ids = seed(pool, OrgRole::Admin).await;
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role)
         VALUES ($1, $2, $3, 'Other admin', 'admin')",
        other,
        ids.org_id,
        format!("{other}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();

    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let mut changes = JoinSet::new();
    let db = pool.clone();
    changes.spawn(async move { remove_admin(&db, ids.org_id, ids.user_id, first).await });

    timeout(std::time::Duration::from_secs(10), async {
        while blocked_connections(pool).await < 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first change must reach the held user lock");

    let db = pool.clone();
    changes.spawn(async move { remove_admin(&db, ids.org_id, other, second).await });

    // The second request either waits behind the first transaction or commits
    // while the first is paused. Observe that boundary, not a timing assumption.
    let early = timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(result) = changes.try_join_next() {
                break Some(result.unwrap());
            }
            if blocked_connections(pool).await >= 2 {
                break None;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("second change must complete or wait for the organization lock");
    blocker.commit().await.unwrap();

    let mut results: Vec<_> = early.into_iter().collect();
    timeout(std::time::Duration::from_secs(10), async {
        while let Some(result) = changes.join_next().await {
            results.push(result.unwrap());
        }
    })
    .await
    .expect("changes must finish after releasing the row lock");

    let remaining = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users
           WHERE org_id = $1 AND active AND org_role = 'admin'"#,
        ids.org_id,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(remaining, 1, "concurrent changes removed every admin");
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .any(|r| matches!(r, Err(ServerFnError::ServerError { code: CONFLICT, .. })))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn demotion_and_deactivation_keep_an_active_admin(pool: PgPool) {
    concurrent_changes_keep_an_admin(&pool, Change::Demote, Change::Deactivate).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn deactivation_and_demotion_keep_an_active_admin(pool: PgPool) {
    concurrent_changes_keep_an_admin(&pool, Change::Deactivate, Change::Demote).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_demotions_keep_an_active_admin(pool: PgPool) {
    concurrent_changes_keep_an_admin(&pool, Change::Demote, Change::Demote).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_deactivations_keep_an_active_admin(pool: PgPool) {
    concurrent_changes_keep_an_admin(&pool, Change::Deactivate, Change::Deactivate).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn the_last_admin_cannot_be_demoted_or_deactivated(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for change in [Change::Demote, Change::Deactivate] {
        assert!(matches!(
            remove_admin(&pool, ids.org_id, ids.user_id, change).await,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn unchanged_admin_access_is_allowed(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let (user, role) = change_user_role(&pool, ids.org_id, ids.user_id, OrgRole::Admin)
        .await
        .unwrap();
    assert_eq!(role, Some(user.org_role));
    let (user, active) = change_user_active(&pool, ids.org_id, ids.user_id, true)
        .await
        .unwrap();
    assert_eq!(active, Some(user.active));
}

#[sqlx::test(migrations = "./migrations")]
async fn access_changes_cannot_target_another_organization(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for change in [Change::Demote, Change::Deactivate] {
        assert!(matches!(
            remove_admin(&pool, ids.org_id, foreign.user_id, change).await,
            Err(ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            })
        ));
    }
}
