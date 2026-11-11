//! Time-entry server functions.

use super::*;

#[cfg(feature = "server")]
mod commands;

/// Eligible, writable tracking choices for one explicitly selected person.
#[server]
pub async fn load_timesheet_tracking(
    context: crate::models::scoped_time::TimesheetWriteContext,
) -> Result<Vec<crate::models::scoped_time::TimesheetTrackingOption>, ServerFnError> {
    let actor = require_user().await.map_err(commands::public_error)?;
    let state = crate::state::global_state().await;
    commands::tracking(&state.db, actor.org_id, actor.id, &context)
        .await
        .map_err(commands::public_error)
}

/// Apply one person-bound operation; client expectations never grant authority.
/// Returns a notice for the person when the operation was adjusted, e.g. a
/// stopped timer clamped to the daily limit.
#[server]
pub async fn apply_timesheet_command(
    context: crate::models::scoped_time::TimesheetWriteContext,
    command: crate::models::scoped_time::TimesheetCommand,
) -> Result<Option<String>, ServerFnError> {
    let actor = require_user().await.map_err(commands::public_error)?;
    let state = crate::state::global_state().await;
    let effects = commands::apply(&state.db, actor.org_id, actor.id, &context, command)
        .await
        .map_err(commands::public_error)?;
    for (entry, event) in effects.entries {
        dispatch_time_entry_event(&entry, event).await;
    }
    for project in effects.projects {
        tokio::spawn(check_project_budget(state, project));
    }
    Ok(effects.notice)
}

