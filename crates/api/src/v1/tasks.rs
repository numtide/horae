//! Tasks, read-only: the catalogue for task readers, otherwise the tasks the
//! token's owner may track.

use axum::Json;
use axum::extract::{Extension, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::types::{Page, Task};
use super::{ProblemResponse, V1State, catalogue};
use crate::cursor::Walk;
use crate::ops::projects::task_catalog::{self, TaskQuery};
use crate::ops::{self, Caller};
use crate::problem::{ApiPath, ApiQuery, Problem};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TasksQuery {
    /// Tasks per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
    /// Only this project's tasks.
    project_id: Option<Uuid>,
    /// Active tasks, or with `false` archived ones.
    #[param(default = true)]
    active: Option<bool>,
    /// Only tasks changed at or after this instant.
    updated_since: Option<DateTime<Utc>>,
}

/// The tasks the owner may see, in `id` order.
#[utoipa::path(
    get,
    path = "/tasks",
    operation_id = "listTasks",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(TasksQuery),
    responses(
        (status = 200, description = "A page of tasks", body = Page<Task>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<TasksQuery>,
) -> Result<Json<Page<Task>>, Problem> {
    let active = query.active.unwrap_or(true);
    let updated_since = query.updated_since;
    let filters = format!(
        "project_id={:?};active={active};updated_since={:?}",
        query.project_id,
        updated_since.map(|instant| instant.timestamp_micros()),
    );
    let walk = Walk {
        key: &state.cursor_key,
        list: "tasks",
        filters: &filters,
    };
    let position = walk.position(query.limit, query.cursor.as_deref(), Utc::now())?;
    let page = TaskQuery {
        project_id: query.project_id,
        active,
        updated_since,
        after: position.after,
        limit: position.limit,
    };
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = task_catalog::visible_page(&mut read.tx, &read.actor, &page).await;
    let rows = read.finish(result).await?;
    Ok(catalogue::page(&walk, rows, position, |task| task.id))
}

/// One task the owner may see, active or archived.
#[utoipa::path(
    get,
    path = "/tasks/{id}",
    operation_id = "getTask",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(("id" = Uuid, Path, description = "The task's id")),
    responses(
        (status = 200, description = "The task", body = Task),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Task>, Problem> {
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = task_catalog::visible_get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.into()))
}
