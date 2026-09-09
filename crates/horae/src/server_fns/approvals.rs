//! Approval server functions.

use super::*;

// ── Approvals (M7) ──────────────────────────────────────────────────────────

/// The conflict submit_week returns when a running timer sits inside the week
/// — one string for both return sites so they cannot drift.
#[cfg(feature = "server")]
const RUNNING_TIMER_CONFLICT: &str = "A timer is running in this week. Stop it before submitting.";

/// True when the user has a running timer dated inside `[ws, we]`.
#[cfg(feature = "server")]
async fn week_has_running_timer(
    db: impl sqlx::PgExecutor<'_>,
    user_id: uuid::Uuid,
    ws: chrono::NaiveDate,
    we: chrono::NaiveDate,
) -> Result<bool, ServerFnError> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM time_entries
           WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3 AND is_running)",
        user_id,
        ws as chrono::NaiveDate,
        we as chrono::NaiveDate,
    )
    .fetch_one(db)
    .await
    .map_err(server_err)?
    .unwrap_or(false))
}

/// Submit a week of time entries for approval.
/// Transitions all 'open' entries in [week_start, week_start+6] to 'submitted'
/// and creates an approval row.
#[server]
pub async fn submit_week(week_start: String) -> Result<Approval, ServerFnError> {
    let user = require_user().await?;
    let user_id = user.id;
    let org_id = user.org_id;
    let state = crate::state::global_state().await;

    let ws = parse_date(&week_start, "week_start")?;
    let (approval, total_minutes) = submit_user_week(&state.db, user_id, org_id, ws).await?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TimesheetSubmitted {
            occurred_at: chrono::Utc::now(),
            org_id,
            submission: submission_payload(&approval, total_minutes),
        });
    Ok(approval)
}

