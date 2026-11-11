use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, time_entry};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

fn manager(ids: &SeedIds, role: BuiltInProfile) -> User {
    User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Test manager".into(),
        oidc_subject: None,
        org_role: crate::server_fns::provisioning::placeholder_role(role),
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

async fn submitted_week(pool: &PgPool, ids: &SeedIds) -> (Approval, Uuid) {
    let entry = time_entry(pool, ids, EntryState::Open).await;
    let (approval, _) =
        submit_user_week(pool, ids.user_id, ids.org_id, "2026-09-07".parse().unwrap())
            .await
            .unwrap();
    (approval, entry)
}

async fn entry_state(pool: &PgPool, entry_id: Uuid) -> EntryState {
    sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM time_entries WHERE id = $1"#,
        entry_id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn foreign_approval_ids_do_not_change_another_organizations_week(pool: PgPool) {
    let foreign = seed(&pool, BuiltInProfile::Member).await;
    let (approval, entry) = submitted_week(&pool, &foreign).await;

    for role in [
        BuiltInProfile::ExecutiveManager,
        BuiltInProfile::Administrator,
    ] {
        let local = seed(&pool, role).await;
        let approved = approve_periods(&pool, &manager(&local, role), &[approval.id])
            .await
            .unwrap();
        assert!(
            approved.is_empty(),
            "foreign approval returned for {role:?}"
        );
        assert_eq!(entry_state(&pool, entry).await, EntryState::Submitted);
        assert_eq!(
            foreign_week_total(&pool, &foreign, &approval, local.org_id).await,
            0
        );
    }
    let unchanged = sqlx::query!(
        "SELECT state::text, approved_by, approved_at FROM approvals WHERE id = $1",
        approval.id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unchanged.state.as_deref(), Some("submitted"));
    assert!(unchanged.approved_by.is_none() && unchanged.approved_at.is_none());
    assert_eq!(
        foreign_week_total(&pool, &foreign, &approval, foreign.org_id).await,
        60
    );
}

async fn foreign_week_total(
    pool: &PgPool,
    foreign: &SeedIds,
    approval: &Approval,
    org_id: Uuid,
) -> i32 {
    week_total_minutes(
        pool,
        foreign.user_id,
        org_id,
        approval.period_start,
        approval.period_end,
    )
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn bulk_approval_only_returns_and_transitions_own_organization(pool: PgPool) {
    // An administrator, so approving their own week is allowed.
    let local = seed(&pool, BuiltInProfile::Administrator).await;
    let foreign = seed(&pool, BuiltInProfile::Member).await;
    let (own_approval, own_entry) = submitted_week(&pool, &local).await;
    let (foreign_approval, foreign_entry) = submitted_week(&pool, &foreign).await;
    let actor = manager(&local, BuiltInProfile::Administrator);

    let approved = approve_periods(
        &pool,
        &actor,
        &[
            own_approval.id,
            foreign_approval.id,
            own_approval.id,
            Uuid::now_v7(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0].id, own_approval.id);
    assert_eq!(approved[0].approved_by, Some(actor.id));
    assert_eq!(entry_state(&pool, own_entry).await, EntryState::Approved);
    assert_eq!(
        entry_state(&pool, foreign_entry).await,
        EntryState::Submitted
    );
    assert!(
        approve_periods(&pool, &actor, &[own_approval.id])
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn foreign_reopen_is_not_found_and_preserves_pending_and_approved_work(pool: PgPool) {
    let locals = [
        (
            seed(&pool, BuiltInProfile::ExecutiveManager).await,
            BuiltInProfile::ExecutiveManager,
        ),
        (
            seed(&pool, BuiltInProfile::Administrator).await,
            BuiltInProfile::Administrator,
        ),
    ];
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let (approval, entry) = submitted_week(&pool, &foreign).await;

    for expected in [EntryState::Submitted, EntryState::Approved] {
        if expected == EntryState::Approved {
            approve_periods(
                &pool,
                &manager(&foreign, BuiltInProfile::Administrator),
                &[approval.id],
            )
            .await
            .unwrap();
        }
        for (local, role) in &locals {
            let result = reopen_period(&pool, &manager(local, *role), approval.id).await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: NOT_FOUND,
                        ..
                    })
                ),
                "{result:?}"
            );
            assert_eq!(entry_state(&pool, entry).await, expected);
        }
        let state = sqlx::query_scalar!(
            r#"SELECT state as "state: EntryState" FROM approvals WHERE id = $1"#,
            approval.id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(state, expected);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn own_organization_can_reopen_without_unlocking_invoiced_entries(pool: PgPool) {
    let local = seed(&pool, BuiltInProfile::Administrator).await;
    let (approval, entry) = submitted_week(&pool, &local).await;
    let actor = manager(&local, BuiltInProfile::Administrator);
    approve_periods(&pool, &actor, &[approval.id])
        .await
        .unwrap();
    let invoiced = time_entry(&pool, &local, EntryState::Invoiced).await;

    let reopened = reopen_period(&pool, &actor, approval.id).await.unwrap();
    assert_eq!(reopened.id, approval.id);
    assert_eq!(entry_state(&pool, entry).await, EntryState::Open);
    assert_eq!(entry_state(&pool, invoiced).await, EntryState::Invoiced);
    let remains = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM approvals WHERE id = $1)",
        approval.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();
    assert!(!remains);
}
