//! The Harvest import jobs on the durable queue of `horae-jobs`: their
//! payloads, enqueue and preview commit, the status projections the UI and
//! the CLI read, retry, and the handlers [`spawn`] registers.

use anyhow::Context;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind, SyncScope};
use horae_jobs::{BoxFuture, Completed, JobContext, JobError, Registry, RetryPolicy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::JobPolicy, state::AppState};

mod lease;
pub mod outbox;
pub mod report;
mod sweeps;
pub use horae_jobs::outbox::{
    OutboxEvent, claim_outbox, enqueue_outbox, mark_outbox_delivered, mark_outbox_failed,
    stop_outbox_delivery,
};
pub use horae_jobs::{ConcurrencyKeyHeld, Worker, cancel};
pub use lease::JobLease;
pub use sweeps::{ensure_all_schedules, ensure_schedules};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum JobPayload {
    HarvestApi {
        mode: ImportMode,
        sync: SyncScope,
    },
    HarvestCsv {
        mode: ImportMode,
    },
    #[cfg(test)]
    Synthetic,
}

fn encode_payload(payload: &JobPayload) -> anyhow::Result<serde_json::Value> {
    horae_jobs::encode_payload(payload)
}

fn decode_payload(value: serde_json::Value) -> anyhow::Result<JobPayload> {
    horae_jobs::decode_payload(value)
}

impl JobPayload {
    fn initial_report(&self) -> anyhow::Result<Option<serde_json::Value>> {
        let (source, mode) = match self {
            Self::HarvestApi { mode, .. } => (SourceKind::HarvestApi, *mode),
            Self::HarvestCsv { mode } => (SourceKind::Csv, *mode),
            #[cfg(test)]
            Self::Synthetic => return Ok(None),
        };
        Ok(Some(serde_json::to_value(ImportReport::new(source, mode))?))
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::HarvestApi { .. } => "harvest_api_import",
            Self::HarvestCsv { .. } => "harvest_csv_import",
            #[cfg(test)]
            Self::Synthetic => "synthetic_test_job",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Idempotency key conflicts with another import request")]
pub struct RequestConflict;

pub async fn request_exists<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    kind: &str,
    key: &str,
) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM horae_jobs
             WHERE org_id = $1 AND kind = $2 AND idempotency_key = $3) AS "exists!""#,
        org_id,
        kind,
        key,
    )
    .fetch_one(exec)
    .await?)
}

#[cfg(any(test, feature = "test-support"))]
pub async fn enqueue(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
) -> anyhow::Result<Uuid> {
    let mut tx = pool.begin().await?;
    let current = crate::importers::harvest::account_switch::gate(&mut tx, org_id).await?;
    anyhow::ensure!(
        current.account_generation == 0,
        crate::importers::harvest::account_switch::ChangeError::OldImport
    );
    let id = enqueue_in(&mut tx, org_id, payload, idempotency_key, policy, 0, None).await?;
    tx.commit().await?;
    Ok(id)
}

#[cfg(any(test, feature = "test-support"))]
pub async fn enqueue_api(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
) -> anyhow::Result<Uuid> {
    let mut tx = pool.begin().await?;
    let id = enqueue_api_request(
        &mut tx,
        org_id,
        payload,
        idempotency_key,
        policy,
        generation,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Record the original actor within the caller's authorized transaction.
/// Existing jobs keep their attribution; this is not worker authorization.
pub async fn enqueue_api_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
    requester_id: Uuid,
) -> anyhow::Result<Uuid> {
    enqueue_api_request(
        tx,
        org_id,
        payload,
        idempotency_key,
        policy,
        generation,
        Some(requester_id),
    )
    .await
}

async fn enqueue_api_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
    original_requester_id: Option<Uuid>,
) -> anyhow::Result<Uuid> {
    use crate::importers::harvest::account_switch::{self, ChangeError};
    let current = account_switch::gate(tx, org_id).await?;
    anyhow::ensure!(
        generation == current.account_generation,
        ChangeError::OldImport
    );
    let existing = sqlx::query_scalar!("SELECT id FROM horae_jobs WHERE org_id = $1 AND kind = 'harvest_api_import' AND idempotency_key = $2", org_id, idempotency_key)
        .fetch_optional(&mut **tx).await?;
    if existing.is_none() {
        let connection = account_switch::status(&mut **tx, org_id, true).await?;
        anyhow::ensure!(connection.connected, ChangeError::NotConnected);
    }
    let id = enqueue_in(
        tx,
        org_id,
        payload,
        idempotency_key,
        policy,
        generation,
        original_requester_id,
    )
    .await?;
    Ok(id)
}

async fn enqueue_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
    original_requester_id: Option<Uuid>,
) -> anyhow::Result<Uuid> {
    let policy = policy.validate()?;
    let id = Uuid::now_v7();
    let encoded = encode_payload(payload)?;
    let report = payload.initial_report()?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts, report, account_generation, original_requester_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           ON CONFLICT (org_id, kind, idempotency_key)
           DO UPDATE SET updated_at = now()
           WHERE horae_jobs.payload = EXCLUDED.payload AND horae_jobs.account_generation = EXCLUDED.account_generation
           RETURNING id"#,
        id,
        org_id,
        payload.kind(),
        encoded,
        idempotency_key,
        policy.max_attempts,
        report,
        generation,
        original_requester_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(RequestConflict)?;
    Ok(row.id)
}

#[cfg(any(test, feature = "test-support"))]
pub async fn enqueue_csv(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    idempotency_key: &str,
    policy: JobPolicy,
) -> anyhow::Result<Uuid> {
    let mut tx = pool.begin().await?;
    let id =
        enqueue_csv_request(&mut tx, org_id, mode, body, idempotency_key, policy, None).await?;
    tx.commit().await?;
    Ok(id)
}

/// Persist the accepted upload and job atomically in the authorization transaction.
/// The original actor is retained across identical submissions and retries.
pub async fn enqueue_csv_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    idempotency_key: &str,
    policy: JobPolicy,
    requester_id: Uuid,
) -> anyhow::Result<Uuid> {
    enqueue_csv_request(
        tx,
        org_id,
        mode,
        body,
        idempotency_key,
        policy,
        Some(requester_id),
    )
    .await
}

