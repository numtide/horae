//! Clients, read-only: the ones the web app's client pickers offer the
//! token's owner.

use axum::Json;
use axum::extract::{Extension, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use super::types::{Client, Page};
use super::{ProblemResponse, V1State, catalogue};
use crate::cursor::Walk;
use crate::ops::clients::{self, ClientQuery};
use crate::ops::{self, Caller};
use crate::problem::{ApiPath, ApiQuery, Problem};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ClientsQuery {
    /// Clients per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
    /// Active clients, or with `false` archived ones.
    #[param(default = true)]
    active: Option<bool>,
    /// Only clients changed at or after this instant.
    updated_since: Option<DateTime<Utc>>,
}

/// The clients the owner may see, in `id` order.
#[utoipa::path(
    get,
    path = "/clients",
    operation_id = "listClients",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(ClientsQuery),
    responses(
        (status = 200, description = "A page of clients", body = Page<Client>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<ClientsQuery>,
) -> Result<Json<Page<Client>>, Problem> {
    let active = query.active.unwrap_or(true);
    let updated_since = query.updated_since;
    let filters = format!(
        "active={active};updated_since={:?}",
        updated_since.map(|instant| instant.timestamp_micros()),
    );
    let walk = Walk {
        key: &state.cursor_key,
        list: "clients",
        filters: &filters,
    };
    let position = walk.position(query.limit, query.cursor.as_deref(), Utc::now())?;
    let page = ClientQuery {
        active,
        updated_since,
        after: position.after,
        limit: position.limit,
    };
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = clients::page(&mut read.tx, &read.actor, &page).await;
    let rows = read.finish(result).await?;
    catalogue::page(&walk, rows, position, |client| client.id)
}

/// One client the owner may see, active or archived.
#[utoipa::path(
    get,
    path = "/clients/{id}",
    operation_id = "getClient",
    security(("token" = []), ("oauth2" = ["catalog:read"])),
    tag = "catalogue",
    params(("id" = Uuid, Path, description = "The client's id")),
    responses(
        (status = 200, description = "The client", body = Client),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<Client>, Problem> {
    let mut read = ops::begin_read(&state.pool, caller).await?;
    let result = clients::get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.into()))
}
