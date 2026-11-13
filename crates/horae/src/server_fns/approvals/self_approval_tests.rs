//! Self-approval: a non-administrator may not approve or reject their own
//! submission, and the check reads the requester's stored permission state in
//! the approval transaction.

use super::tests::{approve_periods, caller, reopen_period, submit_user_week};
use super::*;
use crate::ops::approvals::tests::add_person;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_manager_cannot_approve_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let (approval, entry) = submitted_week(&pool, &ids).await;

    let result = approve_periods(&pool, caller(&ids), &[approval.id]).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, approval.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_manager_cannot_reject_or_reopen_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let (approval, entry) = submitted_week(&pool, &ids).await;

    let result = reopen_period(&pool, caller(&ids), approval.id).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, approval.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_bulk_approval_including_the_managers_own_week_changes_nothing(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let other = add_person(&pool, &ids, BuiltInProfile::Member).await;
    let (own, own_entry) = submitted_week(&pool, &ids).await;
    let (theirs, their_entry) = submitted_week(&pool, &other).await;

    let result = approve_periods(&pool, caller(&ids), &[theirs.id, own.id]).await;

    assert!(is_forbidden(&result), "{result:?}");
    for (approval, entry) in [(own.id, own_entry), (theirs.id, their_entry)] {
        assert_eq!(
            states(&pool, approval, entry).await,
            (Some(EntryState::Submitted), EntryState::Submitted)
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_manager_can_approve_someone_elses_submission(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let other = add_person(&pool, &ids, BuiltInProfile::Member).await;
    let (theirs, entry) = submitted_week(&pool, &other).await;

    let approved = approve_periods(&pool, caller(&ids), &[theirs.id])
        .await
        .unwrap();

    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0].approved_by, Some(ids.user_id));
    assert_eq!(
        states(&pool, theirs.id, entry).await,
        (Some(EntryState::Approved), EntryState::Approved)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn an_admin_can_approve_and_reopen_their_own_submission(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (own, entry) = submitted_week(&pool, &ids).await;

    let approved = approve_periods(&pool, caller(&ids), &[own.id])
        .await
        .unwrap();
    assert_eq!(approved.len(), 1);
    assert_eq!(
        states(&pool, own.id, entry).await,
        (Some(EntryState::Approved), EntryState::Approved)
    );

    reopen_period(&pool, caller(&ids), own.id).await.unwrap();
    assert_eq!(states(&pool, own.id, entry).await, (None, EntryState::Open));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_former_administrator_is_judged_by_their_stored_state(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (own, entry) = submitted_week(&pool, &ids).await;
    // Every grant, but no longer the Administrator identity: the session that
    // signed in as an administrator must not carry it into the decision.
    crate::ops::testing::provision(&pool, &ids, Permission::ALL, false).await;

    let result = approve_periods(&pool, caller(&ids), &[own.id]).await;

    assert!(is_forbidden(&result), "{result:?}");
    assert_eq!(
        states(&pool, own.id, entry).await,
        (Some(EntryState::Submitted), EntryState::Submitted)
    );
}
