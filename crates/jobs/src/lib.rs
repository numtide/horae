//! Durable PostgreSQL-backed background jobs: the queue and its worker, the
//! transactional outbox and the jobs' report chunks.
//!
//! The queue is intentionally small and application-owned: Horae's UUIDv7 and
//! organization invariants apply to queue rows just like every other table.
//! What a job does lives with the code that enqueues it, as a [`Handler`]
//! registered for the job's kind. The migrations stay in `horae-server`.

use std::{collections::HashMap, pin::Pin, time::Duration};

use anyhow::Context;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

mod lease;
pub mod outbox;
pub mod report;
mod worker;

pub use lease::{Interrupted, JobContext};
#[cfg(any(test, feature = "test-support"))]
pub use worker::{ClaimedJob, claim, claim_for_test, execute, run_worker};
pub use worker::{Worker, run_claimed, spawn};

const LEASE: Duration = Duration::from_secs(300);
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

/// The work behind one job kind.
pub trait Handler: Send + Sync + 'static {
    fn kind(&self) -> &'static str;

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
    pub fn register(&mut self, handler: impl Handler) -> &mut Self {
        self.handlers.insert(handler.kind(), Box::new(handler));
        self
    }

    fn get(&self, kind: &str) -> Option<&dyn Handler> {
        self.handlers.get(kind).map(Box::as_ref)
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

/// Deletes finished jobs after 30 days.
pub async fn cleanup(pool: &sqlx::PgPool) -> anyhow::Result<()> {
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
