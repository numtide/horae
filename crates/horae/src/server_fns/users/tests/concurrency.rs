use super::*;
use crate::ops::testing::Recorder;
use crate::testing::seed;
use sqlx::PgPool;
use tokio::{task::JoinSet, time::timeout};
use uuid::Uuid;

async fn archive(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    user: Uuid,
) -> Result<UserListItem, ServerFnError> {
    change_user_active(
        pool,
        &Recorder::default(),
        Caller::session(org, actor),
        user,
        false,
    )
    .await
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

/// Two Administrators archive each other at once: the target is locked before
/// the remaining administrator is, so both cannot pass the check. One wins and
/// the other is refused, or the deadlock check ends it.
async fn archiving_each_other_keeps_an_administrator(pool: &PgPool) {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    let other = second_administrator(pool, ids.org_id).await;
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
    .expect("changes must finish");
    let remaining = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users u
           JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
           WHERE u.org_id = $1 AND u.active AND p.is_administrator"#,
        ids.org_id,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(remaining, 1, "{results:?}");
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn administrators_archiving_each_other_keep_one(pool: PgPool) {
    archiving_each_other_keeps_an_administrator(&pool).await;
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
    archiving_each_other_keeps_an_administrator(&requests).await;
    requests.close().await;
}

/// The operation refuses archiving yourself, and the server function answers
/// with its named status.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_cannot_archive_themselves(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    second_administrator(&pool, ids.org_id).await;
    assert!(matches!(
        archive(&pool, ids.org_id, ids.user_id, ids.user_id).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
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
