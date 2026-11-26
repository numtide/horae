//! Projects, read-only: the ones the Projects page shows the token's owner.

use axum::Json;
use axum::extract::{Extension, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::types::{Page, Project};
use super::{ProblemResponse, V1State, catalogue};
use crate::cursor::Walk;
use crate::ops::projects::reads::{self, ProjectQuery};
use crate::ops::{self, Caller};
use crate::problem::{ApiPath, ApiQuery, Problem};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ProjectsQuery {
    /// Projects per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
    /// Only this client's projects.
    client_id: Option<Uuid>,
    /// Active projects, or with `false` archived ones.
    #[param(default = true)]
    active: Option<bool>,
    /// Only projects changed at or after this instant.
    updated_since: Option<DateTime<Utc>>,
}

/// The projects the owner may see, in `id` order.
#[utoipa::path(
    get,
    path = "/projects",
    operation_id = "listProjects",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(ProjectsQuery),
    responses(
        (status = 200, description = "A page of projects", body = Page<Project>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<ProjectsQuery>,
) -> Result<Json<Page<Project>>, Problem> {
    let active = query.active.unwrap_or(true);
    let updated_since = query.updated_since;
    let filters = format!(
        "client_id={:?};active={active};updated_since={:?}",
        query.client_id,
        updated_since.map(|instant| instant.timestamp_micros()),
    );
    let walk = Walk {
        key: &state.cursor_key,
        list: "projects",
        filters: &filters,
    };
    let position = walk.position(query.limit, query.cursor.as_deref(), Utc::now())?;
    let page = ProjectQuery {
        client_id: query.client_id,
        active,
        updated_since,
        after: position.after,
        limit: position.limit,
    };
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = reads::page(&mut read.tx, &read.actor, &page).await;
    let rows = read.finish(result).await?;
    catalogue::page(&walk, rows, position, |project| project.id)
}

/// One project the owner may see, active or archived.
#[utoipa::path(
    get,
    path = "/projects/{id}",
    operation_id = "getProject",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(("id" = Uuid, Path, description = "The project's id")),
    responses(
        (status = 200, description = "The project", body = Project),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Project>, Problem> {
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = reads::get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.try_into()?))
}
