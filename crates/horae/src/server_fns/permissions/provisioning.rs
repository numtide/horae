//! Permission state for people as they are created.
//!
//! Every path that creates a person writes a built-in profile state in the
//! same transaction, so nobody exists without one.

use horae_core::permissions::catalog::{BuiltInProfile, PERMISSION_CATALOG_VERSION};
use horae_core::types::OrgRole;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

/// The `org_role` stored beside a new person's state.
///
/// The role decides nothing about permission state. It is filled only because
/// the domains not yet moved to permission state still read it; the schema
/// change that drops the column drops this too.
pub(crate) const fn placeholder_role(profile: BuiltInProfile) -> OrgRole {
    match profile {
        BuiltInProfile::Administrator => OrgRole::Admin,
        BuiltInProfile::ExecutiveManager => OrgRole::Manager,
        BuiltInProfile::Member
        | BuiltInProfile::ProjectManager
        | BuiltInProfile::PeopleAdmin
        | BuiltInProfile::Accounting => OrgRole::Member,
    }
}

/// The built-in profile stored under `id` (`administrator`, `member`, …).
pub(crate) fn parse_profile(id: &str) -> Option<BuiltInProfile> {
    BuiltInProfile::ALL
        .iter()
        .copied()
        .find(|profile| stored_id(profile).is_ok_and(|stored| stored == id))
}

/// The identifier a catalog value is stored under.
fn stored_id(value: impl Serialize) -> Result<String, sqlx::Error> {
    serde_json::to_value(value)
        .and_then(serde_json::from_value::<String>)
        .map_err(|error| sqlx::Error::Encode(error.into()))
}

/// Write the state of a person created in this transaction.
pub(crate) async fn provision(
    connection: &mut PgConnection,
    org_id: Uuid,
    user_id: Uuid,
    profile: BuiltInProfile,
) -> Result<(), sqlx::Error> {
    let grants = profile
        .selection()
        .iter()
        .map(stored_id)
        .collect::<Result<Vec<_>, _>>()?;
    sqlx::query!(
        "INSERT INTO person_permission_states
           (id, org_id, user_id, catalog_version, grants, is_administrator, source, built_in_profile)
         VALUES ($1, $2, $3, $4, $5, $6, 'built_in', $7)",
        Uuid::now_v7(),
        org_id,
        user_id,
        PERMISSION_CATALOG_VERSION as i32,
        &grants,
        profile == BuiltInProfile::Administrator,
        stored_id(profile)?,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Whether the person holds the Administrator state and is active.
pub(crate) async fn is_active_administrator(
    connection: &mut PgConnection,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        r#"SELECT EXISTS (
             SELECT 1 FROM person_permission_states p
             JOIN users u ON u.id = p.user_id AND u.org_id = p.org_id
             WHERE p.org_id = $1 AND p.user_id = $2 AND p.is_administrator AND u.active
           ) AS "found!""#,
        org_id,
        user_id,
    )
    .fetch_one(connection)
    .await?;
    Ok(found)
}

/// Whether an active Administrator other than `exclude` remains.
pub(crate) async fn other_active_administrator(
    connection: &mut PgConnection,
    org_id: Uuid,
    exclude: Uuid,
) -> Result<bool, sqlx::Error> {
    let found = sqlx::query_scalar!(
        r#"SELECT EXISTS (
             SELECT 1 FROM person_permission_states p
             JOIN users u ON u.id = p.user_id AND u.org_id = p.org_id
             WHERE p.org_id = $1 AND p.user_id <> $2 AND p.is_administrator AND u.active
           ) AS "found!""#,
        org_id,
        exclude,
    )
    .fetch_one(connection)
    .await?;
    Ok(found)
}

#[cfg(test)]
#[path = "tests/provisioning.rs"]
mod tests;
