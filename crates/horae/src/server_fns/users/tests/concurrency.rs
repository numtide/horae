use super::*;
use crate::ops::testing::Recorder;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;
use tokio::{task::JoinSet, time::timeout};
use uuid::Uuid;

async fn archive(pool: &PgPool, org: Uuid, actor: Uuid, user: Uuid) -> Result<User, ServerFnError> {
    change_user_active(
        pool,
        &Recorder::default(),
        Caller::session(org, actor),
        user,
        false,
    )
    .await
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

/// A second active Administrator in `org`.
async fn second_administrator(pool: &PgPool, org: Uuid) -> Uuid {
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Other admin')",
        other,
        org,
        format!("{other}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    provisioning::provision(&mut connection, org, other, BuiltInProfile::Administrator)
        .await
        .unwrap();
    other
}

/// Two Administrators archive themselves at once: one wins and the other is
/// refused, whichever order the database settles on.
async fn concurrent_archives_keep_an_administrator(pool: &PgPool) {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    let other = second_administrator(pool, ids.org_id).await;

    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let mut changes = JoinSet::new();
    let db = pool.clone();
    changes.spawn(async move { archive(&db, ids.org_id, ids.user_id, ids.user_id).await });

    timeout(std::time::Duration::from_secs(10), async {
        while blocked_connections(pool).await < 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("first change must reach the held user lock");

    let db = pool.clone();
    changes.spawn(async move { archive(&db, ids.org_id, other, other).await });

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
    .expect("second change must complete or wait for a lock");
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
        r#"SELECT COUNT(*) AS "count!" FROM users u
           JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
           WHERE u.org_id = $1 AND u.active AND p.is_administrator"#,
        ids.org_id,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        remaining, 1,
        "concurrent changes removed every administrator"
    );
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .any(|r| matches!(r, Err(ServerFnError::ServerError { code: CONFLICT, .. }))),
        "{results:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_archives_keep_an_active_administrator(pool: PgPool) {
    concurrent_archives_keep_an_administrator(&pool).await;
}

/// Two Administrators archive each other at once: the target is locked before
/// the remaining administrator is, so both cannot pass the check.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn administrators_archiving_each_other_keep_one(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other = second_administrator(&pool, ids.org_id).await;
    let mut changes = JoinSet::new();
    for (actor, target) in [(ids.user_id, other), (other, ids.user_id)] {
        let db = pool.clone();
        changes.spawn(async move { archive(&db, ids.org_id, actor, target).await });
    }
    let mut results = Vec::new();
    timeout(std::time::Duration::from_secs(10), async {
        while let Some(result) = changes.join_next().await {
            results.push(result.unwrap());
        }
    })
    .await
    .unwrap();
    let remaining = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users WHERE org_id = $1 AND active"#,
        ids.org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 1, "{results:?}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn repeatable_read_default_cannot_remove_every_administrator(pool: PgPool) {
    let requests = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
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
    concurrent_archives_keep_an_administrator(&requests).await;
    requests.close().await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_last_administrator_cannot_be_archived(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    assert!(matches!(
        archive(&pool, ids.org_id, ids.user_id, ids.user_id).await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn restoring_an_active_administrator_is_allowed(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let events = Recorder::default();
    let user = change_user_active(
        &pool,
        &events,
        Caller::session(ids.org_id, ids.user_id),
        ids.user_id,
        true,
    )
    .await
    .unwrap();
    assert!(user.active);
    assert!(events.take().is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn access_changes_cannot_target_another_organization(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    assert!(matches!(
        archive(&pool, ids.org_id, ids.user_id, foreign.user_id).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
}
