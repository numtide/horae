//! The permission history: an administrator-only projection of stored
//! receipts, never a source of current authority.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use uuid::Uuid;

use super::{AUTHORITY_REQUIRED, require_administrator};
use crate::models::permission_audit::{
    AuditCursor, AuditEntry, AuditPage, AuditPrincipal, HistoricalAudit, HistoricalSource,
    PersonSnapshot,
};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::{Actor, OpError, Tx};

/// What the history answers for a receipt it cannot show; the cause is logged.
pub(crate) const HISTORY_UNAVAILABLE: &str = "Permission history is unavailable";
pub(crate) const UNSUPPORTED_FORMAT: &str = "Unsupported audit format";
pub(crate) const INVALID_DOCUMENT: &str = "Invalid historical permission audit";

fn unavailable(reason: &str) -> OpError {
    tracing::error!(reason, "Unable to load permission history");
    OpError::Internal(HISTORY_UNAVAILABLE.into())
}

/// A page of the organization's history, newest first. `expected` is the
/// requester the page was loaded for.
pub(crate) async fn page(
    tx: &mut Tx,
    actor: &Actor,
    after: Option<&AuditCursor>,
    expected: Option<PermissionRequester>,
) -> Result<AuditPage, OpError> {
    let requester = actor.expect_requester(expected, AUTHORITY_REQUIRED)?;
    require_administrator(actor)?;
    let mut rows = sqlx::query_as!(
        StoredAudit,
        r#"SELECT id,actor_user_id,operator_id,operator_command,format_version,audit,
                  created_at AS "created_at: DateTime<Utc>"
           FROM permission_change_receipts
           WHERE org_id=$1
             AND ($2::timestamptz IS NULL OR (created_at,id)<($2,$3))
           ORDER BY created_at DESC,id DESC LIMIT 26"#,
        actor.org_id(),
        after.map(|cursor| cursor.created_at) as Option<DateTime<Utc>>,
        after.map(|cursor| cursor.id)
    )
    .fetch_all(&mut **tx)
    .await?;
    let has_more = rows.len() > 25;
    rows.truncate(25);
    let entries = rows
        .into_iter()
        .map(StoredAudit::project)
        .collect::<Result<Vec<_>, _>>()
        .map_err(unavailable)?;
    let next_after = if has_more {
        entries.last().map(|entry| AuditCursor {
            created_at: entry.created_at,
            id: entry.id,
        })
    } else {
        None
    };
    Ok(AuditPage {
        requester,
        entries,
        next_after,
    })
}

/// One historical record. No receipt lookup or decoding happens before the
/// actor is authorized. No adapter serves it: the audit tests check
/// projection and locking through it.
#[cfg(test)]
pub(crate) async fn read(
    tx: &mut Tx,
    actor: &Actor,
    receipt_id: Uuid,
) -> Result<Option<AuditEntry>, OpError> {
    require_administrator(actor)?;
    let row = sqlx::query_as!(
        StoredAudit,
        "SELECT id, actor_user_id, operator_id, operator_command, format_version, audit,
                created_at as \"created_at: DateTime<Utc>\"
        FROM permission_change_receipts WHERE org_id = $1 AND id = $2",
        actor.org_id(),
        receipt_id
    )
    .fetch_optional(&mut **tx)
    .await?;
    row.map(StoredAudit::project)
        .transpose()
        .map_err(unavailable)
}

struct StoredAudit {
    id: Uuid,
    actor_user_id: Option<Uuid>,
    operator_id: Option<String>,
    operator_command: Option<String>,
    format_version: i32,
    audit: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl StoredAudit {
    fn project(self) -> Result<AuditEntry, &'static str> {
        let actor = match (self.actor_user_id, self.operator_id, self.operator_command) {
            (Some(user_id), None, None) => AuditPrincipal::User { user_id },
            (None, Some(invocation_id), Some(command))
                if !invocation_id.trim().is_empty() && !command.trim().is_empty() =>
            {
                AuditPrincipal::Operator {
                    invocation_id,
                    command,
                }
            }
            _ => return Err(INVALID_DOCUMENT),
        };
        Ok(AuditEntry {
            id: self.id,
            actor,
            created_at: self.created_at,
            audit: decode(self.format_version, self.audit)?,
        })
    }
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

/// Decodes a stored audit document, or names why it cannot be shown
/// ([`UNSUPPORTED_FORMAT`] or [`INVALID_DOCUMENT`]).
pub(crate) fn decode(
    version: i32,
    document: serde_json::Value,
) -> Result<HistoricalAudit, &'static str> {
    if version != 1 {
        return Err(UNSUPPORTED_FORMAT);
    }
    let decoded = if document.get("user_id").is_some() {
        serde_json::from_value(document).map(HistoricalAudit::Profile)
    } else if document.get("project_id").is_some() {
        serde_json::from_value(document).map(HistoricalAudit::ProjectManagers)
    } else {
        serde_json::from_value(document).map(HistoricalAudit::Template)
    }
    .map_err(|_| INVALID_DOCUMENT)?;
    if !decoded.valid() {
        return Err(INVALID_DOCUMENT);
    }
    Ok(decoded)
}
