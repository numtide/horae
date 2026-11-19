//! Who reads which time: entry pages, the Timesheet and its people picker, and
//! the time report with its groups, under each grant and concurrent change.

use std::time::Duration;

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::save_state;
use crate::testing::{SeedIds, seed, wait_for_blocked};

mod api_list;
mod time_entries;
mod time_report_groups;
mod time_reports;
mod timesheet_context;
mod timesheet_people;

/// A page of the time entries the actor may read. No adapter serves it: the
/// scope tests read the shared entry reader through it.
async fn entries(
    tx: &mut Tx,
    actor: &Actor,
    query: &TimeEntryQuery,
) -> Result<TimeEntryPage, OpError> {
    let grants = require_time_reads(actor, TIME_READ_REQUIRED)?;
    if !entry_query_is_valid(query) {
        return Err(OpError::bad_input(
            "Invalid time-entry date range or cursor",
        ));
    }
    fetch_entries(tx, actor.requester(), &grants, query).await
}

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, BuiltInProfile::Member).await;
    save_state(
        pool,
        ids.org_id,
        ids.user_id,
        true,
        &BuiltInProfile::Administrator.selection(),
    )
    .await;
    ids
}

async fn person(pool: &PgPool, org: Uuid, admin: bool, profile: BuiltInProfile) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Person')",
        id,
        org,
        format!("{id}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    save_state(pool, org, id, admin, &profile.selection()).await;
    id
}
