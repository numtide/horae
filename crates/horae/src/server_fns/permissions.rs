//! Tests of the stored permission state and the permission editor, which live
//! in `ops::permissions`.

// The tests below reach the storage functions through `use super::*`.
#[cfg(test)]
use crate::models::permissions::{PermissionSource, PersonPermissions};
#[cfg(test)]
use crate::ops::permissions::{load_permission_template, load_person_permissions};
#[cfg(test)]
use horae_core::permissions::catalog::{Permission, PermissionSelection};
#[cfg(test)]
use uuid::Uuid;

/// The permission editor and history as the tests call them.
#[cfg(test)]
#[path = "permissions/tests/calls.rs"]
pub(crate) mod calls;

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
#[path = "permissions/tests/transaction.rs"]
mod transaction_tests;
