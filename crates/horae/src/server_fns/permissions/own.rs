//! Current own-access explanation, with no authority to inspect another person.

use sqlx::PgPool;
use uuid::Uuid;

use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;

use super::{
    PermissionStorageError, ReadIsolation, begin_lock_only_read_async_commit,
    load_person_permissions,
};
use crate::models::own_permissions::OwnPermissions;

#[derive(Debug, thiserror::Error)]
pub(crate) enum OwnPermissionsError {
    #[error("Current active identity is required")]
    Forbidden,
    #[error("Permission state is unavailable")]
    Unavailable,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// IDs must come from the session. The returned DTO is not server authority.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<OwnPermissions, OwnPermissionsError> {
    let mut tx = begin_lock_only_read_async_commit(pool, ReadIsolation::ReadCommitted).await?;
    let access_revision = sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1 FOR SHARE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(OwnPermissionsError::Forbidden)?;
    let actor = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if actor.is_none() {
        return Err(OwnPermissionsError::Forbidden);
    }
    let state = load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .ok_or(OwnPermissionsError::Unavailable)?;
    let managed_person_ids = sqlx::query_scalar!(
        "SELECT managed_user_id FROM person_management_assignments WHERE org_id = $1 AND manager_id = $2 ORDER BY managed_user_id",
        org_id, actor_id
    ).fetch_all(&mut *tx).await?;
    let managed_project_ids = sqlx::query_scalar!(
        "SELECT project_id FROM project_management_assignments WHERE org_id = $1 AND manager_id = $2 ORDER BY project_id",
        org_id, actor_id
    ).fetch_all(&mut *tx).await?;
    let result = OwnPermissions {
        catalog_version: PERMISSION_CATALOG_VERSION,
        grants: state.grants.iter().collect(),
        is_administrator: state.is_administrator,
        access_revision,
        person_revision: state.revision,
        managed_person_ids,
        managed_project_ids,
    };
    tx.commit().await?;
    Ok(result)
}

/// Whether the person holds Administrator authority now. IDs must come from
/// the session's active user.
pub(crate) async fn is_administrator(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<bool, OwnPermissionsError> {
    Ok(read(pool, org_id, actor_id).await?.is_administrator)
}
