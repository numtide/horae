//! Self-approval: a non-administrator may not approve or reject their own
//! submission, and the check reads the requester's current role in the
//! approval transaction.

use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use sqlx::PgPool;
use uuid::Uuid;

fn requester(ids: &SeedIds, role: OrgRole) -> User {
    User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Requester".into(),
        oidc_subject: None,
        org_role: role,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

/// Another person in `ids`' organization, sharing its project and task.
async fn colleague(pool: &PgPool, ids: &SeedIds) -> SeedIds {
    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, 'Colleague', $4)",
        user_id,
        ids.org_id,
        format!("{user_id}@test.com"),
        OrgRole::Member as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    SeedIds { user_id, ..*ids }
}

async fn submitted_week(pool: &PgPool, ids: &SeedIds) -> (Approval, Uuid) {
    let entry = time_entry(pool, ids, EntryState::Open).await;
    let (approval, _) =
        submit_user_week(pool, ids.user_id, ids.org_id, "2026-09-07".parse().unwrap())
            .await
            .unwrap();
    (approval, entry)
}

async fn states(pool: &PgPool, approval: Uuid, entry: Uuid) -> (Option<EntryState>, EntryState) {
    let approval = sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM approvals WHERE id = $1"#,
        approval,
    )
    .fetch_optional(pool)
    .await
    .unwrap();
    let entry = sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM time_entries WHERE id = $1"#,
        entry,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (approval, entry)
}

fn is_forbidden<T: std::fmt::Debug>(result: &Result<T, ServerFnError>) -> bool {
    matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    )
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_manager_cannot_approve_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let (approval, entry) = submitted_week(&pool, &ids).await;

    let result = approve_periods(&pool, &requester(&ids, OrgRole::Manager), &[approval.id]).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, approval.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_manager_cannot_reject_or_reopen_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let (approval, entry) = submitted_week(&pool, &ids).await;

    let result = reopen_period(&pool, &requester(&ids, OrgRole::Manager), approval.id).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, approval.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_bulk_approval_including_the_managers_own_week_changes_nothing(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = colleague(&pool, &ids).await;
    let (own, own_entry) = submitted_week(&pool, &ids).await;
    let (theirs, their_entry) = submitted_week(&pool, &other).await;

    let result = approve_periods(
        &pool,
        &requester(&ids, OrgRole::Manager),
        &[theirs.id, own.id],
    )
    .await;

    assert!(is_forbidden(&result), "{result:?}");
    for (approval, entry) in [(own.id, own_entry), (theirs.id, their_entry)] {
        assert_eq!(
            states(&pool, approval, entry).await,
            (Some(EntryState::Submitted), EntryState::Submitted)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_manager_can_approve_someone_elses_submission(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = colleague(&pool, &ids).await;
    let (theirs, entry) = submitted_week(&pool, &other).await;

    let approved = approve_periods(&pool, &requester(&ids, OrgRole::Manager), &[theirs.id])
        .await
        .unwrap();

    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0].approved_by, Some(ids.user_id));
    assert_eq!(
        states(&pool, theirs.id, entry).await,
        (Some(EntryState::Approved), EntryState::Approved)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn an_admin_can_approve_and_reopen_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let (own, entry) = submitted_week(&pool, &ids).await;
    let admin = requester(&ids, OrgRole::Admin);

    let approved = approve_periods(&pool, &admin, &[own.id]).await.unwrap();
    assert_eq!(approved.len(), 1);
    assert_eq!(
        states(&pool, own.id, entry).await,
        (Some(EntryState::Approved), EntryState::Approved)
    );

    reopen_period(&pool, &admin, own.id).await.unwrap();
    assert_eq!(states(&pool, own.id, entry).await, (None, EntryState::Open));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_demoted_admin_is_judged_by_their_current_role(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let (own, entry) = submitted_week(&pool, &ids).await;
    sqlx::query!(
        "UPDATE users SET org_role = $2 WHERE id = $1",
        ids.user_id,
        OrgRole::Manager as OrgRole,
    )
    .execute(&pool)
    .await
    .unwrap();

    // The session-loaded identity still says Admin; the transaction must not.
    let result = approve_periods(&pool, &requester(&ids, OrgRole::Admin), &[own.id]).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, own.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}
