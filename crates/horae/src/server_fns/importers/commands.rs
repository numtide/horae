//! Session adapters over `ops::importers` for the import jobs, shared by the
//! server functions and their tests.

use super::*;
use crate::{
    config::JobPolicy,
    jobs,
    models::JobStatus,
    ops::{self, Caller, EventSink},
};
use sqlx::PgPool;
use uuid::Uuid;

#[expect(clippy::too_many_arguments, reason = "mirrors the request's fields")]
pub(super) async fn start_api(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    payload: &jobs::JobPayload,
    key: &str,
    policy: JobPolicy,
    generation: i64,
    events: &dyn EventSink,
) -> Result<JobStatus, ServerFnError> {
    let mut write = ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result = ops::importers::start_api(
        &mut write.tx,
        &write.actor,
        payload,
        key,
        policy,
        generation,
    )
    .await;
    Ok(write.commit(result, events).await?)
}

#[expect(clippy::too_many_arguments, reason = "mirrors the request's fields")]
pub(super) async fn start_csv(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    key: &str,
    policy: JobPolicy,
    events: &dyn EventSink,
) -> Result<JobStatus, ServerFnError> {
    let mut write = ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result =
        ops::importers::start_csv(&mut write.tx, &write.actor, mode, body, key, policy).await;
    Ok(write.commit(result, events).await?)
}

pub(super) async fn cancel(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    events: &dyn EventSink,
) -> Result<JobStatus, ServerFnError> {
    let mut write = ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result = ops::importers::cancel(&mut write.tx, &write.actor, id).await;
    Ok(write.commit(result, events).await?)
}

pub(super) async fn retry(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
    events: &dyn EventSink,
) -> Result<JobStatus, ServerFnError> {
    let mut write = ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result = ops::importers::retry(&mut write.tx, &write.actor, id).await;
    Ok(write.commit(result, events).await?)
}

pub(super) async fn commit_preview(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    preview_id: Uuid,
    policy: JobPolicy,
    events: &dyn EventSink,
) -> Result<JobStatus, ServerFnError> {
    let mut write = ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result =
        ops::importers::commit_preview(&mut write.tx, &write.actor, preview_id, policy).await;
    Ok(write.commit(result, events).await?)
}

pub(super) async fn error_summary(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> Result<crate::models::ErrorSummary, ServerFnError> {
    let mut connection = pool.acquire().await.map_err(ops::OpError::from)?;
    let actor =
        ops::Actor::load_unlocked(&mut connection, Caller::session(org_id, actor_id)).await?;
    Ok(ops::importers::error_summary(&mut connection, &actor, id).await?)
}

pub(super) async fn status(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> Result<Option<JobStatus>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, actor_id)).await?;
    let result = ops::importers::status(&mut read.tx, &read.actor, id).await;
    Ok(read.finish(result).await?)
}

pub(super) async fn history(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    limit: i64,
    before: Option<Uuid>,
) -> Result<Vec<JobStatus>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, actor_id)).await?;
    let result = ops::importers::history(&mut read.tx, &read.actor, limit, before).await;
    Ok(read.finish(result).await?)
}

#[cfg(test)]
mod tests;
