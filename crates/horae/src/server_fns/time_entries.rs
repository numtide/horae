//! Time-entry server functions.

use super::*;

/// Validate an optional start time (minutes since midnight, 0..=1439) and clamp
/// the duration so the entry never crosses midnight (Constitution: exactness).
/// Returns the possibly-clamped minutes and the validated start.
#[cfg(feature = "server")]
fn normalize_start(
    minutes: i32,
    start_minute: Option<i32>,
) -> Result<(i32, Option<i32>), ServerFnError> {
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

/// Eligibility and effective billability are resolved in the insert's snapshot,
/// not in a pre-check separated from the write. The timer uniqueness index
/// arbitrates competing starts without a racy existence check.
#[cfg(feature = "server")]
async fn insert_time_entry(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    input: NewTimeEntry<'_>,
) -> Result<TimeEntry, ServerFnError> {
    let mut tx = crate::db::begin_time_entry_write(db, user_id)
        .await
        .map_err(server_err)?;
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
    tx.commit().await.map_err(server_err)?;
    Ok(entry)
}

// ── Time Entries ─────────────────────────────────────────────────────────────

/// Whether this listing is bounded: either it caps the rows, or it closes the
/// date range on both ends. An unlimited, open-ended listing would walk the
/// user's whole history.
#[cfg(feature = "server")]
fn listing_is_bounded(limit: Option<i64>, date_from: Option<&str>, date_to: Option<&str>) -> bool {
    limit.is_some() || (date_from.is_some() && date_to.is_some())
}

/// The session user's entries, newest first.
///
/// `limit` of `None` returns every match, which callers that aggregate — the
/// timesheet sums its own rows — need for their totals to be right. It is only
/// accepted alongside a closed date range; see [`listing_is_bounded`].
#[server]
pub async fn list_time_entries(
    _user_id: Option<String>,
    project_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TimeEntry>, ServerFnError> {
    if !listing_is_bounded(limit, date_from.as_deref(), date_to.as_deref()) {
        return Err(server_err(
            "an unlimited listing needs both date_from and date_to",
        ));
    }

    let session_uid = require_user().await?.id;
    let state = crate::state::global_state().await;

    let project_filter = parse_opt_uuid(project_id, "project_id")?;
    let date_filter = date_from
        .as_deref()
        .map(|s| parse_date(s, "date_from"))
        .transpose()?;
    let date_to_filter = date_to
        .as_deref()
        .map(|s| parse_date(s, "date_to"))
        .transpose()?;

    let entries = sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
         FROM time_entries
         WHERE user_id = $1
           AND ($2::uuid IS NULL OR project_id = $2)
           AND ($3::date IS NULL OR spent_date >= $3)
           AND ($4::date IS NULL OR spent_date <= $4)
         ORDER BY spent_date DESC, created_at DESC
         LIMIT $5::bigint"#,
        session_uid,
        project_filter,
        date_filter as Option<chrono::NaiveDate>,
        date_to_filter as Option<chrono::NaiveDate>,
        limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entries)
}

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

    let entry = insert_time_entry(
        &state.db,
        user.id,
        NewTimeEntry {
            project_id,
            task_id,
            spent_date: chrono::Utc::now().date_naive(),
            minutes: 0,
            notes: notes.as_deref(),
            billable: true,
            start_minute: None,
            is_running: true,
        },
    )
    .await?;

    dispatch_time_entry_event(&entry, TimeEntryEvent::Created).await;
    Ok(entry)
}

/// Stop a running timer and record elapsed minutes.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let mut tx = crate::db::begin_time_entry_write(&state.db, user_id)
        .await
        .map_err(server_err)?;

    // Read the running entry's start time, then compute the exact elapsed
    // minutes in `horae-core` (floored to the minute, no artificial 1-minute
    // minimum) so tracked totals stay exact (FR-003/FR-023).
    let row = sqlx::query!(
        r#"SELECT started_at as "started_at: chrono::DateTime<chrono::Utc>",
                  state as "state: EntryState"
               FROM time_entries
               WHERE id = $1 AND user_id = $2 AND is_running = true"#,
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

    let minutes = horae_core::duration::minutes_between(started_at, chrono::Utc::now()) as i32;

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
    dispatch_time_entry_event(&entry, TimeEntryEvent::Stopped).await;
    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
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

/// Create a manual (non-timer) time entry.
#[server]
pub async fn create_time_entry(
    project_id: String,
    task_id: String,
    spent_date: String,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,
) -> Result<TimeEntry, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let spent_date = parse_date(&spent_date, "date")?;
    let (minutes, start_minute) = normalize_start(minutes, start_minute)?;

    let entry = insert_time_entry(
        &state.db,
        user.id,
        NewTimeEntry {
            project_id,
            task_id,
            spent_date,
            minutes,
            notes: notes.as_deref(),
            billable,
            start_minute,
            is_running: false,
        },
    )
    .await?;

    dispatch_time_entry_event(&entry, TimeEntryEvent::Created).await;
    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
}

