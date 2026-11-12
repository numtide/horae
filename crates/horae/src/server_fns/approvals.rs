//! Approval server functions: session adapters over `ops::approvals`.

use super::*;
use crate::models::scoped_time::TimesheetWriteContext;
#[cfg(feature = "server")]
use crate::ops::{self, Caller};

// ── Approvals (M7) ──────────────────────────────────────────────────────────

/// Submit a week of time entries for approval.
/// Transitions all 'open' entries in [week_start, week_start+6] to 'submitted'
/// and creates an approval row.
#[server]
pub async fn submit_week(
    week_start: String,
    context: TimesheetWriteContext,
) -> Result<Approval, ServerFnError> {
    let user = require_user().await?;
    let user_id = user.id;
    let org_id = user.org_id;
    if context.expected_requester.user_id != user_id
        || context.expected_requester.org_id != org_id
        || context.subject_id != user_id
    {
        return Err(forbidden("Timesheet context changed; refresh the page"));
    }
    let state = crate::state::global_state().await;

    let ws = parse_date(&week_start, "week_start")?;
    submit_as(
        &state.db,
        Caller::session(org_id, user_id),
        ws,
        &*state.plugins,
    )
    .await
}

#[cfg(feature = "server")]
async fn submit_as(
    pool: &sqlx::PgPool,
    caller: Caller,
    week_start: chrono::NaiveDate,
    events: &dyn ops::EventSink,
) -> Result<Approval, ServerFnError> {
    let mut write = ops::begin(pool, caller).await?;
    let result = ops::approvals::submit_week(&mut write.tx, &write.actor, week_start).await;
    Ok(write.commit(result, events).await?)
}

/// List one week's approvals, optionally filtered by state. Requires approval
/// access covering everyone in the week.
#[server]
pub async fn list_approvals(
    status: Option<String>,
    week_start: String,
) -> Result<Vec<ApprovalSummary>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    list_week(
        &state.db,
        Caller::session(user.org_id, user.id),
        status,
        &week_start,
    )
    .await
}

#[cfg(feature = "server")]
async fn list_week(
    pool: &sqlx::PgPool,
    caller: Caller,
    status: Option<String>,
    week_start: &str,
) -> Result<Vec<ApprovalSummary>, ServerFnError> {
    let week_start: chrono::NaiveDate = week_start
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid week_start (use YYYY-MM-DD)"))?;
    let state: Option<EntryState> = status.map(|s| parse_enum(&s, "status")).transpose()?;
    let mut read = ops::begin_read(pool, caller).await?;
    let result = ops::approvals::list(&mut read.tx, &read.actor, state, week_start).await;
    Ok(read.finish(result).await?)
}

#[cfg(feature = "server")]
async fn approve_as(
    pool: &sqlx::PgPool,
    caller: Caller,
    ids: &[uuid::Uuid],
    events: &dyn ops::EventSink,
) -> Result<Vec<Approval>, ServerFnError> {
    let mut write = ops::begin(pool, caller).await?;
    let result = ops::approvals::approve(&mut write.tx, &write.actor, ids).await;
    Ok(write.commit(result, events).await?)
}

/// Approve a single submitted week.
#[server]
pub async fn approve_submission(approval_id: String) -> Result<Approval, ServerFnError> {
    let user = require_user().await?;
    let id = parse_uuid(&approval_id, "approval_id")?;
    let state = crate::state::global_state().await;
    approve_as(
        &state.db,
        Caller::session(user.org_id, user.id),
        &[id],
        &*state.plugins,
    )
    .await?
    .into_iter()
    .next()
    .ok_or_else(|| not_found("Approval not found or not in 'submitted' state"))
}

/// Approve several submitted weeks at once (the "approve visible" action).
/// Returns the number actually approved; foreign or non-submitted ids are
/// skipped, but a selection including a week the requester may not decide is
/// denied outright.
#[server]
pub async fn approve_submissions(approval_ids: Vec<String>) -> Result<usize, ServerFnError> {
    let user = require_user().await?;
    let ids = approval_ids
        .iter()
        .map(|s| parse_uuid(s, "approval_id"))
        .collect::<Result<Vec<_>, _>>()?;
    let state = crate::state::global_state().await;
    Ok(approve_as(
        &state.db,
        Caller::session(user.org_id, user.id),
        &ids,
        &*state.plugins,
    )
    .await?
    .len())
}

/// Reject a submitted week, or reopen an already-approved one. Reopens the
/// period's time entries and deletes the approval row, exactly reversing what
/// submit (and approve) wrote.
#[server]
pub async fn reject_submission(approval_id: String) -> Result<(), ServerFnError> {
    let user = require_user().await?;
    let approval_id = parse_uuid(&approval_id, "approval_id")?;
    let state = crate::state::global_state().await;
    reopen_as(
        &state.db,
        Caller::session(user.org_id, user.id),
        approval_id,
        &*state.plugins,
    )
    .await?;
    Ok(())
}

