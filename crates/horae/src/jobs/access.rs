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
    crate::ops::require_administrator(&mut tx, Caller::session(org_id, actor_id)).await?;
    Ok(tx)
}
