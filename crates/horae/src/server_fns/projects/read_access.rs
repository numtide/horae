//! Hold current project-read authority through payload materialization.

use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{ServerFnError, forbidden};
use crate::models::permissions::PersonPermissions;
use crate::ops::{Caller, OpError, ReadTx, SnapshotLimits};

pub(crate) struct ReadAccess<'a> {
    /// No INSERT/UPDATE/DELETE/nextval: commits asynchronously (see
    /// `begin_lock_only_read_async_commit`). Writes belong in their own
    /// transaction.
    pub tx: Transaction<'a, Postgres>,
    pub permissions: PersonPermissions,
}

impl ReadAccess<'_> {
    /// Absence preserves the existing empty/not-found result for unavailable actors.
    pub async fn begin(
        pool: &PgPool,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Self>, ServerFnError> {
        Self::from_read(crate::ops::begin_read(pool, Caller::session(org_id, actor_id)).await)
    }

    /// Like `begin`, but every later statement reads the same REPEATABLE READ
    /// snapshot (see `ops::begin_read_snapshot`). Callers must issue only plain
    /// reads afterwards.
    pub async fn begin_snapshot(
        pool: &PgPool,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Self>, ServerFnError> {
        Self::from_read(
            crate::ops::begin_read_snapshot(
                pool,
                Caller::session(org_id, actor_id),
                SnapshotLimits::Short,
            )
            .await,
        )
    }

    fn from_read(read: Result<ReadTx, OpError>) -> Result<Option<Self>, ServerFnError> {
        let read = read.map_err(|error| match error {
            // `Actor::load` refuses only for missing or invalid stored state.
            OpError::Forbidden(_) => forbidden("Project permission state is unavailable"),
            error => error.into(),
        })?;
        Ok(read.actor.permissions().cloned().map(|permissions| Self {
            tx: read.tx,
            permissions,
        }))
    }

    pub fn has(&self, permission: Permission) -> bool {
        self.permissions.grants.contains(permission)
    }

    pub fn administrator(&self) -> bool {
        self.permissions.is_administrator
    }
}
