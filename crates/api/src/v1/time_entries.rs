//! Time entries: listed and read with the Timesheet's scope, and written
//! through the Timesheet's own operation (`ops::time::apply` and `patch`), so
//! every rule, lock, refusal and event is the web app's.

use axum::Extension;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use uuid::Uuid;

use super::types::{NewTimeEntry, Page, TimeEntry, TimeEntryPatch};
use super::{ProblemResponse, V1State};
use crate::cursor::Walk;
use crate::models::scoped_time::{TimesheetCommand, TimesheetEntryInput, TimesheetWriteContext};
use crate::ops::time::TimesheetChange;
use crate::ops::time_reads::EntryFilter;
use crate::ops::{self, Actor, Caller, Done, OpError, Tx, WriteTx};
use crate::problem::{ApiJson, ApiQuery, Problem};

/// A page of the list and its filters. A cursor is bound to the filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TimeEntryQuery {
    /// Entries per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// From the previous page's `next_cursor`.
    cursor: Option<String>,
    person_id: Option<Uuid>,
    project_id: Option<Uuid>,
    task_id: Option<Uuid>,
    client_id: Option<Uuid>,
    /// First spent date, inclusive.
    from: Option<NaiveDate>,
    /// Last spent date, inclusive.
    to: Option<NaiveDate>,
    is_running: Option<bool>,
    /// Entries changed at or after this instant.
    updated_since: Option<DateTime<Utc>>,
}

impl TimeEntryQuery {
    fn filter(&self) -> EntryFilter {
        EntryFilter {
            person_id: self.person_id,
            project_id: self.project_id,
            task_id: self.task_id,
            client_id: self.client_id,
            from: self.from,
            to: self.to,
            is_running: self.is_running,
            updated_since: self.updated_since,
        }
    }
}

/// Every entry the owner may read, own, managed or all by their scope.
#[utoipa::path(
    get,
    path = "/time_entries",
    operation_id = "listTimeEntries",
    security(("token" = []), ("oauth2" = ["time:read"])),
    tag = "time entries",
    params(TimeEntryQuery),
    responses(
        (status = 200, description = "A page of time entries", body = Page<TimeEntry>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<TimeEntryQuery>,
) -> Result<Json<Page<TimeEntry>>, Problem> {
    // The same filters give the same text, whatever the page.
    let filters = serde_json::to_string(&TimeEntryQuery {
        limit: None,
        cursor: None,
        ..query.clone()
    })
    .map_err(|_| Problem::internal())?;
    let walk = Walk {
        key: &state.cursor_key,
        list: "time_entries",
        filters: &filters,
    };
    let now = Utc::now();
    let position = walk.position(query.limit, query.cursor.as_deref(), now)?;
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = ops::time_reads::list(
        &mut read.tx,
        &read.actor,
        &query.filter(),
        position.after,
        i64::from(position.limit) + 1,
    )
    .await;
    let mut entries = read.finish(result).await?;
    let next_cursor = walk.next_cursor(&mut entries, position, |entry| entry.id, now);
    Ok(Json(Page {
        data: entries.into_iter().map(TimeEntry::from).collect(),
        next_cursor,
    }))
}

/// One entry the owner may read.
#[utoipa::path(
    get,
    path = "/time_entries/{id}",
    operation_id = "getTimeEntry",
    security(("token" = []), ("oauth2" = ["time:read"])),
    tag = "time entries",
    params(("id" = Uuid, Path, description = "The entry")),
    responses(
        (status = 200, description = "The entry", body = TimeEntry),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<Uuid>,
) -> Result<Json<TimeEntry>, Problem> {
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = ops::time_reads::get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.into()))
}

