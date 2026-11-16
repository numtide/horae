//! Time-entry server functions.

use super::*;
#[cfg(feature = "server")]
use crate::notifications::budgets::check_project_budget;
#[cfg(feature = "server")]
use crate::ops;

/// Eligible, writable tracking choices for one explicitly selected person.
#[server]
pub async fn load_timesheet_tracking(
    context: crate::models::scoped_time::TimesheetWriteContext,
) -> Result<Vec<crate::models::scoped_time::TimesheetTrackingOption>, ServerFnError> {
    let caller = session_caller_or("Timesheet is unavailable").await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::time::tracking(&mut read.tx, &read.actor, &context).await;
    Ok(read.finish(result).await?)
}

/// Apply one person-bound operation; client expectations never grant authority.
/// Returns a notice for the person when the operation was adjusted, e.g. a
/// stopped timer clamped to the daily limit.
#[server]
pub async fn apply_timesheet_command(
    context: crate::models::scoped_time::TimesheetWriteContext,
    command: crate::models::scoped_time::TimesheetCommand,
) -> Result<Option<String>, ServerFnError> {
    let caller = session_caller_or("Timesheet is unavailable").await?;
    let state = crate::state::global_state().await;
    let mut write = ops::begin(&state.db, caller).await?;
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    let change = write.commit(result, &*state.plugins).await?;
    for project in change.projects {
        tokio::spawn(check_project_budget(state, project));
    }
    Ok(change.notice)
}

/// Resolve a Timesheet subject and its rows without a separate identity bootstrap.
#[server]
pub async fn load_timesheet_page(
    query: crate::models::scoped_time::TimesheetQuery,
) -> Result<crate::models::scoped_time::TimesheetPage, ServerFnError> {
    let caller = session_caller_or("Timesheet is unavailable").await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::time_reads::sheet(&mut read.tx, &read.actor, &query).await;
    Ok(read.finish(result).await?)
}

/// Minimal authorized person labels; selection never grants entry access.
#[server]
pub async fn list_timesheet_people(
    query: crate::models::scoped_time::TimesheetPeopleQuery,
) -> Result<crate::models::scoped_time::TimesheetPeoplePage, ServerFnError> {
    let caller = session_caller_or("Timesheet people are unavailable").await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::time_reads::people(&mut read.tx, &read.actor, &query).await;
    Ok(read.finish(result).await?)
}

/// Only the session user's active, assigned project/task combinations; no rates.
#[server]
pub async fn list_time_entry_contexts()
-> Result<Vec<crate::models::time_entry::TimeEntryContext>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::time::contexts(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// Start a timer for the given project and task; a second running timer is a
/// conflict.
#[server]
pub async fn start_timer(
    project_id: String,
    task_id: String,
    notes: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let mut write = ops::begin(&state.db, caller).await?;
    let result = ops::time::start_timer(
        &mut write.tx,
        &write.actor,
        project_id,
        task_id,
        notes.as_deref(),
    )
    .await;
    Ok(write.commit(result, &*state.plugins).await?)
}

/// Stop a running timer and record elapsed minutes, clamped to what is left of
/// the day; the notice, when present, says how much the daily limit cut.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<(TimeEntry, Option<String>), ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;
    let mut write = ops::begin(&state.db, caller).await?;
    let result = ops::time::stop_timer(&mut write.tx, &write.actor, entry_id).await;
    let (entry, notice) = write.commit(result, &*state.plugins).await?;
    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok((entry, notice))
}

/// Return the currently running timer for the authenticated user, if any.
#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await?;
    let result = ops::time::current_timer(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}
