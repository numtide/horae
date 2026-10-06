//! Reusable-profile command values; callers supply authenticated authority separately.

use horae_core::permissions::catalog::Permission;
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
