//! One approval week: exact `period_start` membership, week validation, and
//! approve/reopen leaving adjacent weeks untouched.

use super::tests::insert_entry;
use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use sqlx::PgPool;
use uuid::Uuid;

fn manager(ids: &SeedIds) -> User {
    User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Test manager".into(),
        oidc_subject: None,
        org_role: OrgRole::Manager,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    }
}

fn date(s: &str) -> chrono::NaiveDate {
    s.parse().unwrap()
}

/// Submit the Monday-start week beginning `start`, returning the approval and
/// the one entry it froze.
async fn submitted(pool: &PgPool, ids: &SeedIds, start: &str) -> (Approval, Uuid) {
    let entry = insert_entry(pool, ids, date(start), false).await;
    let (approval, _) = submit_user_week(pool, ids.user_id, ids.org_id, date(start))
        .await
        .unwrap();
    (approval, entry)
}

async fn approval_state(pool: &PgPool, id: Uuid) -> EntryState {
    sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM approvals WHERE id = $1"#,
        id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn entry_state(pool: &PgPool, id: Uuid) -> EntryState {
    sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM time_entries WHERE id = $1"#,
        id,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn ids_of(rows: &[ApprovalSummary]) -> Vec<Uuid> {
    rows.iter().map(|row| row.approval.id).collect()
}

fn is_bad_request(result: &Result<Vec<ApprovalSummary>, ServerFnError>) -> bool {
    matches!(
        result,
        Err(ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        })
    )
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_week_lists_only_its_own_organizations_approvals_starting_that_day(pool: PgPool) {
    let local = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Member).await;
    let (before, _) = submitted(&pool, &local, "2026-08-31").await;
    let (week, _) = submitted(&pool, &local, "2026-09-07").await;
    let (after, _) = submitted(&pool, &local, "2026-09-14").await;
    submitted(&pool, &foreign, "2026-09-07").await;
    let actor = manager(&local);

    let rows = list_week(&pool, &actor, None, "2026-09-07").await.unwrap();
    assert_eq!(ids_of(&rows), vec![week.id]);
    assert!(
        rows.iter()
            .all(|row| row.approval.period_start == date("2026-09-07")
                && row.approval.period_end == date("2026-09-13"))
    );
    for other in [before.id, after.id] {
        assert!(!ids_of(&rows).contains(&other));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn the_status_filter_applies_within_the_week(pool: PgPool) {
    let local = seed(&pool, OrgRole::Manager).await;
    let (week, _) = submitted(&pool, &local, "2026-09-07").await;
    let actor = manager(&local);
    approve_periods(&pool, &actor, &[week.id]).await.unwrap();

    let approved = list_week(&pool, &actor, Some("approved".into()), "2026-09-07")
        .await
        .unwrap();
    let pending = list_week(&pool, &actor, Some("submitted".into()), "2026-09-07")
        .await
        .unwrap();
    assert_eq!(ids_of(&approved), vec![week.id]);
    assert!(pending.is_empty(), "{pending:?}");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn an_unusable_week_start_is_a_bad_request(pool: PgPool) {
    let local = seed(&pool, OrgRole::Manager).await;
    submitted(&pool, &local, "2026-09-07").await;
    let actor = manager(&local);
    // The organization starts weeks on Monday (the column default).
    let last_monday = horae_core::week::week_start(chrono::NaiveDate::MAX, chrono::Weekday::Mon)
        .unwrap()
        .to_string();

    // Each input must reach its own check, not fail earlier for another reason.
    for (week_start, reason) in [
        ("garbage", "Invalid week_start"),
        ("2026-09-08", "first weekday"),
        (last_monday.as_str(), "out of range"),
    ] {
        let result = list_week(&pool, &actor, None, week_start).await;
        assert!(is_bad_request(&result), "{week_start}: {result:?}");
        assert!(
            matches!(&result, Err(ServerFnError::ServerError { message, .. }) if message.contains(reason)),
            "{week_start}: expected {reason:?}, got {result:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn approving_a_week_leaves_the_previous_week_pending(pool: PgPool) {
    let local = seed(&pool, OrgRole::Manager).await;
    let (before, before_entry) = submitted(&pool, &local, "2026-08-31").await;
    let (week, week_entry) = submitted(&pool, &local, "2026-09-07").await;
    let actor = manager(&local);

    let approved = approve_periods(&pool, &actor, &[week.id]).await.unwrap();
    assert_eq!(approved.len(), 1);
    assert_eq!(entry_state(&pool, week_entry).await, EntryState::Approved);
    assert_eq!(
        approval_state(&pool, before.id).await,
        EntryState::Submitted
    );
    assert_eq!(
        entry_state(&pool, before_entry).await,
        EntryState::Submitted
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reopening_a_week_leaves_the_next_week_approved(pool: PgPool) {
    let local = seed(&pool, OrgRole::Manager).await;
    let (week, week_entry) = submitted(&pool, &local, "2026-09-07").await;
    let (after, after_entry) = submitted(&pool, &local, "2026-09-14").await;
    let actor = manager(&local);
    approve_periods(&pool, &actor, &[week.id, after.id])
        .await
        .unwrap();

    reopen_period(&pool, &actor, week.id).await.unwrap();
    assert_eq!(entry_state(&pool, week_entry).await, EntryState::Open);
    assert_eq!(approval_state(&pool, after.id).await, EntryState::Approved);
    assert_eq!(entry_state(&pool, after_entry).await, EntryState::Approved);
}
