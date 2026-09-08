use super::*;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use tokio::{task::JoinSet, time::timeout};

async fn account(pool: &PgPool) -> (Uuid, String) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    (ids.user_id, format!("{}@test.com", ids.user_id))
}

async fn wait_for_links(pool: &PgPool, count: i64) {
    timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting = sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "count!" FROM pg_stat_activity
                   WHERE datname = current_database() AND cardinality(pg_blocking_pids(pid)) > 0"#
            )
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting >= count {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("identity resolution must wait on the held account row");
}

async fn concurrent_links(pool: &PgPool, subjects: [&str; 2]) {
    let (id, email) = account(pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", id)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let mut attempts = JoinSet::new();
    for subject in subjects {
        let db = pool.clone();
        let identity = Identity {
            subject: subject.into(),
            email: Some(email.clone()),
            email_verified: true,
        };
        attempts.spawn(async move {
            let user = resolve_user(&db, &identity).await.unwrap();
            (identity.subject, user)
        });
    }
    wait_for_links(pool, 2).await;
    blocker.commit().await.unwrap();

    let results = timeout(std::time::Duration::from_secs(10), async {
        let mut results = Vec::new();
        while let Some(result) = attempts.join_next().await {
            results.push(result.unwrap());
        }
        results
    })
    .await
    .unwrap();
    let granted = results.iter().filter(|(_, user)| user.is_some()).count();
    assert_eq!(granted, if subjects[0] == subjects[1] { 2 } else { 1 });
    let linked = sqlx::query_scalar!("SELECT oidc_subject FROM users WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .unwrap();
    for (subject, user) in results {
        if let Some(user) = user {
            assert_eq!(linked.as_deref(), Some(subject.as_str()));
            assert_eq!(user.id, id);
            assert_eq!(user.oidc_subject.as_deref(), Some(subject.as_str()));
            assert!(user.active);
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn different_subjects_cannot_both_claim_an_unlinked_account(pool: PgPool) {
    concurrent_links(&pool, ["subject-a", "subject-b"]).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn simultaneous_logins_with_the_same_subject_are_allowed(pool: PgPool) {
    concurrent_links(&pool, ["same-subject", "same-subject"]).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn deactivation_during_first_link_denies_login(pool: PgPool) {
    let (id, email) = account(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", id)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let mut attempts = JoinSet::new();
    let db = pool.clone();
    attempts.spawn(async move {
        resolve_user(
            &db,
            &Identity {
                subject: "new-subject".into(),
                email: Some(email),
                email_verified: true,
            },
        )
        .await
        .unwrap()
    });
    wait_for_links(&pool, 1).await;
    blocker.commit().await.unwrap();
    let user = timeout(std::time::Duration::from_secs(10), attempts.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        user.is_none(),
        "deactivated account must not receive a session"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn existing_subject_wins_over_another_accounts_email(pool: PgPool) {
    let (id, email) = account(&pool).await;
    let (_, other_email) = account(&pool).await;
    let mut identity = Identity {
        subject: "linked-subject".into(),
        email: Some(email),
        email_verified: true,
    };
    assert_eq!(
        resolve_user(&pool, &identity).await.unwrap().unwrap().id,
        id
    );
    identity.email = Some(other_email);
    assert_eq!(
        resolve_user(&pool, &identity).await.unwrap().unwrap().id,
        id
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn unverified_email_cannot_link_an_account(pool: PgPool) {
    let (_, email) = account(&pool).await;
    let identity = Identity {
        subject: "unverified-subject".into(),
        email: Some(email),
        email_verified: false,
    };
    assert!(resolve_user(&pool, &identity).await.unwrap().is_none());
}
