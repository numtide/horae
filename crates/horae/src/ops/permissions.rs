//! Stored permission state: loading it, writing it for new people, and the
//! administrator's permission editor and history over it.
//!
//! The storage functions here decide nothing. To authorize, use
//! [`Actor`](super::Actor), which loads the acting person through
//! [`load_person_permissions`]. The editor ([`editor`], [`profiles`],
//! [`templates`]) and the history ([`audit`]) serve current administrators only.

use horae_core::permissions::catalog::{
    PERMISSION_CATALOG_VERSION, Permission, PermissionSelection, StoredPermissionError,
    profile_label,
};
use serde::{Deserialize, de::IntoDeserializer};
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Actor, OpError};
use crate::models::permissions::{PermissionSource, PermissionTemplate, PersonPermissions};

pub(crate) mod audit;
pub(crate) mod editor;
pub(crate) mod profiles;
pub(crate) mod provisioning;
pub(crate) mod templates;

/// The editor's and the history's refusal for anyone but a current administrator.
pub(crate) const AUTHORITY_REQUIRED: &str = "Current administrator authority is required";
/// A save's refusal when the page was loaded for someone else.
pub(crate) const REQUESTER_CHANGED: &str = "Sign in as the original requester before retrying";
pub(crate) const REQUEST_CONFLICT: &str = "The request identifier was used for different intent";
/// What the editor answers for a fault it does not explain; the cause is logged.
pub(crate) const EDITOR_UNAVAILABLE: &str = "Permission editor is unavailable";

/// Refuses anyone but a current administrator, in the editor's words rather
/// than [`Actor::require_administrator`]'s, which the page already shows.
fn require_administrator(actor: &Actor) -> Result<(), OpError> {
    if actor.administrator() {
        Ok(())
    } else {
        Err(OpError::forbidden(AUTHORITY_REQUIRED))
    }
}

/// Logs a fault the editor does not explain and answers it as unavailable.
fn unavailable(error: impl std::fmt::Display) -> OpError {
    tracing::error!(%error, "Permission editor operation failed");
    OpError::Internal(EDITOR_UNAVAILABLE.into())
}

/// Someone else's stored state failed to load: a storage error maps like any
/// other, invalid state is a fault the editor does not explain.
fn stored(error: PermissionStorageError) -> OpError {
    match error {
        PermissionStorageError::Database(error) => error.into(),
        error => unavailable(error),
    }
}

/// The confirmed grants, refused as given rather than repaired.
fn selection(grants: &[Permission]) -> Result<PermissionSelection, OpError> {
    PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, grants)
        .map_err(|error| OpError::invalid("grants", error))
}

fn increment(revision: i64) -> Result<i64, OpError> {
    revision
        .checked_add(1)
        .ok_or_else(|| unavailable("Permission revision exhausted"))
}

/// The organization's access revision. The actor's `FOR SHARE` lock, or a
/// save's `AccessChange` gate, keeps it until the transaction ends.
async fn access_revision(connection: &mut PgConnection, org_id: Uuid) -> Result<i64, OpError> {
    Ok(sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_one(connection)
    .await?)
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
        PermissionSource::BuiltIn(profile) => Ok(Some(profile_label(profile).to_owned())),
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
