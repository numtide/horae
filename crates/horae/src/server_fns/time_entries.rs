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

/// Ensure the user may log time on the project: admins may log anywhere,
/// everyone else needs an assignment row. Shared by the manual-entry and
/// timer-start paths so both enforce the same rule.
#[cfg(feature = "server")]
async fn ensure_assigned(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    project_id: uuid::Uuid,
    org_role: OrgRole,
) -> Result<(), ServerFnError> {
    if org_role == OrgRole::Admin {
        return Ok(());
    }

    let assigned = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM assignments WHERE project_id = $1 AND user_id = $2)",
        project_id,
        user_id,
    )
    .fetch_one(db)
    .await
    .map_err(server_err)?
    .unwrap_or(false);

    if assigned {
        Ok(())
    } else {
        Err(forbidden("You are not assigned to this project"))
    }
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

    ensure_assigned(&state.db, user.id, project_id, user.org_role).await?;

    let id = uuid::Uuid::now_v7();
    let today = chrono::Utc::now().date_naive();

    // The `one_running_timer_per_user` partial unique index is the real guard,
    // so let it decide: a separate SELECT EXISTS pre-check leaves a window in
    // which two concurrent starts both pass and the loser gets a raw
    // unique-violation 500 instead of the conflict below.
    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, started_at, state)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, true, true, now(), $8)
         ON CONFLICT (user_id) WHERE is_running DO NOTHING
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        id,
        user.org_id,
        user.id,
        project_id,
        task_id,
        today as chrono::NaiveDate,
        notes.as_deref(),
        EntryState::Open as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("A timer is already running. Stop it first."))?;

    dispatch_time_entry_event(&entry, TimeEntryEvent::Created).await;
    Ok(entry)
}

/// Stop a running timer and record elapsed minutes.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;

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
    .fetch_optional(&state.db)
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
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("No running timer found for this entry"))?;

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

    ensure_assigned(&state.db, user.id, project_id, user.org_role).await?;

    let id = uuid::Uuid::now_v7();

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, state, start_minute)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, $10, $11)
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, start_minute, sort_order, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        id,
        user.org_id,
        user.id,
        project_id,
        task_id,
        spent_date as chrono::NaiveDate,
        minutes,
        notes.as_deref(),
        billable,
        EntryState::Open as EntryState,
        start_minute,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

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
    let (minutes, start_minute) = normalize_start(minutes, start_minute)?;

    // Read current values first so a no-op update emits no event (FR-012).
    let before = sqlx::query!(
        r#"SELECT minutes, start_minute, notes, billable FROM time_entries
           WHERE id = $1 AND user_id = $2 AND state = $3"#,
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET minutes = $3, notes = $4, billable = $5, start_minute = $7
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
        minutes,
        notes.as_deref(),
        billable,
        EntryState::Open as EntryState,
        start_minute,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    let changed = before.is_none_or(|b| {
        b.minutes != minutes
            || b.notes.as_deref() != notes.as_deref()
            || b.billable != billable
            || b.start_minute != start_minute
    });
    if changed {
        dispatch_time_entry_event(&entry, TimeEntryEvent::Updated).await;
    }

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
}

/// Delete a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let user_id = require_user().await?.id;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;

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
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

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
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    dispatch_time_entry_event(&entry, TimeEntryEvent::Updated).await;

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
}

/// Place a set of untimed entries on `spent_date` in the given top-to-bottom
/// order. Each id gets its position as `sort_order` and its `spent_date` set to
/// the target day — so this both reorders a day's stack and moves an untimed
/// entry to another day. Only untimed entries are touched; hours and state are
/// left alone, so it works regardless of whether they're locked.
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
    let orders: Vec<i32> = (0..ids.len() as i32).collect();

    sqlx::query!(
        r#"UPDATE time_entries AS t
             SET sort_order = v.ord, spent_date = $4
           FROM unnest($1::uuid[], $2::int4[]) AS v(id, ord)
           WHERE t.id = v.id AND t.user_id = $3 AND t.start_minute IS NULL"#,
        &ids,
        &orders,
        user_id,
        spent_date as chrono::NaiveDate,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::{FORBIDDEN, OrgRole, ensure_assigned, listing_is_bounded, normalize_start};
    use crate::server_fns::test_seed::seed;
    use dioxus::prelude::ServerFnError;
    use sqlx::PgPool;
    use uuid::Uuid;

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

    // ── ensure_assigned (`#[sqlx::test]`, throwaway database per test) ──────
    // These call the crate-internal guard directly — `tests/` cannot import a
    // bin crate's modules, so the real behaviour is pinned here.

    #[sqlx::test(migrations = "./migrations")]
    async fn unassigned_member_is_forbidden(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;

        let err = ensure_assigned(&pool, ids.user_id, ids.project_id, OrgRole::Member)
            .await
            .expect_err("an unassigned member must be refused");
        match err {
            ServerFnError::ServerError { code, .. } => assert_eq!(code, FORBIDDEN),
            other => panic!("expected a 403 ServerError, got {other:?}"),
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn assigned_member_may_log_time(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Member).await;
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
            ensure_assigned(&pool, ids.user_id, ids.project_id, OrgRole::Member)
                .await
                .is_ok()
        );
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn admin_may_log_time_without_assignment(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Admin).await;

        assert!(
            ensure_assigned(&pool, ids.user_id, ids.project_id, OrgRole::Admin)
                .await
                .is_ok()
        );
    }
}
