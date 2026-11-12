//! An entry stored over the daily limit before it existed still moves through
//! approval: only raising its minutes is refused.

use super::tests::{approve_periods, caller, insert_entry, reopen_period, submit_user_week};
use super::*;
use crate::server_fns::test_seed::{assert_minutes_cannot_rise, legacy_minutes, seed};
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_legacy_entry_over_the_limit_is_submitted_approved_rejected_and_reopened(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let week: chrono::NaiveDate = "2026-09-07".parse().unwrap();
    let entry = insert_entry(&pool, &ids, week, false).await;
    legacy_minutes(&pool, entry, 20_000).await;
    let approver = caller(&ids);

    // Reject a submitted week, then reopen an approved one.
    let (submitted, _) = submit_user_week(&pool, ids.user_id, ids.org_id, week)
        .await
        .unwrap();
    reopen_period(&pool, approver, submitted.id).await.unwrap();
    let (submitted, _) = submit_user_week(&pool, ids.user_id, ids.org_id, week)
        .await
        .unwrap();
    approve_periods(&pool, approver, &[submitted.id])
        .await
        .unwrap();
    reopen_period(&pool, approver, submitted.id).await.unwrap();

    let stored = sqlx::query!(
        r#"SELECT minutes, state as "state: EntryState" FROM time_entries WHERE id = $1"#,
        entry
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((stored.minutes, stored.state), (20_000, EntryState::Open));
    assert_minutes_cannot_rise(&pool, entry).await;
}
