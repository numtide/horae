//! Current permission-state authority for materialized, snapshot-consistent reads.

use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{ServerFnError, forbidden, server_err};
use crate::ops::{Caller, OpError, SnapshotLimits};

const ACCESS_REQUIRED: &str = "Current access is required";

#[cfg(test)]
mod reader_tests;

/// Use session-derived IDs. Materialize and commit before external/client waits.
/// The transaction holds every `required` grant from the actor's stored
/// permission state until it ends.
///
/// No INSERT/UPDATE/DELETE/nextval on the returned transaction: it commits
/// asynchronously. Writes belong in their own transaction.
pub(crate) async fn authorized<'p>(
    pool: &'p PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    required: &[Permission],
) -> Result<Transaction<'p, Postgres>, ServerFnError> {
    let read = crate::ops::begin_read_snapshot(
        pool,
        Caller::session(org_id, actor_id),
        SnapshotLimits::LockWait,
    )
    .await
    .map_err(|error| match error {
        OpError::Internal(_) => server_err("Unable to prepare a consistent snapshot"),
        error => error.into(),
    })?;
    if required
        .iter()
        .all(|&permission| read.actor.has(permission))
    {
        Ok(read.tx)
    } else {
        Err(forbidden(ACCESS_REQUIRED))
    }
}
