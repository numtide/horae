//! The signed-in person's own account: the explanation of their permissions,
//! the server version administrators may see, and their name and time zone.
//!
//! Each operation reaches only the actor's own records, so the active identity
//! is all it needs; nobody else's account is reachable from here.

use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;

use super::permissions::profile_name;
use super::{Actor, Done, OpError, Tx};
use crate::models::own_permissions::OwnPermissions;

const IDENTITY_REQUIRED: &str = "Current active identity is required";

/// The actor's stored permissions, for display. The answer cannot authorize
/// later requests: [`Actor::load`] reads the state again each time.
pub async fn permissions(tx: &mut Tx, actor: &Actor) -> Result<OwnPermissions, OpError> {
    let Some(state) = actor.permissions() else {
        return Err(OpError::forbidden(IDENTITY_REQUIRED));
    };
    let org_id = actor.org_id();
    // Read with the state `Actor::load` loaded, under the same lock.
    let access_revision = actor.organization()?.access_revision;
    let profile = profile_name(tx, org_id, state.source).await?;
    let managed_person_ids = sqlx::query_scalar!(
        "SELECT managed_user_id FROM person_management_assignments
         WHERE org_id = $1 AND manager_id = $2 ORDER BY managed_user_id",
        org_id,
        actor.id()
    )
    .fetch_all(&mut **tx)
    .await?;
    let managed_project_ids = sqlx::query_scalar!(
        "SELECT project_id FROM project_management_assignments
         WHERE org_id = $1 AND manager_id = $2 ORDER BY project_id",
        org_id,
        actor.id()
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(OwnPermissions {
        catalog_version: PERMISSION_CATALOG_VERSION,
        grants: state.grants.iter().collect(),
        is_administrator: state.is_administrator,
        profile,
        access_revision,
        person_revision: state.revision,
        managed_person_ids,
        managed_project_ids,
    })
}

/// The running server's version label, for administrators only.
pub fn server_version(actor: &Actor, state: &crate::state::AppState) -> Result<String, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden(IDENTITY_REQUIRED));
    }
    if !actor.administrator() {
        return Err(OpError::forbidden("Administrator authority is required"));
    }
    Ok(state.version.clone())
}

/// Changes the actor's display name and time zone (`None` follows the
/// workspace).
pub async fn update(
    tx: &mut Tx,
    actor: &Actor,
    name: &str,
    timezone: Option<&str>,
) -> Result<Done<()>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden("Account is not active"));
    }
    actor.refuse_oauth()?;
    let name = name.trim();
    if !crate::models::user::profile_name_is_valid(name) {
        return Err(OpError::invalid(
            "name",
            "Enter a name of 1 to 200 characters",
        ));
    }
    let timezone = timezone
        .map(|zone| {
            crate::timezone::parse(zone)
                .map(|zone| zone.name())
                .ok_or_else(|| OpError::invalid("timezone", "Choose a time zone from the list"))
        })
        .transpose()?;
    crate::timezone::replan_for_person_zone(tx, actor.id(), timezone).await?;
    sqlx::query!(
        "UPDATE users SET name = $3, timezone = $4 WHERE id = $1 AND org_id = $2",
        actor.id(),
        actor.org_id(),
        name,
        timezone,
    )
    .execute(&mut **tx)
    .await?;
    Ok(Done::new(()))
}

#[cfg(test)]
mod tests;
