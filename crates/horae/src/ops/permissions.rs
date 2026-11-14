//! Stored permission state: loading it and writing it for new people.
//!
//! These functions read and write storage; they decide nothing. To authorize,
//! use [`Actor`](super::Actor), which loads the acting person through
//! [`load_person_permissions`].

use horae_core::permissions::catalog::{Permission, PermissionSelection, StoredPermissionError};
use serde::{Deserialize, de::IntoDeserializer};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::models::permissions::{PermissionSource, PermissionTemplate, PersonPermissions};

pub(crate) mod provisioning;

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

pub(crate) fn restore_grants(
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
    sqlx::query_as!(
        StoredPersonPermissions,
        "SELECT catalog_version, grants, is_administrator, source, built_in_profile,
                template_id, applied_template_revision, revision
         FROM person_permission_states WHERE org_id = $1 AND user_id = $2",
        org_id,
        user_id
    )
    .fetch_optional(connection)
    .await?
    .map(StoredPersonPermissions::restore)
    .transpose()
}

/// Native stored facts restored without inferring grants or provenance.
pub(crate) struct StoredPersonPermissions {
    pub catalog_version: i32,
    pub grants: Vec<String>,
    pub is_administrator: bool,
    pub source: String,
    pub built_in_profile: Option<String>,
    pub template_id: Option<Uuid>,
    pub applied_template_revision: Option<i64>,
    pub revision: i64,
}

impl StoredPersonPermissions {
    pub(crate) fn restore(self) -> Result<PersonPermissions, PermissionStorageError> {
        let row = self;
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
        Ok(PersonPermissions {
            grants: restore_grants(row.catalog_version, &row.grants)?,
            is_administrator: row.is_administrator,
            source,
            revision: row.revision,
        })
    }
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

/// The name a person's access goes by: their built-in profile's label or the
/// template they were given. Individually chosen grants have none.
pub(crate) async fn profile_name(
    connection: &mut PgConnection,
    org_id: Uuid,
    source: PermissionSource,
) -> Result<Option<String>, sqlx::Error> {
    match source {
        PermissionSource::BuiltIn(profile) => Ok(Some(
            crate::components::permission_description::profile_label(profile).to_owned(),
        )),
        PermissionSource::Template { id, .. } => {
            sqlx::query_scalar!(
                "SELECT name FROM permission_templates WHERE org_id = $1 AND id = $2",
                org_id,
                id
            )
            .fetch_optional(connection)
            .await
        }
        PermissionSource::Individual => Ok(None),
    }
}
