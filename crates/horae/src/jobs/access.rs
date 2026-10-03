//! Current user authority for import commands and retained results, not worker leases.

use horae_core::types::OrgRole;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccessError {
    #[error("Active administrator access required")]
    Forbidden,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Hold current tenant-bound authority through a bounded operation's commit.
pub(crate) async fn begin_admin_access(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<Transaction<'_, Postgres>, AccessError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    crate::db::lock_organization(&mut tx, org_id, crate::db::OrganizationLock::Shared)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => AccessError::Forbidden,
            other => AccessError::Database(other),
        })?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id = $1 AND org_id = $2
           AND active AND org_role = $3 FOR SHARE",
        actor_id,
        org_id,
        OrgRole::Admin as OrgRole,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AccessError::Forbidden)?;
    Ok(tx)
}
