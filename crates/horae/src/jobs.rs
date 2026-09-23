//! Durable PostgreSQL-backed background jobs.
//!
//! The queue is intentionally small and application-owned: Horae's UUIDv7 and
//! organization invariants apply to queue rows just like every other table.

use std::time::Duration;

use anyhow::Context;
use horae_core::importers::harvest::types::{ImportMode, ImportReport, SourceKind, SyncScope};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::JobPolicy, state::AppState};

const LEASE: Duration = Duration::from_secs(300);
const POLL: Duration = Duration::from_secs(2);

mod lease;
pub(crate) mod outbox;
pub(crate) mod report;
pub(crate) use lease::JobLease;

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

#[derive(Serialize, Deserialize)]
struct JobEnvelope {
    version: u16,
    payload: JobPayload,
}

fn encode_payload(payload: &JobPayload) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(JobEnvelope {
        version: 1,
        payload: payload.clone(),
    })?)
}

fn decode_payload(value: serde_json::Value) -> anyhow::Result<JobPayload> {
    // Jobs queued before payload versioning use the original tagged enum.
    if value.get("version").is_none() {
        return serde_json::from_value(value).context("invalid legacy durable job payload");
    }
    anyhow::ensure!(
        value.get("version").and_then(serde_json::Value::as_u64) == Some(1),
        "unsupported durable job payload version"
    );
    let envelope: JobEnvelope =
        serde_json::from_value(value).context("invalid durable job payload")?;
    Ok(envelope.payload)
}

/// Insert an event in the same transaction as the state change that produced
/// it. Consumers can claim undelivered rows independently of the job worker.
pub async fn enqueue_outbox(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    event_kind: &str,
    payload: serde_json::Value,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        r#"INSERT INTO horae_outbox (id, org_id, event_kind, payload)
           VALUES ($1, $2, $3, $4)"#,
        id,
        org_id,
        event_kind,
        payload,
    )
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

#[derive(Debug, Clone)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub org_id: Uuid,
    pub event_kind: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
    pub claim_token: Uuid,
}

/// Claim one event of the consumer's kind. Moving `available_at` acts as a short
/// lease, so a crashed consumer can safely retry it later without taking events
/// intended for another consumer.
pub async fn claim_outbox(
    pool: &sqlx::PgPool,
    event_kind: &str,
) -> anyhow::Result<Option<OutboxEvent>> {
    let claim_token = Uuid::now_v7();
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id FROM horae_outbox
              WHERE delivered_at IS NULL AND failed_at IS NULL AND available_at <= now() AND event_kind = $2
              ORDER BY available_at, created_at
              FOR UPDATE SKIP LOCKED LIMIT 1
           )
           UPDATE horae_outbox o
              SET attempts = o.attempts + 1,
                  claim_token = $1,
                  available_at = now() + interval '5 minutes'
             FROM candidate
            WHERE o.id = candidate.id
        RETURNING o.id, o.org_id, o.event_kind, o.payload, o.attempts,
                  o.claim_token as "claim_token!""#,
        claim_token,
        event_kind,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| OutboxEvent {
        id: r.id,
        org_id: r.org_id,
        event_kind: r.event_kind,
        payload: r.payload,
        attempts: r.attempts,
        claim_token: r.claim_token,
    }))
}

