//! Administrator-only historical projection, never a source of current authority.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::{BuiltInProfile, Permission, PermissionSelection};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{PermissionStorageError, load_person_permissions};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum AuditPrincipal {
    User {
        user_id: Uuid,
    },
    Operator {
        invocation_id: String,
        command: String,
    },
}

#[derive(Debug, Serialize)]
pub(crate) struct AuditEntry {
    pub(super) id: Uuid,
    pub(super) actor: AuditPrincipal,
    created_at: DateTime<Utc>,
    audit: HistoricalAudit,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuditReadError {
    #[error("Current administrator authority is required")]
    Forbidden,
    #[error("Unsupported audit format")]
    UnsupportedFormat,
    #[error("Invalid historical permission audit")]
    InvalidDocument,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub(super) enum HistoricalAudit {
    Template(TemplateAudit),
    Profile(ProfileAudit),
    ProjectManagers(ProjectAudit),
}

// These are versioned historical wire types, not deserializable authority models.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TemplateAudit {
    previous_access_revision: i64,
    access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    before: Option<TemplateSnapshot>,
    #[serde(deserialize_with = "required_option")]
    after: Option<TemplateSnapshot>,
    detached_people: Vec<DetachedPerson>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TemplateSnapshot {
    id: Uuid,
    name: String,
    catalog_version: i32,
    grants: Vec<Permission>,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetachedPerson {
    user_id: Uuid,
    catalog_version: i32,
    grants: Vec<Permission>,
    is_administrator: bool,
    previous_template_id: Uuid,
    previous_applied_revision: i64,
    previous_revision: i64,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileAudit {
    user_id: Uuid,
    previous_access_revision: i64,
    access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    change: Option<ProfileChange>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileChange {
    catalog_version: u32,
    before: PersonSnapshot,
    after: PersonSnapshot,
    removed_projects: Vec<RemovedRelationship>,
    removed_people: Vec<RemovedRelationship>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersonSnapshot {
    grants: Vec<Permission>,
    is_administrator: bool,
    source: HistoricalSource,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum HistoricalSource {
    BuiltIn(BuiltInProfile),
    Template { id: Uuid, applied_revision: i64 },
    Individual,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RemovedRelationship {
    id: Uuid,
    subject_id: Uuid,
    revision: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectAudit {
    project_id: Uuid,
    previous_access_revision: i64,
    access_revision: i64,
    #[serde(deserialize_with = "required_option")]
    change: Option<ProjectChange>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectChange {
    added: Vec<Designation>,
    removed: Vec<Designation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Designation {
    id: Uuid,
    manager_id: Uuid,
    revision: i64,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // A missing field must not be treated as an explicit unchanged outcome.
    Option::<T>::deserialize(deserializer)
}

fn revision_step(before: i64, after: i64, changed: bool) -> bool {
    before >= 0 && before.checked_add(i64::from(changed)) == Some(after)
}

fn valid_grants(version: i32, grants: &[Permission]) -> bool {
    u32::try_from(version)
        .is_ok_and(|version| PermissionSelection::from_stored(version, grants).is_ok())
}

impl PersonSnapshot {
    fn valid(&self, version: u32) -> bool {
        let source_valid = match self.source {
            HistoricalSource::Template {
                applied_revision, ..
            } => applied_revision >= 0,
            _ => true,
        };
        self.revision >= 0
            && source_valid
            && PermissionSelection::from_stored(version, &self.grants).is_ok()
    }
}

impl HistoricalAudit {
    fn valid(&self) -> bool {
        match self {
            Self::Template(audit) => {
                revision_step(audit.previous_access_revision, audit.access_revision, true)
                    && matches!(
                        (&audit.before, &audit.after),
                        (Some(_), None) | (None, Some(_))
                    )
                    && audit.before.iter().chain(&audit.after).all(|snapshot| {
                        snapshot.revision >= 0
                            && valid_grants(snapshot.catalog_version, &snapshot.grants)
                    })
                    && audit.detached_people.iter().all(|person| {
                        audit
                            .before
                            .as_ref()
                            .is_some_and(|template| template.id == person.previous_template_id)
                            && person.previous_applied_revision >= 0
                            && revision_step(person.previous_revision, person.revision, true)
                            && valid_grants(person.catalog_version, &person.grants)
                    })
            }
            Self::Profile(audit) => {
                revision_step(
                    audit.previous_access_revision,
                    audit.access_revision,
                    audit.change.is_some(),
                ) && audit.change.as_ref().is_none_or(|change| {
                    change.before.valid(change.catalog_version)
                        && change.after.valid(change.catalog_version)
                        && revision_step(change.before.revision, change.after.revision, true)
                        && change
                            .removed_projects
                            .iter()
                            .chain(&change.removed_people)
                            .all(|link| link.revision >= 0)
                })
            }
            Self::ProjectManagers(audit) => {
                revision_step(
                    audit.previous_access_revision,
                    audit.access_revision,
                    audit.change.is_some(),
                ) && audit.change.as_ref().is_none_or(|change| {
                    (!change.added.is_empty() || !change.removed.is_empty())
                        && change
                            .added
                            .iter()
                            .chain(&change.removed)
                            .all(|link| link.revision >= 0)
                })
            }
        }
    }
}

pub(super) fn decode(
    version: i32,
    document: serde_json::Value,
) -> Result<HistoricalAudit, AuditReadError> {
    if version != 1 {
        return Err(AuditReadError::UnsupportedFormat);
    }
    let decoded = if document.get("user_id").is_some() {
        serde_json::from_value(document).map(HistoricalAudit::Profile)
    } else if document.get("project_id").is_some() {
        serde_json::from_value(document).map(HistoricalAudit::ProjectManagers)
    } else {
        serde_json::from_value(document).map(HistoricalAudit::Template)
    }
    .map_err(|_| AuditReadError::InvalidDocument)?;
    if !decoded.valid() {
        return Err(AuditReadError::InvalidDocument);
    }
    Ok(decoded)
}

/// Reads one historical record using authenticated requester/tenant IDs.
/// No receipt lookup or snapshot decoding occurs before current authorization.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    requester: Uuid,
    receipt_id: Uuid,
) -> Result<Option<AuditEntry>, AuditReadError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id = $1 FOR SHARE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let active = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE org_id = $1 AND id = $2 AND active)",
        org_id,
        requester
    )
    .fetch_one(&mut *tx)
    .await?;
    if policy != Some(1) || active != Some(true) {
        return Err(AuditReadError::Forbidden);
    }
    if !load_person_permissions(&mut tx, org_id, requester)
        .await?
        .is_some_and(|state| state.is_administrator)
    {
        return Err(AuditReadError::Forbidden);
    }
    let row = sqlx::query!(
        "SELECT id, actor_user_id, operator_id, operator_command, format_version, audit,
                created_at as \"created_at: DateTime<Utc>\"
        FROM permission_change_receipts WHERE org_id = $1 AND id = $2",
        org_id,
        receipt_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let record = row
        .map(|row| {
            let actor = match (row.actor_user_id, row.operator_id, row.operator_command) {
                (Some(user_id), None, None) => AuditPrincipal::User { user_id },
                (None, Some(invocation_id), Some(command))
                    if !invocation_id.trim().is_empty() && !command.trim().is_empty() =>
                {
                    AuditPrincipal::Operator {
                        invocation_id,
                        command,
                    }
                }
                _ => return Err(AuditReadError::InvalidDocument),
            };
            Ok(AuditEntry {
                id: row.id,
                actor,
                created_at: row.created_at,
                audit: decode(row.format_version, row.audit)?,
            })
        })
        .transpose()?;
    tx.commit().await?;
    Ok(record)
}
