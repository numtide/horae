//! Own time: the timer and the project/task picker.
//!
//! The locking and daily-limit helpers are shared with the Timesheet commands
//! in `server_fns/time_entries/commands.rs` until those move here too.

use chrono::{NaiveDate, Timelike};
use horae_core::permissions::catalog::Permission;
use horae_core::types::EntryState;
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::TimeEntry;
use crate::models::time_entry::TimeEntryContext;
use crate::plugin::AppEvent;
use crate::server_fns::time_entry_payload;

const ACTIVE_ACCOUNT_REQUIRED: &str = "Active account required";
const NO_RUNNING_TIMER: &str = "No running timer found for this entry";
const TIME_WRITE_REQUIRED: &str = "Current time-write scope required";

fn require_active(actor: &Actor) -> Result<(), OpError> {
    if actor.is_active() {
        Ok(())
    } else {
        Err(OpError::forbidden(ACTIVE_ACCOUNT_REQUIRED))
    }
}

/// Holds the person's time-write lock, which a timesheet submission takes
/// exclusively, so new time cannot slip into a week being submitted.
async fn begin_own_time_write(tx: &mut Tx, actor: &Actor) -> Result<(), OpError> {
    require_active(actor)?;
    crate::db::lock_time_entry_write(tx, actor.id()).await?;
    Ok(())
}

/// A person's total minutes per day, read and locked ahead of a write so
/// [`ensure_daily_limit`] can tell whether the write grew them.
pub(crate) async fn day_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    dates: &[NaiveDate],
) -> Result<Vec<(NaiveDate, i64)>, OpError> {
    Ok(crate::db::daily_minutes_totals(tx, user_id, dates).await?)
}

/// Refuse a write that grows the person's day past 240 hours, as Harvest does
/// (409 "Daily hours cannot exceed 240."). `before` is [`day_totals`] from
/// ahead of the write; a day already over the limit may still be lowered.
pub(crate) async fn ensure_daily_limit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    before: &[(NaiveDate, i64)],
) -> Result<(), OpError> {
    let dates: Vec<_> = before.iter().map(|(day, _)| *day).collect();
    let after = day_totals(tx, user_id, &dates).await?;
    for ((_, before), (_, after)) in before.iter().zip(&after) {
        horae_core::duration::check_daily_change(*before, *after)
            .map_err(|error| OpError::conflict(error.to_string()))?;
    }
    Ok(())
}

/// The minutes a stopping timer keeps: what is left of its day, so a stop never
/// fails and the timer is never trapped running. The notice says what was cut.
pub(crate) async fn clamp_stopped_minutes(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    spent_date: NaiveDate,
    current_minutes: i32,
    elapsed: u32,
) -> Result<(i32, Option<String>), OpError> {
    use horae_core::duration::{clamp_to_daily_allowance, format_hhmm};
    let [(_, total)] = day_totals(tx, user_id, &[spent_date]).await?[..] else {
        return Err(OpError::Internal("Day total unavailable".into()));
    };
    let kept = clamp_to_daily_allowance(elapsed, total - i64::from(current_minutes));
    let notice = (kept < elapsed).then(|| {
        format!(
            "Daily hours cannot exceed 240. The timer was stopped at {}; {} over the limit was not recorded.",
            format_hhmm(kept.into()),
            format_hhmm((elapsed - kept).into()),
        )
    });
    // At most MAX_DAILY_MINUTES, so it fits.
    Ok((kept as i32, notice))
}

