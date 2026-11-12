//! Weekly timesheet approvals: submitting one's own week, and listing,
//! approving, rejecting and reopening submitted weeks.
//!
//! Submitting needs `TimeWriteOwn`. Everything else needs approval access:
//! `TimeApproveAll`, or `TimeApproveManaged` over the people the actor
//! manages, together with read access to their time. A request is judged on
//! its complete set of weeks (the week listed, or the weeks an approval would
//! change) and is denied whole when any of them is outside that access, never
//! narrowed to the covered part (FR-024). Coverage by project and date (AC08)
//! is not decided here: a week is covered through its owner alone.

use chrono::{Datelike, Days, NaiveDate};
use horae_core::permissions::approvals::{approval_records_covered, may_decide_submission};
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::{ManagementAssignments, ScopedResource};
use horae_core::types::{EntryState, RoundDir};
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::{Approval, ApprovalSummary};
use crate::plugin::AppEvent;
use crate::server_fns::submission_payload;

const ACTIVE_REQUIRED: &str = "Active same-organization account required";
const APPROVAL_REQUIRED: &str = "Approval access is required to review timesheets";
/// Names no person, week or count, so a refused selection reveals nothing
/// about the records the actor cannot see.
const NOT_COVERED: &str = "Your approval access does not cover every selected timesheet";
/// Also names no other record, so a bulk request reveals nothing about the rest.
const OWN_SUBMISSION_FORBIDDEN: &str = "You cannot approve or reject your own submission";
const ALREADY_APPROVED: &str =
    "This week is already approved; it must be reopened before it can be resubmitted";
/// One string for both places submission detects a timer, so they cannot drift.
const RUNNING_TIMER_CONFLICT: &str = "A timer is running in this week. Stop it before submitting.";

/// Submits the actor's own week: every open entry in `[week_start,
/// week_start + 6]` becomes submitted, with its rounded minutes frozen, and
/// the week gets a pending approval.
pub(crate) async fn submit_week(
    tx: &mut Tx,
    actor: &Actor,
    week_start: NaiveDate,
) -> Result<Done<Approval>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden(ACTIVE_REQUIRED));
    }
    if !actor.has(Permission::TimeWriteOwn) {
        return Err(OpError::forbidden("You cannot submit your own time"));
    }
    let (user_id, org_id) = (actor.id(), actor.org_id());
    let week_end = week_end(week_start)?;
    let config = sqlx::query!(
        r#"SELECT week_start, round_minutes, round_dir as "round_dir: RoundDir"
           FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    // Writers take the shared form of this lock before changing any entries.
    // Taking the user-wide lock first covers inserts and cross-week moves, not
    // just rows that happened to exist when submission started.
    sqlx::query!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
        user_id,
    )
    .execute(&mut **tx)
    .await?;
    if week_start.weekday().number_from_monday() as i16 != config.week_start {
        return Err(OpError::invalid(
            "week_start",
            "week_start must match the organization's first weekday",
        ));
    }

    // An approved week cannot be resubmitted: silently downgrading it to
    // pending would erase who approved it and when. It must be reopened first.
    // The row lock keeps a concurrent approval from slipping between this check
    // and the upsert below.
    let existing = sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM approvals
           WHERE user_id = $1 AND period_start = $2
           FOR UPDATE"#,
        user_id,
        week_start as NaiveDate,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if existing == Some(EntryState::Approved) {
        return Err(OpError::conflict(ALREADY_APPROVED));
    }

    // A running timer cannot be frozen before its elapsed minutes are known.
    // The write barrier prevents a timer from starting after this check.
    if week_has_running_timer(&mut **tx, user_id, week_start, week_end).await? {
        return Err(OpError::conflict(RUNNING_TIMER_CONFLICT));
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
        week_start as NaiveDate,
        week_end as NaiveDate,
        EntryState::Submitted as EntryState,
        EntryState::Open as EntryState,
        org_id,
        config.round_minutes,
        config.round_dir as RoundDir,
    )
    .execute(&mut **tx)
    .await?;
    if result.rows_affected() == 0 {
        if week_has_running_timer(&mut **tx, user_id, week_start, week_end).await? {
            return Err(OpError::conflict(RUNNING_TIMER_CONFLICT));
        }
        return Err(OpError::not_found("No open entries found for this week"));
    }

    // The conditional upsert stays as a backstop against downgrading approval.
    let approval = sqlx::query_as!(
        Approval,
        r#"INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, period_start) DO UPDATE
           SET state = $6, submitted_at = now()
           WHERE approvals.state <> $7
         RETURNING id, org_id, user_id,
                   period_start as "period_start: NaiveDate",
                   period_end as "period_end: NaiveDate",
                   state as "state: EntryState",
                   submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                   approved_by,
                   approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        Uuid::now_v7(),
        org_id,
        user_id,
        week_start as NaiveDate,
        week_end as NaiveDate,
        EntryState::Submitted as EntryState,
        EntryState::Approved as EntryState,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::conflict(ALREADY_APPROVED))?;

    let total_minutes = week_total_minutes(&mut **tx, &approval).await?;
    let event = AppEvent::TimesheetSubmitted {
        occurred_at: chrono::Utc::now(),
        org_id,
        submission: submission_payload(&approval, total_minutes),
    };
    Ok(Done::new(approval).with(event))
}

