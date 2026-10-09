//! Hold current project-read authority through payload materialization.

use horae_core::permissions::catalog::Permission;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{ServerFnError, forbidden, server_err};
use crate::models::permissions::PersonPermissions;
use crate::server_fns::permissions::{configure_administration, load_person_permissions};

pub(crate) struct ReadAccess<'a> {
    pub tx: Transaction<'a, Postgres>,
    pub permissions: Option<PersonPermissions>,
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
        let policy = sqlx::query_scalar!(
            "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
            org_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(server_err)?;
        let Some(policy) = policy else {
            return Ok(None);
        };
        if !matches!(policy, 0 | 1) {
            return Err(forbidden("Project permission state is unavailable"));
        }
        let active = sqlx::query_scalar!(
            "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
            org_id,
            actor_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(server_err)?;
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
