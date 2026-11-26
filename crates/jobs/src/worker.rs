use std::time::Duration;

use anyhow::Context;
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

use crate::{Completed, Handler, Interrupted, JobContext, JobError, POLL, Registry, RetryPolicy};

/// `horae_job_attempts.error` holds at most this many bytes.
const ATTEMPT_ERROR_BYTES: usize = 4096;

const CLEANUP_EVERY: Duration = Duration::from_secs(60);

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
    if let Err(error) = warn_unknown_kinds(pool, registry).await {
        tracing::warn!(%error, "durable job kinds could not be listed");
    }
    // Retention scans the jobs table, so it runs once a minute, not before every claim.
    let mut cleaned: Option<std::time::Instant> = None;
    while !*stop.borrow() && stop.has_changed().is_ok() {
        if cleaned.is_none_or(|at| at.elapsed() >= CLEANUP_EVERY) {
            cleaned = Some(std::time::Instant::now());
            if let Err(error) = registry.cleanup(pool).await {
                tracing::warn!(%error, "durable job cleanup failed");
            }
        }
        if *stop.borrow() || stop.has_changed().is_err() {
            break;
        }
        // Finish an in-flight claim instead of dropping a query which may
        // already have assigned a lease in PostgreSQL.
        match claim_registered(pool, registry, &worker_id).await {
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

/// Jobs of a kind this worker has no handler for stay queued, so a server in
/// a rolling upgrade leaves a newer server's jobs alone. Logged once per start.
async fn warn_unknown_kinds(pool: &PgPool, registry: &Registry) -> anyhow::Result<()> {
    let (kinds, _) = registry.claimable();
    let unknown = sqlx::query_scalar!(
        r#"SELECT DISTINCT kind FROM horae_jobs
            WHERE status IN ('queued', 'running') AND kind <> ALL($1::text[])"#,
        &kinds,
    )
    .fetch_all(pool)
    .await?;
    for kind in unknown {
        tracing::warn!(%kind, "durable jobs of a kind without a handler stay queued");
    }
    Ok(())
}

pub struct ClaimedJob {
    pub id: Uuid,
    pub org_id: Uuid,
    pub kind: String,
    pub payload: serde_json::Value,
    pub claim_token: Uuid,
    pub attempt: i32,
    pub stale_after: Duration,
}

/// Claims the next job of a kind the registry runs, renewing its stale limit
/// from the handler.
pub async fn claim_registered(
    pool: &PgPool,
    registry: &Registry,
    worker_id: &str,
) -> anyhow::Result<Option<ClaimedJob>> {
    let (kinds, stale) = registry.claimable();
    claim_kinds(pool, worker_id, Some(&kinds), &stale).await
}

/// Claims the next job of any kind, keeping its stored stale limit.
#[cfg(any(test, feature = "test-support"))]
pub async fn claim(pool: &PgPool, worker_id: &str) -> anyhow::Result<Option<ClaimedJob>> {
    claim_kinds(pool, worker_id, None, &[]).await
}

/// `kinds` limits the claim to those kinds (all when `None`), and
/// `stale_secs[i]` is the stale limit of `kinds[i]`.
async fn claim_kinds(
    pool: &PgPool,
    worker_id: &str,
    kinds: Option<&[String]>,
    stale_secs: &[i32],
) -> anyhow::Result<Option<ClaimedJob>> {
    // An attempt whose worker died before finishing ends as `interrupted`,
    // whether its job ends here or is claimed again below.
    sqlx::query!(
        r#"WITH ended AS (
             UPDATE horae_jobs
                SET status = CASE WHEN cancellation_requested THEN 'cancelled' ELSE 'failed' END,
                    lease_until = NULL, worker_id = NULL, claim_token = NULL, heartbeat_at = NULL,
                    finished_at = now(), updated_at = now(),
                    last_error = CASE WHEN cancellation_requested THEN NULL
                        ELSE COALESCE(last_error, 'Job attempt limit reached after interruption') END
              WHERE (attempts >= max_attempts OR cancellation_requested)
                AND (status = 'queued' OR (status = 'running' AND lease_until < now()))
                AND ($1::text[] IS NULL OR kind = ANY($1))
          RETURNING id
           )
           UPDATE horae_job_attempts a SET outcome = 'interrupted'
             FROM ended WHERE a.job_id = ended.id AND a.outcome IS NULL"#,
        kinds,
    )
    .execute(pool)
    .await?;
    let claim_token = Uuid::now_v7();
    let mut tx = pool.begin().await?;
    let row = sqlx::query!(
        r#"WITH candidate AS (
             SELECT id,
                    COALESCE((SELECT k.secs FROM unnest($3::text[], $4::int[]) AS k(kind, secs)
                               WHERE k.kind = horae_jobs.kind), stale_after_secs) AS stale
               FROM horae_jobs
              WHERE attempts < max_attempts AND NOT cancellation_requested
                AND ($3::text[] IS NULL OR kind = ANY($3))
                AND ((status = 'queued' AND available_at <= now())
                  OR (status = 'running' AND lease_until < now()))
              ORDER BY priority DESC, available_at, created_at
              FOR UPDATE SKIP LOCKED
              LIMIT 1
           )
           UPDATE horae_jobs j
              SET status = 'running', attempts = j.attempts + 1,
                  stale_after_secs = candidate.stale, heartbeat_at = now(),
                  lease_until = now() + candidate.stale * interval '1 second',
                  worker_id = $1, claim_token = $2,
                  started_at = COALESCE(j.started_at, now()), updated_at = now()
             FROM candidate
            WHERE j.id = candidate.id
        RETURNING j.id, j.org_id, j.kind, j.payload, j.attempts, j.stale_after_secs"#,
        worker_id,
        claim_token,
        kinds,
        stale_secs,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    // A reclaimed job's previous attempt lost its worker.
    sqlx::query!(
        "UPDATE horae_job_attempts SET outcome = 'interrupted' WHERE job_id = $1 AND outcome IS NULL",
        row.id,
    )
    .execute(&mut *tx)
    .await?;
    // Numbered across manual retries, which restart the job's attempt budget.
    sqlx::query!(
        r#"INSERT INTO horae_job_attempts (id, org_id, job_id, attempt, worker_id, started_at)
           SELECT $1, $2, $3, COALESCE(max(attempt), 0) + 1, $4, now()
             FROM horae_job_attempts WHERE job_id = $3"#,
        Uuid::now_v7(),
        row.org_id,
        row.id,
        worker_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(ClaimedJob {
        id: row.id,
        org_id: row.org_id,
        kind: row.kind,
        payload: row.payload,
        claim_token,
        attempt: row.attempts,
        stale_after: Duration::from_secs(row.stale_after_secs.unsigned_abs().into()),
    }))
}

fn context(job: &ClaimedJob, stop: watch::Receiver<bool>) -> JobContext {
    JobContext {
        id: job.id,
        org_id: job.org_id,
        token: job.claim_token,
        attempt: job.attempt,
        stale_after: job.stale_after,
        stop,
    }
}

pub async fn execute(
    pool: &PgPool,
    registry: &Registry,
    worker_id: &str,
    job: ClaimedJob,
) -> anyhow::Result<()> {
    let (stop, receiver) = watch::channel(false);
    let ctx = context(&job, receiver);
    let handler = registry.get(&job.kind);
    let policy = handler.map(Handler::policy).unwrap_or_default();
    let work = async {
        let payload = start(pool, worker_id, &job).await?;
        let handler =
            handler.with_context(|| format!("no handler for durable job kind {}", job.kind))?;
        handler.run(&ctx, payload).await
    };
    run_claimed(pool, &ctx, &policy, stop, work).await
}

/// Marks the attempt started and opens its payload, failing the attempt like
/// any other error when the payload cannot be read.
async fn start(
    pool: &PgPool,
    worker_id: &str,
    job: &ClaimedJob,
) -> anyhow::Result<serde_json::Value> {
    // Every kind starts in phase `importing`, as when the queue ran only imports.
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
    crate::decode_payload(job.payload.clone())
}

/// Runs `work` beside the heartbeat and records its outcome; a failure is
/// retried after `policy`'s delay while attempts remain.
pub async fn run_claimed(
    pool: &PgPool,
    ctx: &JobContext,
    policy: &RetryPolicy,
    stop: watch::Sender<bool>,
    work: impl Future<Output = Result<Completed, JobError>>,
) -> anyhow::Result<()> {
    tokio::pin!(work);
    // No detached heartbeat survives an aborted worker. On cancellation, let
    // the handler wind down and roll back before the outcome is recorded.
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
                // A handler that completed inside its own transaction already
                // finished the job. Otherwise a concurrent cancellation still needs ack.
                fail_claimed(pool, ctx, policy, &anyhow::Error::from(Interrupted).into()).await?;
            }
        }
        Err(error) => fail_claimed(pool, ctx, policy, &error).await?,
    }
    Ok(())
}