async fn enqueue_csv_request(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    idempotency_key: &str,
    policy: JobPolicy,
    original_requester_id: Option<Uuid>,
) -> anyhow::Result<Uuid> {
    let policy = policy.validate()?;
    let id = Uuid::now_v7();
    let payload = JobPayload::HarvestCsv { mode };
    let report = payload.initial_report()?;
    let payload = encode_payload(&payload)?;
    crate::importers::harvest::account_switch::gate(tx, org_id).await?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts, report, original_requester_id)
           VALUES ($1, $2, 'harvest_csv_import',
                   $3::jsonb || jsonb_build_object('upload_sha256', encode(sha256($7::bytea), 'hex')),
                   $4, $5, $6, $8)
           ON CONFLICT (org_id, kind, idempotency_key)
           DO UPDATE SET updated_at = now()
           WHERE horae_jobs.payload = EXCLUDED.payload
           RETURNING id"#,
        id,
        org_id,
        payload,
        idempotency_key,
        policy.max_attempts,
        report,
        &body,
        original_requester_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(RequestConflict)?;
    // An identical resubmission must not restore an upload already removed by
    // successful-job retention, or replace an accepted source.
    if row.id == id {
        sqlx::query!(
            r#"INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body)
           VALUES ($1, $2, 'harvest.csv', 'text/csv', $3)
           ON CONFLICT (job_id) DO NOTHING"#,
            row.id,
            org_id,
            body,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(row.id)
}

/// Why a preview cannot be committed. The run page shows the same reasons in
/// advance through [`crate::models::CommitAvailability`].
#[derive(Debug, thiserror::Error)]
pub enum CommitRefused {
    #[error("Only a preview that finished can be committed")]
    NotPreview,
    #[error("A newer import replaced this preview; start a new preview")]
    NewerImport,
    #[error("The Harvest account changed since this preview; start a new preview")]
    PreviousAccount,
    #[error("The CSV upload is no longer retained; upload the file again")]
    MissingUpload,
}

/// Queue the commit of a finished preview with the preview's own input: its
/// sync scope and account generation, or its retained CSV upload. Once per
/// preview: a repeated request returns the commit it already queued. `None`
/// when the preview is not this organization's.
pub async fn commit_preview_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    preview_id: Uuid,
    policy: JobPolicy,
    requester_id: Uuid,
) -> anyhow::Result<Option<Uuid>> {
    use crate::models::CommitAvailability;
    // Locking the preview serializes two tabs committing it at once.
    let Some(preview) = sqlx::query!(
        "SELECT kind, payload, account_generation FROM horae_jobs
         WHERE id = $1 AND org_id = $2 FOR UPDATE",
        preview_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    let key = format!("preview-commit:{preview_id}");
    let queued = sqlx::query_scalar!(
        "SELECT id FROM horae_jobs WHERE org_id = $1 AND kind = $2 AND idempotency_key = $3",
        org_id,
        preview.kind,
        key,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if queued.is_some() {
        return Ok(queued);
    }
    let availability = status(&mut **tx, org_id, preview_id)
        .await?
        .context("preview disappeared")?
        .commit_availability;
    match availability {
        CommitAvailability::Available => {}
        CommitAvailability::NewerImport => anyhow::bail!(CommitRefused::NewerImport),
        CommitAvailability::PreviousAccount => anyhow::bail!(CommitRefused::PreviousAccount),
        CommitAvailability::MissingUpload => anyhow::bail!(CommitRefused::MissingUpload),
        // A committed preview returned its commit above.
        CommitAvailability::Committed
        | CommitAvailability::NotPreview
        | CommitAvailability::UnavailableState
        | CommitAvailability::Unknown => anyhow::bail!(CommitRefused::NotPreview),
    }
    let id = match decode_payload(preview.payload)? {
        JobPayload::HarvestApi { sync, .. } => {
            let payload = JobPayload::HarvestApi {
                mode: ImportMode::Commit,
                sync,
            };
            enqueue_api_request(
                tx,
                org_id,
                &payload,
                &key,
                policy,
                preview.account_generation,
                Some(requester_id),
            )
            .await?
        }
        JobPayload::HarvestCsv { .. } => {
            let body = sqlx::query_scalar!(
                "SELECT body FROM horae_job_uploads WHERE job_id = $1 AND org_id = $2",
                preview_id,
                org_id,
            )
            .fetch_optional(&mut **tx)
            .await?
            .context(CommitRefused::MissingUpload)?;
            enqueue_csv_request(
                tx,
                org_id,
                ImportMode::Commit,
                body,
                &key,
                policy,
                Some(requester_id),
            )
            .await?
        }
        #[cfg(test)]
        JobPayload::Synthetic => anyhow::bail!(CommitRefused::NotPreview),
    };
    Ok(Some(id))
}

/// What an API run reads from Harvest: everything, or the changes since the
/// last sync. `None` for CSV and unreadable payloads.
fn sync_scope(kind: &str, payload: &serde_json::Value) -> Option<SyncScope> {
    match decode_payload(payload.clone()) {
        Ok(JobPayload::HarvestApi { sync, .. }) if kind == "harvest_api_import" => Some(sync),
        _ => None,
    }
}

/// `horae_jobs.error_kind`, whose CHECK constraint lists the same values.
fn error_kind_column(kind: crate::models::JobErrorKind) -> &'static str {
    use crate::models::JobErrorKind;
    match kind {
        JobErrorKind::ReconnectRequired => "reconnect_required",
        JobErrorKind::Forbidden => "forbidden",
        JobErrorKind::InvoicesForbidden => "invoices_forbidden",
        JobErrorKind::Unavailable => "unavailable",
    }
}

fn error_kind(column: Option<&str>) -> Option<crate::models::JobErrorKind> {
    use crate::models::JobErrorKind;
    match column? {
        "reconnect_required" => Some(JobErrorKind::ReconnectRequired),
        "forbidden" => Some(JobErrorKind::Forbidden),
        "invoices_forbidden" => Some(JobErrorKind::InvoicesForbidden),
        "unavailable" => Some(JobErrorKind::Unavailable),
        _ => None,
    }
}

fn retry_availability(
    kind: &str,
    state: &str,
    payload: serde_json::Value,
    generation: i64,
    current_generation: i64,
    has_upload: bool,
) -> crate::models::RetryAvailability {
    use crate::models::RetryAvailability;
    let Ok(payload) = decode_payload(payload) else {
        return RetryAvailability::Unknown;
    };
    if payload.kind() != kind || !matches!(kind, "harvest_api_import" | "harvest_csv_import") {
        return RetryAvailability::Unknown;
    }
    if !matches!(state, "failed" | "cancelled") {
        return RetryAvailability::UnavailableState;
    }
    match payload {
        JobPayload::HarvestApi { .. } if generation != current_generation => {
            RetryAvailability::PreviousAccount
        }
        JobPayload::HarvestCsv { .. } if !has_upload => RetryAvailability::MissingUpload,
        _ => RetryAvailability::Available,
    }
}

/// The request's mode and source, and whether a finished preview may still be
/// committed: once, as the latest live run of its source, against the same
/// account, with its CSV upload retained. `newer_import` ignores failed and
/// cancelled runs and the preview's own commit.
#[allow(clippy::too_many_arguments)]
fn preview_fields(
    kind: &str,
    state: &str,
    payload: &serde_json::Value,
    generation: i64,
    current_generation: i64,
    has_upload: bool,
    newer_import: bool,
    committed: bool,
) -> (
    Option<ImportMode>,
    Option<SourceKind>,
    crate::models::CommitAvailability,
) {
    use crate::models::CommitAvailability;
    let (mode, source) = match decode_payload(payload.clone()) {
        Ok(JobPayload::HarvestApi { mode, .. }) if kind == "harvest_api_import" => {
            (mode, SourceKind::HarvestApi)
        }
        Ok(JobPayload::HarvestCsv { mode }) if kind == "harvest_csv_import" => {
            (mode, SourceKind::Csv)
        }
        _ => return (None, None, CommitAvailability::Unknown),
    };
    let availability = if mode == ImportMode::Commit {
        CommitAvailability::NotPreview
    } else if state != "succeeded" {
        CommitAvailability::UnavailableState
    } else if committed {
        CommitAvailability::Committed
    } else if newer_import {
        CommitAvailability::NewerImport
    } else if source == SourceKind::HarvestApi && generation != current_generation {
        CommitAvailability::PreviousAccount
    } else if source == SourceKind::Csv && !has_upload {
        CommitAvailability::MissingUpload
    } else {
        CommitAvailability::Available
    };
    (Some(mode), Some(source), availability)
}

pub async fn status<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<crate::models::JobStatus>> {
    let row = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  COALESCE(checkpoint->'report', report) AS report, last_error,
                  -- A cancelled job reports no cause, however it was cancelled.
                  CASE WHEN status = 'cancelled' THEN NULL ELSE error_kind END AS error_kind,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>",
                  payload, account_generation,
                  COALESCE((SELECT g.account_generation FROM harvest_connection_generations g
                            WHERE g.org_id = horae_jobs.org_id), 0) AS "current_generation!",
                  EXISTS(SELECT 1 FROM horae_job_uploads u
                         WHERE u.job_id = horae_jobs.id AND u.org_id = horae_jobs.org_id) AS "has_upload!",
                  EXISTS(SELECT 1 FROM horae_jobs n
                         WHERE n.org_id = horae_jobs.org_id AND n.kind = horae_jobs.kind
                           AND (n.created_at, n.id) > (horae_jobs.created_at, horae_jobs.id)
                           AND n.status NOT IN ('cancelled', 'failed')
                           AND n.idempotency_key IS DISTINCT FROM 'preview-commit:' || horae_jobs.id) AS "newer_import!",
                  (SELECT c.id FROM horae_jobs c
                    WHERE c.org_id = horae_jobs.org_id AND c.kind = horae_jobs.kind
                      AND c.idempotency_key = 'preview-commit:' || horae_jobs.id) AS committed_as
             FROM horae_jobs
            WHERE id = $1 AND org_id = $2"#,
        id,
        org_id,
    )
    .fetch_optional(exec)
    .await?;
    Ok(row.map(|r| {
        let (mode, source, commit_availability) = preview_fields(
            &r.kind,
            &r.status,
            &r.payload,
            r.account_generation,
            r.current_generation,
            r.has_upload,
            r.newer_import,
            r.committed_as.is_some(),
        );
        crate::models::JobStatus {
            mode,
            source,
            commit_availability,
            committed_as: r.committed_as,
            sync: sync_scope(&r.kind, &r.payload),
            retry_availability: retry_availability(
                &r.kind,
                &r.status,
                r.payload,
                r.account_generation,
                r.current_generation,
                r.has_upload,
            ),
            id: r.id,
            kind: r.kind,
            status: r.status,
            phase: r.phase,
            processed_count: r.processed_count,
            total_count: r.total_count,
            report: r.report,
            last_error: r.last_error,
            error_kind: error_kind(r.error_kind.as_deref()),
            created_at: r.created_at,
            finished_at: r.finished_at,
        }
    }))
}

