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

pub async fn status(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<Option<crate::models::JobStatus>> {
    let row = sqlx::query!(
        r#"SELECT id, kind, status, phase, processed_count, total_count,
                  report, last_error, created_at, finished_at
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

pub fn spawn(state: &'static AppState) {
    tokio::spawn(async move {
        let worker_id = Uuid::now_v7().to_string();
        loop {
            match claim(&state.db, &worker_id).await {
                Ok(Some(job)) => {
                    if let Err(error) = execute(state, &worker_id, job).await {
                        tracing::warn!(%error, "durable job failed");
                    }
                }
                Ok(None) => tokio::time::sleep(POLL).await,
                Err(error) => {
                    tracing::warn!(%error, "durable job poll failed");
                    tokio::time::sleep(POLL).await;
                }
            }
        }
    });
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
                  lease_until = now() + $1::interval, worker_id = $2,
                  started_at = COALESCE(j.started_at, now()), updated_at = now()
             FROM candidate
            WHERE j.id = candidate.id
        RETURNING j.id, j.org_id, j.payload"#,
        format!("{} seconds", LEASE.as_secs()),
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
            .map(|report| serde_json::to_value(report).unwrap_or_default())
            .map_err(anyhow::Error::from)
        }
        JobPayload::HarvestCsv { .. } => Err(anyhow::anyhow!(
            "CSV durable job handler is not enabled yet"
        )),
    };

    match result {
        Ok(report) => {
            sqlx::query!(
                r#"UPDATE horae_jobs
                      SET status = 'succeeded', report = $1, lease_until = NULL,
                          worker_id = $2, finished_at = now(), updated_at = now()
                    WHERE id = $3 AND worker_id = $2"#,
                report,
                worker_id,
                job.id,
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
}
