//! Permission state for people as they are created or change legacy role.
//!
//! Every path that creates a person writes a built-in profile state, whatever
//! the organization's current policy, so that switching an organization to
//! policy 1 needs no data migration. Under policy 0 nothing reads these rows.

use horae_core::permissions::catalog::{BuiltInProfile, PERMISSION_CATALOG_VERSION};
use horae_core::types::OrgRole;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

/// Permission policy given to organizations created by `horae init` and
/// `horae seed`. It stays 0 until every operation authorizes from permission
/// state; flipping it here is the whole switch for new installations.
pub(crate) const NEW_ORGANIZATION_POLICY_VERSION: i32 = 0;

/// The built-in profile a legacy role provisions.
///
/// 015 has no profile equal to the legacy Manager. Executive Manager is the
/// closest standard one: it keeps every project, report, rate, invoice, client
/// and approval the Manager reaches, adds people administration and editing
/// others' time, and lacks rate editing and company settings.
pub(crate) const fn profile_for_role(role: OrgRole) -> BuiltInProfile {
    match role {
        OrgRole::Admin => BuiltInProfile::Administrator,
        OrgRole::Manager => BuiltInProfile::ExecutiveManager,
        OrgRole::Member => BuiltInProfile::Member,
    }
}

/// The identifier a catalog value is stored under.
fn stored_id(value: impl Serialize) -> Result<String, sqlx::Error> {
    serde_json::to_value(value)
        .and_then(serde_json::from_value::<String>)
        .map_err(|error| sqlx::Error::Encode(error.into()))
}

struct StoredProfile {
    grants: Vec<String>,
    is_administrator: bool,
    profile: String,
}

impl StoredProfile {
    fn new(profile: BuiltInProfile) -> Result<Self, sqlx::Error> {
        Ok(Self {
            grants: profile
                .selection()
                .iter()
                .map(stored_id)
                .collect::<Result<_, _>>()?,
            is_administrator: profile == BuiltInProfile::Administrator,
            profile: stored_id(profile)?,
        })
    }
}

/// Write the state of a person created in this transaction.
pub(crate) async fn provision(
    connection: &mut PgConnection,
    org_id: Uuid,
    user_id: Uuid,
    profile: BuiltInProfile,
) -> Result<(), sqlx::Error> {
    let stored = StoredProfile::new(profile)?;
    sqlx::query!(
        "INSERT INTO person_permission_states
           (id, org_id, user_id, catalog_version, grants, is_administrator, source, built_in_profile)
         VALUES ($1, $2, $3, $4, $5, $6, 'built_in', $7)",
        Uuid::now_v7(),
        org_id,
        user_id,
        PERMISSION_CATALOG_VERSION as i32,
        &stored.grants,
        stored.is_administrator,
        stored.profile,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// Replace a person's state with the profile of their new legacy role.
///
/// Only for policy 0, where roles still decide and nobody can edit permission
/// state, so nothing chosen in the permission editor is overwritten. That
/// holds only because an organization never returns from policy 1 to 0: after
/// such a return, this would replace an editor-chosen template or selection
/// without an audit receipt. A person created before provisioning existed gets
/// their first state here.
pub(crate) async fn follow_role(
    connection: &mut PgConnection,
    org_id: Uuid,
    user_id: Uuid,
    role: OrgRole,
) -> Result<(), sqlx::Error> {
    let stored = StoredProfile::new(profile_for_role(role))?;
    sqlx::query!(
        "INSERT INTO person_permission_states
           (id, org_id, user_id, catalog_version, grants, is_administrator, source, built_in_profile)
         VALUES ($1, $2, $3, $4, $5, $6, 'built_in', $7)
         ON CONFLICT (org_id, user_id) DO UPDATE SET
           catalog_version = EXCLUDED.catalog_version, grants = EXCLUDED.grants,
           is_administrator = EXCLUDED.is_administrator, source = 'built_in',
           built_in_profile = EXCLUDED.built_in_profile, template_id = NULL,
           applied_template_revision = NULL,
           revision = person_permission_states.revision + 1
         WHERE (person_permission_states.catalog_version, person_permission_states.grants,
                person_permission_states.is_administrator, person_permission_states.source,
                person_permission_states.built_in_profile)
           IS DISTINCT FROM
               (EXCLUDED.catalog_version, EXCLUDED.grants, EXCLUDED.is_administrator,
                EXCLUDED.source, EXCLUDED.built_in_profile)",
        Uuid::now_v7(),
        org_id,
        user_id,
        PERMISSION_CATALOG_VERSION as i32,
        &stored.grants,
        stored.is_administrator,
        stored.profile,
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