pub async fn list<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    limit: i64,
    before: Option<Uuid>,
) -> anyhow::Result<Vec<crate::models::JobStatus>> {
    let rows = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  COALESCE(checkpoint->'report', report) AS report, last_error,
                  -- A cancelled job reports no cause, however it was cancelled.
                  CASE WHEN status = 'cancelled' THEN NULL ELSE error_kind END AS error_kind,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>",
                  payload, account_generation,
                  COALESCE((SELECT g.account_generation FROM harvest_connection_generations g
                            WHERE g.org_id = horae_jobs.org_id), 0) AS "current_generation!",
                  EXISTS(SELECT 1 FROM horae_job_uploads u
                         WHERE u.job_id = horae_jobs.id AND u.org_id = horae_jobs.org_id) AS "has_upload!",
                  EXISTS(SELECT 1 FROM horae_jobs n
                         WHERE n.org_id = horae_jobs.org_id AND n.kind = horae_jobs.kind
                           AND (n.created_at, n.id) > (horae_jobs.created_at, horae_jobs.id)
                           AND n.status NOT IN ('cancelled', 'failed')
                           AND n.idempotency_key IS DISTINCT FROM 'preview-commit:' || horae_jobs.id) AS "newer_import!",
                  (SELECT c.id FROM horae_jobs c
                    WHERE c.org_id = horae_jobs.org_id AND c.kind = horae_jobs.kind
                      AND c.idempotency_key = 'preview-commit:' || horae_jobs.id) AS committed_as
             FROM horae_jobs
            -- The sweeps (`sweeps::SWEEPS`) run every minute: only the ones that
            -- failed or were cancelled are worth listing.
            WHERE org_id = $1
              AND NOT (kind IN ('long_timer_sweep', 'budget_alert_sweep', 'oauth_cleanup')
                       AND status IN ('queued', 'running', 'succeeded'))
              AND ($3::uuid IS NULL OR (created_at, id) < (
                  SELECT created_at, id FROM horae_jobs WHERE id = $3 AND org_id = $1
              ))
            ORDER BY created_at DESC, id DESC
            LIMIT $2"#,
        org_id,
        limit.clamp(1, 100),
        before,
    )
    .fetch_all(exec)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let (mode, source, commit_availability) = preview_fields(
                &r.kind,
                &r.status,
                &r.payload,
                r.account_generation,
                r.current_generation,
                r.has_upload,
                r.newer_import,
                r.committed_as.is_some(),
            );
            crate::models::JobStatus {
                mode,
                source,
                commit_availability,
                committed_as: r.committed_as,
                sync: sync_scope(&r.kind, &r.payload),
                retry_availability: retry_availability(
                    &r.kind,
                    &r.status,
                    r.payload,
                    r.account_generation,
                    r.current_generation,
                    r.has_upload,
                ),
                id: r.id,
                kind: r.kind,
                status: r.status,
                phase: r.phase,
                processed_count: r.processed_count,
                total_count: r.total_count,
                report: r.report,
                last_error: r.last_error,
                error_kind: error_kind(r.error_kind.as_deref()),
                created_at: r.created_at,
                finished_at: r.finished_at,
            }
        })
        .collect())
}

#[cfg(test)]
pub async fn retry(pool: &sqlx::PgPool, org_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
    let mut tx = pool.begin().await?;
    let retried = retry_in(&mut tx, org_id, id).await?;
    tx.commit().await?;
    Ok(retried)
}

/// Retry within the caller's authorization transaction, retaining generation fencing.
pub async fn retry_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<bool> {
    let current = crate::importers::harvest::account_switch::gate(tx, org_id).await?;
    let generation = sqlx::query_scalar!("SELECT account_generation FROM horae_jobs WHERE id = $1 AND org_id = $2 AND kind = 'harvest_api_import'", id, org_id)
        .fetch_optional(&mut **tx).await?;
    anyhow::ensure!(
        generation.is_none_or(|generation| generation == current.account_generation),
        crate::importers::harvest::account_switch::ChangeError::OldImport
    );
    // Another job may hold this job's concurrency key since it ended.
    let key_held = sqlx::query_scalar!(
        r#"SELECT EXISTS(
             SELECT 1 FROM horae_jobs j JOIN horae_jobs other
                    ON other.org_id = j.org_id AND other.kind = j.kind
                   AND other.concurrency_key = j.concurrency_key AND other.id <> j.id
              WHERE j.id = $1 AND j.org_id = $2
                AND other.status IN ('queued', 'running') AND NOT other.cancellation_requested
           ) AS "held!""#,
        id,
        org_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    anyhow::ensure!(!key_held, ConcurrencyKeyHeld);
    let result = sqlx::query!(
        r#"UPDATE horae_jobs
              SET status = 'queued', available_at = now(), lease_until = NULL, heartbeat_at = NULL,
                  attempts = 0, worker_id = NULL, last_error = NULL, error_kind = NULL, phase = NULL,
                  claim_token = NULL, cancellation_requested = false,
                  finished_at = NULL, updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('failed', 'cancelled')
              AND (kind <> 'harvest_csv_import' OR EXISTS (
                  SELECT 1 FROM horae_job_uploads u WHERE u.job_id = horae_jobs.id AND u.org_id = $2
              ))"#,
        id,
        org_id,
    )
    .execute(&mut **tx)
    .await
    // A job enqueued with the key after the check above.
    .map_err(|error| {
        if horae_jobs::is_concurrency_key_conflict(&error) {
            anyhow::Error::new(ConcurrencyKeyHeld)
        } else {
            error.into()
        }
    })?;
    Ok(result.rows_affected() == 1)
}

/// The durable worker, running the import kinds.
pub fn spawn(state: &AppState) -> Worker {
    horae_jobs::spawn(state.db.clone(), registry(state))
}