async fn fail_claimed(
    pool: &PgPool,
    ctx: &JobContext,
    policy: &RetryPolicy,
    error: &JobError,
) -> anyhow::Result<()> {
    let message = error.error.to_string();
    let delay = i32::try_from(policy.delay(ctx.attempt).as_secs()).unwrap_or(i32::MAX);
    sqlx::query!(
        r#"WITH failed AS (
             UPDATE horae_jobs
                SET status = CASE WHEN cancellation_requested THEN 'cancelled'
                                  WHEN $5 OR attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
                    available_at = CASE WHEN cancellation_requested OR $5 OR attempts >= max_attempts
                                        THEN available_at ELSE now() + $7::int * interval '1 second' END,
                    finished_at = CASE WHEN cancellation_requested OR $5 OR attempts >= max_attempts THEN now() ELSE NULL END,
                    lease_until = NULL, worker_id = NULL, claim_token = NULL, heartbeat_at = NULL,
                    last_error = CASE WHEN cancellation_requested THEN NULL ELSE $1 END,
                    error_kind = CASE WHEN cancellation_requested THEN NULL ELSE $6 END,
                    updated_at = now()
              WHERE id = $2 AND org_id = $3 AND claim_token = $4 AND status = 'running'
                AND lease_until > clock_timestamp()
          RETURNING id, status
           )
           UPDATE horae_job_attempts a
              SET finished_at = now(),
                  outcome = CASE failed.status WHEN 'queued' THEN 'retrying' ELSE failed.status END,
                  error = CASE WHEN failed.status = 'cancelled' THEN NULL ELSE $8 END
             FROM failed
            WHERE a.job_id = failed.id AND a.outcome IS NULL"#,
        message,
        ctx.id,
        ctx.org_id,
        ctx.token,
        error.permanent,
        error.error_kind,
        delay,
        truncate(&message, ATTEMPT_ERROR_BYTES),
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// `text` cut to at most `bytes` bytes, on a character boundary.
pub(crate) fn truncate(text: &str, bytes: usize) -> &str {
    let mut end = text.len().min(bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Claims the next job without running it, for tests that drive an attempt by hand.
#[cfg(any(test, feature = "test-support"))]
pub async fn claim_for_test(pool: &PgPool) -> (JobContext, watch::Sender<bool>) {
    let job = claim(pool, "test-worker").await.unwrap().unwrap();
    let (stop, receiver) = watch::channel(false);
    (context(&job, receiver), stop)
}
