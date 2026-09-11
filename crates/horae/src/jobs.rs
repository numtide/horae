//! Durable PostgreSQL-backed background jobs.
//!
//! The queue is intentionally small and application-owned: Horae's UUIDv7 and
//! organization invariants apply to queue rows just like every other table.

use std::time::Duration;

use anyhow::Context;
use horae_core::importers::harvest::types::{ImportMode, SyncScope};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

const LEASE: Duration = Duration::from_secs(300);
const POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum JobPayload {
    HarvestApi { mode: ImportMode, sync: SyncScope },
    HarvestCsv { mode: ImportMode },
}

/// Insert an event in the same transaction as the state change that produced
/// it. Consumers can claim undelivered rows independently of the job worker.
#[allow(dead_code)]
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

#[derive(Debug)]
#[allow(dead_code)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub org_id: Uuid,
    pub event_kind: String,
    pub payload: serde_json::Value,
    pub attempts: i32,
}

/// Claim one event for delivery. Moving `available_at` acts as a short lease,
/// so a crashed consumer can safely retry it later.
#[allow(dead_code)]
pub async fn claim_outbox(pool: &sqlx::PgPool) -> anyhow::Result<Option<OutboxEvent>> {
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id FROM horae_outbox
              WHERE delivered_at IS NULL AND available_at <= now()
              ORDER BY available_at, created_at
              FOR UPDATE SKIP LOCKED LIMIT 1
           )
           UPDATE horae_outbox o
              SET attempts = o.attempts + 1,
                  available_at = now() + interval '5 minutes'
             FROM candidate
            WHERE o.id = candidate.id
        RETURNING o.id, o.org_id, o.event_kind, o.payload, o.attempts"#
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| OutboxEvent {
        id: r.id,
        org_id: r.org_id,
        event_kind: r.event_kind,
        payload: r.payload,
        attempts: r.attempts,
    }))
}

#[allow(dead_code)]
pub async fn mark_outbox_delivered(pool: &sqlx::PgPool, id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "UPDATE horae_outbox SET delivered_at = now(), last_error = NULL WHERE id = $1 AND delivered_at IS NULL",
        id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[allow(dead_code)]
pub async fn mark_outbox_failed(
    pool: &sqlx::PgPool,
    id: Uuid,
    error: &str,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_outbox
              SET available_at = now() + LEAST(power(2::double precision, attempts), 300)::int * interval '1 second',
                  last_error = $2
            WHERE id = $1 AND delivered_at IS NULL"#,
        id,
        error,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

impl JobPayload {
    fn kind(&self) -> &'static str {
        match self {
            Self::HarvestApi { .. } => "harvest_api_import",
            Self::HarvestCsv { .. } => "harvest_csv_import",
        }
    }
}

pub async fn enqueue(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    payload: &JobPayload,
    idempotency_key: &str,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    let encoded = serde_json::to_value(payload)?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (org_id, kind, idempotency_key)
           DO UPDATE SET updated_at = now()
           RETURNING id"#,
        id,
        org_id,
        payload.kind(),
        encoded,
        idempotency_key,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.id)
}

pub async fn enqueue_csv(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    mode: ImportMode,
    body: Vec<u8>,
    idempotency_key: &str,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    let payload = serde_json::to_value(JobPayload::HarvestCsv { mode })?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query!(
        r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key)
           VALUES ($1, $2, 'harvest_csv_import', $3, $4)
           ON CONFLICT (org_id, kind, idempotency_key)
           DO UPDATE SET updated_at = now()
           RETURNING id"#,
        id,
        org_id,
        payload,
        idempotency_key,
    )
    .fetch_one(&mut *tx)
    .await?;
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
    tx.commit().await?;
    Ok(row.id)
}

