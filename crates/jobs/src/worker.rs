use std::time::Duration;

use anyhow::Context;
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{Completed, Handler, Interrupted, JobContext, JobError, LEASE, POLL, Registry};

pub struct Worker {
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl Worker {
    pub fn new(stop: watch::Sender<bool>, task: tokio::task::JoinHandle<()>) -> Self {
        Self { stop, task }
    }

    pub fn stop_sender(&self) -> watch::Sender<bool> {
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

/// Runs the registry's handlers on claimed jobs until shutdown.
pub fn spawn(pool: PgPool, registry: Registry) -> Worker {
    let (stop, receiver) = watch::channel(false);
    let task = tokio::spawn(async move { run_worker(&pool, &registry, receiver).await });
    Worker { stop, task }
}

pub async fn run_worker(pool: &PgPool, registry: &Registry, mut stop: watch::Receiver<bool>) {
    let worker_id = Uuid::now_v7().to_string();
    while !*stop.borrow() && stop.has_changed().is_ok() {
        if let Err(error) = registry.cleanup(pool).await {
            tracing::warn!(%error, "durable job cleanup failed");
        }
        if *stop.borrow() || stop.has_changed().is_err() {
            break;
        }
        // Finish an in-flight claim instead of dropping a query which may
        // already have assigned a lease in PostgreSQL.
        match claim(pool, &worker_id).await {
            Ok(Some(job)) => {
                if let Err(error) = execute(pool, registry, &worker_id, job).await {
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

pub struct ClaimedJob {
    pub id: Uuid,
    pub org_id: Uuid,
    pub kind: String,
    pub payload: serde_json::Value,
    pub claim_token: Uuid,
}

pub async fn claim(pool: &PgPool, worker_id: &str) -> anyhow::Result<Option<ClaimedJob>> {
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
        RETURNING j.id, j.org_id, j.kind, j.payload"#,
        LEASE.as_secs() as i32,
        worker_id,
        claim_token,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| ClaimedJob {
        id: r.id,
        org_id: r.org_id,
        kind: r.kind,
        payload: r.payload,
        claim_token,
    }))
}

pub async fn execute(
    pool: &PgPool,
    registry: &Registry,
    worker_id: &str,
    job: ClaimedJob,
) -> anyhow::Result<()> {
    let (stop, receiver) = watch::channel(false);
    let ctx = JobContext {
        id: job.id,
        org_id: job.org_id,
        token: job.claim_token,
        stop: receiver,
    };
    let work = async {
        let (handler, payload) = start(pool, registry, worker_id, &job).await?;
        handler.run(&ctx, payload).await
    };
    run_claimed(pool, &ctx, stop, work).await
}

/// Marks the attempt started and finds its handler, failing the attempt like
/// any other error when the payload or the kind is unknown.
async fn start<'r>(
    pool: &PgPool,
    registry: &'r Registry,
    worker_id: &str,
    job: &ClaimedJob,
) -> anyhow::Result<(&'r dyn Handler, serde_json::Value)> {
    let started = sqlx::query!(
        r#"UPDATE horae_jobs SET phase = 'importing', updated_at = now()
            WHERE id = $1 AND worker_id = $2 AND claim_token = $3
              AND status = 'running' AND NOT cancellation_requested
              AND lease_until > clock_timestamp()"#,
        job.id,
        worker_id,
        job.claim_token,
    )
    .execute(pool)
    .await?;
    if started.rows_affected() != 1 {
        return Err(Interrupted.into());
    }
    let payload = crate::decode_payload(job.payload.clone())?;
    let handler = registry
        .get(&job.kind)
        .with_context(|| format!("no handler for durable job kind {}", job.kind))?;
    Ok((handler, payload))
}

pub async fn run_claimed(
    pool: &PgPool,
    ctx: &JobContext,
    stop: watch::Sender<bool>,
    work: impl Future<Output = Result<Completed, JobError>>,
) -> anyhow::Result<()> {
    tokio::pin!(work);
    // No detached heartbeat survives an aborted worker. On cancellation, let
    // the importer join its producer and flush rollback before recording ack.
    let result = tokio::select! {
        result = &mut work => result,
        monitor = ctx.monitor(pool) => {
            tracing::info!(job_id = %ctx.id, error = ?monitor.err(), "stopping durable execution");
            stop.send_replace(true);
            work.await
        }
    };
    match result {
        Ok(Completed {
            report,
            processed_count,
        }) => {
            let completed = {
                let mut connection = pool.acquire().await?;
                ctx.complete(&mut connection, &report, processed_count)
                    .await?
            };
            if !completed {
                // Committing imports already completed atomically with their
                // writes. Otherwise a concurrent cancellation still needs ack.
                fail_claimed(pool, ctx, &anyhow::Error::from(Interrupted).into()).await?;
            }
        }
        Err(error) => fail_claimed(pool, ctx, &error).await?,
    }
    Ok(())
}

async fn fail_claimed(pool: &PgPool, ctx: &JobContext, error: &JobError) -> anyhow::Result<()> {
    sqlx::query!(
        r#"UPDATE horae_jobs
              SET status = CASE WHEN cancellation_requested THEN 'cancelled'
                                WHEN $5 OR attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
                  available_at = now() + LEAST(power(2::double precision, LEAST(attempts, 9)), 300)::int * interval '1 second',
                  finished_at = CASE WHEN cancellation_requested OR $5 OR attempts >= max_attempts THEN now() ELSE NULL END,
                  lease_until = NULL, worker_id = NULL, claim_token = NULL,
                  last_error = CASE WHEN cancellation_requested THEN NULL ELSE $1 END,
                  error_kind = CASE WHEN cancellation_requested THEN NULL ELSE $6 END,
                  updated_at = now()
            WHERE id = $2 AND org_id = $3 AND claim_token = $4 AND status = 'running'
              AND lease_until > clock_timestamp()"#,
        error.error.to_string(),
        ctx.id,
        ctx.org_id,
        ctx.token,
        error.permanent,
        error.error_kind,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Claims the next job without running it, for tests that drive an attempt by hand.
#[cfg(any(test, feature = "test-support"))]
pub async fn claim_for_test(pool: &PgPool) -> (JobContext, watch::Sender<bool>) {
    let job = claim(pool, "test-worker").await.unwrap().unwrap();
    let (stop, receiver) = watch::channel(false);
    (
        JobContext {
            id: job.id,
            org_id: job.org_id,
            token: job.claim_token,
            stop: receiver,
        },
        stop,
    )
}
