//! Permission command values; callers supply authenticated authority separately.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Confirmed intent; actor identity is supplied separately by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
