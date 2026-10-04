//! Historical permission-change projection. Never use this DTO as current authority.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditPrincipal {
    User {
        user_id: Uuid,
    },
    Operator {
        invocation_id: String,
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    not(feature = "server"),
    expect(
        dead_code,
        reason = "Shared audit response; the history UI is not connected yet"
    )
)]
pub struct AuditEntry {
    pub id: Uuid,
    pub actor: AuditPrincipal,
    pub created_at: DateTime<Utc>,
    pub audit: HistoricalAudit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub enum HistoricalAudit {
    Template(TemplateAudit),
    Profile(ProfileAudit),
    ProjectManagers(ProjectAudit),
}

// These are versioned historical wire types, not deserializable authority models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateAudit {
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub before: Option<TemplateSnapshot>,
    #[serde(deserialize_with = "required_option")]
    pub after: Option<TemplateSnapshot>,
    pub detached_people: Vec<DetachedPerson>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateSnapshot {
    pub id: Uuid,
    pub name: String,
    pub catalog_version: i32,
    pub grants: Vec<Permission>,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DetachedPerson {
    pub user_id: Uuid,
    pub catalog_version: i32,
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub previous_template_id: Uuid,
    pub previous_applied_revision: i64,
    pub previous_revision: i64,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileAudit {
    pub user_id: Uuid,
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub change: Option<ProfileChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileChange {
    pub catalog_version: u32,
    pub before: PersonSnapshot,
    pub after: PersonSnapshot,
    pub removed_projects: Vec<RemovedRelationship>,
    pub removed_people: Vec<RemovedRelationship>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSnapshot {
    pub grants: Vec<Permission>,
    pub is_administrator: bool,
    pub source: HistoricalSource,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum HistoricalSource {
    BuiltIn(BuiltInProfile),
    Template { id: Uuid, applied_revision: i64 },
    Individual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemovedRelationship {
    pub id: Uuid,
    pub subject_id: Uuid,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectAudit {
    pub project_id: Uuid,
    pub previous_access_revision: i64,
    pub access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    pub change: Option<ProjectChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectChange {
    pub added: Vec<Designation>,
    pub removed: Vec<Designation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Designation {
    pub id: Uuid,
    pub manager_id: Uuid,
    pub revision: i64,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // A missing field must not be treated as an explicit unchanged outcome.
    Option::<T>::deserialize(deserializer)
}