/// One week's approvals in the organization, optionally in one state. The
/// whole week must be within the actor's approval access, or nothing is listed.
pub(crate) async fn list(
    tx: &mut Tx,
    actor: &Actor,
    state: Option<EntryState>,
    week_start: NaiveDate,
) -> Result<Vec<ApprovalSummary>, OpError> {
    require_approval_access(actor)?;
    let first_day = sqlx::query_scalar!(
        "SELECT week_start FROM organizations WHERE id = $1",
        actor.org_id()
    )
    .fetch_one(&mut **tx)
    .await?;
    if week_start.weekday().number_from_monday() as i16 != first_day {
        return Err(OpError::invalid(
            "week_start",
            "week_start must match the organization's first weekday",
        ));
    }
    week_end(week_start)?;

    // Hours are aggregated per row from the user's entries in the approval's
    // period (actual `minutes`, split by `billable`) via a lateral join, so the
    // whole table comes back in one query rather than a lookup per approval.
    let rows = sqlx::query!(
        r#"SELECT a.id, a.org_id, a.user_id, u.name as user_name,
                a.period_start as "period_start: NaiveDate",
                a.period_end as "period_end: NaiveDate",
                a.state as "state: EntryState",
                a.submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                a.approved_by,
                a.approved_at as "approved_at: chrono::DateTime<chrono::Utc>",
                COALESCE(t.total_minutes, 0) as "total_minutes!",
                COALESCE(t.billable_minutes, 0) as "billable_minutes!"
         FROM approvals a
         JOIN users u ON u.id = a.user_id AND u.org_id = a.org_id
         LEFT JOIN LATERAL (
             SELECT (SUM(minutes))::bigint as total_minutes,
                    (SUM(minutes) FILTER (WHERE billable))::bigint as billable_minutes
             FROM time_entries te
             WHERE te.org_id = a.org_id
               AND te.user_id = a.user_id
               AND te.spent_date BETWEEN a.period_start AND a.period_end
         ) t ON true
         WHERE a.org_id = $2
           AND a.period_start = $3
           AND ($1::entry_state IS NULL OR a.state = $1)
         ORDER BY u.name, a.id"#,
        state as Option<EntryState>,
        actor.org_id(),
        week_start as NaiveDate,
    )
    .fetch_all(&mut **tx)
    .await?;

    let owners: Vec<Uuid> = rows.iter().map(|row| row.user_id).collect();
    require_coverage(tx, actor, &owners).await?;
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
            user_name: r.user_name,
            total_minutes: r.total_minutes,
            billable_minutes: r.billable_minutes,
        })
        .collect())
}

