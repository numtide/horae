//! Permission commands and reads that have not moved to `ops` yet. Their
//! stored state is loaded through `ops::permissions`.

use sqlx::PgConnection;

// The tests below reach the storage functions through `use super::*`.
#[cfg(test)]
use crate::ops::permissions::{
    PermissionStorageError, load_permission_template, load_person_permissions,
};
#[cfg(test)]
use crate::models::permissions::{PermissionSource, PersonPermissions};
#[cfg(test)]
use horae_core::permissions::catalog::{Permission, PermissionSelection};
#[cfg(test)]
use uuid::Uuid;

pub(crate) mod audit;
pub(crate) mod directory;
pub(crate) mod editor;
pub(crate) mod own;
pub(crate) mod profiles;
pub(crate) mod project_management;
pub(crate) mod project_people;
pub(crate) mod templates;

/// Bound authorization transactions independently of pooled connection defaults.
pub(super) async fn configure_administration(
    connection: &mut PgConnection,
) -> Result<(), sqlx::Error> {
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *connection)
        .await?;
    crate::ops::configure_transaction_limits(connection).await
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
