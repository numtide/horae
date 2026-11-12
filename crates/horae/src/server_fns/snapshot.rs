//! Current permission-state authority for materialized, snapshot-consistent reads.

use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{ServerFnError, conflict, forbidden, server_err};
use crate::ops::{Actor, Caller, OpError};

const MAX_ATTEMPTS: usize = 3;
const ACCESS_REQUIRED: &str = "Current access is required";

#[cfg(test)]
mod reader_tests;

fn database_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(%error, "Unable to authorize financial snapshot");
    server_err("Unable to prepare a consistent snapshot")
}

/// Whether the actor's stored state grants `required`. The actor loads with
/// the organization, then the person, locked `FOR SHARE`.
async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
    required: Permission,
) -> Result<bool, OpError> {
    let actor = Actor::load(tx, Caller::session(org_id, actor_id)).await?;
    Ok(actor.has(required))
}

/// Use session-derived IDs. Materialize and commit before external/client waits.
/// The transaction holds the actor's `required` grant from the stored
/// permission state until it ends.
///
/// No INSERT/UPDATE/DELETE/nextval on the returned transaction: it commits
/// asynchronously. Writes belong in their own transaction.
pub(crate) async fn authorized(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    required: Permission,
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
        match authorize(&mut tx, org_id, actor_id, required).await {
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
                return Err(forbidden(ACCESS_REQUIRED));
            }
            // `Actor::load` reports a serialization failure as a conflict: an
            // access change committed after this snapshot began. Start a new
            // snapshot; retrying just its query would remain stale.
            Err(OpError::Conflict(_)) => {
                tx.rollback().await.map_err(database_error)?;
            }
            Err(OpError::Internal(_)) => {
                tx.rollback().await.map_err(database_error)?;
                return Err(server_err("Unable to prepare a consistent snapshot"));
            }
            Err(error) => {
                tx.rollback().await.map_err(database_error)?;
                return Err(error.into());
            }
        }
    }
    Err(conflict("Access changed while reading. Please try again."))
}