/// Update a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn update_time_entry(
    entry_id: String,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,
) -> Result<TimeEntry, ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let (entry, changed) = update_entry(
        &state.db,
        user_id,
        entry_id,
        minutes,
        notes.as_deref(),
        billable,
        start_minute,
    )
    .await?;
    if changed {
        dispatch_time_entry_event(&entry, TimeEntryEvent::Updated).await;
        tokio::spawn(check_project_budget(state, entry.project_id));
    }

    Ok(entry)
}

#[cfg(feature = "server")]
async fn update_entry(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    entry_id: uuid::Uuid,
    minutes: i32,
    notes: Option<&str>,
    billable: bool,
    start_minute: Option<i32>,
) -> Result<(TimeEntry, bool), ServerFnError> {
    let (minutes, start_minute) = normalize_start(minutes, start_minute)?;
    let mut tx = crate::db::begin_time_entry_write(db, user_id)
        .await
        .map_err(server_err)?;
    // Lock before comparing so a competing edit cannot turn a stale no-op
    // into an unreported change, or cause duplicate update events (FR-012).
    let before = sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                  spent_date as "spent_date: chrono::NaiveDate",
                  minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                  started_at as "started_at: chrono::DateTime<chrono::Utc>",
                  state as "state: EntryState", invoice_id,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>",
                  updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
           FROM time_entries
           WHERE id = $1 AND user_id = $2 AND state = $3
           FOR UPDATE"#,
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"WITH effective AS (
           SELECT e.id, $5 AND COALESCE((
               SELECT p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)
               FROM projects p JOIN tasks t ON t.id = e.task_id
               LEFT JOIN project_tasks pt ON pt.project_id = p.id AND pt.task_id = t.id
               WHERE p.id = e.project_id
             ), false) AS billable
           FROM time_entries e WHERE e.id = $1
         )
         UPDATE time_entries e
         SET minutes = $3, notes = $4, start_minute = $7, billable = effective.billable
         FROM effective
         WHERE e.id = effective.id AND e.id = $1 AND e.user_id = $2 AND e.state = $6
           AND (e.minutes, e.notes, e.start_minute, e.billable)
               IS DISTINCT FROM ($3, $4, $7, effective.billable)
         RETURNING e.id, e.org_id, e.user_id, e.project_id, e.task_id,
                   e.spent_date as "spent_date: chrono::NaiveDate",
                   e.minutes, e.start_minute, e.sort_order, e.rounded_minutes, e.notes, e.billable, e.is_running,
                   e.started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   e.state as "state: EntryState", e.invoice_id,
                   e.created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   e.updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        minutes,
        notes,
        billable,
        EntryState::Open as EntryState,
        start_minute,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let changed = entry.is_some();
    tx.commit().await.map_err(server_err)?;
    Ok((entry.unwrap_or(before), changed))
}
/// Delete a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let mut tx = crate::db::begin_time_entry_write(&state.db, user_id)
        .await
        .map_err(server_err)?;

    // Delete and capture the row in one statement so the "only open entries"
    // guard holds atomically (no TOCTOU) and the event carries the removed
    // entry's details.
    let entry = sqlx::query_as!(
        TimeEntry,
        r#"DELETE FROM time_entries
           WHERE id = $1 AND user_id = $2 AND state = $3
           RETURNING id, org_id, user_id, project_id, task_id,
                     spent_date as "spent_date: chrono::NaiveDate",
                     minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                     started_at as "started_at: chrono::DateTime<chrono::Utc>",
                     state as "state: EntryState", invoice_id,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>",
                     updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    tx.commit().await.map_err(server_err)?;
    dispatch_time_entry_event(&entry, TimeEntryEvent::Deleted).await;

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(())
}

/// Reschedule a timed entry from a calendar drag: move it (new date and/or start
/// minute) and/or resize it (new duration) in one authorized call. Only allowed
/// while the entry is 'open'.
#[server]
pub async fn reschedule_time_entry(
    entry_id: String,
    spent_date: String,
    start_minute: i32,
    minutes: i32,
) -> Result<TimeEntry, ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let spent_date = parse_date(&spent_date, "date")?;
    let (minutes, start_minute) = normalize_start(minutes, Some(start_minute))?;
    let mut tx = crate::db::begin_time_entry_write(&state.db, user_id)
        .await
        .map_err(server_err)?;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET spent_date = $3, start_minute = $4, minutes = $5
         WHERE id = $1 AND user_id = $2 AND state = $6
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        spent_date as chrono::NaiveDate,
        start_minute,
        minutes,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    tx.commit().await.map_err(server_err)?;
    dispatch_time_entry_event(&entry, TimeEntryEvent::Updated).await;

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
}