#[cfg(feature = "server")]
async fn submit_user_week(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    org_id: uuid::Uuid,
    ws: chrono::NaiveDate,
) -> Result<(Approval, i32), ServerFnError> {
    use chrono::Datelike;
    let we = ws
        .checked_add_days(chrono::Days::new(6))
        .ok_or_else(|| err(BAD_REQUEST, "week_start is out of range"))?;

    let mut tx = pool.begin().await.map_err(server_err)?;
    // Writers acquire the shared form before changing any entries. Taking the
    // user-wide lock first covers inserts and cross-week moves, not just rows
    // that happened to exist when submission started.
    sqlx::query!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
        user_id,
    ).execute(&mut *tx).await.map_err(server_err)?;
    let config = sqlx::query!(
        r#"SELECT week_start, round_minutes, round_dir as "round_dir: horae_core::types::RoundDir"
           FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    if ws.weekday().number_from_monday() as i16 != config.week_start {
        return Err(err(
            BAD_REQUEST,
            "week_start must match the organization's first weekday",
        ));
    }

    // A week a manager has already approved cannot be resubmitted — silently
    // downgrading the approval back to pending would erase who approved it and
    // when. It must be reopened (rejected) first. The row lock keeps a
    // concurrent approve from slipping between this check and the upsert below.
    let existing = sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM approvals
           WHERE user_id = $1 AND period_start = $2
           FOR UPDATE"#,
        user_id,
        ws as chrono::NaiveDate,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    if existing == Some(EntryState::Approved) {
        return Err(conflict(
            "This week is already approved; a manager must reopen it before it can be resubmitted",
        ));
    }

    // A running timer cannot be frozen before its elapsed minutes are known.
    // The write barrier prevents a timer from starting after this check.
    if week_has_running_timer(&mut *tx, user_id, ws, we).await? {
        return Err(conflict(RUNNING_TIMER_CONFLICT));
    }

    // Freeze and transition in the same UPDATE, using the shared SQL rounding
    // function already checked against horae-core. A competing row update is
    // re-evaluated against its current minutes, not a previously fetched vector.
    let result = sqlx::query!(
        "UPDATE time_entries
         SET state = $4,
             rounded_minutes = effective_minutes(minutes, NULL, $7, $8)
         WHERE user_id = $1
           AND org_id = $6
           AND spent_date BETWEEN $2 AND $3
           AND state = $5
           AND NOT EXISTS (SELECT 1 FROM time_entries r
                            WHERE r.user_id = $1
                              AND r.spent_date BETWEEN $2 AND $3
                              AND r.is_running)",
        user_id,
        ws as chrono::NaiveDate,
        we as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
        EntryState::Open as EntryState,
        org_id,
        config.round_minutes,
        config.round_dir as horae_core::types::RoundDir,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    if result.rows_affected() == 0 {
        if week_has_running_timer(&mut *tx, user_id, ws, we).await? {
            return Err(conflict(RUNNING_TIMER_CONFLICT));
        }
        return Err(not_found("No open entries found for this week"));
    }

    // Retain the conditional upsert as a backstop against downgrading approval.
    let id = uuid::Uuid::now_v7();
    let approval = sqlx::query_as!(
        Approval,
        r#"INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, period_start) DO UPDATE
           SET state = $6, submitted_at = now()
           WHERE approvals.state <> $7
         RETURNING id, org_id, user_id,
                   period_start as "period_start: chrono::NaiveDate",
                   period_end as "period_end: chrono::NaiveDate",
                   state as "state: EntryState",
                   submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                   approved_by,
                   approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        id,
        org_id,
        user_id,
        ws as chrono::NaiveDate,
        we as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
        EntryState::Approved as EntryState,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| {
        conflict(
            "This week is already approved; a manager must reopen it before it can be resubmitted",
        )
    })?;

    let total_minutes = week_total_minutes(&mut *tx, user_id, ws, we).await?;
    tx.commit().await.map_err(server_err)?;
    Ok((approval, total_minutes))
}

/// List approvals, optionally filtered by state. Requires manager role.
#[server]
pub async fn list_approvals(status: Option<String>) -> Result<Vec<ApprovalSummary>, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let state_filter: Option<EntryState> = status.map(|s| parse_enum(&s, "status")).transpose()?;

    // Hours are aggregated per row from the user's entries in the approval's
    // period (actual `minutes`, split by `billable`) via a lateral join, so the
    // whole table comes back in one query rather than a lookup per approval.
    let rows = sqlx::query!(
        r#"SELECT a.id, a.org_id, a.user_id,
                a.period_start as "period_start: chrono::NaiveDate",
                a.period_end as "period_end: chrono::NaiveDate",
                a.state as "state: EntryState",
                a.submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                a.approved_by,
                a.approved_at as "approved_at: chrono::DateTime<chrono::Utc>",
                COALESCE(t.total_minutes, 0) as "total_minutes!",
                COALESCE(t.billable_minutes, 0) as "billable_minutes!"
         FROM approvals a
         LEFT JOIN LATERAL (
             SELECT (SUM(minutes))::bigint as total_minutes,
                    (SUM(minutes) FILTER (WHERE billable))::bigint as billable_minutes
             FROM time_entries te
             WHERE te.org_id = a.org_id
               AND te.user_id = a.user_id
               AND te.spent_date BETWEEN a.period_start AND a.period_end
         ) t ON true
         WHERE a.org_id = $2
           AND ($1::entry_state IS NULL OR a.state = $1)
         ORDER BY a.period_start DESC"#,
        state_filter as Option<EntryState>,
        manager.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(rows
        .into_iter()
        .map(|r| ApprovalSummary {
            approval: Approval {
                id: r.id,
                org_id: r.org_id,
                user_id: r.user_id,
                period_start: r.period_start,
                period_end: r.period_end,
                state: r.state,
                submitted_at: r.submitted_at,
                approved_by: r.approved_by,
                approved_at: r.approved_at,
            },
            total_minutes: r.total_minutes,
            billable_minutes: r.billable_minutes,
        })
        .collect())
}

/// Approve every submitted approval in `ids` within one transaction: flip each
/// row Submitted→Approved, transition its period's submitted entries, and return
/// the rows actually approved (ids not in 'submitted' are skipped). Shared by the
/// single- and bulk-approve server functions so the transition lives in one place.
#[cfg(feature = "server")]
async fn approve_ids(manager: &User, ids: &[uuid::Uuid]) -> Result<Vec<Approval>, ServerFnError> {
    let state = crate::state::global_state().await;
    let mut tx = state.db.begin().await.map_err(server_err)?;

    let approvals = sqlx::query_as!(
        Approval,
        r#"UPDATE approvals
             SET state = $2, approved_by = $3, approved_at = now()
           WHERE id = ANY($1) AND state = $4
        RETURNING id, org_id, user_id,
                  period_start as "period_start: chrono::NaiveDate",
                  period_end as "period_end: chrono::NaiveDate",
                  state as "state: EntryState",
                  submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                  approved_by,
                  approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        ids,
        EntryState::Approved as EntryState,
        manager.id,
        EntryState::Submitted as EntryState,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;

    // Transition the submitted entries of every approved period to approved.
    let approved_ids: Vec<uuid::Uuid> = approvals.iter().map(|a| a.id).collect();
    sqlx::query!(
        r#"UPDATE time_entries te
              SET state = $2
             FROM approvals a
            WHERE a.id = ANY($1)
              AND te.user_id = a.user_id
              AND te.spent_date BETWEEN a.period_start AND a.period_end
              AND te.state = $3"#,
        &approved_ids,
        EntryState::Approved as EntryState,
        EntryState::Submitted as EntryState,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;

    // Sum every approved period's tracked minutes in one grouped query instead of
    // a round-trip per approval. This matches `week_total_minutes` exactly: all of
    // the user's entries in the period, unfiltered by state.
    let total_rows = sqlx::query!(
        r#"SELECT a.id as "id!",
                  COALESCE(SUM(te.minutes), 0)::int as "total!"
           FROM approvals a
           LEFT JOIN time_entries te
             ON te.user_id = a.user_id
            AND te.spent_date BETWEEN a.period_start AND a.period_end
           WHERE a.id = ANY($1)
           GROUP BY a.id"#,
        &approved_ids,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;
    let totals: std::collections::HashMap<uuid::Uuid, i32> =
        total_rows.into_iter().map(|r| (r.id, r.total)).collect();

    // Announce each approval (FR-019) once the transition is durably committed.
    for a in &approvals {
        let total_minutes = totals.get(&a.id).copied().unwrap_or(0);
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::SubmissionApproved {
                occurred_at: chrono::Utc::now(),
                org_id: a.org_id,
                submission: submission_payload(a, total_minutes),
            });
    }

    Ok(approvals)
}

/// Ensure the caller may approve, mapping an insufficient role to a 403.
#[cfg(feature = "server")]
fn ensure_can_approve(manager: &User) -> Result<(), ServerFnError> {
    if horae_core::state::can_transition(
        EntryState::Submitted,
        EntryState::Approved,
        manager.org_role,
    ) {
        Ok(())
    } else {
        Err(forbidden("Insufficient role to approve submissions"))
    }
}

/// Approve a single submitted week. Requires manager role.
#[server]
pub async fn approve_submission(approval_id: String) -> Result<Approval, ServerFnError> {
    let manager = require_manager().await?;
    ensure_can_approve(&manager)?;
    let id = parse_uuid(&approval_id, "approval_id")?;

    approve_ids(&manager, &[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| not_found("Approval not found or not in 'submitted' state"))
}

/// Approve several submitted weeks at once (the "approve visible" action).
/// Returns the number actually approved; ids not in 'submitted' are skipped.
#[server]
pub async fn approve_submissions(approval_ids: Vec<String>) -> Result<usize, ServerFnError> {
    let manager = require_manager().await?;
    ensure_can_approve(&manager)?;
    let ids = approval_ids
        .iter()
        .map(|s| parse_uuid(s, "approval_id"))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(approve_ids(&manager, &ids).await?.len())
}

/// Reject a submitted week, or reopen an already-approved one. Requires
/// manager role. Reopens the period's time entries and deletes the approval
/// row, exactly reversing what submit (and approve) wrote.
#[server]
pub async fn reject_submission(approval_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;

    let state = crate::state::global_state().await;
    let approval_id = parse_uuid(&approval_id, "approval_id")?;

    let mut tx = state.db.begin().await.map_err(server_err)?;

    // Fetch the approval to know user + period + current state, locking the
    // row so a concurrent approve can't interleave with the reopen below.
    let approval = sqlx::query_as!(
        Approval,
        r#"SELECT id, org_id, user_id,
                period_start as "period_start: chrono::NaiveDate",
                period_end as "period_end: chrono::NaiveDate",
                state as "state: EntryState",
                submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                approved_by,
                approved_at as "approved_at: chrono::DateTime<chrono::Utc>"
         FROM approvals WHERE id = $1
         FOR UPDATE"#,
        approval_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Approval not found"))?;

    // Rejecting a pending week and reopening an approved one are both a
    // transition back to 'open'; the core state machine decides who may do
    // which from the approval's current state.
    if !horae_core::state::can_transition(approval.state, EntryState::Open, manager.org_role) {
        return Err(forbidden("Insufficient role to reject submissions"));
    }

    // Reopen entries: submitted ones (pending week) or approved ones
    // (reopened week), clearing the rounding persisted at submit time.
    sqlx::query!(
        "UPDATE time_entries
         SET state = $4, rounded_minutes = NULL
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND (state = $5 OR state = $6)",
        approval.user_id,
        approval.period_start as chrono::NaiveDate,
        approval.period_end as chrono::NaiveDate,
        EntryState::Open as EntryState,
        EntryState::Submitted as EntryState,
        EntryState::Approved as EntryState,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    // Delete the approval row (per schema: "reject deletes the row")
    sqlx::query!("DELETE FROM approvals WHERE id = $1", approval_id)
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;

    let total_minutes = week_total_minutes(
        &state.db,
        approval.user_id,
        approval.period_start,
        approval.period_end,
    )
    .await?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::SubmissionRejected {
            occurred_at: chrono::Utc::now(),
            org_id: approval.org_id,
            submission: submission_payload(&approval, total_minutes),
        });

    Ok(())
}

// DB-backed guard tests (`#[sqlx::test]`, throwaway database per test). They
// call the crate-internal guard directly, which `tests/` cannot do for a bin
// crate — so the real behaviour is pinned here, and the integration tests only
// mirror the SQL statements.
#[cfg(all(test, feature = "server"))]
mod tests {
    use super::week_has_running_timer;
    use crate::server_fns::test_seed::{SeedIds, seed};
    use horae_core::types::{EntryState, OrgRole};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn insert_entry(
        pool: &PgPool,
        ids: &SeedIds,
        spent_date: chrono::NaiveDate,
        is_running: bool,
    ) {
        sqlx::query!(
            "INSERT INTO time_entries \
               (id, org_id, user_id, project_id, task_id, spent_date, \
                minutes, billable, is_running, started_at, state) \
             VALUES ($1, $2, $3, $4, $5, $6, 0, true, $7, \
                     CASE WHEN $7 THEN now() END, $8)",
            Uuid::now_v7(),
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
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn sees_a_running_timer_inside_the_week(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let ws = chrono::Utc::now().date_naive();
        let we = ws + chrono::Duration::days(6);

        insert_entry(&pool, &ids, ws, true).await;

        assert!(
            week_has_running_timer(&pool, ids.user_id, ws, we)
                .await
                .unwrap()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn ignores_stopped_and_out_of_week_timers(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
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
mod submission_tests;