/// Resolve a Timesheet subject and its rows without a separate identity bootstrap.
#[server]
pub async fn load_timesheet_page(
    query: crate::models::scoped_time::TimesheetQuery,
) -> Result<crate::models::scoped_time::TimesheetPage, ServerFnError> {
    use permissions::time_entries::TimeReadError;

    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate Timesheet read");
            server_err("Timesheet is unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::time_entries::sheet(&state.db, user.org_id, user.id, &query)
        .await
        .map_err(|error| match error {
            TimeReadError::Forbidden => {
                forbidden("Timesheet access or identity has changed; refresh the page")
            }
            TimeReadError::InvalidQuery => {
                err(BAD_REQUEST, "Invalid Timesheet date range or cursor")
            }
            error => {
                tracing::error!(%error, "Timesheet read failed");
                server_err("Timesheet is unavailable")
            }
        })
}

/// Minimal authorized person labels; selection never grants entry access.
#[server]
pub async fn list_timesheet_people(
    query: crate::models::scoped_time::TimesheetPeopleQuery,
) -> Result<crate::models::scoped_time::TimesheetPeoplePage, ServerFnError> {
    use permissions::time_entries::TimeReadError;

    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate timesheet people read");
            server_err("Timesheet people are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::time_entries::people(&state.db, user.org_id, user.id, &query)
        .await
        .map_err(|error| match error {
            TimeReadError::Forbidden => forbidden("Current time-read authority is required"),
            TimeReadError::InvalidQuery => err(BAD_REQUEST, "Invalid timesheet people query"),
            error => {
                tracing::error!(%error, "Scoped timesheet people read failed");
                server_err("Timesheet people are unavailable")
            }
        })
}

/// Read a bounded page of time entries under current canonical permissions.
#[server]
pub async fn list_visible_time_entries(
    query: crate::models::scoped_time::TimeEntryQuery,
) -> Result<crate::models::scoped_time::TimeEntryPage, ServerFnError> {
    use permissions::time_entries::TimeReadError;

    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate time-entry read");
            server_err("Time entries are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::time_entries::read(&state.db, user.org_id, user.id, &query)
        .await
        .map_err(|error| match error {
            TimeReadError::Forbidden => forbidden("Current time-read authority is required"),
            TimeReadError::InvalidQuery => {
                err(BAD_REQUEST, "Invalid time-entry date range or cursor")
            }
            error => {
                tracing::error!(%error, "Scoped time-entry read failed");
                server_err("Time entries are unavailable")
            }
        })
}

/// Validate an optional start time (minutes since midnight, 0..=1439) and clamp
/// the duration so the entry never crosses midnight (Constitution: exactness).
/// Returns the possibly-clamped minutes and the validated start.
#[cfg(feature = "server")]
fn normalize_start(
    minutes: i32,
    start_minute: Option<i32>,
) -> Result<(i32, Option<i32>), ServerFnError> {
    // One entry past the day's limit can never fit; refuse it before the
    // database trigger turns it into an opaque error.
    horae_core::duration::check_daily_total(minutes.into()).map_err(conflict)?;
    match start_minute {
        None => Ok((minutes, None)),
        Some(sm) if (0..=1439).contains(&sm) => {
            // Snap the start to the grid so every write path (drag-move/resize,
            // typed times) lands on a tidy boundary (FR-008); snap first, then
            // clamp the duration against the snapped start.
            let snapped = horae_core::time_of_day::snap(sm, horae_core::time_of_day::SNAP_STEP)
                .clamp(0, 1439);
            let clamped =
                horae_core::time_of_day::clamp_to_day(snapped as u16, minutes.max(0) as u32) as i32;
            Ok((clamped, Some(snapped)))
        }
        Some(_) => Err(server_err("start time must be within the day (0..=1439)")),
    }
}

/// Only the session user's active, assigned project/task combinations; no rates.
#[server]
pub async fn list_time_entry_contexts()
-> Result<Vec<crate::models::time_entry::TimeEntryContext>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    fetch_time_entry_contexts(&state.db, user.id)
        .await
        .map_err(server_err)
}

#[cfg(feature = "server")]
async fn fetch_time_entry_contexts(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Vec<crate::models::time_entry::TimeEntryContext>, sqlx::Error> {
    sqlx::query_as!(
        crate::models::time_entry::TimeEntryContext,
        r#"SELECT project_id as "project_id!", task_id as "task_id!", billable as "billable!"
           FROM time_entry_contexts WHERE user_id = $1 ORDER BY project_id, task_id"#,
        user_id,
    )
    .fetch_all(db)
    .await
}

#[cfg(feature = "server")]
struct NewTimeEntry<'a> {
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    spent_date: chrono::NaiveDate,
    minutes: i32,
    notes: Option<&'a str>,
    billable: bool,
    start_minute: Option<i32>,
    is_running: bool,
}

/// Keep current account activity stable through the interactive mutation.
#[cfg(feature = "server")]
async fn begin_active_time_write(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, ServerFnError> {
    let mut tx = pool.begin().await.map_err(server_err)?;
    // Bound the activity check independently of pooled connection defaults.
    super::permissions::configure_administration(&mut tx)
        .await
        .map_err(server_err)?;
    // Routing only: recheck the same tenant and activity after the organization gate.
    let org_id = sqlx::query_scalar!("SELECT org_id FROM users WHERE id=$1", user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(server_err)?
        .ok_or_else(|| forbidden("Active account required"))?;
    crate::db::lock_organization(&mut tx, org_id, crate::db::OrganizationLock::Shared)
        .await
        .map_err(server_err)?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id=$1 AND org_id=$2 AND active FOR SHARE",
        user_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| forbidden("Active account required"))?;
    crate::db::lock_time_entry_write(&mut tx, user_id)
        .await
        .map_err(server_err)?;
    Ok(tx)
}

/// A person's total minutes per day, read and locked ahead of a write so
/// [`ensure_daily_limit`] can tell whether the write grew them.
#[cfg(feature = "server")]
async fn day_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    dates: &[chrono::NaiveDate],
) -> Result<Vec<(chrono::NaiveDate, i64)>, ServerFnError> {
    crate::db::daily_minutes_totals(tx, user_id, dates)
        .await
        .map_err(server_err)
}

/// Refuse a write that grows the person's day past 240 hours, as Harvest does
/// (409 "Daily hours cannot exceed 240."). `before` is [`day_totals`] from
/// ahead of the write; a day already over the limit may still be lowered.
#[cfg(feature = "server")]
async fn ensure_daily_limit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    before: &[(chrono::NaiveDate, i64)],
) -> Result<(), ServerFnError> {
    let dates: Vec<_> = before.iter().map(|(day, _)| *day).collect();
    let after = day_totals(tx, user_id, &dates).await?;
    for ((_, before), (_, after)) in before.iter().zip(&after) {
        horae_core::duration::check_daily_change(*before, *after).map_err(conflict)?;
    }
    Ok(())
}

/// The minutes a stopping timer keeps: what is left of its day, so a stop never
/// fails and the timer is never trapped running. The notice says what was cut.
#[cfg(feature = "server")]
async fn clamp_stopped_minutes(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    spent_date: chrono::NaiveDate,
    current_minutes: i32,
    elapsed: u32,
) -> Result<(i32, Option<String>), ServerFnError> {
    use horae_core::duration::{clamp_to_daily_allowance, format_hhmm};
    let [(_, total)] = day_totals(tx, user_id, &[spent_date]).await?[..] else {
        return Err(server_err("Day total unavailable"));
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
#[cfg(feature = "server")]
async fn lock_task_access(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
) -> Result<(), ServerFnError> {
    let restricted = sqlx::query_scalar!(
        "SELECT s.restricted FROM project_task_settings s
         JOIN users u ON u.org_id = s.org_id AND u.id = $3
         WHERE s.project_id = $1 AND s.task_id = $2 FOR SHARE OF s",
        project_id,
        task_id,
        user_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
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
        .await
        .map_err(server_err)?;
        if grant.is_none() {
            return Err(conflict(
                "Task access is restricted. Refresh the timesheet and try again.",
            ));
        }
    }
    Ok(())
}

/// Eligibility and effective billability are resolved in the insert's snapshot,
/// not in a pre-check separated from the write. The timer uniqueness index
/// arbitrates competing starts without a racy existence check.
#[cfg(feature = "server")]
async fn insert_time_entry(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    input: NewTimeEntry<'_>,
) -> Result<TimeEntry, ServerFnError> {
    let mut tx = begin_active_time_write(db, user_id).await?;
    lock_task_access(&mut tx, user_id, input.project_id, input.task_id).await?;
    let day = day_totals(&mut tx, user_id, &[input.spent_date]).await?;
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
        uuid::Uuid::now_v7(), user_id, input.project_id, input.task_id,
        input.spent_date as chrono::NaiveDate, input.minutes, input.notes, input.billable,
        input.is_running, input.start_minute,
    ).fetch_optional(&mut *tx).await.map_err(server_err)?
    .ok_or_else(|| conflict("Project/task is unavailable or not assigned, or a timer is already running."))?;
    ensure_daily_limit(&mut tx, user_id, &day).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(entry)
}

// ── Time Entries ─────────────────────────────────────────────────────────────

/// Start a timer for the given project and task. Only one timer may run at a
/// time per user, enforced by the `one_running_timer_per_user` partial unique
/// index; a second start is reported as a conflict.
#[server]
pub async fn start_timer(
    project_id: String,
    task_id: String,
    notes: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let entry =
        start_timer_record(&state.db, user.id, project_id, task_id, notes.as_deref()).await?;
    dispatch_time_entry_event(&entry, TimeEntryEvent::Created).await;
    Ok(entry)
}

/// A running entry on the person's today, in their own zone.
#[cfg(feature = "server")]
async fn start_timer_record(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
    notes: Option<&str>,
) -> Result<TimeEntry, ServerFnError> {
    insert_time_entry(
        db,
        user_id,
        NewTimeEntry {
            project_id,
            task_id,
            spent_date: crate::timezone::today_for_person(db, user_id)
                .await
                .map_err(server_err)?,
            minutes: 0,
            notes,
            billable: true,
            start_minute: None,
            is_running: true,
        },
    )
    .await
}

/// Stop a running timer and record elapsed minutes, clamped to what is left of
/// the day; the notice, when present, says how much the daily limit cut.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<(TimeEntry, Option<String>), ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let (entry, notice) = stop_entry_timer(&state.db, user_id, entry_id).await?;
    dispatch_time_entry_event(&entry, TimeEntryEvent::Stopped).await;
    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok((entry, notice))
}

#[cfg(feature = "server")]
async fn stop_entry_timer(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    entry_id: uuid::Uuid,
) -> Result<(TimeEntry, Option<String>), ServerFnError> {
    // Stopping an own running timer remains possible after task access is revoked.
    let mut tx = begin_active_time_write(db, user_id).await?;

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
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("No running timer found for this entry"))?;

    // A locked (submitted/approved/invoiced) entry must not receive minutes:
    // its rounded_minutes were persisted at lock time, so a write here would
    // put time on the books that billing never sees.
    if row.state != EntryState::Open {
        return Err(conflict(
            "This entry was locked while its timer ran and can no longer be stopped. \
             Ask a manager to reject the submission first.",
        ));
    }

    let started_at = row
        .started_at
        .ok_or_else(|| not_found("No running timer found for this entry"))?;

    let elapsed = horae_core::duration::minutes_between(started_at, chrono::Utc::now());
    let (minutes, notice) =
        clamp_stopped_minutes(&mut tx, user_id, row.spent_date, row.minutes, elapsed).await?;

    // Record the clock time the timer started as the entry's start time (D9), so
    // it lands on the calendar at the right hour. `started_at` is UTC, matching
    // how `spent_date` is derived. A timer that ran across midnight would exceed
    // the day, so it stays untimed in that case.
    use chrono::Timelike;
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
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("No running timer found for this entry"))?;

    tx.commit().await.map_err(server_err)?;
    Ok((entry, notice))
}