/// Approves every submitted week in `ids`, with its submitted entries, and
/// returns the weeks approved. Ids that are foreign or not pending are skipped;
/// if any approved week is the actor's own (for a non-administrator) or outside
/// their approval access, the whole request is denied.
pub(crate) async fn approve(
    tx: &mut Tx,
    actor: &Actor,
    ids: &[Uuid],
) -> Result<Done<Vec<Approval>>, OpError> {
    require_approval_access(actor)?;
    let approvals = sqlx::query_as!(
        Approval,
        r#"UPDATE approvals
             SET state = $2, approved_by = $3, approved_at = now()
           WHERE id = ANY($1) AND state = $4 AND org_id = $5
        RETURNING id, org_id, user_id,
                  period_start as "period_start: NaiveDate",
                  period_end as "period_end: NaiveDate",
                  state as "state: EntryState",
                  submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                  approved_by,
                  approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        ids,
        EntryState::Approved as EntryState,
        actor.id(),
        EntryState::Submitted as EntryState,
        actor.org_id(),
    )
    .fetch_all(&mut **tx)
    .await?;
    // Judge exactly the rows this request changed. An error leaves the
    // transaction uncommitted, which rolls the update back.
    let owners: Vec<Uuid> = approvals.iter().map(|a| a.user_id).collect();
    require_others(actor, &owners)?;
    require_coverage(tx, actor, &owners).await?;

    let approved_ids: Vec<Uuid> = approvals.iter().map(|a| a.id).collect();
    sqlx::query!(
        r#"UPDATE time_entries te
              SET state = $2
             FROM approvals a
            WHERE a.id = ANY($1)
              AND te.user_id = a.user_id
              AND te.org_id = a.org_id
              AND te.spent_date BETWEEN a.period_start AND a.period_end
              AND te.state = $3"#,
        &approved_ids,
        EntryState::Approved as EntryState,
        EntryState::Submitted as EntryState,
    )
    .execute(&mut **tx)
    .await?;

    // Every approved week's tracked minutes in one grouped query. Like
    // `week_total_minutes`, all of the user's entries in the period count,
    // whatever their state.
    let totals: std::collections::HashMap<Uuid, i32> = sqlx::query!(
        r#"SELECT a.id as "id!",
                  COALESCE(SUM(te.minutes), 0)::int as "total!"
           FROM approvals a
           LEFT JOIN time_entries te
             ON te.user_id = a.user_id
            AND te.org_id = a.org_id
            AND te.spent_date BETWEEN a.period_start AND a.period_end
           WHERE a.id = ANY($1) AND a.org_id = $2
           GROUP BY a.id"#,
        &approved_ids,
        actor.org_id(),
    )
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|row| (row.id, row.total))
    .collect();

    // Announce each approval (FR-019) once the transition has committed.
    let events: Vec<AppEvent> = approvals
        .iter()
        .map(|a| AppEvent::SubmissionApproved {
            occurred_at: chrono::Utc::now(),
            org_id: a.org_id,
            submission: submission_payload(a, totals.get(&a.id).copied().unwrap_or(0)),
        })
        .collect();
    Ok(Done {
        value: approvals,
        events,
    })
}

/// Rejects a pending week or reopens an approved one: its entries go back to
/// open and the approval row is deleted, exactly reversing what submission
/// (and approval) wrote. Returns the approval as it was.
pub(crate) async fn reopen(
    tx: &mut Tx,
    actor: &Actor,
    approval_id: Uuid,
) -> Result<Done<Approval>, OpError> {
    require_approval_access(actor)?;
    // Lock the row so a concurrent approval cannot interleave with the reopen.
    let approval = sqlx::query_as!(
        Approval,
        r#"SELECT id, org_id, user_id,
                period_start as "period_start: NaiveDate",
                period_end as "period_end: NaiveDate",
                state as "state: EntryState",
                submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                approved_by,
                approved_at as "approved_at: chrono::DateTime<chrono::Utc>"
         FROM approvals WHERE id = $1 AND org_id = $2
         FOR UPDATE"#,
        approval_id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found("Approval not found"))?;

    // Reopening an approved week is the same decision as rejecting it. Decided
    // before the state, so someone who may not decide this week cannot learn
    // whether it has been invoiced.
    require_others(actor, &[approval.user_id])?;
    require_coverage(tx, actor, &[approval.user_id]).await?;
    // Rejecting a pending week and reopening an approved one are both a move
    // back to open; invoiced time never reopens.
    if !horae_core::state::can_transition(approval.state, EntryState::Open) {
        return Err(OpError::conflict("This week can no longer be reopened"));
    }

    // Reopen submitted entries (pending week) or approved ones (reopened
    // week), clearing the rounding persisted at submission.
    sqlx::query!(
        "UPDATE time_entries
         SET state = $4, rounded_minutes = NULL
         WHERE user_id = $1
           AND org_id = $7
           AND spent_date BETWEEN $2 AND $3
           AND (state = $5 OR state = $6)",
        approval.user_id,
        approval.period_start as NaiveDate,
        approval.period_end as NaiveDate,
        EntryState::Open as EntryState,
        EntryState::Submitted as EntryState,
        EntryState::Approved as EntryState,
        actor.org_id(),
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM approvals WHERE id = $1 AND org_id = $2",
        approval_id,
        actor.org_id(),
    )
    .execute(&mut **tx)
    .await?;

    let total_minutes = week_total_minutes(&mut **tx, &approval).await?;
    let event = AppEvent::SubmissionRejected {
        occurred_at: chrono::Utc::now(),
        org_id: approval.org_id,
        submission: submission_payload(&approval, total_minutes),
    };
    Ok(Done::new(approval).with(event))
}

