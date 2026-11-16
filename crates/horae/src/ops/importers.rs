//! The Harvest importer's jobs and connection reads, for administrators only:
//! no permission grant covers them, so each operation refuses anyone but an
//! active administrator (FR-001).
//!
//! The queue itself (`jobs`) and the Harvest connection engine
//! (`importers::harvest`) keep their SQL; an operation authorizes, validates
//! and runs them inside the adapter's transaction. Import jobs publish their
//! plugin events through the outbox when they run (see the module docs of
//! [`crate::ops`]), so no operation here returns any.

use horae_core::importers::harvest::client::ClientSettings;
use horae_core::importers::harvest::types::{ConnectionStatus, ImportMode, ImportReport};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::config::{HarvestDeployment, JobPolicy};
use crate::importers::harvest::{account_switch, client_settings, csv_source};
use crate::jobs::{self, JobPayload};
use crate::models::{ErrorSummary, JobStatus};

const JOB_NOT_FOUND: &str = "Import job not found";
const NOT_RETRYABLE: &str = "Import job not found or is not retryable";
const UNRECOGNIZED_CSV: &str = "Not a recognizable Harvest CSV; check required columns";
const REQUEST_CONFLICT: &str = "Request ID already belongs to different import input";
const SUBMISSION_FAILED: &str = "Unable to submit import";

/// Whether the organization has a usable Harvest connection (never the tokens).
pub(crate) async fn connection_status(
    tx: &mut Tx,
    actor: &Actor,
    deployment: &HarvestDeployment,
) -> Result<ConnectionStatus, OpError> {
    actor.require_administrator()?;
    let configured = client_settings::resolve(&mut **tx, deployment, actor.org_id())
        .await
        .map_err(storage)?
        .is_some();
    account_switch::status(&mut **tx, actor.org_id(), configured)
        .await
        .map_err(storage)
}

/// The organization's Harvest OAuth client as administrators may see it:
/// never the secret.
pub(crate) async fn client_settings(
    tx: &mut Tx,
    actor: &Actor,
    deployment: &HarvestDeployment,
) -> Result<ClientSettings, OpError> {
    actor.require_administrator()?;
    client_settings::view(&mut **tx, deployment, actor.org_id())
        .await
        .map_err(storage)
}

/// Queue a Harvest API import under `key`, or return the job an identical
/// earlier request queued.
pub(crate) async fn start_api(
    tx: &mut Tx,
    actor: &Actor,
    payload: &JobPayload,
    key: &str,
    policy: JobPolicy,
    generation: i64,
) -> Result<Done<JobStatus>, OpError> {
    actor.require_administrator()?;
    let id = jobs::enqueue_api_in(
        tx,
        actor.org_id(),
        payload,
        key,
        policy,
        generation,
        actor.id(),
    )
    .await
    .map_err(enqueue_error)?;
    Ok(Done::new(required_job(tx, actor, id).await?))
}

/// Queue a CSV import of `body` under `key`. A repeated request returns the
/// job it queued without checking the upload again.
pub(crate) async fn start_csv(
    tx: &mut Tx,
    actor: &Actor,
    mode: ImportMode,
    body: Vec<u8>,
    key: &str,
    policy: JobPolicy,
) -> Result<Done<JobStatus>, OpError> {
    actor.require_administrator()?;
    let org_id = actor.org_id();
    if !jobs::request_exists(&mut **tx, org_id, "harvest_csv_import", key)
        .await
        .map_err(storage)?
    {
        csv_source::validate_upload_headers(&body)
            .map_err(|_| OpError::bad_input(UNRECOGNIZED_CSV))?;
    }
    let id = jobs::enqueue_csv_in(tx, org_id, mode, body, key, policy, actor.id())
        .await
        .map_err(enqueue_error)?;
    Ok(Done::new(required_job(tx, actor, id).await?))
}

/// Ask a queued or running job to stop, and report the state it is left in.
pub(crate) async fn cancel(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
) -> Result<Done<JobStatus>, OpError> {
    actor.require_administrator()?;
    jobs::cancel(&mut **tx, actor.org_id(), id)
        .await
        .map_err(storage)?;
    // Completion can race cancellation; report the actual retained state.
    Ok(Done::new(required_job(tx, actor, id).await?))
}

/// Queue a failed or cancelled job again.
pub(crate) async fn retry(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
) -> Result<Done<JobStatus>, OpError> {
    actor.require_administrator()?;
    if !jobs::retry_in(tx, actor.org_id(), id)
        .await
        .map_err(enqueue_error)?
    {
        return Err(OpError::not_found(NOT_RETRYABLE));
    }
    Ok(Done::new(required_job(tx, actor, id).await?))
}