/// Return the currently running timer for the authenticated user, if any.
#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;

    let entry = sqlx::query_as!(
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
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entry)
}

#[cfg(all(test, feature = "server"))]
mod update_tests;

#[cfg(all(test, feature = "server"))]
mod activity_tests;

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        CONFLICT, NewTimeEntry, OrgRole, fetch_time_entry_contexts, insert_time_entry,
        normalize_start, start_timer_record,
    };
    use crate::server_fns::test_seed::seed;
    use dioxus::prelude::ServerFnError;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn linked_seed(pool: &PgPool, role: OrgRole) -> crate::server_fns::test_seed::SeedIds {
        let ids = seed(pool, role).await;
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
            ids.project_id,
            ids.task_id
        )
        .execute(pool)
        .await
        .unwrap();
        ids
    }

    fn manual_entry(ids: &crate::server_fns::test_seed::SeedIds) -> NewTimeEntry<'static> {
        NewTimeEntry {
            project_id: ids.project_id,
            task_id: ids.task_id,
            spent_date: "2026-09-07".parse().unwrap(),
            minutes: 60,
            notes: None,
            billable: true,
            start_minute: None,
            is_running: false,
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_timer_is_dated_in_the_persons_zone(pool: PgPool) {
        use chrono::Timelike;
        let ids = linked_seed(&pool, OrgRole::Admin).await;
        // UTC+14 or UTC−12, whichever is on another date than UTC right now.
        let (zone, hours) = if chrono::Utc::now().hour() >= 11 {
            ("Pacific/Kiritimati", 14)
        } else {
            ("Etc/GMT+12", -12)
        };
        sqlx::query!(
            "UPDATE users SET timezone = $2 WHERE id = $1",
            ids.user_id,
            zone
        )
        .execute(&pool)
        .await
        .unwrap();
        let day = || (chrono::Utc::now() + chrono::Duration::hours(hours)).date_naive();
        let before = day();
        let entry = start_timer_record(&pool, ids.user_id, ids.project_id, ids.task_id, None)
            .await
            .unwrap();
        assert!([before, day()].contains(&entry.spent_date), "{entry:?}");
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_and_picker_reject_inactive_or_foreign_contexts(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Admin).await;
        let other = linked_seed(&pool, OrgRole::Admin).await;
        assert_eq!(
            fetch_time_entry_contexts(&pool, ids.user_id)
                .await
                .unwrap()
                .len(),
            1
        );
        for (project_id, task_id) in [
            (ids.project_id, other.task_id),
            (other.project_id, other.task_id),
            (ids.project_id, Uuid::now_v7()),
        ] {
            for is_running in [false, true] {
                let input = NewTimeEntry {
                    project_id,
                    task_id,
                    is_running,
                    ..manual_entry(&ids)
                };
                assert!(insert_time_entry(&pool, ids.user_id, input).await.is_err());
            }
        }
        for inactive in ["client", "project", "task", "user"] {
            sqlx::query!(
                "UPDATE clients SET active = ($2 <> 'client') WHERE id = $1",
                ids.client_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE projects SET active = ($2 <> 'project') WHERE id = $1",
                ids.project_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE tasks SET active = ($2 <> 'task') WHERE id = $1",
                ids.task_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE users SET active = ($2 <> 'user') WHERE id = $1",
                ids.user_id,
                inactive
            )
            .execute(&pool)
            .await
            .unwrap();
            assert!(
                fetch_time_entry_contexts(&pool, ids.user_id)
                    .await
                    .unwrap()
                    .is_empty(),
                "{inactive}"
            );
            for is_running in [false, true] {
                assert!(
                    insert_time_entry(
                        &pool,
                        ids.user_id,
                        NewTimeEntry {
                            is_running,
                            ..manual_entry(&ids)
                        }
                    )
                    .await
                    .is_err(),
                    "{inactive}"
                );
            }
        }
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM time_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(0)
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_and_picker_require_a_project_task_link(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;
        assert!(
            fetch_time_entry_contexts(&pool, ids.user_id)
                .await
                .unwrap()
                .is_empty()
        );
        for is_running in [false, true] {
            assert!(
                insert_time_entry(
                    &pool,
                    ids.user_id,
                    NewTimeEntry {
                        is_running,
                        ..manual_entry(&ids)
                    }
                )
                .await
                .is_err()
            );
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn new_time_uses_effective_billability_for_manual_entries_and_timers(pool: PgPool) {
        for project_billable in [false, true] {
            for task_billable in [false, true] {
                for requested_billable in [false, true] {
                    for is_running in [false, true] {
                        let ids = linked_seed(&pool, OrgRole::Admin).await;
                        sqlx::query!("UPDATE projects SET project_type = CASE WHEN $2 THEN 'time_and_materials'::project_type ELSE 'non_billable'::project_type END WHERE id = $1", ids.project_id, project_billable).execute(&pool).await.unwrap();
                        sqlx::query!("UPDATE project_tasks SET billable = $3 WHERE project_id = $1 AND task_id = $2", ids.project_id, ids.task_id, task_billable).execute(&pool).await.unwrap();
                        let context = fetch_time_entry_contexts(&pool, ids.user_id).await.unwrap();
                        assert_eq!(context[0].billable, project_billable && task_billable);
                        let entry = insert_time_entry(
                            &pool,
                            ids.user_id,
                            NewTimeEntry {
                                billable: requested_billable,
                                is_running,
                                ..manual_entry(&ids)
                            },
                        )
                        .await
                        .unwrap();
                        assert_eq!(
                            entry.billable,
                            requested_billable && project_billable && task_billable
                        );
                        assert_eq!(entry.is_running, is_running);
                        assert_eq!(entry.started_at.is_some(), is_running);
                    }
                }
            }
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn concurrent_timer_starts_insert_exactly_one_entry(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Admin).await;
        let (first, second) = tokio::join!(
            insert_time_entry(
                &pool,
                ids.user_id,
                NewTimeEntry {
                    is_running: true,
                    ..manual_entry(&ids)
                }
            ),
            insert_time_entry(
                &pool,
                ids.user_id,
                NewTimeEntry {
                    is_running: true,
                    ..manual_entry(&ids)
                }
            ),
        );
        assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM time_entries")
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(1)
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn inactive_project_cannot_accept_new_time_even_for_admin(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Admin).await;
        sqlx::query!(
            "UPDATE projects SET active = false WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_err()
        );
    }

    #[test]
    fn snaps_unaligned_start_to_the_grid() {
        // A drag-move can hand in a sub-grid start (e.g. 4:04); every write path
        // must snap it (FR-008). 244 → 240 (4:00), duration unchanged.
        let (minutes, start) = normalize_start(210, Some(244)).unwrap();
        assert_eq!(start, Some(240));
        assert_eq!(minutes, 210);
    }

    #[test]
    fn clamps_duration_against_the_snapped_start() {
        // Snap first (1430 → 1425), then clamp so start + minutes never crosses
        // midnight (FR-012): 1425 + 60 would exceed the day, clamped to 15.
        let (minutes, start) = normalize_start(60, Some(1430)).unwrap();
        assert_eq!(start, Some(1425));
        assert_eq!(minutes, 15);
    }

    #[test]
    fn untimed_start_is_left_untouched() {
        let (minutes, start) = normalize_start(90, None).unwrap();
        assert_eq!(start, None);
        assert_eq!(minutes, 90);
    }

    #[test]
    fn rejects_out_of_range_start() {
        assert!(normalize_start(60, Some(-1)).is_err());
        assert!(normalize_start(60, Some(1440)).is_err());
    }

    // New time is checked through the actual insert and the shared selector view.

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn unassigned_member_is_forbidden(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Member).await;

        let err = insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
            .await
            .expect_err("an unassigned member must be refused");
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, CONFLICT),
            other => panic!("expected a 409 ServerError, got {other:?}"),
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn assigned_member_may_log_time(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Member).await;
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            ids.project_id,
            ids.user_id,
        )
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn admin_may_log_time_without_assignment(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Admin).await;

        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn submission_barrier_blocks_manual_entries_and_timer_starts(pool: PgPool) {
        for is_running in [false, true] {
            let ids = linked_seed(&pool, OrgRole::Admin).await;
            let mut submission = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *submission)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                r#"SELECT pg_advisory_xact_lock(hashtextextended('horae.timesheet:' || $1::uuid::text, 0)) as "lock!: ()""#,
                ids.user_id,
            ).execute(&mut *submission).await.unwrap();
            let run_pool = pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move {
                let mut input = manual_entry(&ids);
                input.is_running = is_running;
                insert_time_entry(&run_pool, ids.user_id, input).await
            });
            crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
            submission.commit().await.unwrap();
            let entry = tokio::time::timeout(std::time::Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .unwrap();
            assert_eq!(entry.is_running, is_running);
        }
    }
}
