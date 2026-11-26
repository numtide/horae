//! Durable PostgreSQL-backed background jobs: the queue and its worker, the
//! scheduler and its stored schedules, the transactional outbox and the jobs'
//! report chunks.
//!
//! The queue is intentionally small and application-owned: Horae's UUIDv7 and
//! organization invariants apply to queue rows just like every other table.
//! What a job does lives with the code that enqueues it, as a [`Handler`]
//! registered for the job's kind. The migrations stay in `horae-server`.

use std::{collections::HashMap, pin::Pin, time::Duration};

use anyhow::Context;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::PgConnection;
use uuid::Uuid;

mod lease;
pub mod outbox;
pub mod report;
mod schedule;
mod scheduler;
mod worker;

pub use lease::{Interrupted, JobContext};
pub use schedule::{
    Cadence, Schedule, ZoneSource, delete_schedule, next_due, set_schedule_enabled, upsert_schedule,
};
pub use scheduler::{Scheduler, ZoneResolver, fire_due, spawn_scheduler};
#[cfg(any(test, feature = "test-support"))]
pub use worker::{ClaimedJob, claim, claim_for_test, execute, run_worker};
pub use worker::{Worker, claim_registered, run_claimed, spawn};

/// How often a running job records that it is alive, at most. A kind with a
/// short stale limit beats four times within it.
const HEARTBEAT: Duration = Duration::from_secs(15);
/// How often idle workers and consumers look for work.
pub const POLL: Duration = Duration::from_secs(2);

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// What a successful run stores on its job.
pub struct Completed {
    pub report: serde_json::Value,
    pub processed_count: i64,
}

/// A failed run. A transient error requeues the job with backoff until its
/// attempts run out; a permanent one fails it at once. `error_kind` is written
/// to `horae_jobs.error_kind` as given, so it must be a value its CHECK allows.
#[derive(Debug)]
pub struct JobError {
    pub error: anyhow::Error,
    pub permanent: bool,
    pub error_kind: Option<&'static str>,
}

impl From<anyhow::Error> for JobError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error,
            permanent: false,
            error_kind: None,
        }
    }
}

/// When a kind's failed attempts are tried again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Attempts in total, the first included: the value enqueuers pass as
    /// [`Enqueue::max_attempts`]. The worker never reads it; it stops at the
    /// job's stored `max_attempts`.
    pub max_attempts: i32,
    pub first_delay: Duration,
    pub multiplier: u32,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    /// The queue's original curve: 2, 4, 8 … seconds, at most five minutes.
    fn default() -> Self {
        Self {
            max_attempts: 5,
            first_delay: Duration::from_secs(2),
            multiplier: 2,
            max_delay: Duration::from_secs(300),
        }
    }
}

impl RetryPolicy {
    /// The wait after attempt `attempt` (1-based) fails:
    /// `min(first_delay × multiplier^(attempt-1), max_delay)`.
    pub fn delay(&self, attempt: i32) -> Duration {
        let exponent = u32::try_from(attempt.saturating_sub(1)).unwrap_or(0);
        let factor = self.multiplier.checked_pow(exponent).unwrap_or(u32::MAX);
        self.first_delay.saturating_mul(factor).min(self.max_delay)
    }
}

/// The work behind one job kind.
pub trait Handler: Send + Sync + 'static {
    fn kind(&self) -> &'static str;

    fn policy(&self) -> RetryPolicy;

    /// How long a running job may go without a heartbeat before another worker
    /// may claim it again; between 15 s and an hour.
    fn stale_after(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Runs one attempt. `payload` is the job's payload inside its envelope.
    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>>;

    /// Retention of what the kind keeps beside its jobs, run by the worker
    /// before it deletes expired jobs.
    fn cleanup<'a>(&'a self, _pool: &'a sqlx::PgPool) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// The handlers a worker runs, by job kind.
#[derive(Default)]
pub struct Registry {
    handlers: HashMap<&'static str, Box<dyn Handler>>,
}

impl Registry {
    /// Panics when the kind already has a handler, so a copied `kind()` cannot
    /// make one kind run another's handler.
    pub fn register(&mut self, handler: impl Handler) -> &mut Self {
        let kind = handler.kind();
        assert!(
            self.handlers.insert(kind, Box::new(handler)).is_none(),
            "duplicate handler for job kind {kind}"
        );
        self
    }

    fn get(&self, kind: &str) -> Option<&dyn Handler> {
        self.handlers.get(kind).map(Box::as_ref)
    }

    /// The registered kinds and, in the same order, their stale limits in
    /// seconds, as the claim query takes them.
    fn claimable(&self) -> (Vec<String>, Vec<i32>) {
        self.handlers
            .iter()
            .map(|(kind, handler)| {
                // Clamped to the column's CHECK, which would otherwise fail every claim.
                let secs = handler.stale_after().as_secs().clamp(15, 3600) as i32;
                (kind.to_string(), secs)
            })
            .unzip()
    }

    async fn cleanup(&self, pool: &sqlx::PgPool) -> anyhow::Result<()> {
        for handler in self.handlers.values() {
            handler.cleanup(pool).await?;
        }
        cleanup(pool).await
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    version: u16,
    payload: T,
}

/// A job's stored payload: `payload` in a versioned envelope.
pub fn encode_payload<T: Serialize>(payload: &T) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::to_value(Envelope {
        version: 1,
        payload,
    })?)
}

pub fn decode_payload<T: DeserializeOwned>(value: serde_json::Value) -> anyhow::Result<T> {
    anyhow::ensure!(
        value.get("version").and_then(serde_json::Value::as_u64) == Some(1),
        "unsupported durable job payload version"
    );
    let envelope: Envelope<T> =
        serde_json::from_value(value).context("invalid durable job payload")?;
    Ok(envelope.payload)
}

