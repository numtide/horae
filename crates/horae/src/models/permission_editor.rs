//! Permission editor wire values; the server always reloads current authority.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Confirmed intent; actor identity is supplied separately by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Editor transport; the Workspace consumer is not connected yet"
    )
)]
pub struct TemplateCommand {
    pub request_id: Uuid,
    pub expected_access_revision: i64,
    pub action: TemplateAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TemplateAction {
    Create {
        name: String,
        grants: Vec<Permission>,
    },
    Delete {
        id: Uuid,
        expected_revision: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Editor transport; the Workspace consumer is not connected yet"
    )
)]
pub struct TemplateOutcome {
    pub template_id: Uuid,
    pub access_revision: i64,
    pub detached_people: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileCommand {
    pub request_id: Uuid,
    pub expected_access_revision: i64,
    pub user_id: Uuid,
    pub expected_person_revision: i64,
    pub action: ProfileAction,
    pub grants: Vec<Permission>,
    pub remove_projects: Vec<Uuid>,
    pub remove_people: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProfileAction {
    Edit,
    BuiltIn { profile: BuiltInProfile },
    Template { id: Uuid, expected_revision: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileOutcome {
    pub user_id: Uuid,
    pub access_revision: i64,
    pub person_revision: i64,
    pub changed: bool,
}

/// Unsaved explicit proposal; revisions fence confirmation, not authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDraft {
    pub user_id: Uuid,
    pub expected_access_revision: i64,
    pub expected_person_revision: i64,
    pub action: ProfileAction,
    pub grants: Vec<Permission>,
}

/// Display provenance, never accepted as authoritative permission state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProfileSource {
    BuiltIn(BuiltInProfile),
    Template { id: Uuid, applied_revision: i64 },
    Individual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionSnapshot {
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub source: ProfileSource,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateChoice {
    pub id: Uuid,
    pub name: String,
    pub grants: Vec<Permission>,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionEditor {
    pub user_id: Uuid,
    pub name: String,
    pub active: bool,
    pub access_revision: i64,
    pub permissions: PermissionSnapshot,
    pub templates: Vec<TemplateChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationshipRemoval {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePreview {
    pub user_id: Uuid,
    pub access_revision: i64,
    pub before: PermissionSnapshot,
    pub after: PermissionSnapshot,
    pub changed: bool,
    pub remove_projects: Vec<RelationshipRemoval>,
    pub remove_people: Vec<RelationshipRemoval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateAssignee {
    pub user_id: Uuid,
    pub name: String,
    pub permissions: PermissionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Editor transport; the Workspace consumer is not connected yet"
    )
)]
pub struct TemplateDeletionPreview {
    pub access_revision: i64,
    pub template: TemplateChoice,
    pub people: Vec<TemplateAssignee>,
}
