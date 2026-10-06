//! Display-only permission facts, never trusted input to server authorization.

use horae_core::permissions::catalog::Permission;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The current person's permission explanation, not an authorization token.
/// Management relationships still require independent action and state checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnPermissions {
    pub catalog_version: u32,
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub access_revision: i64,
    pub person_revision: i64,
    pub managed_person_ids: Vec<Uuid>,
    pub managed_project_ids: Vec<Uuid>,
}
