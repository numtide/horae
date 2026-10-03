//! Current manager authority for materialized, snapshot-consistent reads.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{OrgRole, ServerFnError, conflict, forbidden, server_err};
use crate::db::{OrganizationLock, lock_organization};

const MAX_ATTEMPTS: usize = 3;

fn database_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(%error, "Unable to authorize financial snapshot");
    server_err("Unable to prepare a consistent snapshot")
}

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, sqlx::Error> {
    match lock_organization(tx, org_id, OrganizationLock::Shared).await {
        Ok(()) => {}
        Err(sqlx::Error::RowNotFound) => return Ok(false),
        Err(error) => return Err(error),
    }
    let role = sqlx::query_scalar!(
        r#"SELECT org_role as "role: OrgRole" FROM users WHERE id=$1 AND org_id=$2 AND active FOR SHARE"#,
        actor_id, org_id,
    ).fetch_optional(&mut **tx).await?;
    Ok(role.is_some_and(|role| role.is_manager_or_above()))
}

/// Use session-derived IDs. Materialize and commit before external/client waits.
/// This preserves the legacy manager boundary, not canonical grant enforcement.
pub(super) async fn manager(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<Transaction<'_, Postgres>, ServerFnError> {
    for _ in 0..MAX_ATTEMPTS {
        let mut tx = pool.begin().await.map_err(database_error)?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ WRITE")
            .execute(&mut *tx)
            .await
            .map_err(database_error)?;
        let previous_timeout = sqlx::query_scalar!(
            "SELECT setting::bigint AS \"timeout!\" FROM pg_settings WHERE name='lock_timeout'"
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        let bounded_timeout = if previous_timeout == 0 {
            5000
        } else {
            previous_timeout.min(5000)
        };
        sqlx::query!(
            "SELECT set_config('lock_timeout', $1, true)",
            format!("{bounded_timeout}ms")
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        match authorize(&mut tx, org_id, actor_id).await {
            Ok(true) => {
                sqlx::query!(
                    "SELECT set_config('lock_timeout', $1, true)",
                    format!("{previous_timeout}ms")
                )
                .fetch_one(&mut *tx)
                .await
                .map_err(database_error)?;
                return Ok(tx);
            }
            Ok(false) => {
                tx.rollback().await.map_err(database_error)?;
                return Err(forbidden("Active manager access required"));
            }
            Err(error) => {
                tx.rollback().await.map_err(database_error)?;
                if error
                    .as_database_error()
                    .is_none_or(|error| error.code().as_deref() != Some("40001"))
                {
                    return Err(database_error(error));
                }
                // A legacy actor update need not advance the organization row.
                // Start a new snapshot; retrying just its query remains stale.
            }
        }
    }
    Err(conflict("Access changed while reading. Please try again."))
}