/// A job to queue. `payload` is stored in the versioned envelope.
pub struct Enqueue<'a> {
    pub org_id: Uuid,
    pub kind: &'a str,
    pub payload: serde_json::Value,
    /// Another request with the same key returns the job it created, or
    /// conflicts when its payload differs.
    pub idempotency_key: Option<&'a str>,
    /// While a queued or running job of this organization and kind holds the
    /// key, enqueuing returns that job instead of creating one.
    pub concurrency_key: Option<&'a str>,
    /// Not claimed before this time; now when `None`.
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Higher is claimed first.
    pub priority: i16,
    /// Usually the kind's `RetryPolicy::max_attempts`.
    pub max_attempts: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enqueued {
    Created(Uuid),
    Existing(Uuid),
}

impl Enqueued {
    pub fn id(self) -> Uuid {
        match self {
            Self::Created(id) | Self::Existing(id) => id,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnqueueError {
    #[error("idempotency key conflicts with another job request")]
    Conflict,
    #[error("the job's key kept changing hands while it was enqueued")]
    Contended,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Another job of the same organization, kind and concurrency key is queued
/// or running, so this one cannot be queued again.
#[derive(Debug, thiserror::Error)]
#[error("another job with the same concurrency key is queued or running")]
pub struct ConcurrencyKeyHeld;

/// Whether `error` is the concurrency key's unique index refusing a row.
pub fn is_concurrency_key_conflict(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
        == Some("horae_jobs_concurrency_key_idx")
}

/// Queues a job inside the caller's transaction, so it exists only if the
/// caller's change commits.
///
/// Written for READ COMMITTED, PostgreSQL's default. Under REPEATABLE READ or
/// SERIALIZABLE a concurrent enqueue of the same key fails with a
/// serialization error, and a job committed after the caller's snapshot is
/// not found, which ends in [`EnqueueError::Contended`].
pub async fn enqueue_in(
    connection: &mut PgConnection,
    request: Enqueue<'_>,
) -> Result<Enqueued, EnqueueError> {
    let payload = serde_json::json!({ "version": 1, "payload": request.payload });
    // A key can be released between the insert and its lookup, so the insert
    // is tried again; a few times bound it should the lookups never match.
    for _ in 0..3 {
        let id = Uuid::now_v7();
        // Either key's conflict leaves the row out; which one is looked up below.
        let created = sqlx::query_scalar!(
            r#"INSERT INTO horae_jobs (id, org_id, kind, payload, idempotency_key, concurrency_key,
                                       available_at, priority, max_attempts)
               VALUES ($1, $2, $3, $4, $5, $6, COALESCE($7, now()), $8, $9)
               ON CONFLICT DO NOTHING
               RETURNING id"#,
            id,
            request.org_id,
            request.kind,
            payload,
            request.idempotency_key,
            request.concurrency_key,
            // Unchecked: where another crate enables sqlx's `time` feature,
            // the macro expects `OffsetDateTime` for timestamptz.
            request.run_at as _,
            request.priority,
            request.max_attempts,
        )
        .fetch_optional(&mut *connection)
        .await?;
        if let Some(id) = created {
            return Ok(Enqueued::Created(id));
        }
        if let Some(key) = request.idempotency_key {
            let existing = sqlx::query!(
                r#"SELECT id, payload = $4 AS "same!" FROM horae_jobs
                    WHERE org_id = $1 AND kind = $2 AND idempotency_key = $3"#,
                request.org_id,
                request.kind,
                key,
                payload,
            )
            .fetch_optional(&mut *connection)
            .await?;
            if let Some(existing) = existing {
                return if existing.same {
                    Ok(Enqueued::Existing(existing.id))
                } else {
                    Err(EnqueueError::Conflict)
                };
            }
        }
        if let Some(key) = request.concurrency_key {
            let existing = sqlx::query_scalar!(
                r#"SELECT id FROM horae_jobs
                    WHERE org_id = $1 AND kind = $2 AND concurrency_key = $3
                      AND status IN ('queued', 'running') AND NOT cancellation_requested"#,
                request.org_id,
                request.kind,
                key,
            )
            .fetch_optional(&mut *connection)
            .await?;
            if let Some(id) = existing {
                return Ok(Enqueued::Existing(id));
            }
        }
    }
    Err(EnqueueError::Contended)
}

pub async fn cancel<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
    org_id: Uuid,
    id: Uuid,
) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        r#"UPDATE horae_jobs
              SET cancellation_requested = true,
                  status = CASE WHEN status = 'queued' THEN 'cancelled' ELSE status END,
                  phase = 'cancelling',
                  finished_at = CASE WHEN status = 'queued' THEN now() ELSE finished_at END,
                  -- A queued job ends here; a running one clears it in fail_claimed.
                  error_kind = CASE WHEN status = 'queued' THEN NULL ELSE error_kind END,
                  updated_at = now()
            WHERE id = $1 AND org_id = $2 AND status IN ('queued', 'running')
              AND NOT cancellation_requested"#,
        id,
        org_id,
    )
    .execute(exec)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// Deletes finished jobs after 30 days, and closes attempts their job
/// outlived.
pub async fn cleanup(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    // A worker without attempt records (an older binary during a rolling
    // upgrade) can finish a job whose open attempt another worker began.
    sqlx::query!(
        r#"UPDATE horae_job_attempts a SET outcome = 'interrupted'
             FROM horae_jobs j
            WHERE a.job_id = j.id AND a.outcome IS NULL
              AND j.status IN ('succeeded', 'failed', 'cancelled')"#
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

#[cfg(test)]
pub(crate) static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../server/migrations");

#[cfg(test)]
mod tests;
