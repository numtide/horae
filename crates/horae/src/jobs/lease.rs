use anyhow::Context;
use sqlx::{PgConnection, PgPool};
use tokio::sync::watch;
use uuid::Uuid;

/// One execution attempt, not a worker identity: retry must never revive an
/// older attempt's right to commit, even when the same worker claims it again.
pub(crate) struct JobLease {
    pub(super) id: Uuid,
    pub(super) org_id: Uuid,
    pub(super) token: Uuid,
    pub(super) stop: watch::Receiver<bool>,
}

#[derive(Debug, thiserror::Error)]
#[error("job execution interrupted or lease lost")]
pub(super) struct Interrupted;

impl JobLease {
    pub(crate) fn check_organization(&self, org_id: Uuid) -> anyhow::Result<()> {
        anyhow::ensure!(self.org_id == org_id, "job organization mismatch");
        Ok(())
    }

    /// Cancel only the SQL consumer. Its caller still closes the source channel,
    /// joins the blocking producer, and releases the import session before ack.
    pub(crate) async fn run<T>(
        &self,
        work: impl Future<Output = anyhow::Result<T>>,
    ) -> anyhow::Result<T> {
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

    pub(super) async fn renew(&self, pool: &PgPool) -> anyhow::Result<()> {
        let renewed = sqlx::query!(
            r#"UPDATE horae_jobs
                  SET lease_until = clock_timestamp() + $1::int * interval '1 second',
                      updated_at = clock_timestamp()
                WHERE id = $2 AND org_id = $3 AND claim_token = $4
                  AND status = 'running' AND NOT cancellation_requested
                  AND lease_until > clock_timestamp()"#,
            super::LEASE.as_secs() as i32,
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

    pub(super) async fn monitor(&self, pool: &PgPool) -> anyhow::Result<()> {
        let mut poll = tokio::time::interval(super::POLL);
        let mut renewal = tokio::time::interval(super::LEASE / 3);
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

    /// With an import transaction, the job's report and domain writes become
    /// visible together. The conditional update locks the job until commit, so
    /// cancellation/reclaim cannot slip between the ownership check and COMMIT.
    pub(crate) async fn complete(
        &self,
        connection: &mut PgConnection,
        report: &serde_json::Value,
        processed_count: i64,
    ) -> anyhow::Result<bool> {
        let updated = sqlx::query!(
            r#"UPDATE horae_jobs
                  SET status = 'succeeded', report = $1, processed_count = $2,
                      lease_until = NULL, worker_id = NULL, claim_token = NULL,
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