/// Preserve archived history while enforcing current task grants. Lock both
/// the restriction and a matching grant so revocation cannot race the write.
pub(crate) async fn lock_task_access(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    project_id: Uuid,
    task_id: Uuid,
) -> Result<(), OpError> {
    let restricted = sqlx::query_scalar!(
        "SELECT s.restricted FROM project_task_settings s
         JOIN users u ON u.org_id = s.org_id AND u.id = $3
         WHERE s.project_id = $1 AND s.task_id = $2 FOR SHARE OF s",
        project_id,
        task_id,
        user_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or(false);
    if restricted {
        let grant = sqlx::query_scalar!(
            "SELECT id FROM project_task_members
             WHERE project_id = $1 AND task_id = $2 AND user_id = $3 FOR SHARE",
            project_id,
            task_id,
            user_id,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if grant.is_none() {
            return Err(OpError::conflict(
                "Task access is restricted. Refresh the timesheet and try again.",
            ));
        }
    }
    Ok(())
}

fn created(entry: &TimeEntry) -> AppEvent {
    AppEvent::TimeEntryCreated {
        occurred_at: chrono::Utc::now(),
        org_id: entry.org_id,
        time_entry: time_entry_payload(entry),
    }
}

fn stopped(entry: &TimeEntry) -> AppEvent {
    AppEvent::TimeEntryStopped {
        occurred_at: chrono::Utc::now(),
        org_id: entry.org_id,
        time_entry: time_entry_payload(entry),
    }
}

/// The actor's active, assigned project/task combinations; no rates.
pub(crate) async fn contexts(tx: &mut Tx, actor: &Actor) -> Result<Vec<TimeEntryContext>, OpError> {
    require_active(actor)?;
    Ok(sqlx::query_as!(
        TimeEntryContext,
        r#"SELECT project_id as "project_id!", task_id as "task_id!", billable as "billable!"
           FROM time_entry_contexts WHERE user_id = $1 ORDER BY project_id, task_id"#,
        actor.id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// New time for the actor. Starting a timer is the only production caller;
/// tests also add manual entries through it.
pub(crate) struct NewTimeEntry<'a> {
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    pub minutes: i32,
    pub notes: Option<&'a str>,
    pub billable: bool,
    pub start_minute: Option<i32>,
    pub is_running: bool,
}

/// Eligibility and effective billability are resolved in the insert's snapshot,
/// not in a pre-check separated from the write. The timer uniqueness index
/// arbitrates competing starts without a racy existence check.
pub(crate) async fn insert(
    tx: &mut Tx,
    actor: &Actor,
    input: NewTimeEntry<'_>,
) -> Result<TimeEntry, OpError> {
    begin_own_time_write(tx, actor).await?;
    // As on the Timesheet: own time needs the own or the everyone time-write grant.
    if !(actor.has(Permission::TimeWriteOwn) || actor.has(Permission::TimeWriteAll)) {
        return Err(OpError::forbidden(TIME_WRITE_REQUIRED));
    }
    let user_id = actor.id();
    lock_task_access(tx, user_id, input.project_id, input.task_id).await?;
    let day = day_totals(tx, user_id, &[input.spent_date]).await?;
    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes,
            billable, is_running, started_at, state, start_minute)
           SELECT $1, org_id, user_id, project_id, task_id, $5, $6, $7,
                  billable AND $8, $9, CASE WHEN $9 THEN now() END, 'open', $10
           FROM time_entry_contexts
           WHERE user_id = $2 AND project_id = $3 AND task_id = $4
           ON CONFLICT (user_id) WHERE is_running DO NOTHING
           RETURNING id, org_id, user_id, project_id, task_id,
                     spent_date as "spent_date: chrono::NaiveDate",
                     minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                     started_at as "started_at: chrono::DateTime<chrono::Utc>",
                     state as "state: EntryState", invoice_id,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>",
                     updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        Uuid::now_v7(), user_id, input.project_id, input.task_id,
        input.spent_date as chrono::NaiveDate, input.minutes, input.notes, input.billable,
        input.is_running, input.start_minute,
    ).fetch_optional(&mut **tx).await?
    .ok_or_else(|| OpError::conflict("Project/task is unavailable or not assigned, or a timer is already running."))?;
    ensure_daily_limit(tx, user_id, &day).await?;
    Ok(entry)
}

/// Starts a timer on the actor's today, in their own zone. Only one timer may
/// run at a time per person, enforced by the `one_running_timer_per_user`
/// partial unique index; a second start is reported as a conflict.
pub(crate) async fn start_timer(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    task_id: Uuid,
    notes: Option<&str>,
) -> Result<Done<TimeEntry>, OpError> {
    let spent_date = crate::timezone::today_for_person(&mut **tx, actor.id()).await?;
    let entry = insert(
        tx,
        actor,
        NewTimeEntry {
            project_id,
            task_id,
            spent_date,
            minutes: 0,
            notes,
            billable: true,
            start_minute: None,
            is_running: true,
        },
    )
    .await?;
    let event = created(&entry);
    Ok(Done::new(entry).with(event))
}

/// Stops the actor's running timer and records the elapsed minutes, clamped to
/// what is left of the day; the notice, when present, says how much the daily
/// limit cut. The stopped entry carries the project whose budget the adapter
/// checks after the commit.
pub(crate) async fn stop_timer(
    tx: &mut Tx,
    actor: &Actor,
    entry_id: Uuid,
) -> Result<Done<(TimeEntry, Option<String>)>, OpError> {
    // Stopping an own running timer remains possible after task access is revoked.
    begin_own_time_write(tx, actor).await?;
    let user_id = actor.id();

    // Read the running entry's start time, then compute the exact elapsed
    // minutes in `horae-core` (floored to the minute, no artificial 1-minute
    // minimum) so tracked totals stay exact (FR-003/FR-023). The row is locked
    // before the day, as on every other write path, so racing stops queue
    // instead of deadlocking and the clamp reads current minutes.
    let row = sqlx::query!(
        r#"SELECT started_at as "started_at: chrono::DateTime<chrono::Utc>",
                  state as "state: EntryState",
                  spent_date as "spent_date: chrono::NaiveDate", minutes
               FROM time_entries
               WHERE id = $1 AND user_id = $2 AND is_running = true FOR UPDATE"#,
        entry_id,
        user_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    // A locked (submitted/approved/invoiced) entry must not receive minutes:
    // its rounded_minutes were persisted at lock time, so a write here would
    // put time on the books that billing never sees.
    if row.state != EntryState::Open {
        return Err(OpError::conflict(
            "This entry was locked while its timer ran and can no longer be stopped. \
             Ask a manager to reject the submission first.",
        ));
    }

    let started_at = row
        .started_at
        .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    let elapsed = horae_core::duration::minutes_between(started_at, chrono::Utc::now());
    let (minutes, notice) =
        clamp_stopped_minutes(tx, user_id, row.spent_date, row.minutes, elapsed).await?;

    // Record the clock time the timer started as the entry's start time (D9), so
    // it lands on the calendar at the right hour. `started_at` is UTC, matching
    // how `spent_date` is derived. A timer that ran across midnight would exceed
    // the day, so it stays untimed in that case.
    let raw_start = started_at.hour() as i32 * 60 + started_at.minute() as i32;
    let snapped = horae_core::time_of_day::snap(raw_start, horae_core::time_of_day::SNAP_STEP)
        .min(i32::from(horae_core::time_of_day::DAY_MINUTES) - 1);
    let start_minute =
        (snapped + minutes <= i32::from(horae_core::time_of_day::DAY_MINUTES)).then_some(snapped);

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET is_running = false,
             minutes = $3,
             start_minute = $4,
             started_at = NULL,
             notified_long_running_at = NULL
         WHERE id = $1 AND user_id = $2 AND is_running = true AND state = $5
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        minutes,
        start_minute,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NO_RUNNING_TIMER))?;

    let event = stopped(&entry);
    Ok(Done::new((entry, notice)).with(event))
}

/// The actor's running timer, if any.
pub(crate) async fn current_timer(
    tx: &mut Tx,
    actor: &Actor,
) -> Result<Option<TimeEntry>, OpError> {
    require_active(actor)?;
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
         FROM time_entries
         WHERE user_id = $1 AND is_running = true
         LIMIT 1"#,
        actor.id(),
    )
    .fetch_optional(&mut **tx)
    .await?)
}

#[cfg(test)]
mod tests;
