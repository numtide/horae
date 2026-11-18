//! People, read-only: the ones the web app's people pickers offer the
//! token's owner.

use axum::Json;
use axum::extract::{Extension, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::types::{Page, Person};
use super::{ProblemResponse, V1State, catalogue};
use crate::cursor::Walk;
use crate::ops::people::{self, PersonQuery};
use crate::ops::{self, Caller};
use crate::problem::{ApiPath, ApiQuery, Problem};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PeopleQuery {
    /// People per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
    /// Active people, or with `false` archived ones (which need permission
    /// to view everyone).
    #[param(default = true)]
    active: Option<bool>,
    /// Only people changed at or after this instant.
    updated_since: Option<DateTime<Utc>>,
}

/// The people the owner may see, in `id` order.
#[utoipa::path(
    get,
    path = "/people",
    operation_id = "listPeople",
    tag = "catalogue",
    params(PeopleQuery),
    responses(
        (status = 200, description = "A page of people", body = Page<Person>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<PeopleQuery>,
) -> Result<Json<Page<Person>>, Problem> {
    let active = query.active.unwrap_or(true);
    let updated_since = query.updated_since;
    let filters = format!(
        "active={active};updated_since={:?}",
        updated_since.map(|instant| instant.timestamp_micros()),
    );
    let walk = Walk {
        key: &state.cursor_key,
        list: "people",
        filters: &filters,
    };
    let position = walk.position(query.limit, query.cursor.as_deref(), Utc::now())?;
    let page = PersonQuery {
        active,
        updated_since,
        after: position.after,
        limit: position.limit,
    };
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = people::page(&mut read.tx, &read.actor, &page).await;
    let rows = read.finish(result).await?;
    Ok(catalogue::page(&walk, rows, position, |person| person.id))
}

/// One person the owner may see. An archived person needs permission to
/// view everyone.
#[utoipa::path(
    get,
    path = "/people/{id}",
    operation_id = "getPerson",
    tag = "catalogue",
    params(("id" = Uuid, Path, description = "The person's id")),
    responses(
        (status = 200, description = "The person", body = Person),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Person>, Problem> {
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = people::get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.into()))
}
