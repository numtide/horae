//! Strict permission storage, internal commands and the own-access snapshot reader.

use horae_core::permissions::catalog::{Permission, PermissionSelection, StoredPermissionError};
use serde::{Deserialize, de::IntoDeserializer};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::models::permissions::{PermissionSource, PermissionTemplate, PersonPermissions};

pub(crate) mod audit;
pub(crate) mod directory;
pub(crate) mod editor;
pub(crate) mod own;
pub(crate) mod preflight;
pub(crate) mod profiles;
pub(crate) mod project_management;
pub(crate) mod project_people;
pub(crate) mod templates;
pub(crate) mod time_entries;

/// Bound permission administration independently of pooled connection defaults.
async fn configure_administration(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *connection)
        .await?;
    sqlx::query!(
        "SELECT set_config(name,
            (CASE WHEN setting::bigint = 0 THEN limits.milliseconds
             ELSE LEAST(setting::bigint, limits.milliseconds) END)::text, true)
         FROM pg_settings
         JOIN (VALUES ('statement_timeout', 5000::bigint),
                      ('idle_in_transaction_session_timeout', 10000::bigint))
              AS limits(setting_name, milliseconds) ON name = limits.setting_name"
    )
    .fetch_all(connection)
    .await?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PermissionStorageError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Selection(#[from] StoredPermissionError),
    #[error("Invalid stored permission identifier")]
    Identifier(#[from] serde::de::value::Error),
    #[error("Invalid stored permission provenance")]
    Provenance,
}

fn restore_grants(
    version: i32,
    ids: &[String],
) -> Result<PermissionSelection, PermissionStorageError> {
    let version =
        u32::try_from(version).map_err(|_| StoredPermissionError::UnsupportedCatalogVersion)?;
    let grants = ids
        .iter()
        .map(|id| Permission::deserialize(id.as_str().into_deserializer()))
        .collect::<Result<Vec<_>, serde::de::value::Error>>()?;
    Ok(PermissionSelection::from_stored(version, &grants)?)
}

/// Loads canonical facts for a server-selected tenant/user without repairing them.
/// Missing state is distinct from invalid state. The caller owns authentication,
/// active-policy checks, current-user checks and transactional locks.
pub(crate) async fn load_person_permissions(
    connection: &mut PgConnection,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<Option<PersonPermissions>, PermissionStorageError> {
    let Some(row) = sqlx::query!(
        "SELECT catalog_version, grants, is_administrator, source, built_in_profile,
                template_id, applied_template_revision, revision
         FROM person_permission_states WHERE org_id = $1 AND user_id = $2",
        org_id,
        user_id
    )
    .fetch_optional(connection)
    .await?
    else {
        return Ok(None);
    };
    let source = match (
        row.source.as_str(),
        row.built_in_profile.as_deref(),
        row.template_id,
        row.applied_template_revision,
    ) {
        ("built_in", Some(profile), None, None) => {
            PermissionSource::BuiltIn(Deserialize::deserialize(
                serde::de::value::StrDeserializer::<serde::de::value::Error>::new(profile),
            )?)
        }
        ("template", None, Some(id), Some(applied_revision)) if applied_revision >= 0 => {
            PermissionSource::Template {
                id,
                applied_revision,
            }
        }
        ("individual", None, None, None) => PermissionSource::Individual,
        _ => return Err(PermissionStorageError::Provenance),
    };
    Ok(Some(PersonPermissions {
        grants: restore_grants(row.catalog_version, &row.grants)?,
        is_administrator: row.is_administrator,
        source,
        revision: row.revision,
    }))
}

/// Loads a tenant-scoped reusable selection without deriving person authority.
pub(crate) async fn load_permission_template(
    connection: &mut PgConnection,
    org_id: Uuid,
    template_id: Uuid,
) -> Result<Option<PermissionTemplate>, PermissionStorageError> {
    let Some(row) = sqlx::query!(
        "SELECT name, catalog_version, grants, revision FROM permission_templates
         WHERE org_id = $1 AND id = $2",
        org_id,
        template_id
    )
    .fetch_optional(connection)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(PermissionTemplate {
        name: row.name,
        grants: restore_grants(row.catalog_version, &row.grants)?,
        revision: row.revision,
    }))
}

#[cfg(test)]
#[path = "permissions/tests/storage.rs"]
mod storage_tests;

#[cfg(test)]
#[path = "permissions/tests/templates.rs"]
mod template_tests;

#[cfg(test)]
#[path = "permissions/tests/profiles.rs"]
mod profile_tests;

#[cfg(test)]
#[path = "permissions/tests/project_management.rs"]
mod project_management_tests;

#[cfg(test)]
#[path = "permissions/tests/audit.rs"]
mod audit_tests;

#[cfg(test)]
#[path = "permissions/tests/own.rs"]
mod own_tests;

#[cfg(test)]
#[path = "permissions/tests/preflight.rs"]
mod preflight_tests;