/// Approval access of either scope; whether it covers particular weeks is
/// [`require_coverage`]'s question.
fn require_approval_access(actor: &Actor) -> Result<(), OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden(ACTIVE_REQUIRED));
    }
    if actor.has(Permission::TimeApproveAll) || actor.has(Permission::TimeApproveManaged) {
        Ok(())
    } else {
        Err(OpError::forbidden(APPROVAL_REQUIRED))
    }
}

/// The self-approval rule: only an administrator decides their own week.
fn require_others(actor: &Actor, owners: &[Uuid]) -> Result<(), OpError> {
    if owners
        .iter()
        .all(|&owner| may_decide_submission(actor.id(), actor.administrator(), owner))
    {
        Ok(())
    } else {
        Err(OpError::forbidden(OWN_SUBMISSION_FORBIDDEN))
    }
}

/// Denies the request unless the actor's approval and time-read access covers
/// every owner's week. The management assignments are read inside the
/// transaction whose organization lock holds off access changes.
async fn require_coverage(tx: &mut Tx, actor: &Actor, owners: &[Uuid]) -> Result<(), OpError> {
    let Some(grants) = actor.grants() else {
        return Err(OpError::forbidden(APPROVAL_REQUIRED));
    };
    let managed = sqlx::query_scalar!(
        "SELECT managed_user_id FROM person_management_assignments
         WHERE org_id = $1 AND manager_id = $2",
        actor.org_id(),
        actor.id(),
    )
    .fetch_all(&mut **tx)
    .await?;
    let assignments = ManagementAssignments {
        actor_id: actor.id(),
        org_id: actor.org_id(),
        people: &managed,
        projects: &[],
    };
    let weeks: Vec<ScopedResource> = owners
        .iter()
        .map(|&owner| ScopedResource {
            org_id: actor.org_id(),
            person_id: Some(owner),
            project_id: None,
        })
        .collect();
    if approval_records_covered(grants, actor.identity(), &assignments, &weeks, &[]) {
        Ok(())
    } else {
        Err(OpError::forbidden(NOT_COVERED))
    }
}

fn week_end(week_start: NaiveDate) -> Result<NaiveDate, OpError> {
    week_start
        .checked_add_days(Days::new(6))
        .ok_or_else(|| OpError::invalid("week_start", "week_start is out of range"))
}

/// True when the user has a running timer dated inside `[start, end]`.
pub(crate) async fn week_has_running_timer(
    db: impl sqlx::PgExecutor<'_>,
    user_id: Uuid,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<bool, OpError> {
    Ok(sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM time_entries
           WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3 AND is_running)",
        user_id,
        start as NaiveDate,
        end as NaiveDate,
    )
    .fetch_one(db)
    .await?
    .unwrap_or(false))
}

/// Tracked minutes in an approval's period, whatever the entries' state, for
/// the submission events.
pub(crate) async fn week_total_minutes(
    db: impl sqlx::PgExecutor<'_>,
    approval: &Approval,
) -> Result<i32, OpError> {
    Ok(sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0)::int as "total!"
           FROM time_entries
           WHERE user_id = $1 AND org_id = $4 AND spent_date BETWEEN $2 AND $3"#,
        approval.user_id,
        approval.period_start as NaiveDate,
        approval.period_end as NaiveDate,
        approval.org_id,
    )
    .fetch_one(db)
    .await?)
}

#[cfg(test)]
pub(crate) mod tests;