pub async fn mark_outbox_delivered(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_outbox SET delivered_at = now(), last_error = NULL, claim_token = NULL
            WHERE id = $1 AND org_id = $2 AND claim_token = $3
              AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()"#,
        event.id,
        event.org_id,
        event.claim_token,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn mark_outbox_failed(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
    error: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_outbox
              SET available_at = now() + LEAST(power(2::double precision, LEAST(attempts, 9)), 300)::int * interval '1 second',
                  last_error = $4, claim_token = NULL
            WHERE id = $1 AND org_id = $2 AND claim_token = $3
              AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()"#,
        event.id,
        event.org_id,
        event.claim_token,
        error,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Retain a terminal failure without acknowledging delivery or permitting another claim.
pub(crate) async fn stop_outbox_delivery(
    pool: &sqlx::PgPool,
    event: &OutboxEvent,
    error: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "UPDATE horae_outbox SET failed_at = now(), last_error = $4, claim_token = NULL
         WHERE id = $1 AND org_id = $2 AND claim_token = $3
           AND delivered_at IS NULL AND failed_at IS NULL AND available_at > now()",
        event.id,
        event.org_id,
        event.claim_token,
        error,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
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

pub async fn request_exists(
    pool: &sqlx::PgPool,
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
    .fetch_one(pool)
    .await?)
}

#[cfg(test)]
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
    let id = enqueue_in(&mut tx, org_id, payload, idempotency_key, policy, 0).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn enqueue_api(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
) -> anyhow::Result<Uuid> {
    use crate::importers::harvest::account_switch::{self, ChangeError};
    let mut tx = pool.begin().await?;
    let current = account_switch::gate(&mut tx, org_id).await?;
    anyhow::ensure!(
        generation == current.account_generation,
        ChangeError::OldImport
    );
    let existing = sqlx::query_scalar!("SELECT id FROM horae_jobs WHERE org_id = $1 AND kind = 'harvest_api_import' AND idempotency_key = $2", org_id, idempotency_key)
        .fetch_optional(&mut *tx).await?;
    if existing.is_none() {
        let connection = account_switch::status(&mut *tx, org_id, true).await?;
        anyhow::ensure!(connection.connected, ChangeError::NotConnected);
    }
    let id = enqueue_in(
        &mut tx,
        org_id,
        payload,
        idempotency_key,
        policy,
        generation,
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

async fn enqueue_in(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
    policy: JobPolicy,
    generation: i64,
) -> anyhow::Result<Uuid> {
    let policy = policy.validate()?;
    let id = Uuid::now_v7();
    let encoded = encode_payload(payload)?;
    let report = payload.initial_report()?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts, report, account_generation)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(RequestConflict)?;
    Ok(row.id)
}

pub async fn enqueue_csv(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    idempotency_key: &str,
    policy: JobPolicy,
) -> anyhow::Result<Uuid> {
    let policy = policy.validate()?;
    let id = Uuid::now_v7();
    let payload = JobPayload::HarvestCsv { mode };
    let report = payload.initial_report()?;
    let payload = encode_payload(&payload)?;
    let mut tx = pool.begin().await?;
    crate::importers::harvest::account_switch::gate(&mut tx, org_id).await?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, max_attempts, report)
           VALUES ($1, $2, 'harvest_csv_import',
                   $3::jsonb || jsonb_build_object('upload_sha256', encode(sha256($7::bytea), 'hex')),
                   $4, $5, $6)
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
    )
    .fetch_optional(&mut *tx)
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
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(row.id)
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

pub async fn status(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<crate::models::JobStatus>> {
    let row = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  COALESCE(checkpoint->'report', report) AS report, last_error,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>",
                  payload, account_generation,
                  COALESCE((SELECT g.account_generation FROM harvest_connection_generations g
                            WHERE g.org_id = horae_jobs.org_id), 0) AS "current_generation!",
                  EXISTS(SELECT 1 FROM horae_job_uploads u
                         WHERE u.job_id = horae_jobs.id AND u.org_id = horae_jobs.org_id) AS "has_upload!"
             FROM horae_jobs
            WHERE id = $1 AND org_id = $2"#,
        id,
        org_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| crate::models::JobStatus {
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
        created_at: r.created_at,
        finished_at: r.finished_at,
    }))
}

pub async fn list(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    limit: i64,
    before: Option<Uuid>,
) -> anyhow::Result<Vec<crate::models::JobStatus>> {
    let rows = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  COALESCE(checkpoint->'report', report) AS report, last_error,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>",
                  payload, account_generation,
                  COALESCE((SELECT g.account_generation FROM harvest_connection_generations g
                            WHERE g.org_id = horae_jobs.org_id), 0) AS "current_generation!",
                  EXISTS(SELECT 1 FROM horae_job_uploads u
                         WHERE u.job_id = horae_jobs.id AND u.org_id = horae_jobs.org_id) AS "has_upload!"
             FROM horae_jobs
            WHERE org_id = $1
              AND ($3::uuid IS NULL OR (created_at, id) < (
                  SELECT created_at, id FROM horae_jobs WHERE id = $3 AND org_id = $1
              ))
            ORDER BY created_at DESC, id DESC
            LIMIT $2"#,
        org_id,
        limit.clamp(1, 100),
        before,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| crate::models::JobStatus {
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
            created_at: r.created_at,
            finished_at: r.finished_at,
        })
        .collect())
}

