//! Hold current project-read authority through payload materialization.

use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{ServerFnError, conflict, forbidden, server_err};
use crate::models::permissions::PersonPermissions;
use crate::server_fns::permissions::{
    configure_administration, configure_transaction_limits, load_person_permissions,
};

const SNAPSHOT_ATTEMPTS: usize = 3;
const ACCESS_CHANGED: &str = "Access changed while reading. Please try again.";

pub(crate) struct ReadAccess<'a> {
    pub tx: Transaction<'a, Postgres>,
    pub permissions: Option<PersonPermissions>,
}

/// A begin that saw its authority rows change after its snapshot started.
enum Begin<'a> {
    Stale(Transaction<'a, Postgres>),
    Failed(ServerFnError),
}

impl From<ServerFnError> for Begin<'_> {
    fn from(error: ServerFnError) -> Self {
        Self::Failed(error)
    }
}

impl<'a> ReadAccess<'a> {
    /// Absence preserves the existing empty/not-found result for unavailable actors.
    pub async fn begin(
        pool: &'a PgPool,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Self>, ServerFnError> {
        let mut tx = pool.begin().await.map_err(server_err)?;
        configure_administration(&mut tx)
            .await
            .map_err(server_err)?;
        match Self::authorize(tx, org_id, actor_id).await {
            Ok(access) => Ok(access),
            // READ COMMITTED re-reads a changed row rather than failing.
            Err(Begin::Stale(_)) => Err(conflict(ACCESS_CHANGED)),
            Err(Begin::Failed(error)) => Err(error),
        }
    }

    /// Like `begin`, but every later statement reads the same REPEATABLE READ
    /// snapshot. READ WRITE only permits the authority locks; nothing is
    /// written. Callers must issue only plain reads afterwards: the locks are
    /// the sole statements that can fail with 40001, so retrying the begin in
    /// a fresh transaction is enough.
    pub async fn begin_snapshot(
        pool: &'a PgPool,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Self>, ServerFnError> {
        for _ in 0..SNAPSHOT_ATTEMPTS {
            let mut tx = pool.begin().await.map_err(server_err)?;
            sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ WRITE")
                .execute(&mut *tx)
                .await
                .map_err(server_err)?;
            configure_transaction_limits(&mut tx)
                .await
                .map_err(server_err)?;
            match Self::authorize(tx, org_id, actor_id).await {
                Ok(access) => return Ok(access),
                Err(Begin::Stale(tx)) => tx.rollback().await.map_err(server_err)?,
                Err(Begin::Failed(error)) => return Err(error),
            }
        }
        Err(conflict(ACCESS_CHANGED))
    }

    async fn authorize(
        mut tx: Transaction<'a, Postgres>,
        org_id: Uuid,
        actor_id: Uuid,
    ) -> Result<Option<Self>, Begin<'a>> {
        let policy = match sqlx::query_scalar!(
            "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
            org_id,
        )
        .fetch_optional(&mut *tx)
        .await
        {
            Err(error) if serialization_failure(&error) => return Err(Begin::Stale(tx)),
            result => result.map_err(server_err)?,
        };
        let Some(policy) = policy else {
            return Ok(None);
        };
        if !matches!(policy, 0 | 1) {
            return Err(forbidden("Project permission state is unavailable").into());
        }
        let active = match sqlx::query_scalar!(
            "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
            org_id,
            actor_id,
        )
        .fetch_optional(&mut *tx)
        .await
        {
            Err(error) if serialization_failure(&error) => return Err(Begin::Stale(tx)),
            result => result.map_err(server_err)?,
        };
        if active.is_none() {
            return Ok(None);
        }
        let permissions = if policy == 1 {
            Some(
                load_person_permissions(&mut tx, org_id, actor_id)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Unable to load project-read permissions");
                        forbidden("Project permission state is unavailable")
                    })?
                    .ok_or_else(|| forbidden("Project permission state is unavailable"))?,
            )
        } else {
            None
        };
        Ok(Some(Self { tx, permissions }))
    }

    pub fn legacy(&self) -> bool {
        self.permissions.is_none()
    }

    pub fn has(&self, permission: Permission) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.grants.contains(permission))
    }

    pub fn administrator(&self) -> bool {
        self.permissions
            .as_ref()
            .is_some_and(|state| state.is_administrator)
    }
}

fn serialization_failure(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .is_some_and(|error| error.code().as_deref() == Some("40001"))
}
