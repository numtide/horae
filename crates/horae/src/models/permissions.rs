//! Server-loaded facts, never a deserializable client authorization payload.

use horae_core::permissions::catalog::{BuiltInProfile, PermissionSelection};
use serde::Serialize;
use uuid::Uuid;

/// Explicit application provenance, independent of computed profile labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub(crate) enum PermissionSource {
    BuiltIn(BuiltInProfile),
    Template { id: Uuid, applied_revision: i64 },
    Individual,
}

/// Canonical saved grants and independently recorded administrative identity.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct PersonPermissions {
    pub grants: PermissionSelection,
    pub is_administrator: bool,
    pub source: PermissionSource,
    pub revision: i64,
}

/// Reusable grants; the name and selection cannot confer administrative identity.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct PermissionTemplate {
    pub name: String,
    pub grants: PermissionSelection,
    pub revision: i64,
}