pub async fn cancel(pool: &sqlx::PgPool, org_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_jobs
              SET cancellation_requested = true,
                  status = CASE WHEN status = 'queued' THEN 'cancelled' ELSE status END,
                  phase = 'cancelling',
                  finished_at = CASE WHEN status = 'queued' THEN now() ELSE finished_at END,
                  updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('queued', 'running')
              AND NOT cancellation_requested"#,
        id,
        org_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn retry(pool: &sqlx::PgPool, org_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
    let mut tx = pool.begin().await?;
    let current = crate::importers::harvest::account_switch::gate(&mut tx, org_id).await?;
    let generation = sqlx::query_scalar!("SELECT account_generation FROM horae_jobs WHERE id = $1 AND org_id = $2 AND kind = 'harvest_api_import'", id, org_id)
        .fetch_optional(&mut *tx).await?;
    anyhow::ensure!(
        generation.is_none_or(|generation| generation == current.account_generation),
        crate::importers::harvest::account_switch::ChangeError::OldImport
    );
    let result = sqlx::query!(
        r#"UPDATE horae_jobs
              SET status = 'queued', available_at = now(), lease_until = NULL,
                  attempts = 0, worker_id = NULL, last_error = NULL, phase = NULL,
                  claim_token = NULL, cancellation_requested = false,
                  finished_at = NULL, updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('failed', 'cancelled')
              AND (kind <> 'harvest_csv_import' OR EXISTS (
                  SELECT 1 FROM horae_job_uploads u WHERE u.job_id = horae_jobs.id AND u.org_id = $2
              ))"#,
        id,
        org_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result.rows_affected() == 1)
}

pub struct Worker {
    stop: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl Worker {
    pub(crate) fn new(
        stop: tokio::sync::watch::Sender<bool>,
        task: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self { stop, task }
    }

    pub fn stop_sender(&self) -> tokio::sync::watch::Sender<bool> {
        self.stop.clone()
    }

    pub fn request_shutdown(&self) {
        self.stop.send_replace(true);
    }

    /// Drain the current job before dropping the runtime. If the deadline is
    /// exceeded, await task cancellation so its lease can expire without a
    /// detached worker continuing to perform writes.
    pub async fn shutdown(mut self, grace: Duration) -> anyhow::Result<()> {
        self.request_shutdown();
        match tokio::time::timeout(grace, &mut self.task).await {
            Ok(result) => result.context("durable worker stopped unexpectedly"),
            Err(_) => {
                self.task.abort();
                match (&mut self.task).await {
                    Err(error) if error.is_cancelled() => {
                        tracing::warn!("durable worker drain timed out; active lease will expire");
                        Ok(())
                    }
                    result => result.context("durable worker stopped unexpectedly"),
                }
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        self.task.abort();
    }
}

pub fn spawn(state: &AppState) -> Worker {
    let (stop, receiver) = tokio::sync::watch::channel(false);
    let state = state.clone();
    let task = tokio::spawn(async move { run_worker(&state, receiver).await });
    Worker { stop, task }
}

async fn run_worker(state: &AppState, mut stop: tokio::sync::watch::Receiver<bool>) {
    let worker_id = Uuid::now_v7().to_string();
    while !*stop.borrow() && stop.has_changed().is_ok() {
        if let Err(error) = cleanup(&state.db).await {
            tracing::warn!(%error, "durable job cleanup failed");
        }
        if *stop.borrow() || stop.has_changed().is_err() {
            break;
        }
        // Finish an in-flight claim instead of dropping a query which may
        // already have assigned a lease in PostgreSQL.
        match claim(&state.db, &worker_id).await {
            Ok(Some(job)) => {
                if let Err(error) = execute(state, &worker_id, job).await {
                    tracing::warn!(%error, "durable job failed");
                }
                continue;
            }
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "durable job poll failed"),
        }
        tokio::select! {
            _ = stop.changed() => break,
            _ = tokio::time::sleep(POLL) => {}
        }
    }
}

async fn cleanup(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        r#"DELETE FROM horae_job_uploads u
             USING horae_jobs j
            WHERE u.job_id = j.id
              AND j.status = 'succeeded'
              AND j.finished_at < now() - interval '1 day'"#
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        r#"DELETE FROM horae_jobs
            WHERE status IN ('succeeded', 'failed', 'cancelled')
              AND finished_at < now() - interval '30 days'"#
    )
    .execute(pool)
    .await?;
    Ok(())
}

struct ClaimedJob {
    id: Uuid,
    org_id: Uuid,
    payload: serde_json::Value,
    claim_token: Uuid,
}

async fn claim(pool: &sqlx::PgPool, worker_id: &str) -> anyhow::Result<Option<ClaimedJob>> {
    sqlx::query!(
        r#"UPDATE horae_jobs
              SET status = CASE WHEN cancellation_requested THEN 'cancelled' ELSE 'failed' END,
                  lease_until = NULL, worker_id = NULL, claim_token = NULL,
                  finished_at = now(), updated_at = now(),
                  last_error = CASE WHEN cancellation_requested THEN NULL
                      ELSE COALESCE(last_error, 'Job attempt limit reached after interruption') END
            WHERE (attempts >= max_attempts OR cancellation_requested)
              AND (status = 'queued' OR (status = 'running' AND lease_until < now()))"#
    )
    .execute(pool)
    .await?;
    let claim_token = Uuid::now_v7();
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id
               FROM horae_jobs
              WHERE attempts < max_attempts AND NOT cancellation_requested
                AND ((status = 'queued' AND available_at <= now())
                  OR (status = 'running' AND lease_until < now()))
              ORDER BY available_at, created_at
              FOR UPDATE SKIP LOCKED
              LIMIT 1
           )
           UPDATE horae_jobs j
              SET status = 'running', attempts = j.attempts + 1,
                  lease_until = now() + $1::int * interval '1 second', worker_id = $2,
                  claim_token = $3,
                  started_at = COALESCE(j.started_at, now()), updated_at = now()
             FROM candidate
            WHERE j.id = candidate.id
        RETURNING j.id, j.org_id, j.payload"#,
        LEASE.as_secs() as i32,
        worker_id,
        claim_token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| ClaimedJob {
        id: r.id,
        org_id: r.org_id,
        payload: r.payload,
        claim_token,
    }))
}

