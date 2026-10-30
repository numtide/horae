use super::linking::{identity, wait_for_links};
use crate::auth::oidc::link::{LinkError, LinkOutcome, Rejection, link};
use crate::auth::oidc::resolve_user;
use horae_core::types::OrgRole;
use sqlx::PgPool;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

const ISSUER: &str = "https://id.example.com";

async fn account(pool: &PgPool) -> Uuid {
    crate::server_fns::test_seed::seed(pool, OrgRole::Member)
        .await
        .user_id
}

async fn bind(pool: &PgPool, id: Uuid, issuer: Option<&str>, subject: &str) {
    sqlx::query!(
        "UPDATE users SET oidc_issuer = $2, oidc_subject = $3 WHERE id = $1",
        id,
        issuer,
        subject
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Every account column, so a test can prove a refusal or no-op wrote nothing.
async fn users(pool: &PgPool) -> String {
    sqlx::query_scalar!(r#"SELECT json_agg(u ORDER BY u.id)::text AS "users!" FROM users u"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn binding(pool: &PgPool, id: Uuid) -> (Option<String>, Option<String>) {
    let row = sqlx::query!(
        "SELECT oidc_issuer, oidc_subject FROM users WHERE id = $1",
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.oidc_issuer, row.oidc_subject)
}

fn rejected(result: Result<LinkOutcome, LinkError>) -> Rejection {
    match result {
        Err(LinkError::Rejected(rejection)) => rejection,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn links_an_unbound_account_exactly_and_repeats_as_a_no_op(pool: PgPool) {
    let id = account(&pool).await;
    let before = sqlx::query!(
        r#"SELECT email, name, org_role::text AS "role!", cost_rate_cents,
                  billable_rate_cents, active FROM users WHERE id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    // Case and the trailing slash are kept as given.
    let (issuer, subject) = ("https://id.example.com/", "Sub-ÄB c");

    assert_eq!(
        link(&pool, id, issuer, subject).await.unwrap(),
        LinkOutcome::Linked
    );
    assert_eq!(
        binding(&pool, id).await,
        (Some(issuer.into()), Some(subject.into()))
    );
    let after_link = users(&pool).await;

    assert_eq!(
        link(&pool, id, issuer, subject).await.unwrap(),
        LinkOutcome::Unchanged
    );
    assert_eq!(users(&pool).await, after_link);

    let after = sqlx::query!(
        r#"SELECT email, name, org_role::text AS "role!", cost_rate_cents,
                  billable_rate_cents, active FROM users WHERE id = $1"#,
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            before.email,
            before.name,
            before.role,
            before.cost_rate_cents
        ),
        (after.email, after.name, after.role, after.cost_rate_cents)
    );
    assert_eq!(
        (before.billable_rate_cents, before.active),
        (after.billable_rate_cents, after.active)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn records_the_issuer_on_a_legacy_binding_with_the_same_subject(pool: PgPool) {
    let id = account(&pool).await;
    bind(&pool, id, None, "legacy-subject").await;

    assert_eq!(
        link(&pool, id, ISSUER, "legacy-subject").await.unwrap(),
        LinkOutcome::IssuerRecorded
    );
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("legacy-subject".into()))
    );
    assert_eq!(
        link(&pool, id, ISSUER, "legacy-subject").await.unwrap(),
        LinkOutcome::Unchanged
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn refusals_write_nothing(pool: PgPool) {
    let unbound = account(&pool).await;
    let inactive = account(&pool).await;
    let holder = account(&pool).await;
    let bound = account(&pool).await;
    let legacy = account(&pool).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", inactive)
        .execute(&pool)
        .await
        .unwrap();
    bind(&pool, holder, Some(ISSUER), "held").await;
    bind(&pool, bound, Some(ISSUER), "bound").await;
    bind(&pool, legacy, None, "legacy").await;
    let before = users(&pool).await;

    let cases = [
        (Uuid::now_v7(), ISSUER, "free", Rejection::UserNotFound),
        (inactive, ISSUER, "free", Rejection::Inactive),
        (unbound, ISSUER, "held", Rejection::SubjectInUse),
        (bound, ISSUER, "other", Rejection::AlreadyLinked),
        (
            bound,
            "https://other.example",
            "bound",
            Rejection::AlreadyLinked,
        ),
        (legacy, ISSUER, "other", Rejection::AlreadyLinked),
        (legacy, ISSUER, "held", Rejection::AlreadyLinked),
    ];
    for (id, issuer, subject, expected) in cases {
        assert_eq!(
            rejected(link(&pool, id, issuer, subject).await),
            expected,
            "{subject}"
        );
    }
    assert_eq!(users(&pool).await, before);
}

async fn email(pool: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar!("SELECT email FROM users WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn a_subject_taken_by_an_uncommitted_link_is_refused_by_the_unique_index(pool: PgPool) {
    let first = account(&pool).await;
    let second = account(&pool).await;
    // Another link holds the subject but has not committed, so `second`'s
    // holder read sees nothing and only the unique index can refuse it.
    let mut other = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE users SET oidc_issuer = $2, oidc_subject = 'race' WHERE id = $1",
        first,
        ISSUER
    )
    .execute(&mut *other)
    .await
    .unwrap();

    let db = pool.clone();
    let racing = tokio::spawn(async move { link(&db, second, ISSUER, "race").await });
    wait_for_links(&pool, 1).await;
    other.commit().await.unwrap();

    let result = timeout(Duration::from_secs(10), racing)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rejected(result), Rejection::SubjectInUse);
    assert_eq!(binding(&pool, second).await, (None, None));
    assert_eq!(
        binding(&pool, first).await,
        (Some(ISSUER.into()), Some("race".into()))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_callback_ordered_after_a_link_of_its_subject_gets_no_session(pool: PgPool) {
    let linked = account(&pool).await;
    let unbound = account(&pool).await;
    let unbound_email = email(&pool, unbound).await;
    // Hold the unbound account so the callback's binding update waits.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", unbound)
        .fetch_one(&mut *hold)
        .await
        .unwrap();

    let db = pool.clone();
    let callback = tokio::spawn(async move {
        let token = identity(ISSUER, "contested", Some(unbound_email), true);
        resolve_user(&db, &token)
            .await
            .map(|user| user.map(|u| u.id))
    });
    wait_for_links(&pool, 1).await;
    assert_eq!(
        link(&pool, linked, ISSUER, "contested").await.unwrap(),
        LinkOutcome::Linked
    );
    hold.commit().await.unwrap();

    let resolved = timeout(Duration::from_secs(10), callback)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !matches!(resolved, Ok(Some(_))),
        "no session for {resolved:?}"
    );
    assert_eq!(binding(&pool, unbound).await, (None, None));
    assert_eq!(
        binding(&pool, linked).await,
        (Some(ISSUER.into()), Some("contested".into()))
    );
}

/// Lock `ids` so operations started next queue behind this transaction, in the
/// order they reach the lock. Committing it releases them in that order.
async fn hold(pool: &PgPool, ids: &[Uuid]) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = ANY($1) FOR UPDATE", ids)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx
}

type Spawned<T> = tokio::task::JoinHandle<T>;

fn spawn_link(
    pool: &PgPool,
    id: Uuid,
    subject: &'static str,
) -> Spawned<Result<LinkOutcome, LinkError>> {
    let db = pool.clone();
    tokio::spawn(async move { link(&db, id, ISSUER, subject).await })
}

/// A verified-email sign-in for `id`, reporting which account it resolved to.
fn spawn_callback(pool: &PgPool, email: String, subject: &'static str) -> Spawned<Option<Uuid>> {
    let db = pool.clone();
    tokio::spawn(async move {
        let token = identity(ISSUER, subject, Some(email), true);
        resolve_user(&db, &token).await.unwrap().map(|user| user.id)
    })
}

async fn finish<T>(task: Spawned<T>) -> T {
    timeout(Duration::from_secs(10), task)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn a_link_ordered_before_a_same_identity_callback_lets_it_resolve(pool: PgPool) {
    let id = account(&pool).await;
    let held = hold(&pool, &[id]).await;
    let linking = spawn_link(&pool, id, "same");
    wait_for_links(&pool, 1).await;
    let callback = spawn_callback(&pool, email(&pool, id).await, "same");
    wait_for_links(&pool, 2).await;
    held.commit().await.unwrap();

    assert_eq!(finish(linking).await.unwrap(), LinkOutcome::Linked);
    // The callback's conditional update finds the account bound and rereads
    // the pair: the same identity resolves to the same account.
    assert_eq!(finish(callback).await, Some(id));
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("same".into()))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_link_ordered_before_another_subjects_callback_keeps_its_binding(pool: PgPool) {
    let id = account(&pool).await;
    let held = hold(&pool, &[id]).await;
    let linking = spawn_link(&pool, id, "operator");
    wait_for_links(&pool, 1).await;
    let callback = spawn_callback(&pool, email(&pool, id).await, "provider");
    wait_for_links(&pool, 2).await;
    held.commit().await.unwrap();

    assert_eq!(finish(linking).await.unwrap(), LinkOutcome::Linked);
    assert_eq!(finish(callback).await, None);
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("operator".into()))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_callback_ordered_before_a_link_of_another_subject_is_never_replaced(pool: PgPool) {
    let id = account(&pool).await;
    let held = hold(&pool, &[id]).await;
    let callback = spawn_callback(&pool, email(&pool, id).await, "provider");
    wait_for_links(&pool, 1).await;
    let linking = spawn_link(&pool, id, "operator");
    wait_for_links(&pool, 2).await;
    held.commit().await.unwrap();

    assert_eq!(finish(callback).await, Some(id));
    assert_eq!(rejected(finish(linking).await), Rejection::AlreadyLinked);
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("provider".into()))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_callback_ordered_before_a_link_of_the_same_identity_makes_it_a_no_op(pool: PgPool) {
    let id = account(&pool).await;
    let held = hold(&pool, &[id]).await;
    let callback = spawn_callback(&pool, email(&pool, id).await, "same");
    wait_for_links(&pool, 1).await;
    let linking = spawn_link(&pool, id, "same");
    wait_for_links(&pool, 2).await;
    held.commit().await.unwrap();

    assert_eq!(finish(callback).await, Some(id));
    assert_eq!(finish(linking).await.unwrap(), LinkOutcome::Unchanged);
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("same".into()))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn two_links_of_one_subject_to_different_accounts_bind_exactly_one(pool: PgPool) {
    let first = account(&pool).await;
    let second = account(&pool).await;
    // Release both together so neither sees the other's binding first.
    let held = hold(&pool, &[first, second]).await;
    let a = spawn_link(&pool, first, "contested");
    let b = spawn_link(&pool, second, "contested");
    wait_for_links(&pool, 2).await;
    held.commit().await.unwrap();

    let results = [(first, finish(a).await), (second, finish(b).await)];
    let winners: Vec<Uuid> = results
        .iter()
        .filter(|(_, result)| matches!(result, Ok(LinkOutcome::Linked)))
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(winners.len(), 1, "{results:?}");
    for (id, result) in results {
        if id == winners[0] {
            assert_eq!(
                binding(&pool, id).await,
                (Some(ISSUER.into()), Some("contested".into()))
            );
        } else {
            assert_eq!(rejected(result), Rejection::SubjectInUse);
            assert_eq!(binding(&pool, id).await, (None, None));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn a_deactivation_ordered_first_makes_the_link_write_nothing(pool: PgPool) {
    let id = account(&pool).await;
    let mut deactivation = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", id)
        .execute(&mut *deactivation)
        .await
        .unwrap();

    let db = pool.clone();
    let linking = tokio::spawn(async move { link(&db, id, ISSUER, "late").await });
    wait_for_links(&pool, 1).await;
    deactivation.commit().await.unwrap();

    let result = timeout(Duration::from_secs(10), linking)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(rejected(result), Rejection::Inactive);
    assert_eq!(binding(&pool, id).await, (None, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn a_deactivation_ordered_after_a_link_still_revokes_sign_in(pool: PgPool) {
    let id = account(&pool).await;
    assert_eq!(
        link(&pool, id, ISSUER, "early").await.unwrap(),
        LinkOutcome::Linked
    );
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", id)
        .execute(&pool)
        .await
        .unwrap();

    // The binding stays; sign-in refuses the inactive account. Each request's
    // `require_user` re-check is existing behavior and is not retested here.
    let token = identity(ISSUER, "early", None, false);
    assert!(resolve_user(&pool, &token).await.unwrap().is_none());
    assert_eq!(
        binding(&pool, id).await,
        (Some(ISSUER.into()), Some("early".into()))
    );
}
