//! The permission history: an administrator-only projection of stored
//! receipts, never a source of current authority. Members get no view of it,
//! not even of changes to their own access.

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use uuid::Uuid;

use super::{AUTHORITY_REQUIRED, require_administrator};
use crate::models::permission_audit::{
    AuditCursor, AuditEntry, AuditFilter, AuditKind, AuditName, AuditPage, AuditPeriod,
    AuditPrincipal, HistoricalAudit, HistoricalSource, PersonSnapshot,
};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::{Actor, OpError, Tx};

/// What the history answers for a receipt it cannot show; the cause is logged.
pub const HISTORY_UNAVAILABLE: &str = "Permission history is unavailable";
pub const UNSUPPORTED_FORMAT: &str = "Unsupported audit format";
pub const INVALID_DOCUMENT: &str = "Invalid historical permission audit";

fn unavailable(reason: &str) -> OpError {
    tracing::error!(reason, "Unable to load permission history");
    OpError::Internal(HISTORY_UNAVAILABLE.into())
}

/// A page of the organization's history, newest first, narrowed by `filter`.
/// `expected` is the requester the page was loaded for. Local times are in
/// the requester's zone.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    filter: &AuditFilter,
    after: Option<&AuditCursor>,
    expected: Option<PermissionRequester>,
) -> Result<AuditPage, OpError> {
    let requester = actor.expect_requester(expected, AUTHORITY_REQUIRED)?;
    require_administrator(actor)?;
    // The kind test mirrors `decode`, which tells the documents apart by the same keys.
    let mut rows = sqlx::query_as!(
        StoredAudit,
        r#"SELECT id,actor_user_id,operator_id,operator_command,format_version,audit,
                  created_at AS "created_at: DateTime<Utc>"
           FROM permission_change_receipts
           WHERE org_id=$1
             AND ($2::timestamptz IS NULL OR (created_at,id)<($2,$3))
             AND ($4::text IS NULL OR $4 = CASE
                   WHEN audit->'user_id' IS NOT NULL THEN 'person'
                   WHEN audit->'project_id' IS NOT NULL THEN 'managers'
                   ELSE 'profile' END)
             AND ($5::uuid IS NULL
                  OR actor_user_id=$5
                  OR audit->>'user_id'=$5::text
                  OR audit->'detached_people' @> jsonb_build_array(jsonb_build_object('user_id',$5::uuid))
                  OR audit->'change'->'added' @> jsonb_build_array(jsonb_build_object('manager_id',$5::uuid))
                  OR audit->'change'->'removed' @> jsonb_build_array(jsonb_build_object('manager_id',$5::uuid))
                  OR audit->'change'->'removed_people' @> jsonb_build_array(jsonb_build_object('subject_id',$5::uuid)))
             AND ($6::int IS NULL OR created_at >= now() - make_interval(days => $6))
           ORDER BY created_at DESC,id DESC LIMIT 26"#,
        actor.org_id(),
        after.map(|cursor| cursor.created_at) as Option<DateTime<Utc>>,
        after.map(|cursor| cursor.id),
        filter.kind.map(AuditKind::as_str),
        filter.person,
        filter.period.map(AuditPeriod::days),
    )
    .fetch_all(&mut **tx)
    .await?;
    let has_more = rows.len() > 25;
    rows.truncate(25);
    let zone = viewer_zone(tx, actor).await?;
    let entries = rows
        .into_iter()
        .map(|row| row.project(zone))
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
    let people = sqlx::query_as!(
        AuditName,
        "SELECT id, name FROM users WHERE org_id=$1 ORDER BY name, id",
        actor.org_id()
    )
    .fetch_all(&mut **tx)
    .await?;
    let (project_ids, profile_ids) = referenced(&entries);
    let projects = sqlx::query_as!(
        AuditName,
        "SELECT id, name FROM projects WHERE org_id=$1 AND id=ANY($2)",
        actor.org_id(),
        &project_ids
    )
    .fetch_all(&mut **tx)
    .await?;
    let profiles = sqlx::query_as!(
        AuditName,
        "SELECT id, name FROM permission_templates WHERE org_id=$1 AND id=ANY($2)",
        actor.org_id(),
        &profile_ids
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(AuditPage {
        requester,
        entries,
        next_after,
        time_zone: zone.name().to_owned(),
        today: Utc::now().with_timezone(&zone).date_naive(),
        people,
        projects,
        profiles,
    })
}

/// The actor's zone; one the zone database no longer knows reads as UTC.
async fn viewer_zone(tx: &mut Tx, actor: &Actor) -> Result<Tz, OpError> {
    let name = crate::timezone::person_zone(&mut **tx, actor.id()).await?;
    Ok(crate::timezone::parse(&name).unwrap_or(Tz::UTC))
}

/// The projects and custom profiles `entries` name by id.
fn referenced(entries: &[AuditEntry]) -> (Vec<Uuid>, Vec<Uuid>) {
    let (mut projects, mut profiles) = (Vec::new(), Vec::new());
    for entry in entries {
        match &entry.audit {
            HistoricalAudit::ProjectManagers(audit) => projects.push(audit.project_id),
            HistoricalAudit::Profile(audit) => {
                if let Some(change) = &audit.change {
                    projects.extend(change.removed_projects.iter().map(|link| link.subject_id));
                    for snapshot in [&change.before, &change.after] {
                        if let HistoricalSource::Template { id, .. } = snapshot.source {
                            profiles.push(id);
                        }
                    }
                }
            }
            HistoricalAudit::Template(_) => {}
        }
    }
    (projects, profiles)
}

/// One historical record. No receipt lookup or decoding happens before the
/// actor is authorized. No adapter serves it: the audit tests check
/// projection and locking through it.
#[cfg(any(test, feature = "test-support"))]
pub async fn read(
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
    let zone = viewer_zone(tx, actor).await?;
    row.map(|row| row.project(zone))
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
    fn project(self, zone: Tz) -> Result<AuditEntry, &'static str> {
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
            local: self.created_at.with_timezone(&zone).naive_local(),
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

fn valid_snapshot(snapshot: &PersonSnapshot, version: u32) -> bool {
    let source_valid = match snapshot.source {
        HistoricalSource::Template {
            applied_revision, ..
        } => applied_revision >= 0,
        _ => true,
    };
    snapshot.revision >= 0
        && source_valid
        && PermissionSelection::from_stored(version, &snapshot.grants).is_ok()
}

fn valid_audit(audit: &HistoricalAudit) -> bool {
    match audit {
        HistoricalAudit::Template(audit) => {
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
        HistoricalAudit::Profile(audit) => {
            revision_step(
                audit.previous_access_revision,
                audit.access_revision,
                audit.change.is_some(),
            ) && audit.change.as_ref().is_none_or(|change| {
                valid_snapshot(&change.before, change.catalog_version)
                    && valid_snapshot(&change.after, change.catalog_version)
                    && revision_step(change.before.revision, change.after.revision, true)
                    && change
                        .removed_projects
                        .iter()
                        .chain(&change.removed_people)
                        .all(|link| link.revision >= 0)
            })
        }
        HistoricalAudit::ProjectManagers(audit) => {
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

/// Decodes a stored audit document, or names why it cannot be shown
/// ([`UNSUPPORTED_FORMAT`] or [`INVALID_DOCUMENT`]).
pub fn decode(version: i32, document: serde_json::Value) -> Result<HistoricalAudit, &'static str> {
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
    if !valid_audit(&decoded) {
        return Err(INVALID_DOCUMENT);
    }
    Ok(decoded)
}