/// Queue the commit of a finished preview with the preview's own input.
/// Repeating it returns the same commit job.
pub(crate) async fn commit_preview(
    tx: &mut Tx,
    actor: &Actor,
    preview_id: Uuid,
    policy: JobPolicy,
) -> Result<Done<JobStatus>, OpError> {
    actor.require_administrator()?;
    let id = jobs::commit_preview_in(tx, actor.org_id(), preview_id, policy, actor.id())
        .await
        .map_err(enqueue_error)?
        .ok_or_else(|| OpError::not_found(JOB_NOT_FOUND))?;
    Ok(Done::new(required_job(tx, actor, id).await?))
}

/// One job of the organization, or `None` when it has no such job.
pub(crate) async fn status(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
) -> Result<Option<JobStatus>, OpError> {
    actor.require_administrator()?;
    jobs::status(&mut **tx, actor.org_id(), id)
        .await
        .map_err(storage)
}

/// The organization's jobs, newest first, before the job `before`.
pub(crate) async fn history(
    tx: &mut Tx,
    actor: &Actor,
    limit: i64,
    before: Option<Uuid>,
) -> Result<Vec<JobStatus>, OpError> {
    actor.require_administrator()?;
    jobs::list(&mut **tx, actor.org_id(), limit, before)
        .await
        .map_err(storage)
}

/// A finished run's errors grouped by problem, and the people behind them.
///
/// It runs on a plain connection with an actor loaded without locks
/// ([`Actor::load_unlocked`]): the report no longer changes once the run ends,
/// so the read never queues behind an import or an access change, and every
/// query is scoped by the actor's organization.
pub(crate) async fn error_summary(
    connection: &mut PgConnection,
    actor: &Actor,
    id: Uuid,
) -> Result<ErrorSummary, OpError> {
    actor.require_administrator()?;
    jobs::report::summarize(connection, actor.org_id(), id)
        .await
        .map_err(storage)?
        .ok_or_else(|| OpError::not_found(JOB_NOT_FOUND))
}

/// What a download of a job's error details serves: the archived chunks
/// before `end`, then `tail`, the errors still held in the report itself.
pub(crate) struct ErrorDetails {
    pub end: i64,
    pub tail: Vec<u8>,
}

/// The error details of the job's current report, or `None` when the
/// organization has no such job or the job has no report yet.
pub(crate) async fn error_details(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
) -> Result<Option<ErrorDetails>, OpError> {
    actor.require_administrator()?;
    let Some(report) = jobs::status(&mut **tx, actor.org_id(), id)
        .await
        .map_err(storage)?
        .and_then(|job| job.report)
    else {
        return Ok(None);
    };
    let report: ImportReport = serde_json::from_value(report).map_err(internal)?;
    let end = i64::try_from(report.archived_error_chunks()).map_err(internal)?;
    let mut tail = Vec::new();
    for error in report.row_errors {
        serde_json::to_writer(&mut tail, &error).map_err(internal)?;
        tail.push(b'\n');
    }
    Ok(Some(ErrorDetails { end, tail }))
}

/// Up to 16 archived chunks of the job's error details, from `next` up to `end`.
pub(crate) async fn error_chunks(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    next: i64,
    end: i64,
) -> Result<Vec<Vec<u8>>, OpError> {
    actor.require_administrator()?;
    jobs::report::chunks(&mut **tx, actor.org_id(), id, next, end)
        .await
        .map_err(storage)
}

async fn required_job(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<JobStatus, OpError> {
    jobs::status(&mut **tx, actor.org_id(), id)
        .await
        .map_err(storage)?
        .ok_or_else(|| OpError::not_found(JOB_NOT_FOUND))
}

/// Why queueing a job failed, as the importer page explains it.
fn enqueue_error(error: anyhow::Error) -> OpError {
    if let Some(policy) = error.downcast_ref::<account_switch::ChangeError>() {
        return match policy {
            account_switch::ChangeError::NotConnected => OpError::not_found(policy.to_string()),
            _ => OpError::conflict(policy.to_string()),
        };
    }
    if let Some(refused) = error.downcast_ref::<jobs::CommitRefused>() {
        return OpError::conflict(refused.to_string());
    }
    if error.is::<jobs::RequestConflict>() {
        return OpError::conflict(REQUEST_CONFLICT);
    }
    tracing::error!(%error, "durable import submission failed");
    OpError::Internal(SUBMISSION_FAILED.into())
}

/// A storage failure reached through the queue's `anyhow` results: a database
/// error maps like any other, anything else is logged and hidden.
fn storage(error: anyhow::Error) -> OpError {
    match error.downcast::<sqlx::Error>() {
        Ok(error) => OpError::from(error),
        Err(error) => internal(error),
    }
}

fn internal(error: impl std::fmt::Display) -> OpError {
    tracing::error!(%error, "Import job read failed");
    OpError::Internal("Could not complete the request. Please retry.".into())
}
