use anyhow::Context;
use sqlx::{PgConnection, PgPool};
use tokio::sync::watch;
use uuid::Uuid;

/// One execution attempt, not a worker identity: retry must never revive an
/// older attempt's right to commit, even when the same worker claims it again.
#[derive(Clone)]
pub struct JobContext {
    pub(crate) id: Uuid,
    pub(crate) org_id: Uuid,
    pub(crate) token: Uuid,
    pub(crate) stop: watch::Receiver<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("job execution interrupted or lease lost")]
pub struct Interrupted;

impl JobContext {
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn org_id(&self) -> Uuid {
        self.org_id
    }

    /// The attempt's claim token, which every fenced write checks.
    pub fn token(&self) -> Uuid {
        self.token
    }

    pub async fn load_checkpoint(
        &self,
        connection: &mut PgConnection,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let row = sqlx::query!(
            r#"SELECT checkpoint FROM horae_jobs
                WHERE id = $1 AND org_id = $2 AND claim_token = $3
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            self.id,
            self.org_id,
            self.token,
        )
        .fetch_optional(connection)
        .await?
        .context(Interrupted)?;
        Ok(row.checkpoint)
    }

    /// Save inside the handler's transaction, so cursor/progress and domain
    /// changes commit together under the same cancellation and ownership fence.
    /// The public report contains only these confirmed outcomes, even if a later
    /// batch fails or the final attempt's lease expires.
    ///
    /// `total_count` belongs to `phase`: the records that phase handles, when
    /// known. It is not in the unit of `processed_count`, which spans phases.
    pub async fn save_checkpoint(
        &self,
        connection: &mut PgConnection,
        checkpoint: &serde_json::Value,
        report: &serde_json::Value,
        phase: &str,
        processed_count: i64,
        total_count: Option<i64>,
    ) -> anyhow::Result<()> {
        let updated = sqlx::query!(
            r#"UPDATE horae_jobs
                  SET checkpoint = $1, phase = $2, processed_count = $3, report = $7,
                      total_count = $8, updated_at = clock_timestamp()
                WHERE id = $4 AND org_id = $5 AND claim_token = $6
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            checkpoint,
            phase,
            processed_count,
            self.id,
            self.org_id,
            self.token,
            report,
            total_count,
        )
        .execute(connection)
        .await?;
        anyhow::ensure!(updated.rows_affected() == 1, Interrupted);
        Ok(())
    }

    pub fn check_organization(&self, org_id: Uuid) -> anyhow::Result<()> {
        anyhow::ensure!(self.org_id == org_id, "job organization mismatch");
        Ok(())
    }

    /// Stops only `work` when the attempt is cancelled or loses its lease.
    /// Whatever the handler started beside it (a producer thread, a session)
    /// is the caller's to wind down before the run returns.
    pub async fn run<T>(&self, work: impl Future<Output = anyhow::Result<T>>) -> anyhow::Result<T> {
        let mut stop = self.stop.clone();
        if *stop.borrow() {
            return Err(Interrupted.into());
        }
        tokio::select! {
            biased;
            _ = stop.wait_for(|stopped| *stopped) => Err(Interrupted.into()),
            result = work => result,
        }
    }

    pub(crate) async fn renew(&self, pool: &PgPool) -> anyhow::Result<()> {
        let renewed = sqlx::query!(
            r#"UPDATE horae_jobs
                  SET lease_until = clock_timestamp() + $1::int * interval '1 second',
                      updated_at = clock_timestamp()
                WHERE id = $2 AND org_id = $3 AND claim_token = $4
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            crate::LEASE.as_secs() as i32,
            self.id,
            self.org_id,
            self.token,
        )
        .execute(pool)
        .await?;
        if renewed.rows_affected() != 1 {
            return Err(Interrupted.into());
        }
        Ok(())
    }

    pub(crate) async fn monitor(&self, pool: &PgPool) -> anyhow::Result<()> {
        let mut poll = tokio::time::interval(crate::POLL);
        let mut renewal = tokio::time::interval(crate::LEASE / 3);
        loop {
            tokio::select! {
                _ = renewal.tick() => self.renew(pool).await?,
                _ = poll.tick() => {
                    let live = sqlx::query_scalar!(
                        r#"SELECT id FROM horae_jobs
                            WHERE id = $1 AND org_id = $2 AND claim_token = $3
                              AND status = 'running' AND NOT cancellation_requested
                              AND lease_until > clock_timestamp()"#,
                        self.id, self.org_id, self.token,
                    ).fetch_optional(pool).await?;
                    live.context(Interrupted)?;
                }
            }
        }
    }

    /// Inside the handler's transaction, the job's report and its domain writes
    /// become visible together. The conditional update locks the job until commit, so
    /// cancellation/reclaim cannot slip between the ownership check and COMMIT.
    pub async fn complete(
        &self,
        connection: &mut PgConnection,
        report: &serde_json::Value,
        processed_count: i64,
    ) -> anyhow::Result<bool> {
        let updated = sqlx::query!(
            r#"UPDATE horae_jobs
                  SET status = 'succeeded', report = $1, processed_count = $2, total_count = $2,
                      lease_until = NULL, worker_id = NULL, claim_token = NULL, checkpoint = NULL,
                      finished_at = clock_timestamp(), updated_at = clock_timestamp()
                WHERE id = $3 AND org_id = $4 AND claim_token = $5
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            report,
            processed_count,
            self.id,
            self.org_id,
            self.token,
        )
        .execute(connection)
        .await?;
        Ok(updated.rows_affected() == 1)
    }
}