#[cfg(feature = "server")]
async fn reopen_as(
    pool: &sqlx::PgPool,
    caller: Caller,
    approval_id: uuid::Uuid,
    events: &dyn ops::EventSink,
) -> Result<Approval, ServerFnError> {
    let mut write = ops::begin(pool, caller).await?;
    let result = ops::approvals::reopen(&mut write.tx, &write.actor, approval_id).await;
    Ok(write.commit(result, events).await?)
}

// DB-backed tests (`#[sqlx::test]`, throwaway database per test). They call
// the crate-internal functions directly, which `tests/` cannot do for a bin
// crate — so the real behaviour is pinned here, and the integration tests only
// mirror the SQL statements.
#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{Approval, approve_as, reopen_as, submit_as};
    use crate::ops::Caller;
    use crate::ops::approvals::week_has_running_timer;
    use crate::ops::testing::Recorder;
    use crate::plugin::AppEvent;
    use crate::server_fns::test_seed::{SeedIds, seed};
    use dioxus::prelude::ServerFnError;
    use horae_core::permissions::catalog::BuiltInProfile;
    use horae_core::types::EntryState;
    use sqlx::PgPool;
    use uuid::Uuid;

    /// The seeded person, as a signed-in session.
    pub(super) fn caller(ids: &SeedIds) -> Caller {
        Caller::session(ids.org_id, ids.user_id)
    }

    /// Submits as the given person and returns the approval with the total the
    /// submission event announced.
    pub(super) async fn submit_user_week(
        pool: &PgPool,
        user_id: Uuid,
        org_id: Uuid,
        week_start: chrono::NaiveDate,
    ) -> Result<(Approval, i32), ServerFnError> {
        let events = Recorder::default();
        let approval =
            submit_as(pool, Caller::session(org_id, user_id), week_start, &events).await?;
        match events.take().as_slice() {
            [AppEvent::TimesheetSubmitted { submission, .. }] => {
                Ok((approval, submission.total_minutes))
            }
            other => panic!("expected one submission event, got {other:?}"),
        }
    }

    pub(super) async fn approve_periods(
        pool: &PgPool,
        caller: Caller,
        ids: &[Uuid],
    ) -> Result<Vec<Approval>, ServerFnError> {
        approve_as(pool, caller, ids, &Recorder::default()).await
    }

    pub(super) async fn reopen_period(
        pool: &PgPool,
        caller: Caller,
        approval_id: Uuid,
    ) -> Result<Approval, ServerFnError> {
        reopen_as(pool, caller, approval_id, &Recorder::default()).await
    }

    pub(super) async fn insert_entry(
        pool: &PgPool,
        ids: &SeedIds,
        spent_date: chrono::NaiveDate,
        is_running: bool,
    ) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO time_entries \
               (id, org_id, user_id, project_id, task_id, spent_date, \
                minutes, billable, is_running, started_at, state) \
             VALUES ($1, $2, $3, $4, $5, $6, 0, true, $7, \
                     CASE WHEN $7 THEN now() END, $8)",
            id,
            ids.org_id,
            ids.user_id,
            ids.project_id,
            ids.task_id,
            spent_date as chrono::NaiveDate,
            is_running,
            EntryState::Open as EntryState,
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn sees_a_running_timer_inside_the_week(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let ws = chrono::Utc::now().date_naive();
        let we = ws + chrono::Duration::days(6);

        insert_entry(&pool, &ids, ws, true).await;

        assert!(
            week_has_running_timer(&pool, ids.user_id, ws, we)
                .await
                .unwrap()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn ignores_stopped_and_out_of_week_timers(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let ws = chrono::Utc::now().date_naive();
        let we = ws + chrono::Duration::days(6);

        // A stopped entry inside the week and a running timer dated after it.
        insert_entry(&pool, &ids, ws, false).await;
        insert_entry(&pool, &ids, we + chrono::Duration::days(1), true).await;

        assert!(
            !week_has_running_timer(&pool, ids.user_id, ws, we)
                .await
                .unwrap()
        );
    }
}

#[cfg(all(test, feature = "server"))]
mod legacy_minutes_tests;

#[cfg(all(test, feature = "server"))]
mod submission_tests;

#[cfg(all(test, feature = "server"))]
mod isolation_tests;

#[cfg(all(test, feature = "server"))]
mod period_tests;

#[cfg(all(test, feature = "server"))]
mod self_approval_tests;
