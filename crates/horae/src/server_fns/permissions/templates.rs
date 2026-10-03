//! Transactional reusable-profile commands. No runtime endpoint or policy activation.

use horae_core::permissions::catalog::{
    InvalidTemplateName, PERMISSION_CATALOG_VERSION, Permission, PermissionSelection,
    StoredPermissionError, validate_template_name,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{PermissionStorageError, load_person_permissions, restore_grants};

/// Confirmed intent; actor identity is supplied separately by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TemplateCommand {
    pub request_id: Uuid,
    pub expected_access_revision: i64,
    pub action: TemplateAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TemplateAction {
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
pub(crate) struct TemplateOutcome {
    pub template_id: Uuid,
    pub access_revision: i64,
    pub detached_people: usize,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TemplateCommandError {
    #[error("Current administrator authority is required")]
    Forbidden,
    #[error("The permission state has changed; confirm again")]
    Stale,
    #[error("The request identifier was used for different intent")]
    RequestConflict,
    #[error("A profile with this name already exists")]
    NameConflict,
    #[error("At most 50 reusable profiles are allowed")]
    Limit,
    #[error("Profile not found")]
    NotFound,
    #[error("Permission revision exhausted")]
    RevisionExhausted,
    #[error("Unsupported stored command format")]
    ReceiptVersion,
    #[error(transparent)]
    Name(#[from] InvalidTemplateName),
    #[error(transparent)]
    Selection(#[from] StoredPermissionError),
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
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
struct TemplateAudit {
    previous_access_revision: i64,
    access_revision: i64,
    before: Option<TemplateSnapshot>,
    after: Option<TemplateSnapshot>,
    /// Each entry transitions from its recorded template to individual provenance.
    detached_people: Vec<DetachedPerson>,
}

fn increment(revision: i64) -> Result<i64, TemplateCommandError> {
    revision
        .checked_add(1)
        .ok_or(TemplateCommandError::RevisionExhausted)
}

impl TemplateCommand {
    fn canonical(&self) -> Result<Self, TemplateCommandError> {
        let action = match &self.action {
            TemplateAction::Create { name, grants } => TemplateAction::Create {
                name: validate_template_name(name)?.to_owned(),
                grants: PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, grants)?
                    .iter()
                    .collect(),
            },
            action @ TemplateAction::Delete { .. } => action.clone(),
        };
        Ok(Self {
            request_id: self.request_id,
            expected_access_revision: self.expected_access_revision,
            action,
        })
    }
}

/// Authenticates fresh canonical authority, then atomically changes state and history.
/// The organization gate precedes all locks; no user/project DML occurs here.
/// Callers must supply authenticated tenant/actor IDs, never client-selected authority.
pub(crate) async fn execute(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    request: &TemplateCommand,
) -> Result<TemplateOutcome, TemplateCommandError> {
    let mut tx = pool.begin().await?;
    // Fresh reads after the gate wait are required even if the database default changes.
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    let org = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id = $1 FOR UPDATE", org_id
    ).fetch_optional(&mut *tx).await?.ok_or(TemplateCommandError::Forbidden)?;
    let active = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE org_id = $1 AND id = $2 AND active)",
        org_id,
        actor_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if org.permission_policy_version != 1 || active != Some(true) {
        return Err(TemplateCommandError::Forbidden);
    }
    let authority = load_person_permissions(&mut tx, org_id, actor_id).await?;
    if !authority.is_some_and(|state| state.is_administrator) {
        return Err(TemplateCommandError::Forbidden);
    }
    let intent = request.canonical()?;
    if let Some(receipt) = sqlx::query!(
        "SELECT format_version, intent, result FROM permission_change_receipts
         WHERE org_id = $1 AND actor_user_id = $2 AND request_id = $3",
        org_id,
        actor_id,
        intent.request_id
    )
    .fetch_optional(&mut *tx)
    .await?
    {
        if receipt.format_version != 1 {
            return Err(TemplateCommandError::ReceiptVersion);
        }
        if serde_json::from_value::<TemplateCommand>(receipt.intent)? != intent {
            return Err(TemplateCommandError::RequestConflict);
        }
        let result = serde_json::from_value(receipt.result)?;
        tx.commit().await?;
        return Ok(result);
    }
    if intent.expected_access_revision != org.access_revision {
        return Err(TemplateCommandError::Stale);
    }
    let next_revision = increment(org.access_revision)?;
    let (before, after, detached_people) = match &intent.action {
        TemplateAction::Create { name, grants } => (
            None,
            Some(create(&mut tx, org_id, name, grants).await?),
            Vec::new(),
        ),
        TemplateAction::Delete {
            id,
            expected_revision,
        } => {
            let (before, people) = delete(&mut tx, org_id, *id, *expected_revision).await?;
            (Some(before), None, people)
        }
    };
    let template_id = match (&before, &after) {
        (Some(template), _) | (_, Some(template)) => template.id,
        _ => return Err(TemplateCommandError::NotFound),
    };
    let result = TemplateOutcome {
        template_id,
        access_revision: next_revision,
        detached_people: detached_people.len(),
    };
    let audit = TemplateAudit {
        previous_access_revision: org.access_revision,
        access_revision: next_revision,
        before,
        after,
        detached_people,
    };
    sqlx::query!(
        "UPDATE organizations SET access_revision = $2 WHERE id = $1",
        org_id,
        next_revision
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO permission_change_receipts
         (id, org_id, actor_user_id, request_id, format_version, intent, result, audit)
         VALUES ($1, $2, $3, $4, 1, $5, $6, $7)",
        Uuid::now_v7(),
        org_id,
        actor_id,
        intent.request_id,
        serde_json::to_value(&intent)?,
        serde_json::to_value(&result)?,
        serde_json::to_value(&audit)?
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result)
}

async fn create(
    connection: &mut PgConnection,
    org_id: Uuid,
    name: &str,
    grants: &[Permission],
) -> Result<TemplateSnapshot, TemplateCommandError> {
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM permission_templates WHERE org_id = $1",
        org_id
    )
    .fetch_one(&mut *connection)
    .await?;
    if count.is_some_and(|count| count >= 50) {
        return Err(TemplateCommandError::Limit);
    }
    let id = Uuid::now_v7();
    let grant_ids: Vec<String> = serde_json::from_value(serde_json::to_value(grants)?)?;
    sqlx::query!(
        "INSERT INTO permission_templates (id, org_id, name, catalog_version, grants)
         VALUES ($1, $2, $3, 1, $4)",
        id,
        org_id,
        name,
        &grant_ids
    )
    .execute(connection)
    .await
    .map_err(|error| {
        if error
            .as_database_error()
            .is_some_and(|error| error.constraint() == Some("permission_templates_org_name_key"))
        {
            TemplateCommandError::NameConflict
        } else {
            TemplateCommandError::Database(error)
        }
    })?;
    Ok(TemplateSnapshot {
        id,
        name: name.to_owned(),
        catalog_version: 1,
        grants: grants.to_vec(),
        revision: 0,
    })
}

async fn delete(
    connection: &mut PgConnection,
    org_id: Uuid,
    id: Uuid,
    expected_revision: i64,
) -> Result<(TemplateSnapshot, Vec<DetachedPerson>), TemplateCommandError> {
    let row = sqlx::query!(
        "SELECT name, catalog_version, grants, revision FROM permission_templates WHERE org_id = $1 AND id = $2 FOR UPDATE", org_id, id
    ).fetch_optional(&mut *connection).await?.ok_or(TemplateCommandError::NotFound)?;
    if row.revision != expected_revision {
        return Err(TemplateCommandError::Stale);
    }
    let template = TemplateSnapshot {
        id,
        name: row.name,
        catalog_version: row.catalog_version,
        grants: restore_grants(row.catalog_version, &row.grants)?
            .iter()
            .collect(),
        revision: row.revision,
    };
    let rows = sqlx::query!(
        "SELECT user_id, catalog_version, grants, is_administrator, applied_template_revision, revision
         FROM person_permission_states WHERE org_id = $1 AND template_id = $2 ORDER BY user_id FOR UPDATE", org_id, id
    ).fetch_all(&mut *connection).await?;
    let people = rows
        .into_iter()
        .map(|row| {
            Ok(DetachedPerson {
                user_id: row.user_id,
                catalog_version: row.catalog_version,
                grants: restore_grants(row.catalog_version, &row.grants)?
                    .iter()
                    .collect(),
                is_administrator: row.is_administrator,
                previous_template_id: id,
                previous_applied_revision: row
                    .applied_template_revision
                    .ok_or(PermissionStorageError::Provenance)?,
                previous_revision: row.revision,
                revision: increment(row.revision)?,
            })
        })
        .collect::<Result<Vec<_>, TemplateCommandError>>()?;
    sqlx::query!(
        "UPDATE person_permission_states SET source = 'individual', template_id = NULL,
         applied_template_revision = NULL, built_in_profile = NULL, revision = revision + 1
         WHERE org_id = $1 AND template_id = $2",
        org_id,
        id
    )
    .execute(&mut *connection)
    .await?;
    sqlx::query!(
        "DELETE FROM permission_templates WHERE org_id = $1 AND id = $2",
        org_id,
        id
    )
    .execute(connection)
    .await?;
    Ok((template, people))
}
