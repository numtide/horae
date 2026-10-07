//! Administrator-only historical projection, never a source of current authority.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{PermissionStorageError, load_person_permissions};
use crate::models::permission_audit::{
    AuditCursor, AuditEntry, AuditPage, HistoricalSource, PersonSnapshot,
};
pub(super) use crate::models::permission_audit::{AuditPrincipal, HistoricalAudit};
use crate::models::permission_editor::PermissionRequester;

pub(crate) async fn page(
    pool: &PgPool,
    org_id: Uuid,
    requester_id: Uuid,
    after: Option<&AuditCursor>,
    expected_requester: Option<PermissionRequester>,
) -> Result<AuditPage, AuditReadError> {
    let requester = PermissionRequester {
        org_id,
        user_id: requester_id,
    };
    if expected_requester.is_some_and(|expected| expected != requester) {
        return Err(AuditReadError::Forbidden);
    }
    let mut tx = begin_read(pool, org_id, requester_id).await?;
    let mut rows = sqlx::query_as!(
        StoredAudit,
        r#"SELECT id,actor_user_id,operator_id,operator_command,format_version,audit,
                  created_at AS "created_at: DateTime<Utc>"
           FROM permission_change_receipts
           WHERE org_id=$1
             AND ($2::timestamptz IS NULL OR (created_at,id)<($2,$3))
           ORDER BY created_at DESC,id DESC LIMIT 26"#,
        org_id,
        after.map(|cursor| cursor.created_at) as Option<DateTime<Utc>>,
        after.map(|cursor| cursor.id)
    )
    .fetch_all(&mut *tx)
    .await?;
    let has_more = rows.len() > 25;
    rows.truncate(25);
    let entries = rows
        .into_iter()
        .map(StoredAudit::project)
        .collect::<Result<Vec<_>, _>>()?;
    let next_after = if has_more {
        entries.last().map(|entry| AuditCursor {
            created_at: entry.created_at,
            id: entry.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(AuditPage {
        requester,
        entries,
        next_after,
    })
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
    fn project(self) -> Result<AuditEntry, AuditReadError> {
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
            _ => return Err(AuditReadError::InvalidDocument),
        };
        Ok(AuditEntry {
            id: self.id,
            actor,
            created_at: self.created_at,
            audit: decode(self.format_version, self.audit)?,
        })
    }
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

/// Hold the same current authority through either historical projection.
async fn begin_read(
    pool: &PgPool,
    org_id: Uuid,
    requester: Uuid,
) -> Result<Transaction<'_, Postgres>, AuditReadError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "SELECT set_config(name,
            (CASE WHEN setting::bigint = 0 THEN limits.milliseconds
             ELSE LEAST(setting::bigint, limits.milliseconds) END)::text, true)
         FROM pg_settings
         JOIN (VALUES ('statement_timeout', 5000::bigint),
                      ('idle_in_transaction_session_timeout', 10000::bigint))
              AS limits(setting_name, milliseconds) ON name = limits.setting_name"
    )
    .fetch_all(&mut *tx)
    .await?;
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id = $1 FOR SHARE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let active = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        requester
    )
    .fetch_optional(&mut *tx)
    .await?;
    if policy != Some(1) || active.is_none() {
        return Err(AuditReadError::Forbidden);
    }
    if !load_person_permissions(&mut tx, org_id, requester)
        .await?
        .is_some_and(|state| state.is_administrator)
    {
        return Err(AuditReadError::Forbidden);
    }
    Ok(tx)
}

/// Reads one historical record using authenticated requester/tenant IDs.
/// No receipt lookup or snapshot decoding occurs before current authorization.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    requester: Uuid,
    receipt_id: Uuid,
) -> Result<Option<AuditEntry>, AuditReadError> {
    let mut tx = begin_read(pool, org_id, requester).await?;
    let row = sqlx::query_as!(
        StoredAudit,
        "SELECT id, actor_user_id, operator_id, operator_command, format_version, audit,
                created_at as \"created_at: DateTime<Utc>\"
        FROM permission_change_receipts WHERE org_id = $1 AND id = $2",
        org_id,
        receipt_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let record = row.map(StoredAudit::project).transpose()?;
    tx.commit().await?;
    Ok(record)
}