pub async fn status(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<crate::models::JobStatus>> {
    let row = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  report, last_error,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>"
             FROM horae_jobs
            WHERE id = $1 AND org_id = $2"#,
        id,
        org_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| crate::models::JobStatus {
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
) -> anyhow::Result<Vec<crate::models::JobStatus>> {
    let rows = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  report, last_error,
                  created_at as "created_at!: chrono::DateTime<chrono::Utc>",
                  finished_at as "finished_at: chrono::DateTime<chrono::Utc>"
             FROM horae_jobs
            WHERE org_id = $1
            ORDER BY created_at DESC
            LIMIT $2"#,
        org_id,
        limit.clamp(1, 100),
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| crate::models::JobStatus {
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
              SET status = 'cancelled', lease_until = NULL, worker_id = NULL,
                  finished_at = now(), updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('queued', 'running')"#,
        id,
        org_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn retry(pool: &sqlx::PgPool, org_id: Uuid, id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_jobs
              SET status = 'queued', available_at = now(), lease_until = NULL,
                  worker_id = NULL, last_error = NULL, finished_at = NULL, updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('failed', 'cancelled')"#,
        id,
        org_id,
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub fn spawn(state: &'static AppState) -> tokio::sync::watch::Sender<bool> {
    let (shutdown, mut shutdown_rx) = tokio::sync::watch::channel(false);
    tokio::spawn(async move {
        let worker_id = Uuid::now_v7().to_string();
        loop {
            if *shutdown_rx.borrow() {
                break;
            }
            if let Err(error) = cleanup(&state.db).await {
                tracing::warn!(%error, "durable job cleanup failed");
            }
            tokio::select! {
                _ = shutdown_rx.changed() => break,
                result = claim(&state.db, &worker_id) => {
                    match result {
                        Ok(Some(job)) => {
                            if let Err(error) = execute(state, &worker_id, job).await {
                                tracing::warn!(%error, "durable job failed");
                            }
                        }
                        Ok(None) => {
                            tokio::select! {
                                _ = shutdown_rx.changed() => break,
                                _ = tokio::time::sleep(POLL) => {}
                            }
                        }
                        Err(error) => {
                            tracing::warn!(%error, "durable job poll failed");
                            tokio::select! {
                                _ = shutdown_rx.changed() => break,
                                _ = tokio::time::sleep(POLL) => {}
                            }
                        }
                    }
                }
            }
        }
    });
    shutdown
}

async fn cleanup(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    sqlx::query!(
        r#"DELETE FROM horae_job_uploads u
             USING horae_jobs j
            WHERE u.job_id = j.id
              AND j.status IN ('succeeded', 'failed', 'cancelled')
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
    payload: JobPayload,
}

async fn claim(pool: &sqlx::PgPool, worker_id: &str) -> anyhow::Result<Option<ClaimedJob>> {
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id
               FROM horae_jobs
              WHERE (status = 'queued' AND available_at <= now())
                 OR (status = 'running' AND lease_until < now())
              ORDER BY available_at, created_at
              FOR UPDATE SKIP LOCKED
              LIMIT 1
           )
           UPDATE horae_jobs j
              SET status = 'running', attempts = j.attempts + 1,
                  lease_until = now() + $1::int * interval '1 second', worker_id = $2,
                  started_at = COALESCE(j.started_at, now()), updated_at = now()
             FROM candidate
            WHERE j.id = candidate.id
        RETURNING j.id, j.org_id, j.payload"#,
        LEASE.as_secs() as i32,
        worker_id,
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        Ok(ClaimedJob {
            id: r.id,
            org_id: r.org_id,
            payload: serde_json::from_value(r.payload).context("invalid durable job payload")?,
        })
    })
    .transpose()
}

async fn execute(state: &AppState, worker_id: &str, job: ClaimedJob) -> anyhow::Result<()> {
    sqlx::query!(
        r#"UPDATE horae_jobs SET phase = 'importing', updated_at = now()
            WHERE id = $1 AND worker_id = $2"#,
        job.id,
        worker_id,
    )
    .execute(&state.db)
    .await?;

    let (heartbeat_stop, mut heartbeat_rx) = tokio::sync::oneshot::channel();
    let heartbeat_pool = state.db.clone();
    let heartbeat_worker = worker_id.to_owned();
    let heartbeat_job = job.id;
    let heartbeat = tokio::spawn(async move {
        let mut tick = tokio::time::interval(LEASE / 3);
        tick.tick().await;
        loop {
            tokio::select! {
                _ = &mut heartbeat_rx => break,
                _ = tick.tick() => {
                    if let Err(error) = sqlx::query!(
                        r#"UPDATE horae_jobs
                              SET lease_until = now() + $1::int * interval '1 second',
                                  updated_at = now()
                            WHERE id = $2 AND worker_id = $3 AND status = 'running'"#,
                        LEASE.as_secs() as i32,
                        heartbeat_job,
                        heartbeat_worker,
                    )
                    .execute(&heartbeat_pool)
                    .await {
                        tracing::warn!(%error, job_id = %heartbeat_job, "durable job heartbeat failed");
                    }
                }
            }
        }
    });

    let result = match &job.payload {
        JobPayload::HarvestApi { mode, sync } => {
            let cfg = state.harvest.clone().context("Harvest is not configured")?;
            let currency = sqlx::query_scalar!(
                "SELECT default_currency FROM organizations WHERE id = $1",
                job.org_id
            )
            .fetch_one(&state.db)
            .await?;
            crate::importers::harvest::run_api_import(
                &state.db, job.org_id, &currency, &cfg, *mode, *sync,
            )
            .await
            .map(|report| {
                let processed = report.summary.clients.processed()
                    + report.summary.projects.processed()
                    + report.summary.tasks.processed()
                    + report.summary.time_entries.processed();
                (
                    serde_json::to_value(report).unwrap_or_default(),
                    processed as i64,
                )
            })
            .map_err(anyhow::Error::from)
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
            crate::importers::harvest::csv_source::import_body(
                &state.db,
                job.org_id,
                &currency,
                axum::body::Body::from(upload.body),
                *mode,
            )
            .await
            .map(|report| {
                let processed = report.summary.clients.processed()
                    + report.summary.projects.processed()
                    + report.summary.tasks.processed()
                    + report.summary.time_entries.processed();
                (
                    serde_json::to_value(report).unwrap_or_default(),
                    processed as i64,
                )
            })
            .map_err(anyhow::Error::from)
        }
    };

    let _ = heartbeat_stop.send(());
    let _ = heartbeat.await;

    match result {
        Ok((report, processed_count)) => {
            sqlx::query!(
                r#"UPDATE horae_jobs
                      SET status = 'succeeded', report = $1, processed_count = $2,
                          lease_until = NULL,
                          worker_id = $4, finished_at = now(), updated_at = now()
                    WHERE id = $3 AND worker_id = $4"#,
                report,
                processed_count,
                job.id,
                worker_id,
            )
            .execute(&state.db)
            .await?;
        }
        Err(error) => {
            sqlx::query!(
                r#"UPDATE horae_jobs
                      SET status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
                          available_at = now() + LEAST(power(2::double precision, attempts), 300)::int * interval '1 second',
                          lease_until = NULL, worker_id = NULL, last_error = $1, updated_at = now()
                    WHERE id = $2 AND worker_id = $3"#,
                error.to_string(),
                job.id,
                worker_id,
            )
            .execute(&state.db)
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    async fn org(pool: &sqlx::PgPool) -> Uuid {
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

    #[sqlx::test]
    async fn claims_are_atomic_and_expired_leases_recover(pool: sqlx::PgPool) {
        let org_id = org(&pool).await;
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let first = enqueue(&pool, org_id, &payload, "first").await.unwrap();
        let second = enqueue(&pool, org_id, &payload, "second").await.unwrap();

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
        let payload = JobPayload::HarvestApi {
            mode: ImportMode::DryRun,
            sync: SyncScope::Incremental,
        };
        let id = enqueue(&pool, org_id, &payload, "cancel-retry")
            .await
            .unwrap();
        assert!(cancel(&pool, org_id, id).await.unwrap());
        assert!(!cancel(&pool, org_id, id).await.unwrap());
        assert!(retry(&pool, org_id, id).await.unwrap());
        assert!(!retry(&pool, org_id, id).await.unwrap());
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
        let event = claim_outbox(&pool).await.unwrap().unwrap();
        assert_eq!(event.id, id);
        assert!(mark_outbox_delivered(&pool, id).await.unwrap());
        assert!(!mark_outbox_delivered(&pool, id).await.unwrap());
    }
}
