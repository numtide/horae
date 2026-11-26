use std::ops::Deref;

use horae_jobs::JobContext;
use sqlx::PgConnection;

/// An import attempt: the queue's [`JobContext`] plus the checks only the
/// Harvest imports need.
pub struct JobLease(pub(super) JobContext);

impl Deref for JobLease {
    type Target = JobContext;

    fn deref(&self) -> &JobContext {
        &self.0
    }
}

impl JobLease {
    /// Checked while holding the import lock, before using any credentials.
    pub(crate) async fn check_account_generation(
        &self,
        connection: &mut PgConnection,
    ) -> anyhow::Result<()> {
        let current = sqlx::query_scalar!(r#"SELECT EXISTS(
            SELECT 1 FROM horae_jobs j LEFT JOIN harvest_connection_generations g ON g.org_id = j.org_id
            WHERE j.id = $1 AND j.org_id = $2 AND j.claim_token = $3
              AND j.account_generation = COALESCE(g.account_generation, 0)
              AND j.status = 'running' AND NOT j.cancellation_requested
              AND j.lease_until > clock_timestamp()) AS "current!""#, self.id(), self.org_id(), self.token())
            .fetch_one(connection).await?;
        anyhow::ensure!(
            current,
            crate::importers::harvest::account_switch::ChangeError::OldImport
        );
        Ok(())
    }
}