/// A worker of its own for the sweeps, so that a long import, which holds
/// [`spawn`]'s worker for as long as it runs, does not delay them.
pub fn spawn_sweeps(state: &AppState) -> Worker {
    horae_jobs::spawn(state.db.clone(), sweeps::registry(state))
}

/// The scheduler, enqueuing what the stored schedules are due for.
pub fn spawn_scheduler(state: &AppState) -> horae_jobs::Scheduler {
    horae_jobs::spawn_scheduler(
        state.db.clone(),
        std::sync::Arc::new(crate::timezone::Zones::new(state.db.clone())),
    )
}

fn registry(state: &AppState) -> Registry {
    let mut registry = Registry::default();
    registry
        .register(ImportJob {
            state: state.clone(),
            kind: "harvest_api_import",
        })
        .register(ImportJob {
            state: state.clone(),
            kind: "harvest_csv_import",
        });
    #[cfg(test)]
    registry.register(ImportJob {
        state: state.clone(),
        kind: "synthetic_test_job",
    });
    registry
}

/// Runs whichever import the payload names, as the worker did before kinds
/// had handlers, so a row's kind and payload never disagree on what runs.
struct ImportJob {
    state: AppState,
    kind: &'static str,
}

/// The imports keep the queue's original curve. Each job stops at the
/// `max_attempts` stored when it was queued (`HORAE_JOB_MAX_ATTEMPTS`).
fn import_policy(policy: JobPolicy) -> RetryPolicy {
    RetryPolicy {
        max_attempts: policy.max_attempts,
        ..RetryPolicy::default()
    }
}

impl horae_jobs::Handler for ImportJob {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn policy(&self) -> RetryPolicy {
        import_policy(self.state.job_policy)
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>> {
        Box::pin(async move {
            let lease = JobLease(ctx.clone());
            run_import(&self.state, &lease, payload)
                .await
                .map(|(report, processed_count)| Completed {
                    report,
                    processed_count,
                })
                .map_err(job_error)
        })
    }

    /// The CSV upload is the only thing an import keeps beside its job.
    fn cleanup<'a>(&'a self, pool: &'a sqlx::PgPool) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            if self.kind == "harvest_csv_import" {
                delete_retired_uploads(pool).await?;
            }
            Ok(())
        })
    }
}

async fn delete_retired_uploads(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        r#"DELETE FROM horae_job_uploads u
             USING horae_jobs j
            WHERE u.job_id = j.id
              AND j.status = 'succeeded'
              AND j.finished_at < now() - interval '1 day'"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn cleanup(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    delete_retired_uploads(pool).await?;
    horae_jobs::cleanup(pool).await
}

/// Retrying a revoked or under-privileged connection cannot succeed.
fn job_error(error: anyhow::Error) -> JobError {
    use crate::importers::harvest::ApiImportError;
    JobError {
        permanent: ApiImportError::is_permanent(&error),
        error_kind: ApiImportError::job_error_kind(&error).map(error_kind_column),
        error,
    }
}

async fn run_import(
    state: &AppState,
    lease: &JobLease,
    payload: serde_json::Value,
) -> anyhow::Result<(serde_json::Value, i64)> {
    let payload: JobPayload =
        serde_json::from_value(payload).context("invalid durable job payload")?;
    let org_id = lease.org_id();
    match payload {
        JobPayload::HarvestApi { mode, sync } => {
            let cfg = crate::importers::harvest::client_settings::resolve(
                &state.db,
                &state.harvest,
                org_id,
            )
            .await?
            .context("Harvest is not configured")?;
            let currency = sqlx::query_scalar!(
                "SELECT default_currency FROM organizations WHERE id = $1",
                org_id
            )
            .fetch_one(&state.db)
            .await?;
            let report = crate::importers::harvest::run_api_import_with_lease(
                &state.db, org_id, &currency, &cfg, mode, sync, lease,
            )
            .await?;
            crate::importers::harvest::job_report(&report)
        }
        JobPayload::HarvestCsv { mode } => {
            let currency = sqlx::query_scalar!(
                "SELECT default_currency FROM organizations WHERE id = $1",
                org_id
            )
            .fetch_one(&state.db)
            .await?;
            let upload = sqlx::query!(
                "SELECT body FROM horae_job_uploads WHERE job_id = $1 AND org_id = $2",
                lease.id(),
                org_id,
            )
            .fetch_one(&state.db)
            .await?;
            let report = crate::importers::harvest::csv_source::import_body_with_lease(
                &state.db,
                org_id,
                &currency,
                crate::importers::harvest::csv_source::csv_body(upload.body),
                mode,
                Some(lease),
            )
            .await?;
            crate::importers::harvest::job_report(&report)
        }
        #[cfg(test)]
        JobPayload::Synthetic => Ok((serde_json::json!({"synthetic": true}), 0)),
    }
}

#[cfg(test)]
async fn claim(
    pool: &sqlx::PgPool,
    worker_id: &str,
) -> anyhow::Result<Option<horae_jobs::ClaimedJob>> {
    horae_jobs::claim(pool, worker_id).await
}

#[cfg(test)]
async fn execute(
    state: &AppState,
    worker_id: &str,
    job: horae_jobs::ClaimedJob,
) -> anyhow::Result<()> {
    horae_jobs::execute(&state.db, &registry(state), worker_id, job).await
}

#[cfg(test)]
async fn run_worker(state: &AppState, stop: tokio::sync::watch::Receiver<bool>) {
    horae_jobs::run_worker(&state.db, &registry(state), stop).await
}

pub async fn run_claimed(
    pool: &sqlx::PgPool,
    lease: &JobLease,
    stop: tokio::sync::watch::Sender<bool>,
    work: impl Future<Output = anyhow::Result<(serde_json::Value, i64)>>,
) -> anyhow::Result<()> {
    // Boxed: an import's future is large, and nesting it by value in this
    // adapter overflows a test thread's stack in debug builds.
    let work = Box::pin(work);
    let work = async {
        work.await
            .map(|(report, processed_count)| Completed {
                report,
                processed_count,
            })
            .map_err(job_error)
    };
    // Only the delays are read here; the attempt limit is the job's stored one.
    let policy = import_policy(JobPolicy::default());
    horae_jobs::run_claimed(pool, lease, &policy, stop, work).await
}

#[cfg(any(test, feature = "test-support"))]
pub async fn claim_lease_for_test(
    pool: &sqlx::PgPool,
) -> (JobLease, tokio::sync::watch::Sender<bool>) {
    let (ctx, stop) = horae_jobs::claim_for_test(pool).await;
    (JobLease(ctx), stop)
}

