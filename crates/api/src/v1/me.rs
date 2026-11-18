//! Identity: who the token acts as, what they may track, and their running
//! timer.

use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::IntoParams;
use uuid::Uuid;

use super::ProblemResponse;
use super::types::{Me, Page, TimeEntry, Trackable};
use crate::models::scoped_time::TimesheetWriteContext;
use crate::ops::{self, Actor, Caller, OpError, Tx};
use crate::problem::{ApiQuery, Problem};

/// The token's owner, their organization and the token.
#[utoipa::path(
    get,
    path = "/me",
    operation_id = "getMe",
    tag = "identity",
    responses(
        (status = 200, description = "The token's owner", body = Me),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn me(
    State(pool): State<PgPool>,
    Extension(caller): Extension<Caller>,
) -> Result<Json<Me>, Problem> {
    let mut read = ops::begin_read(&pool, caller).await?;
    let result = identity(&mut read.tx, &read.actor).await;
    let (identity, permissions) = read.finish(result).await?;
    Ok(Json(Me::new(caller.org_id, identity, permissions)))
}

async fn identity(
    tx: &mut Tx,
    actor: &Actor,
) -> Result<
    (
        ops::api_tokens::TokenIdentity,
        crate::models::own_permissions::OwnPermissions,
    ),
    OpError,
> {
    let identity = ops::api_tokens::identity(tx, actor).await?;
    let permissions = ops::profile::permissions(tx, actor).await?;
    Ok((identity, permissions))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TrackableQuery {
    /// Another person the owner may write time for; the owner when absent.
    person_id: Option<Uuid>,
}

/// The project and task pairs a person may track time on today: the owner,
/// or a person the owner may write time for.
#[utoipa::path(
    get,
    path = "/me/trackable",
    operation_id = "listTrackable",
    tag = "identity",
    params(TrackableQuery),
    responses(
        (status = 200, description = "Pairs the person may track, all on one page", body = Page<Trackable>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn trackable(
    State(pool): State<PgPool>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<TrackableQuery>,
) -> Result<Json<Page<Trackable>>, Problem> {
    let mut read = ops::begin_read(&pool, caller).await?;
    let context = TimesheetWriteContext {
        expected_requester: read.actor.requester(),
        subject_id: query.person_id.unwrap_or(caller.user_id),
    };
    let result = ops::time::tracking(&mut read.tx, &read.actor, &context).await;
    let options = read.finish(result).await?;
    Ok(Json(Page {
        data: options.into_iter().map(Trackable::from).collect(),
        next_cursor: None,
    }))
}

/// The owner's running timer.
#[utoipa::path(
    get,
    path = "/me/timer",
    operation_id = "getRunningTimer",
    tag = "identity",
    responses(
        (status = 200, description = "The running timer", body = TimeEntry),
        (status = 204, description = "No timer is running"),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn timer(
    State(pool): State<PgPool>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, Problem> {
    let mut read = ops::begin_read(&pool, caller).await?;
    let result = ops::time::current_timer(&mut read.tx, &read.actor).await;
    Ok(match read.finish(result).await? {
        Some(entry) => Json(TimeEntry::from(entry)).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}

#[cfg(test)]
mod tests;