/// Place a set of untimed entries on `spent_date` in the given top-to-bottom
/// order. Each id gets its position as `sort_order` and its `spent_date` set to
/// the target day — so this both reorders a day's stack and moves an untimed
/// entry to another day. Locked or running entries may only be reordered within
/// their existing day. An invalid entry rejects the whole operation.
#[server]
pub async fn reorder_untimed_entries(
    spent_date: String,
    ordered_ids: Vec<String>,
) -> Result<(), ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let spent_date = parse_date(&spent_date, "date")?;
    let ids = ordered_ids
        .iter()
        .map(|s| parse_uuid(s, "entry_id"))
        .collect::<Result<Vec<_>, _>>()?;
    reorder_entries(&state.db, user_id, spent_date, &ids).await
}

#[cfg(feature = "server")]
async fn reorder_entries(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
    spent_date: chrono::NaiveDate,
    ids: &[uuid::Uuid],
) -> Result<(), ServerFnError> {
    let count = i32::try_from(ids.len()).map_err(|_| conflict("Too many entries to reorder"))?;
    let orders: Vec<i32> = (0..count).collect();
    let mut tx = crate::db::begin_time_entry_write(pool, user_id)
        .await
        .map_err(server_err)?;

    let updated = sqlx::query!(
        r#"UPDATE time_entries AS t
             SET sort_order = v.ord, spent_date = $4
           FROM unnest($1::uuid[], $2::int4[]) AS v(id, ord)
           WHERE t.id = v.id AND t.user_id = $3 AND t.start_minute IS NULL
             AND (t.spent_date = $4 OR (t.state = 'open' AND NOT t.is_running))"#,
        ids,
        &orders,
        user_id,
        spent_date as chrono::NaiveDate,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?
    .rows_affected();

    if updated != ids.len() as u64 {
        return Err(conflict(
            "Some entries cannot be moved or reordered. Refresh the timesheet and try again.",
        ));
    }
    tx.commit().await.map_err(server_err)?;
    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod update_tests;

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{
        CONFLICT, EntryState, NewTimeEntry, OrgRole, fetch_time_entry_contexts, insert_time_entry,
        listing_is_bounded, normalize_start, reorder_entries, update_entry,
    };
    use crate::server_fns::test_seed::{seed, time_entry};
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

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
    async fn edits_preserve_archived_time_but_cannot_override_non_billability(pool: PgPool) {
        for non_billable_project in [false, true] {
            let ids = linked_seed(&pool, OrgRole::Admin).await;
            let entry = insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .unwrap();
            sqlx::query!("UPDATE projects SET active = false, project_type = CASE WHEN $2 THEN 'non_billable'::project_type ELSE project_type END WHERE id = $1", ids.project_id, non_billable_project).execute(&pool).await.unwrap();
            sqlx::query!(
                "UPDATE project_tasks SET billable = $2 WHERE project_id = $1",
                ids.project_id,
                non_billable_project
            )
            .execute(&pool)
            .await
            .unwrap();
            let (updated, changed) = update_entry(
                &pool,
                ids.user_id,
                entry.id,
                90,
                Some("Kept"),
                true,
                Some(540),
            )
            .await
            .unwrap();
            assert!(changed);
            assert!(!updated.billable);
            assert_eq!(
                (
                    updated.minutes,
                    updated.start_minute,
                    updated.notes.as_deref()
                ),
                (90, Some(540), Some("Kept"))
            );
            let (_, changed) = update_entry(
                &pool,
                ids.user_id,
                entry.id,
                90,
                Some("Kept"),
                true,
                Some(540),
            )
            .await
            .unwrap();
            assert!(!changed, "effective no-op must not emit an update event");
            let (updated, _) =
                update_entry(&pool, ids.user_id, entry.id, 60, None, true, Some(1425))
                    .await
                    .unwrap();
            assert_eq!((updated.minutes, updated.start_minute), (15, Some(1425)));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
    async fn moving_locked_time_rejects_the_entire_reorder(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        for state in [
            EntryState::Submitted,
            EntryState::Approved,
            EntryState::Invoiced,
        ] {
            let open = time_entry(&pool, &ids, EntryState::Open).await;
            let locked = time_entry(&pool, &ids, state).await;
            let target = "2026-09-08".parse().unwrap();

            let result = reorder_entries(&pool, ids.user_id, target, &[open, locked]).await;

            assert!(result.is_err(), "must reject moving {state:?} time");
            let unchanged = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM time_entries WHERE id = ANY($1) AND spent_date = '2026-09-07' AND sort_order = 0",
                &[open, locked],
            ).fetch_one(&pool).await.unwrap();
            assert_eq!(unchanged, Some(2));
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn an_open_entry_can_move_into_a_day_with_locked_time(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let open = time_entry(&pool, &ids, EntryState::Open).await;
        let locked = time_entry(&pool, &ids, EntryState::Approved).await;
        sqlx::query!(
            "UPDATE time_entries SET spent_date = '2026-09-08' WHERE id = $1",
            open
        )
        .execute(&pool)
        .await
        .unwrap();

        reorder_entries(
            &pool,
            ids.user_id,
            "2026-09-07".parse().unwrap(),
            &[open, locked],
        )
        .await
        .unwrap();

        let entries = sqlx::query!(
            r#"SELECT id, state as "state: EntryState", sort_order FROM time_entries
               WHERE user_id = $1 AND spent_date = '2026-09-07' ORDER BY sort_order"#,
            ids.user_id,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|e| (e.id, e.state, e.sort_order))
                .collect::<Vec<_>>(),
            vec![
                (open, EntryState::Open, 0),
                (locked, EntryState::Approved, 1)
            ]
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_reorder_rejects_foreign_missing_timed_and_duplicate_ids(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let other = seed(&pool, OrgRole::Member).await;
        let open = time_entry(&pool, &ids, EntryState::Open).await;
        let foreign = time_entry(&pool, &other, EntryState::Open).await;
        let timed = time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE time_entries SET start_minute = 540 WHERE id = $1",
            timed
        )
        .execute(&pool)
        .await
        .unwrap();
        for invalid in [foreign, Uuid::now_v7(), timed, open] {
            let result = reorder_entries(
                &pool,
                ids.user_id,
                "2026-09-08".parse().unwrap(),
                &[open, invalid],
            )
            .await;
            assert!(result.is_err());
        }
        assert_eq!(
            sqlx::query_scalar!("SELECT sort_order FROM time_entries WHERE id = $1", open)
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_running_timer_cannot_be_moved_to_another_day(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
        let running = time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE time_entries SET is_running = true, started_at = now() WHERE id = $1",
            running
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            reorder_entries(
                &pool,
                ids.user_id,
                "2026-09-08".parse().unwrap(),
                &[running]
            )
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

    #[test]
    fn a_limit_bounds_a_listing_on_its_own() {
        assert!(listing_is_bounded(Some(50), None, None));
    }

    #[test]
    fn an_unlimited_listing_needs_both_ends_of_the_range() {
        assert!(listing_is_bounded(
            None,
            Some("2026-08-31"),
            Some("2026-09-06")
        ));
        assert!(!listing_is_bounded(None, Some("2026-08-31"), None));
        assert!(!listing_is_bounded(None, None, Some("2026-09-06")));
        assert!(!listing_is_bounded(None, None, None));
    }

    // New time is checked through the actual insert and the shared selector view.

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
    async fn admin_may_log_time_without_assignment(pool: PgPool) {
        let ids = linked_seed(&pool, OrgRole::Admin).await;

        assert!(
            insert_time_entry(&pool, ids.user_id, manual_entry(&ids))
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
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

    #[sqlx::test(migrations = "./migrations")]
    async fn submission_barrier_blocks_edits_and_cross_week_reorders(pool: PgPool) {
        for reorder in [false, true] {
            let ids = linked_seed(&pool, OrgRole::Admin).await;
            let entry = time_entry(&pool, &ids, EntryState::Open).await;
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
                if reorder {
                    reorder_entries(
                        &run_pool,
                        ids.user_id,
                        "2026-09-14".parse().unwrap(),
                        &[entry],
                    )
                    .await
                } else {
                    update_entry(&run_pool, ids.user_id, entry, 121, None, true, None)
                        .await
                        .map(|_| ())
                }
            });
            crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
            // The submission wins. A waiting mutation must recheck the locked
            // entry state after it acquires the barrier, not use an old read.
            sqlx::query!(
                "UPDATE time_entries SET state = 'submitted', rounded_minutes = 60 WHERE id = $1",
                entry
            )
            .execute(&mut *submission)
            .await
            .unwrap();
            submission.commit().await.unwrap();
            let result = tokio::time::timeout(std::time::Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError { code: CONFLICT, .. })
                ),
                "{result:?}"
            );
            let stored = sqlx::query!(
                r#"SELECT minutes, spent_date as "spent_date: chrono::NaiveDate" FROM time_entries WHERE id = $1"#,
                entry
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(
                (stored.minutes, stored.spent_date),
                (60, "2026-09-07".parse().unwrap())
            );
        }
    }
}