async fn execute(state: &AppState, worker_id: &str, job: ClaimedJob) -> anyhow::Result<()> {
    let (stop, receiver) = tokio::sync::watch::channel(false);
    let lease = JobLease {
        id: job.id,
        org_id: job.org_id,
        token: job.claim_token,
        stop: receiver,
    };
    let work = async {
        let started = sqlx::query!(
            r#"UPDATE horae_jobs SET phase = 'importing', updated_at = now()
                WHERE id = $1 AND worker_id = $2 AND claim_token = $3
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            job.id,
            worker_id,
            job.claim_token,
        )
        .execute(&state.db)
        .await?;
        if started.rows_affected() != 1 {
            return Err(lease::Interrupted.into());
        }
        let payload = decode_payload(job.payload.clone())?;
        match &payload {
            JobPayload::HarvestApi { mode, sync } => {
                let cfg = state.harvest.clone().context("Harvest is not configured")?;
                let currency = sqlx::query_scalar!(
                    "SELECT default_currency FROM organizations WHERE id = $1",
                    job.org_id
                )
                .fetch_one(&state.db)
                .await?;
                let report = crate::importers::harvest::run_api_import_with_lease(
                    &state.db, job.org_id, &currency, &cfg, *mode, *sync, &lease,
                )
                .await?;
                crate::importers::harvest::job_report(&report)
            }
            JobPayload::HarvestCsv { mode } => {
                let currency = sqlx::query_scalar!(
                    "SELECT default_currency FROM organizations WHERE id = $1",
                    job.org_id
                )
                .fetch_one(&state.db)
                .await?;
                let upload = sqlx::query!(
                    "SELECT body FROM horae_job_uploads WHERE job_id = $1 AND org_id = $2",
                    job.id,
                    job.org_id,
                )
                .fetch_one(&state.db)
                .await?;
                let report = crate::importers::harvest::csv_source::import_body_with_lease(
                    &state.db,
                    job.org_id,
                    &currency,
                    axum::body::Body::from(upload.body),
                    *mode,
                    Some(&lease),
                )
                .await?;
                crate::importers::harvest::job_report(&report)
            }
            #[cfg(test)]
            JobPayload::Synthetic => Ok((serde_json::json!({"synthetic": true}), 0)),
        }
    };
    run_claimed(&state.db, &lease, stop, work).await
}

