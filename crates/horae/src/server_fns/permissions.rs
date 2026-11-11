//! Strict permission storage and internal commands; policy activation is separate.

use horae_core::permissions::catalog::{Permission, PermissionSelection, StoredPermissionError};
use serde::{Deserialize, de::IntoDeserializer};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
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
pub(crate) mod time_reports;

/// Bound authorization transactions independently of pooled connection defaults.
pub(super) async fn configure_administration(
    connection: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *connection)
        .await?;
    configure_transaction_limits(connection).await
}

#[derive(Clone, Copy)]
pub(crate) enum ReadIsolation {
    ReadCommitted,
    RepeatableRead,
}

/// Begin a read whose only writes are `FOR SHARE` authority locks, with the
/// isolation named in `BEGIN` itself rather than a separate `SET TRANSACTION`.
///
/// No INSERT/UPDATE/DELETE/nextval: this transaction commits asynchronously.
/// READ WRITE is what lets the locks be taken (READ ONLY rejects `FOR SHARE`),
/// so Postgres cannot reject a write here; a data write would still commit,
/// but could be lost on a server crash shortly after its commit returned.
pub(crate) async fn begin_lock_only_read_async_commit(
    pool: &PgPool,
    isolation: ReadIsolation,
) -> Result<Transaction<'static, Postgres>, sqlx::Error> {
    let begin = match isolation {
        ReadIsolation::ReadCommitted => "BEGIN ISOLATION LEVEL READ COMMITTED, READ WRITE",
        ReadIsolation::RepeatableRead => "BEGIN ISOLATION LEVEL REPEATABLE READ, READ WRITE",
    };
    let mut tx = pool.begin_with(begin).await?;
    configure_limits(&mut tx, Commit::LockOnly).await?;
    Ok(tx)
}

/// Set local limits only after the caller selects its transaction isolation.
pub(crate) async fn configure_transaction_limits(
    connection: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    configure_limits(connection, Commit::Durable).await
}

#[derive(Clone, Copy, PartialEq)]
enum Commit {
    Durable,
    LockOnly,
}

async fn configure_limits(
    connection: &mut PgConnection,
    commit: Commit,
) -> Result<(), sqlx::Error> {
    // `current_setting` instead of `pg_settings`, which materializes every GUC
    // on each call. Millisecond settings always display with a unit Postgres
    // also accepts as interval input (ms, s, min, h, d), or as a bare 0.
    //
    // A lock-only read commits without waiting for its WAL flush. Its FOR
    // SHARE locks give it an xid, so a synchronous commit costs an fsync even
    // though nothing durable was written. Locks are held and released exactly
    // as before, and commit visibility does not depend on the flush, so
    // concurrent access changes still wait for, or invalidate, the read. A
    // crash can only lose the commit record of a transaction that changed no
    // data, and its locks die with the server either way.
    sqlx::query!(
        "SELECT set_config(name, value, true)
         FROM (
           SELECT limits.name,
             (CASE WHEN current.milliseconds = 0 THEN limits.milliseconds
              ELSE LEAST(current.milliseconds, limits.milliseconds) END)::text AS value
           FROM (VALUES ('statement_timeout', 5000::bigint),
                        ('idle_in_transaction_session_timeout', 10000::bigint))
                AS limits(name, milliseconds)
           CROSS JOIN LATERAL (SELECT (extract(epoch FROM current_setting(limits.name)::interval)
                                       * 1000)::bigint AS milliseconds) current
           UNION ALL
           SELECT 'synchronous_commit', 'off' WHERE $1
         ) settings",
        commit == Commit::LockOnly,
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
#[path = "permissions/tests/preflight.rs"]
mod preflight_tests;

#[cfg(test)]
#[path = "permissions/tests/audit.rs"]
mod audit_tests;

#[cfg(test)]
#[path = "permissions/tests/project_management.rs"]
mod project_management_tests;

#[cfg(test)]
#[path = "permissions/tests/own.rs"]
mod own_tests;

#[cfg(test)]
#[path = "permissions/tests/transaction.rs"]
mod transaction_tests;