#[cfg(test)]
mod tests {
    #[test]
    fn retry_snapshot_does_not_guess_unknown_payloads_or_sources() {
        use crate::models::RetryAvailability;
        let payload = JobPayload::HarvestCsv {
            mode: ImportMode::DryRun,
        };
        for (kind, encoded) in [
            (
                "harvest_csv_import",
                serde_json::json!({"version": 2, "payload": payload}),
            ),
            (
                "harvest_csv_import",
                serde_json::json!({"kind": "HarvestCsv"}),
            ),
            (
                "harvest_csv_import",
                serde_json::to_value(&payload).unwrap(),
            ),
            ("future_import", encode_payload(&payload).unwrap()),
            ("harvest_api_import", encode_payload(&payload).unwrap()),
        ] {
            assert_eq!(
                retry_availability(kind, "failed", encoded, 0, 0, true),
                RetryAvailability::Unknown
            );
        }
        assert_eq!(
            retry_availability(
                "harvest_csv_import",
                "failed",
                encode_payload(&payload).unwrap(),
                0,
                9,
                true
            ),
            RetryAvailability::Available
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn cancelling_a_job_waiting_to_retry_clears_its_error_kind(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Full,
        };
        // One job cancelled while queued, one the reaper finds cancelled
        // after its lease expired; both failed an attempt with `unavailable`.
        let mut ids = Vec::new();
        for key in ["queued-retry", "abandoned-retry"] {
            let id = enqueue(&pool, org_id, &payload, key, JobPolicy::default())
                .await
                .unwrap();
            sqlx::query!(
                "UPDATE horae_jobs SET error_kind = 'unavailable', last_error = 'HTTP 503',
                        available_at = now() + interval '1 hour' WHERE id = $1",
                id
            )
            .execute(&pool)
            .await
            .unwrap();
            ids.push(id);
        }
        assert!(cancel(&pool, org_id, ids[0]).await.unwrap());
        sqlx::query!(
            "UPDATE horae_jobs SET status = 'running', cancellation_requested = true,
                    lease_until = now() - interval '1 minute' WHERE id = $1",
            ids[1]
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(claim(&pool, "reaper-test").await.unwrap().is_none());
        for id in ids {
            let job = status(&pool, org_id, id).await.unwrap().unwrap();
            assert_eq!(job.status, "cancelled");
            assert_eq!(job.error_kind, None, "{id}");
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn retry_snapshot_matches_status_and_list_without_exposing_payload(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let api = enqueue(
            &pool,
            org_id,
            &JobPayload::HarvestApi {
                mode: ImportMode::DryRun,
                sync: SyncScope::Full,
            },
            "projection-api",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let csv = enqueue_csv(
            &pool,
            org_id,
            ImportMode::DryRun,
            b"fixture".to_vec(),
            "projection-csv",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        for id in [api, csv] {
            let snapshot =
                serde_json::to_value(status(&pool, org_id, id).await.unwrap().unwrap()).unwrap();
            assert_eq!(snapshot["retry_availability"], "unavailable_state");
            assert!(cancel(&pool, org_id, id).await.unwrap());
        }
        for snapshot in list(&pool, org_id, 20, None).await.unwrap() {
            assert_eq!(
                Some(snapshot.clone()),
                status(&pool, org_id, snapshot.id).await.unwrap()
            );
            let encoded = serde_json::to_value(snapshot).unwrap();
            assert_eq!(encoded["retry_availability"], "available");
            assert!(encoded.get("payload").is_none());
        }
        sqlx::query!("DELETE FROM horae_job_uploads WHERE job_id = $1", csv)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(status(&pool, org_id, csv).await.unwrap().unwrap()).unwrap()["retry_availability"],
            "missing_upload"
        );
        sqlx::query!("UPDATE harvest_connection_generations SET account_generation = account_generation + 1 WHERE org_id = $1", org_id).execute(&pool).await.unwrap();
        assert_eq!(
            serde_json::to_value(status(&pool, org_id, api).await.unwrap().unwrap()).unwrap()["retry_availability"],
            "previous_account"
        );
        assert_eq!(
            serde_json::to_value(status(&pool, org_id, csv).await.unwrap().unwrap()).unwrap()["retry_availability"],
            "missing_upload"
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn concurrent_csv_resubmission_preserves_identity_after_upload_cleanup(
        pool: sqlx::PgPool,
    ) {
        let org_id = org(&pool).await;
        let submit = || {
            enqueue_csv(
                &pool,
                org_id,
                ImportMode::DryRun,
                b"original".to_vec(),
                "concurrent-csv-key",
                JobPolicy::default(),
            )
        };
        let (first, second) = tokio::join!(submit(), submit());
        let id = first.unwrap();
        assert_eq!(second.unwrap(), id);
        // Simulate upload retention without changing the retained job identity.
        sqlx::query!("DELETE FROM horae_job_uploads WHERE job_id = $1", id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(submit().await.unwrap(), id);
        assert!(
            sqlx::query!("SELECT job_id FROM horae_job_uploads WHERE job_id = $1", id)
                .fetch_optional(&pool)
                .await
                .unwrap()
                .is_none()
        );
        let error = enqueue_csv(
            &pool,
            org_id,
            ImportMode::DryRun,
            b"modified".to_vec(),
            "concurrent-csv-key",
            JobPolicy::default(),
        )
        .await
        .unwrap_err();
        assert!(error.is::<RequestConflict>());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn submission_key_cannot_change_csv_bytes_or_mode(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue_csv(
            &pool,
            org_id,
            ImportMode::DryRun,
            b"original".to_vec(),
            "csv-content-key",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            enqueue_csv(
                &pool,
                org_id,
                ImportMode::DryRun,
                b"original".to_vec(),
                "csv-content-key",
                JobPolicy::default()
            )
            .await
            .unwrap(),
            id
        );
        assert!(
            enqueue_csv(
                &pool,
                org_id,
                ImportMode::DryRun,
                b"modified".to_vec(),
                "csv-content-key",
                JobPolicy::default()
            )
            .await
            .is_err()
        );
        assert!(
            enqueue_csv(
                &pool,
                org_id,
                ImportMode::Commit,
                b"original".to_vec(),
                "csv-content-key",
                JobPolicy::default()
            )
            .await
            .is_err()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn submission_key_cannot_change_api_mode_or_scope(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: horae_core::importers::harvest::types::SyncScope::Full,
        };
        let id = enqueue(
            &pool,
            org_id,
            &payload,
            "api-content-key",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            enqueue(
                &pool,
                org_id,
                &payload,
                "api-content-key",
                JobPolicy::default()
            )
            .await
            .unwrap(),
            id
        );
        for payload in [
            JobPayload::HarvestApi {
                mode: ImportMode::Commit,
                sync: horae_core::importers::harvest::types::SyncScope::Full,
            },
            JobPayload::HarvestApi {
                mode: ImportMode::DryRun,
                sync: horae_core::importers::harvest::types::SyncScope::Incremental,
            },
        ] {
            assert!(
                enqueue(
                    &pool,
                    org_id,
                    &payload,
                    "api-content-key",
                    JobPolicy::default()
                )
                .await
                .is_err()
            );
        }
    }

    use super::*;

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn import_adapters_reject_a_lease_from_another_organization(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let foreign = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "foreign-lease",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let (lease, _stop) = claim_lease_for_test(&pool).await;
        let csv = crate::importers::harvest::csv_source::import_body_with_lease(
            &pool,
            foreign,
            "USD",
            crate::importers::harvest::csv_source::csv_body(Vec::new()),
            ImportMode::Commit,
            Some(&lease),
        )
        .await
        .unwrap_err();
        assert_eq!(csv.to_string(), "job organization mismatch");
        let cfg = crate::config::HarvestConfig {
            client_id: "test".into(),
            client_secret: "test".into(),
            redirect_url: "http://localhost/auth/harvest/callback".into(),
            encryption_key_hex: "11".repeat(32),
        };
        let api = crate::importers::harvest::run_api_import_with_lease(
            &pool,
            foreign,
            "USD",
            &cfg,
            ImportMode::Commit,
            SyncScope::Full,
            &lease,
        )
        .await
        .unwrap_err();
        assert_eq!(api.to_string(), "job organization mismatch");
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().status,
            "running"
        );
        assert!(list(&pool, foreign, 10, None).await.unwrap().is_empty());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn reclaimed_attempt_rejects_stale_results_even_with_the_same_worker_name(
        pool: sqlx::PgPool,
    ) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "stale",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let old = claim(&pool, "same-worker").await.unwrap().unwrap();
        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        let current = claim(&pool, "same-worker").await.unwrap().unwrap();
        assert_ne!(old.claim_token, current.claim_token);
        execute(&state(pool.clone()), "same-worker", old)
            .await
            .unwrap();
        let pending = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(pending.status, "running");
        assert!(pending.report.is_none());
        execute(&state(pool.clone()), "same-worker", current)
            .await
            .unwrap();
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().status,
            "succeeded"
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn cancellation_survives_a_crash_before_the_worker_acknowledges(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "cancelled-crash",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let _old = claim(&pool, "crashed").await.unwrap().unwrap();
        assert!(cancel(&pool, org_id, id).await.unwrap());
        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(claim(&pool, "recovery").await.unwrap().is_none());
        let cancelled = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(cancelled.status, "cancelled");
        assert!(cancelled.finished_at.is_some());
        assert!(cancelled.report.is_none());
        assert!(retry(&pool, org_id, id).await.unwrap());
        assert_eq!(claim(&pool, "recovery").await.unwrap().unwrap().id, id);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn cancellation_waits_for_the_claimed_execution_before_allowing_retry(
        pool: sqlx::PgPool,
    ) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "running-cancel",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let job = claim(&pool, "worker").await.unwrap().unwrap();
        assert!(cancel(&pool, org_id, id).await.unwrap());
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().status,
            "running"
        );
        assert!(!retry(&pool, org_id, id).await.unwrap());
        execute(&state(pool.clone()), "worker", job).await.unwrap();
        let cancelled = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(cancelled.status, "cancelled");
        assert!(cancelled.report.is_none());
        assert!(retry(&pool, org_id, id).await.unwrap());
    }

    #[test]
    fn payload_versions_reject_unversioned_and_unknown_versions() {
        let payload = JobPayload::HarvestCsv {
            mode: ImportMode::DryRun,
        };
        let versioned = encode_payload(&payload).unwrap();
        assert_eq!(versioned["version"], 1);
        assert_eq!(decode_payload(versioned).unwrap().kind(), payload.kind());
        assert!(decode_payload(serde_json::to_value(&payload).unwrap()).is_err());
        assert!(decode_payload(serde_json::json!({"version": 2, "payload": payload})).is_err());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn api_job_keeps_its_policy_across_duplicate_enqueue_and_manual_retry(
        pool: sqlx::PgPool,
    ) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let id = enqueue(
            &pool,
            org_id,
            &payload,
            "policy",
            JobPolicy { max_attempts: 2 },
        )
        .await
        .unwrap();
        assert_eq!(
            enqueue(
                &pool,
                org_id,
                &payload,
                "policy",
                JobPolicy { max_attempts: 9 }
            )
            .await
            .unwrap(),
            id
        );
        // A replacement server's configuration must not alter an existing job.
        let replacement = state(pool.clone()).with_job_policy(JobPolicy { max_attempts: 1 });
        for expected in ["queued", "failed"] {
            let job = claim(&pool, "replacement").await.unwrap().unwrap();
            execute(&replacement, "replacement", job).await.unwrap();
            assert_eq!(
                status(&pool, org_id, id).await.unwrap().unwrap().status,
                expected
            );
            sqlx::query!(
                "UPDATE horae_jobs SET available_at = now() WHERE id = $1",
                id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        assert!(claim(&pool, "replacement").await.unwrap().is_none());
        assert!(retry(&pool, org_id, id).await.unwrap());
        let job = claim(&pool, "replacement").await.unwrap().unwrap();
        execute(&replacement, "replacement", job).await.unwrap();
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().status,
            "queued"
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn csv_job_persists_its_policy_with_the_upload(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue_csv(
            &pool,
            org_id,
            ImportMode::DryRun,
            Vec::new(),
            "csv-policy",
            JobPolicy { max_attempts: 1 },
        )
        .await
        .unwrap();
        assert_eq!(
            enqueue_csv(
                &pool,
                org_id,
                ImportMode::DryRun,
                Vec::new(),
                "csv-policy",
                JobPolicy::default(),
            )
            .await
            .unwrap(),
            id
        );
        let stored = sqlx::query!(
            "SELECT j.max_attempts, u.body FROM horae_jobs j JOIN horae_job_uploads u ON u.job_id = j.id WHERE j.id = $1",
            id
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(stored.max_attempts, 1);
        assert!(stored.body.is_empty());
        let job = claim(&pool, "csv-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "csv-worker", job)
            .await
            .unwrap();
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert_eq!(failed.last_error.as_deref(), Some("the file is empty"));
        assert_eq!(
            failed.report,
            Some(
                serde_json::to_value(ImportReport::new(SourceKind::Csv, ImportMode::DryRun))
                    .unwrap()
            )
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn invalid_policy_cannot_enqueue_a_job_or_upload(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        for max_attempts in [-1, 0, 101, i32::MAX] {
            let policy = JobPolicy { max_attempts };
            assert!(
                enqueue(
                    &pool,
                    org_id,
                    &JobPayload::Synthetic,
                    "invalid-policy",
                    policy
                )
                .await
                .is_err()
            );
            assert!(
                enqueue_csv(
                    &pool,
                    org_id,
                    ImportMode::DryRun,
                    b"csv".to_vec(),
                    "invalid-policy",
                    policy
                )
                .await
                .is_err()
            );
        }
        assert!(list(&pool, org_id, 100, None).await.unwrap().is_empty());
        assert_eq!(
            sqlx::query_scalar!(
                "SELECT count(*) FROM horae_job_uploads WHERE org_id = $1",
                org_id
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            Some(0)
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn expired_final_attempt_fails_without_claiming_again(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "crashed",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let _claimed = claim(&pool, "crashed-worker").await.unwrap().unwrap();
        sqlx::query!(
            "UPDATE horae_jobs SET attempts = max_attempts, lease_until = now() - interval '1 second' WHERE id = $1", id
        ).execute(&pool).await.unwrap();
        let next_id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "next",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            claim(&pool, "replacement").await.unwrap().unwrap().id,
            next_id
        );
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert!(failed.finished_at.is_some());
        assert!(
            failed
                .last_error
                .as_deref()
                .unwrap()
                .contains("attempt limit")
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn final_failure_is_retained_and_manual_retry_gets_a_new_budget(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let id = enqueue(
            &pool,
            org_id,
            &payload,
            "final-error",
            JobPolicy { max_attempts: 1 },
        )
        .await
        .unwrap();
        let job = claim(&pool, "worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "worker", job).await.unwrap();
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert!(failed.finished_at.is_some());
        assert!(retry(&pool, org_id, id).await.unwrap());
        assert_eq!(
            failed.report,
            Some(
                serde_json::to_value(ImportReport::new(
                    SourceKind::HarvestApi,
                    ImportMode::DryRun
                ))
                .unwrap()
            )
        );
        assert_eq!(
            failed.last_error.as_deref(),
            Some("Harvest is not configured")
        );
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().report,
            failed.report
        );
        assert_eq!(claim(&pool, "manual-retry").await.unwrap().unwrap().id, id);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn invalid_stored_payloads_reach_a_terminal_error(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        for (index, payload) in [
            serde_json::json!({"kind": "unknown"}),
            serde_json::json!({"version": 999, "payload": {}}),
            serde_json::json!({"version": 1, "payload": {}}),
        ]
        .into_iter()
        .enumerate()
        {
            let id = enqueue(
                &pool,
                org_id,
                &JobPayload::Synthetic,
                &format!("invalid-{index}"),
                JobPolicy::default(),
            )
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE horae_jobs SET payload = $2, max_attempts = 1 WHERE id = $1",
                id,
                payload
            )
            .execute(&pool)
            .await
            .unwrap();
            let job = claim(&pool, "invalid-worker").await.unwrap().unwrap();
            execute(&state(pool.clone()), "invalid-worker", job)
                .await
                .unwrap();
            let failed = status(&pool, org_id, id).await.unwrap().unwrap();
            assert_eq!(failed.status, "failed");
            assert!(failed.finished_at.is_some());
            assert!(failed.last_error.is_some());
            assert!(claim(&pool, "another-worker").await.unwrap().is_none());
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn cleanup_retains_retryable_uploads_and_deletes_expired_jobs(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue_csv(
            &pool,
            org_id,
            ImportMode::DryRun,
            b"csv".to_vec(),
            "retention",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert!(cancel(&pool, org_id, id).await.unwrap());
        sqlx::query!(
            "UPDATE horae_jobs SET finished_at = now() - interval '2 days' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        cleanup(&pool).await.unwrap();
        assert!(
            retry(&pool, org_id, id).await.unwrap(),
            "upload must survive while the job is retryable"
        );
        assert!(cancel(&pool, org_id, id).await.unwrap());
        sqlx::query!(
            "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        cleanup(&pool).await.unwrap();
        assert!(status(&pool, org_id, id).await.unwrap().is_none());
        assert!(
            sqlx::query!("SELECT job_id FROM horae_job_uploads WHERE job_id = $1", id)
                .fetch_optional(&pool)
                .await
                .unwrap()
                .is_none()
        );
    }

    /// An API import row as servers before the extraction stored it.
    #[test]
    fn an_api_payload_queued_before_the_extraction_still_decodes() {
        let stored = serde_json::json!({
            "version": 1,
            "payload": {"kind": "HarvestApi", "value": {"mode": "DryRun", "sync": "Full"}}
        });
        assert!(matches!(
            decode_payload(stored.clone()).unwrap(),
            JobPayload::HarvestApi {
                mode: ImportMode::DryRun,
                sync: SyncScope::Full
            }
        ));
        assert_eq!(
            sync_scope("harvest_api_import", &stored),
            Some(SyncScope::Full)
        );
    }

    /// A job enqueued by a server from before the queue had its own crate:
    /// the row and payload are written literally, as that server stored them.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn a_job_queued_before_the_extraction_is_claimed_and_finished(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        sqlx::query!(
            "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Sync User')",
            Uuid::now_v7(),
            org_id,
            "known@example.com",
        )
        .execute(&pool)
        .await
        .unwrap();
        let id = Uuid::now_v7();
        let csv = b"Date,Client,Project,Task,Hours,Email\n2026-01-01,Client,Project,Task,0.25,known@example.com\n";
        sqlx::query!(
            r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts)
               VALUES ($1, $2, 'harvest_csv_import',
                       '{"version": 1, "payload": {"kind": "HarvestCsv", "value": {"mode": "DryRun"}}}'::jsonb
                         || jsonb_build_object('upload_sha256', encode(sha256($3::bytea), 'hex')),
                       'before-extraction', 5)"#,
            id,
            org_id,
            csv.as_slice(),
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body)
             VALUES ($1, $2, 'harvest.csv', 'text/csv', $3)",
            id,
            org_id,
            csv.as_slice(),
        )
        .execute(&pool)
        .await
        .unwrap();
        let job = claim(&pool, "upgraded-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "upgraded-worker", job)
            .await
            .unwrap();
        let finished = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(finished.status, "succeeded", "{:?}", finished.last_error);
        assert_eq!(finished.source, Some(SourceKind::Csv));
        assert_eq!(finished.mode, Some(ImportMode::DryRun));
        let report: ImportReport = serde_json::from_value(finished.report.unwrap()).unwrap();
        assert_eq!(report.source, SourceKind::Csv);
    }

    fn state(pool: sqlx::PgPool) -> AppState {
        AppState::new(
            pool,
            std::sync::Arc::new(crate::plugin::PluginRegistry::empty()),
        )
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn stopped_worker_leaves_queued_jobs_unclaimed(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "shutdown",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let (stop, receiver) = tokio::sync::watch::channel(true);
        run_worker(&state(pool.clone()), receiver).await;
        drop(stop);
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().status,
            "queued"
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn synthetic_job_executes_through_claim_and_completion(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "execution",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let job = claim(&pool, "synthetic-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "synthetic-worker", job)
            .await
            .unwrap();
        let completed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(completed.status, "succeeded");
        assert_eq!(
            completed.report,
            Some(serde_json::json!({"synthetic": true}))
        );
        assert!(completed.finished_at.is_some());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn missing_configuration_records_error_and_requeues(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let id = enqueue(
            &pool,
            org_id,
            &payload,
            "unconfigured",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let job = claim(&pool, "unconfigured-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "unconfigured-worker", job)
            .await
            .unwrap();
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "queued");
        assert_eq!(
            failed.last_error.as_deref(),
            Some("Harvest is not configured")
        );
        assert!(
            claim(&pool, "retry-worker").await.unwrap().is_none(),
            "retry must respect backoff"
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn missing_csv_upload_records_error_and_requeues(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::HarvestCsv {
                mode: ImportMode::DryRun,
            },
            "missing-upload",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let job = claim(&pool, "csv-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "csv-worker", job)
            .await
            .unwrap();
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "queued");
        assert!(failed.last_error.is_some());
    }

    #[test]
    fn payload_kind_is_stable_and_round_trips() {
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let encoded = serde_json::to_value(&payload).unwrap();
        let decoded: JobPayload = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.kind(), "harvest_api_import");
    }

    #[test]
    fn synthetic_job_uses_the_same_registered_envelope() {
        let encoded = serde_json::to_value(JobPayload::Synthetic).unwrap();
        let decoded: JobPayload = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.kind(), "synthetic_test_job");
    }

    pub(super) async fn org(pool: &sqlx::PgPool) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
            id
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    async fn failed_csv_report(pool: sqlx::PgPool, mode: ImportMode) {
        let org_id = org(&pool).await;
        sqlx::query!(
            "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Sync User')",
            Uuid::now_v7(),
            org_id,
            "known@example.com",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut csv = String::from("Date,Client,Project,Task,Hours,Email\n");
        // Quarter hours keep 500 rows within one person's 240-hour day.
        csv.push_str(&"2026-01-01,Client,Project,Task,0.25,known@example.com\n".repeat(499));
        csv.push_str("bad-date,Client,Project,Task,0.25,known@example.com\n");
        csv.push_str("2026-01-01,Client,Project,Task,0.25,known@example.com\n");
        let id = enqueue_csv(
            &pool,
            org_id,
            mode,
            csv.into_bytes(),
            "partial-report",
            JobPolicy { max_attempts: 1 },
        )
        .await
        .unwrap();
        sqlx::query!("ALTER TABLE horae_jobs ADD CONSTRAINT reject_job_completion CHECK (status <> 'succeeded')")
            .execute(&pool).await.unwrap();
        let job = claim(&pool, "report-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "report-worker", job)
            .await
            .unwrap();
        let failed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(failed.status, "failed");
        assert!(
            failed
                .last_error
                .as_deref()
                .unwrap()
                .contains("reject_job_completion")
        );
        let partial: ImportReport = serde_json::from_value(
            failed
                .report
                .clone()
                .expect("confirmed batches must have an inspectable report"),
        )
        .unwrap();
        assert_eq!(partial.mode, mode);
        assert_eq!(partial.summary.time_entries.created, 499);
        assert_eq!(partial.summary.time_entries.errored, 1);
        assert_eq!(partial.row_errors.len(), 1);
        assert!(partial.reconciles());
        assert_eq!(list(&pool, org_id, 20, None).await.unwrap()[0], failed);
        let stored = sqlx::query!("SELECT report FROM horae_jobs WHERE id = $1", id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            stored.report, failed.report,
            "the report must commit with its checkpoint"
        );
        // Older workers stored outcomes only in the checkpoint. Read those jobs
        // without exposing the source cursor or private simulation state.
        sqlx::query!("UPDATE horae_jobs SET report = NULL WHERE id = $1", id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            list(&pool, org_id, 20, None).await.unwrap()[0].report,
            failed.report
        );
        sqlx::query!(
            "UPDATE horae_jobs SET finished_at = now() - interval '2 days' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        cleanup(&pool).await.unwrap();
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().report,
            failed.report
        );
        assert!(retry(&pool, org_id, id).await.unwrap());
        assert_eq!(
            status(&pool, org_id, id).await.unwrap().unwrap().report,
            failed.report
        );
        let _crashed = claim(&pool, "crashed-retry").await.unwrap().unwrap();
        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(claim(&pool, "recovery").await.unwrap().is_none());
        let interrupted = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(interrupted.status, "failed");
        assert_eq!(interrupted.report, failed.report);
        assert_eq!(
            interrupted.last_error.as_deref(),
            Some("Job attempt limit reached after interruption")
        );
        assert!(retry(&pool, org_id, id).await.unwrap());
        sqlx::query!("ALTER TABLE horae_jobs DROP CONSTRAINT reject_job_completion")
            .execute(&pool)
            .await
            .unwrap();
        let job = claim(&pool, "retry-worker").await.unwrap().unwrap();
        execute(&state(pool.clone()), "retry-worker", job)
            .await
            .unwrap();
        let completed = status(&pool, org_id, id).await.unwrap().unwrap();
        assert_eq!(completed.status, "succeeded");
        assert!(completed.last_error.is_none());
        let report: ImportReport = serde_json::from_value(completed.report.unwrap()).unwrap();
        assert_eq!(report.summary.time_entries.created, 500);
        assert_eq!(report.summary.time_entries.errored, 1);
        assert_eq!(report.row_errors, partial.row_errors);
        assert_eq!(
            sqlx::query_scalar!(
                "SELECT count(*) FROM time_entries WHERE org_id = $1",
                org_id
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            Some(if mode == ImportMode::Commit { 500 } else { 0 })
        );
        sqlx::query!(
            "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        cleanup(&pool).await.unwrap();
        assert!(status(&pool, org_id, id).await.unwrap().is_none());
        assert!(list(&pool, org_id, 20, None).await.unwrap().is_empty());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn failed_csv_commit_retains_confirmed_report_through_history_and_retry(
        pool: sqlx::PgPool,
    ) {
        failed_csv_report(pool, ImportMode::Commit).await;
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn failed_csv_preview_retains_confirmed_report_through_history_and_retry(
        pool: sqlx::PgPool,
    ) {
        failed_csv_report(pool, ImportMode::DryRun).await;
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn history_paginates_tied_timestamps_and_rejects_foreign_cursors(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let foreign_org = org(&pool).await;
        let mut ids = Vec::new();
        for index in 0..24 {
            ids.push(
                enqueue(
                    &pool,
                    org_id,
                    &JobPayload::Synthetic,
                    &format!("history-{index}"),
                    JobPolicy::default(),
                )
                .await
                .unwrap(),
            );
        }
        sqlx::query!(
            "UPDATE horae_jobs SET created_at = '2000-01-01'::timestamptz WHERE org_id = $1",
            org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        ids.sort_unstable_by(|left, right| right.cmp(left));
        let first = list(&pool, org_id, 20, None).await.unwrap();
        assert_eq!(
            first.iter().map(|job| job.id).collect::<Vec<_>>(),
            ids[..20]
        );
        let second = list(&pool, org_id, 20, Some(first[19].id)).await.unwrap();
        assert_eq!(
            second.iter().map(|job| job.id).collect::<Vec<_>>(),
            ids[20..]
        );
        assert!(
            list(&pool, org_id, 20, Some(ids[23]))
                .await
                .unwrap()
                .is_empty()
        );

        let foreign = enqueue(
            &pool,
            foreign_org,
            &JobPayload::Synthetic,
            "foreign",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert!(
            list(&pool, org_id, 20, Some(foreign))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            list(&pool, org_id, 20, Some(Uuid::now_v7()))
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(list(&pool, org_id, 0, None).await.unwrap().len(), 1);

        // Timestamp ordering must take precedence over UUID order.
        sqlx::query!(
            "UPDATE horae_jobs SET created_at = '2001-01-01'::timestamptz WHERE id = $1",
            ids[23]
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(list(&pool, org_id, 1, None).await.unwrap()[0].id, ids[23]);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn cancellation_and_retry_are_scoped_and_idempotent(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let foreign_org = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let id = enqueue(
            &pool,
            org_id,
            &payload,
            "cancel-retry",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        assert!(status(&pool, foreign_org, id).await.unwrap().is_none());
        assert!(list(&pool, foreign_org, 20, None).await.unwrap().is_empty());
        assert!(!cancel(&pool, foreign_org, id).await.unwrap());
        assert!(cancel(&pool, org_id, id).await.unwrap());
        assert!(!cancel(&pool, org_id, id).await.unwrap());
        assert!(!retry(&pool, foreign_org, id).await.unwrap());
        assert!(retry(&pool, org_id, id).await.unwrap());
        assert!(!retry(&pool, org_id, id).await.unwrap());
        assert!(
            status(&pool, org_id, id)
                .await
                .unwrap()
                .unwrap()
                .phase
                .is_none()
        );
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn retry_refuses_a_job_whose_concurrency_key_another_job_holds(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let keyed = || async {
            let mut tx = pool.begin().await.unwrap();
            let enqueued = horae_jobs::enqueue_in(
                &mut tx,
                horae_jobs::Enqueue {
                    org_id,
                    kind: "synthetic_test_job",
                    payload: serde_json::to_value(JobPayload::Synthetic).unwrap(),
                    idempotency_key: None,
                    concurrency_key: Some("pass"),
                    run_at: None,
                    priority: 0,
                    max_attempts: 1,
                },
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            enqueued
        };
        let ended = keyed().await.id();
        assert!(cancel(&pool, org_id, ended).await.unwrap());
        let holder = keyed().await;
        assert!(matches!(holder, horae_jobs::Enqueued::Created(_)));

        let refused = retry(&pool, org_id, ended).await.unwrap_err();
        assert!(refused.is::<ConcurrencyKeyHeld>(), "{refused:#}");

        assert!(cancel(&pool, org_id, holder.id()).await.unwrap());
        assert!(retry(&pool, org_id, ended).await.unwrap());
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn upload_organization_must_match_its_job(pool: sqlx::PgPool) {
        let owner = org(&pool).await;
        let foreign = org(&pool).await;
        let id = enqueue(
            &pool,
            owner,
            &JobPayload::HarvestCsv {
                mode: ImportMode::DryRun,
            },
            "upload-owner",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let result = sqlx::query!(
            r#"INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body)
           VALUES ($1, $2, 'harvest.csv', 'text/csv', $3)
           ON CONFLICT (job_id) DO NOTHING"#,
            id,
            foreign,
            b"private upload".as_slice(),
        )
        .execute(&pool)
        .await;
        assert!(
            result.is_err(),
            "a foreign organization must not own another organization's upload"
        );
        let error = result.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23503")
        );
        sqlx::query!(
            r#"INSERT INTO horae_job_uploads (job_id, org_id, filename, content_type, body)
           VALUES ($1, $2, 'harvest.csv', 'text/csv', $3)
           ON CONFLICT (job_id) DO NOTHING"#,
            id,
            owner,
            b"owner upload".as_slice(),
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(cancel(&pool, owner, id).await.unwrap());
        assert!(retry(&pool, owner, id).await.unwrap());
    }
}