pub(crate) async fn run_claimed(
    pool: &sqlx::PgPool,
    lease: &JobLease,
    stop: tokio::sync::watch::Sender<bool>,
    work: impl Future<Output = anyhow::Result<(serde_json::Value, i64)>>,
) -> anyhow::Result<()> {
    tokio::pin!(work);
    // No detached heartbeat survives an aborted worker. On cancellation, let
    // the importer join its producer and flush rollback before recording ack.
    let result = tokio::select! {
        result = &mut work => result,
        monitor = lease.monitor(pool) => {
            tracing::info!(job_id = %lease.id, error = ?monitor.err(), "stopping durable execution");
            stop.send_replace(true);
            work.await
        }
    };
    match result {
        Ok((report, processed_count)) => {
            let completed = {
                let mut connection = pool.acquire().await?;
                lease
                    .complete(&mut connection, &report, processed_count)
                    .await?
            };
            if !completed {
                // Committing imports already completed atomically with their
                // writes. Otherwise a concurrent cancellation still needs ack.
                fail_claimed(pool, lease, &lease::Interrupted.into()).await?;
            }
        }
        Err(error) => fail_claimed(pool, lease, &error).await?,
    }
    Ok(())
}

async fn fail_claimed(
    pool: &sqlx::PgPool,
    lease: &JobLease,
    error: &anyhow::Error,
) -> anyhow::Result<()> {
    sqlx::query!(
                r#"UPDATE horae_jobs
                      SET status = CASE WHEN cancellation_requested THEN 'cancelled'
                                        WHEN attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
                          available_at = now() + LEAST(power(2::double precision, LEAST(attempts, 9)), 300)::int * interval '1 second',
                          finished_at = CASE WHEN cancellation_requested OR attempts >= max_attempts THEN now() ELSE NULL END,
                          lease_until = NULL, worker_id = NULL, claim_token = NULL,
                          last_error = CASE WHEN cancellation_requested THEN NULL ELSE $1 END,
                          updated_at = now()
                    WHERE id = $2 AND org_id = $3 AND claim_token = $4 AND status = 'running'
                      AND lease_until > clock_timestamp()"#,
                error.to_string(),
                lease.id, lease.org_id, lease.token,
    ).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn claim_lease_for_test(
    pool: &sqlx::PgPool,
) -> (JobLease, tokio::sync::watch::Sender<bool>) {
    let job = claim(pool, "test-worker").await.unwrap().unwrap();
    let (stop, receiver) = tokio::sync::watch::channel(false);
    (
        JobLease {
            id: job.id,
            org_id: job.org_id,
            token: job.claim_token,
            stop: receiver,
        },
        stop,
    )
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
                serde_json::to_value(&payload).unwrap(),
                0,
                9,
                true
            ),
            RetryAvailability::Available
        );
    }

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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
            axum::body::Body::empty(),
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

    #[sqlx::test]
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

    #[sqlx::test]
    #[serial_test::serial]
    async fn expired_lease_cannot_renew_or_commit_using_an_old_transaction_timestamp(
        pool: sqlx::PgPool,
    ) {
        let org_id = org(&pool).await;
        let id = enqueue(
            &pool,
            org_id,
            &JobPayload::Synthetic,
            "expired-transaction",
            JobPolicy::default(),
        )
        .await
        .unwrap();
        let (lease, _stop) = claim_lease_for_test(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        sqlx::query!("SELECT now()")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(
            !lease
                .complete(&mut tx, &serde_json::json!({}), 0)
                .await
                .unwrap()
        );
        assert!(lease.renew(&pool).await.is_err());
        tx.rollback().await.unwrap();
    }

    #[sqlx::test]
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

    #[sqlx::test]
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
    fn payload_versions_preserve_legacy_jobs_and_reject_unknown_versions() {
        let payload = JobPayload::HarvestCsv {
            mode: ImportMode::DryRun,
        };
        let versioned = encode_payload(&payload).unwrap();
        assert_eq!(versioned["version"], 1);
        assert_eq!(decode_payload(versioned).unwrap().kind(), payload.kind());
        assert_eq!(
            decode_payload(serde_json::to_value(&payload).unwrap())
                .unwrap()
                .kind(),
            payload.kind()
        );
        assert!(decode_payload(serde_json::json!({"version": 2, "payload": payload})).is_err());
    }

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    fn state(pool: sqlx::PgPool) -> AppState {
        AppState::new(
            pool,
            std::sync::Arc::new(crate::plugin::PluginRegistry::empty()),
        )
    }

    #[tokio::test]
    async fn shutdown_waits_for_in_flight_work() {
        let (stop, mut receiver) = tokio::sync::watch::channel(false);
        let (finish, finished) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            receiver.changed().await.unwrap();
            finished.await.unwrap();
        });
        let worker = Worker { stop, task };
        let shutdown = worker.shutdown(Duration::from_secs(5));
        tokio::pin!(shutdown);
        assert!(futures_util::poll!(&mut shutdown).is_pending());
        finish.send(()).unwrap();
        shutdown.await.unwrap();
    }

    #[tokio::test]
    async fn shutdown_deadline_drops_and_joins_in_flight_work() {
        let (stop, _receiver) = tokio::sync::watch::channel(false);
        let (started, running) = tokio::sync::oneshot::channel();
        let (completion, dropped) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let _completion = completion;
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        running.await.unwrap();
        Worker { stop, task }
            .shutdown(Duration::ZERO)
            .await
            .unwrap();
        assert!(
            dropped.await.is_err(),
            "shutdown returned before dropping the task"
        );
    }

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
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
        csv.push_str(&"2026-01-01,Client,Project,Task,1,known@example.com\n".repeat(499));
        csv.push_str("bad-date,Client,Project,Task,1,known@example.com\n");
        csv.push_str("2026-01-01,Client,Project,Task,1,known@example.com\n");
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

    #[sqlx::test]
    #[serial_test::serial]
    async fn failed_csv_commit_retains_confirmed_report_through_history_and_retry(
        pool: sqlx::PgPool,
    ) {
        failed_csv_report(pool, ImportMode::Commit).await;
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn failed_csv_preview_retains_confirmed_report_through_history_and_retry(
        pool: sqlx::PgPool,
    ) {
        failed_csv_report(pool, ImportMode::DryRun).await;
    }

    #[sqlx::test]
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

    #[sqlx::test]
    async fn claims_are_atomic_and_expired_leases_recover(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let first = enqueue(&pool, org_id, &payload, "first", JobPolicy::default())
            .await
            .unwrap();
        let second = enqueue(&pool, org_id, &payload, "second", JobPolicy::default())
            .await
            .unwrap();

        let (left, right) = tokio::join!(claim(&pool, "worker-a"), claim(&pool, "worker-b"));
        let left = left.unwrap().unwrap();
        let right = right.unwrap().unwrap();
        assert_ne!(left.id, right.id);
        assert_eq!(
            std::collections::HashSet::from([left.id, right.id]),
            std::collections::HashSet::from([first, second])
        );

        sqlx::query!(
            "UPDATE horae_jobs SET lease_until = now() - interval '1 second' WHERE id = $1",
            left.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let recovered = claim(&pool, "worker-c").await.unwrap().unwrap();
        assert_eq!(recovered.id, left.id);
    }

    #[sqlx::test]
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

    #[sqlx::test]
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

    #[sqlx::test]
    #[serial_test::serial]
    async fn outbox_claims_only_the_requested_event_kind(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let mail_id = enqueue_outbox(&mut tx, org_id, "budget_email", serde_json::json!({}))
            .await
            .unwrap();
        let project_id = enqueue_outbox(&mut tx, org_id, "project_created", serde_json::json!({}))
            .await
            .unwrap();
        tx.commit().await.unwrap();

        assert!(claim_outbox(&pool, "unknown").await.unwrap().is_none());
        let project = claim_outbox(&pool, "project_created")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.id, project_id);
        assert_eq!(project.event_kind, "project_created");
        assert_eq!(project.attempts, 1);
        assert!(
            claim_outbox(&pool, "project_created")
                .await
                .unwrap()
                .is_none()
        );
        let mail = claim_outbox(&pool, "budget_email").await.unwrap().unwrap();
        assert_eq!(mail.id, mail_id);
        assert_eq!(
            mail.attempts, 1,
            "Other consumers must not touch this event's lease"
        );
        assert!(mark_outbox_delivered(&pool, &project).await.unwrap());
        assert!(mark_outbox_delivered(&pool, &mail).await.unwrap());
        assert!(
            claim_outbox(&pool, "project_created")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn outbox_same_kind_consumers_cannot_claim_the_same_lease(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let id = enqueue_outbox(&mut tx, org_id, "project_created", serde_json::json!({}))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let (first, second) = tokio::join!(
            claim_outbox(&pool, "project_created"),
            claim_outbox(&pool, "project_created"),
        );
        let claimed: Vec<_> = [first.unwrap(), second.unwrap()]
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, id);
        assert_eq!(claimed[0].attempts, 1);
    }

    #[sqlx::test]
    async fn outbox_delivery_is_idempotent(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let id = enqueue_outbox(
            &mut tx,
            org_id,
            "jobs.test",
            serde_json::json!({"ok": true}),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let event = claim_outbox(&pool, "jobs.test").await.unwrap().unwrap();
        assert_eq!(event.id, id);
        assert!(mark_outbox_delivered(&pool, &event).await.unwrap());
        assert!(!mark_outbox_delivered(&pool, &event).await.unwrap());
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn outbox_rollback_does_not_publish_an_event(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        enqueue_outbox(&mut tx, org_id, "rollback", serde_json::json!({}))
            .await
            .unwrap();
        assert!(claim_outbox(&pool, "rollback").await.unwrap().is_none());
        tx.rollback().await.unwrap();
        assert!(claim_outbox(&pool, "rollback").await.unwrap().is_none());
    }

    #[sqlx::test]
    #[serial_test::serial]
    async fn outbox_stale_claims_cannot_acknowledge_or_reschedule_delivery(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let mut tx = pool.begin().await.unwrap();
        let id = enqueue_outbox(&mut tx, org_id, "delivery", serde_json::json!({}))
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let original = claim_outbox(&pool, "delivery").await.unwrap().unwrap();
        assert!(claim_outbox(&pool, "delivery").await.unwrap().is_none());
        sqlx::query!(
            "UPDATE horae_outbox SET available_at = now() - interval '1 second' WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(!mark_outbox_delivered(&pool, &original).await.unwrap());
        let replacement = claim_outbox(&pool, "delivery").await.unwrap().unwrap();
        assert_ne!(original.claim_token, replacement.claim_token);
        assert_eq!(replacement.attempts, 2);
        assert!(!mark_outbox_delivered(&pool, &original).await.unwrap());
        assert!(
            !mark_outbox_failed(&pool, &original, "stale failure")
                .await
                .unwrap()
        );
        let mut foreign = replacement.clone();
        foreign.org_id = org(&pool).await;
        assert!(!mark_outbox_delivered(&pool, &foreign).await.unwrap());
        assert!(
            !mark_outbox_failed(&pool, &foreign, "foreign failure")
                .await
                .unwrap()
        );
        assert!(
            mark_outbox_failed(&pool, &replacement, "delivery failed")
                .await
                .unwrap()
        );
        assert!(!mark_outbox_delivered(&pool, &replacement).await.unwrap());
        assert!(
            !mark_outbox_failed(&pool, &replacement, "duplicate failure")
                .await
                .unwrap()
        );
        assert!(
            claim_outbox(&pool, "delivery").await.unwrap().is_none(),
            "failure must back off"
        );
        let failure = sqlx::query!(
            "SELECT last_error, attempts FROM horae_outbox WHERE id = $1",
            id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(failure.last_error.as_deref(), Some("delivery failed"));
        assert_eq!(failure.attempts, 2);
    }
}
