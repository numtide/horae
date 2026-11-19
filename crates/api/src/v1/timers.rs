//! Starting a timer, through the Timesheet's start.

use axum::Extension;
use axum::extract::State;
use axum::response::Response;

use super::time_entries::{commit, created, for_person};
use super::types::{NewTimer, TimeEntry};
use super::{ProblemResponse, V1State};
use crate::models::scoped_time::TimesheetCommand;
use crate::ops::{self, Caller};
use crate::problem::{ApiJson, Problem};

/// Starts a timer on the person's today. A timer already running for them is
/// stopped first, as in the web app.
#[utoipa::path(
    post,
    path = "/timers",
    operation_id = "startTimer",
    tag = "timers",
    request_body = NewTimer,
    responses(
        (status = 201, description = "Started", body = TimeEntry,
            headers(("Location" = String, description = "The new entry"))),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn start(
    State(state): State<V1State>,
    axum::Extension(caller): Extension<Caller>,
    ApiJson(timer): ApiJson<NewTimer>,
) -> Result<Response, Problem> {
    let mut write = ops::begin(&state.pool, caller).await?;
    let context = for_person(&write.actor, timer.person_id);
    let command = TimesheetCommand::StartTimer {
        project_id: timer.project_id,
        task_id: timer.task_id,
        notes: timer.notes,
    };
    let result = ops::time::apply(&mut write.tx, &write.actor, &context, command).await;
    Ok(created(commit(&state, write, result).await?))
}
