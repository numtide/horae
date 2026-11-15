//! Current user authority for import commands and retained results, not worker leases.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::ops::{Caller, OpError};

/// Hold current tenant-bound authority through a bounded operation's commit:
/// the organization and the person stay locked `FOR SHARE`, so a concurrent
/// access change waits for the operation or is seen by it.
pub(crate) async fn begin_admin_access(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<Transaction<'static, Postgres>, OpError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    if let Err(error) =
        crate::ops::require_administrator(&mut tx, Caller::session(org_id, actor_id)).await
    {
        // Release the locks the refusal took with the response, not in the
        // pool's deferred cleanup.
        let _ = tx.rollback().await;
        return Err(error);
    }
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use horae_core::permissions::catalog::BuiltInProfile;

    use super::*;
    use crate::server_fns::test_seed::{organization_lockable, seed};

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    async fn a_refused_member_releases_the_locks(pool: PgPool) {
        let ids = seed(&pool, BuiltInProfile::Member).await;
        let result = begin_admin_access(&pool, ids.org_id, ids.user_id).await;
        assert!(matches!(result, Err(OpError::Forbidden(_))));
        assert!(organization_lockable(&pool, ids.org_id));
    }
}