/// Records time with a duration, as the Timesheet's create does.
#[utoipa::path(
    post,
    path = "/time_entries",
    operation_id = "createTimeEntry",
    security(("token" = []), ("oauth2" = ["time:write"])),
    tag = "time entries",
    request_body = NewTimeEntry,
    responses(
        (status = 201, description = "Created", body = TimeEntry,
            headers(("Location" = String, description = "The new entry"))),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn create(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiJson(new): ApiJson<NewTimeEntry>,
) -> Result<Response, Problem> {
    let mut write = ops::begin(&state.pool, caller).await?;
    let context = for_person(&write.actor, new.person_id);
    let command = TimesheetCommand::Create {
        entry: TimesheetEntryInput {
            project_id: new.project_id,
            task_id: new.task_id,
            spent_date: new.spent_date,
            minutes: new.minutes,
            notes: new.notes,
            billable: new.billable,
            start_minute: new.start_minute,
        },
    };
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    let entry = commit(&state, write, result).await?;
    Ok(created(entry))
}

/// Changes the given fields of an entry; the rest keep their values.
#[utoipa::path(
    patch,
    path = "/time_entries/{id}",
    operation_id = "updateTimeEntry",
    security(("token" = []), ("oauth2" = ["time:write"])),
    tag = "time entries",
    params(("id" = Uuid, Path, description = "The entry")),
    request_body = TimeEntryPatch,
    responses(
        (status = 200, description = "Updated", body = TimeEntry),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn update(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<Uuid>,
    ApiJson(patch): ApiJson<TimeEntryPatch>,
) -> Result<Json<TimeEntry>, Problem> {
    let mut write = ops::begin(&state.pool, caller).await?;
    let result = match of_entry(&mut write.tx, &write.actor, id).await {
        Ok(context) => {
            ops::time::patch(&mut write.tx, &write.actor, &context, id, patch.into()).await
        }
        Err(error) => Err(error),
    };
    Ok(Json(commit(&state, write, result).await?))
}

/// Deletes an entry.
#[utoipa::path(
    delete,
    path = "/time_entries/{id}",
    operation_id = "deleteTimeEntry",
    security(("token" = []), ("oauth2" = ["time:write"])),
    tag = "time entries",
    params(("id" = Uuid, Path, description = "The entry")),
    responses(
        (status = 204, description = "Deleted"),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn delete(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, Problem> {
    let command = TimesheetCommand::Delete {
        entry_ids: vec![id],
    };
    let mut write = ops::begin(&state.pool, caller).await?;
    let result = on_entry(&mut write.tx, &write.actor, id, command).await;
    let change = write.commit(result, &*state.events).await?;
    state.time_changed(change.projects);
    Ok(StatusCode::NO_CONTENT)
}

/// Stops the entry's running timer.
#[utoipa::path(
    post,
    path = "/time_entries/{id}/stop",
    operation_id = "stopTimer",
    security(("token" = []), ("oauth2" = ["time:write"])),
    tag = "timers",
    params(("id" = Uuid, Path, description = "The entry")),
    responses(
        (status = 200, description = "Stopped", body = TimeEntry),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn stop(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<Uuid>,
) -> Result<Json<TimeEntry>, Problem> {
    let command = TimesheetCommand::StopTimer { entry_id: id };
    let mut write = ops::begin(&state.pool, caller).await?;
    let result = on_entry(&mut write.tx, &write.actor, id, command).await;
    Ok(Json(commit(&state, write, result).await?))
}

/// Runs a stopped entry of today again, keeping its minutes; a running timer
/// is stopped first. Another day's entry cannot restart (409).
#[utoipa::path(
    post,
    path = "/time_entries/{id}/restart",
    operation_id = "restartTimer",
    security(("token" = []), ("oauth2" = ["time:write"])),
    tag = "timers",
    params(("id" = Uuid, Path, description = "The entry")),
    responses(
        (status = 200, description = "Running again", body = TimeEntry),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn restart(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<Uuid>,
) -> Result<Json<TimeEntry>, Problem> {
    let command = TimesheetCommand::RestartTimer { entry_id: id };
    let mut write = ops::begin(&state.pool, caller).await?;
    let result = on_entry(&mut write.tx, &write.actor, id, command).await;
    Ok(Json(commit(&state, write, result).await?))
}

/// A write for `person`, the caller when absent. No page stands between the
/// token and the write, so the requester is the actor itself.
pub(super) fn for_person(actor: &Actor, person: Option<Uuid>) -> TimesheetWriteContext {
    TimesheetWriteContext {
        expected_requester: actor.requester(),
        subject_id: person.unwrap_or(actor.id()),
    }
}

/// A write for the entry's person. An entry the actor may not read is not
/// found, as in the web app.
async fn of_entry(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<TimesheetWriteContext, OpError> {
    let entry = ops::time_reads::get(tx, actor, id).await?;
    Ok(for_person(actor, Some(entry.user_id)))
}

async fn on_entry(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    command: TimesheetCommand,
) -> Result<Done<TimesheetChange>, OpError> {
    let context = of_entry(tx, actor, id).await?;
    ops::time::apply(tx, actor, &context, command).await
}

/// Commits a Timesheet write and answers the entry it saved.
pub(super) async fn commit(
    state: &V1State,
    write: WriteTx,
    result: Result<Done<TimesheetChange>, OpError>,
) -> Result<TimeEntry, Problem> {
    let change = write.commit(result, &*state.events).await?;
    state.time_changed(change.projects);
    change.entry.map(TimeEntry::from).ok_or_else(|| {
        tracing::error!("A Timesheet write answered no entry");
        Problem::internal()
    })
}

pub(super) fn created(entry: TimeEntry) -> Response {
    let location = format!("/api/v1/time_entries/{}", entry.id);
    (
        StatusCode::CREATED,
        [(header::LOCATION, location)],
        Json(entry),
    )
        .into_response()
}

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod tests;
