//! Session-bound importer operations, separate from queue worker mechanics.

use super::*;
use crate::{config::JobPolicy, jobs, models::JobStatus};
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn start_api(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    payload: &jobs::JobPayload,
    key: &str,
    policy: JobPolicy,
    generation: i64,
) -> Result<JobStatus, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    let id = jobs::enqueue_api_in(&mut tx, org_id, payload, key, policy, generation)
        .await
        .map_err(map_enqueue_error)?;
    let result = required_job(&mut tx, org_id, id).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

pub(super) async fn start_csv(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    key: &str,
    policy: JobPolicy,
) -> Result<JobStatus, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    if !jobs::request_exists(&mut *tx, org_id, "harvest_csv_import", key)
        .await
        .map_err(server_err)?
    {
        crate::importers::harvest::csv_source::validate_upload_headers(&body).map_err(|_| {
            err(
                BAD_REQUEST,
                "Not a recognizable Harvest CSV; check required columns",
            )
        })?;
    }
    let id = jobs::enqueue_csv_in(&mut tx, org_id, mode, body, key, policy)
        .await
        .map_err(map_enqueue_error)?;
    let result = required_job(&mut tx, org_id, id).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

pub(super) async fn cancel(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> Result<JobStatus, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    jobs::cancel(&mut *tx, org_id, id)
        .await
        .map_err(server_err)?;
    // Completion can race cancellation; report the actual retained state.
    let result = required_job(&mut tx, org_id, id).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

pub(super) async fn retry(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> Result<JobStatus, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    if !jobs::retry_in(&mut tx, org_id, id)
        .await
        .map_err(map_enqueue_error)?
    {
        return Err(err(NOT_FOUND, "Import job not found or is not retryable"));
    }
    let result = required_job(&mut tx, org_id, id).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

pub(super) async fn status(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    id: Uuid,
) -> Result<Option<JobStatus>, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    let result = jobs::status(&mut *tx, org_id, id)
        .await
        .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

pub(super) async fn history(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    limit: i64,
    before: Option<Uuid>,
) -> Result<Vec<JobStatus>, ServerFnError> {
    let mut tx = begin_access(pool, org_id, actor_id).await?;
    let result = jobs::list(&mut *tx, org_id, limit, before)
        .await
        .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(result)
}

async fn required_job(
    connection: &mut sqlx::PgConnection,
    org_id: Uuid,
    id: Uuid,
) -> Result<JobStatus, ServerFnError> {
    jobs::status(connection, org_id, id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| err(NOT_FOUND, "Import job not found"))
}

async fn begin_access(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, ServerFnError> {
    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    crate::db::lock_organization(&mut tx, org_id, crate::db::OrganizationLock::Shared)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => forbidden("Active administrator access required"),
            other => server_err(other),
        })?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND org_id = $2
           AND active AND org_role = $3 FOR SHARE",
        actor_id,
        org_id,
        OrgRole::Admin as OrgRole,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| forbidden("Active administrator access required"))?;
    Ok(tx)
}

#[cfg(test)]
mod tests;
