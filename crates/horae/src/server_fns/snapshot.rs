//! Current manager authority for materialized, snapshot-consistent reads.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{OrgRole, ServerFnError, conflict, forbidden, server_err};

const MAX_ATTEMPTS: usize = 3;

#[cfg(test)]
mod reader_tests;

fn database_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(%error, "Unable to authorize financial snapshot");
    server_err("Unable to prepare a consistent snapshot")
}

async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, sqlx::Error> {
    match sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR SHARE",
        org_id
    )
    .fetch_one(&mut **tx)
    .await
    {
        Ok(_) => {}
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
pub(crate) async fn manager(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<Transaction<'_, Postgres>, ServerFnError> {
    for _ in 0..MAX_ATTEMPTS {
        let mut tx = pool
            .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ, READ WRITE")
            .await
            .map_err(database_error)?;
        // Every caller only reads after the `FOR SHARE` authority locks, so
        // this transaction's only writes are lock markers. Its commit need not
        // wait for the WAL flush: the locks are held and released exactly as
        // before, and a crash can only lose the commit record of a transaction
        // that changed no data. The setting is local and ends with the commit.
        // The materialized CTE reads the inherited lock_timeout before it is
        // bounded, for restoring after authorization.
        let previous_timeout = sqlx::query_scalar!(
            r#"WITH previous AS MATERIALIZED (
                 SELECT current_setting('lock_timeout') AS setting,
                   (extract(epoch FROM current_setting('lock_timeout')::interval) * 1000)::bigint
                     AS milliseconds
               )
               SELECT setting AS "setting!"
               FROM previous,
                 LATERAL set_config('lock_timeout',
                   (CASE WHEN milliseconds = 0 THEN 5000 ELSE LEAST(milliseconds, 5000) END)::text,
                   true) bounded,
                 LATERAL set_config('synchronous_commit', 'off', true) asynchronous"#
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(database_error)?;
        match authorize(&mut tx, org_id, actor_id).await {
            Ok(true) => {
                sqlx::query!(
                    "SELECT set_config('lock_timeout', $1, true)",
                    previous_timeout
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
