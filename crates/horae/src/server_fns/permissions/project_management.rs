//! Project delegation changes scope, never grants or tracking membership.

use horae_core::permissions::catalog::Permission;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{PermissionStorageError, load_person_permissions};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename = "replace_project_managers", deny_unknown_fields)]
pub(crate) struct ProjectManagersCommand {
    pub request_id: Uuid,
    pub expected_access_revision: i64,
    pub project_id: Uuid,
    pub manager_ids: Vec<Uuid>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectManagersOutcome {
    pub project_id: Uuid,
    pub access_revision: i64,
    pub changed: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectManagersError {
    #[error("Current project editing authority is required")]
    Forbidden,
    #[error("Project not found")]
    NotFound,
    #[error("The manager selection is not eligible")]
    Ineligible,
    #[error("A manager may only be selected once")]
    Duplicate,
    #[error("Permission state has changed; confirm again")]
    Stale,
    #[error("The project is being edited; retry the complete request")]
    Busy,
    #[error("The request identifier was used for different intent")]
    RequestConflict,
    #[error("Unsupported stored command format")]
    ReceiptVersion,
    #[error("Permission revision exhausted")]
    RevisionExhausted,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Serialize)]
struct Designation {
    id: Uuid,
    manager_id: Uuid,
    revision: i64,
}

#[derive(Serialize)]
struct ScopeChange {
    added: Vec<Designation>,
    removed: Vec<Designation>,
}

#[derive(Serialize)]
struct ProjectManagersAudit {
    project_id: Uuid,
    previous_access_revision: i64,
    access_revision: i64,
    change: Option<ScopeChange>,
}

/// IDs identifying the actor and organization must come from authentication.
/// Internal only until all runtime access paths enforce the canonical policy.
pub(crate) async fn execute(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    request: &ProjectManagersCommand,
) -> Result<ProjectManagersOutcome, ProjectManagersError> {
    let mut tx = pool.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    let org = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id = $1 FOR UPDATE",
        org_id
    ).fetch_optional(&mut *tx).await?.ok_or(ProjectManagersError::Forbidden)?;
    let active = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE org_id = $1 AND id = $2 AND active)",
        org_id,
        actor_id
    )
    .fetch_one(&mut *tx)
    .await?;
    if org.permission_policy_version != 1 || active != Some(true) {
        return Err(ProjectManagersError::Forbidden);
    }
    let actor = load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .ok_or(ProjectManagersError::Forbidden)?;
    if !actor.grants.contains(Permission::ProjectWriteAll) {
        if !actor.grants.contains(Permission::ProjectWriteManaged) {
            return Err(ProjectManagersError::Forbidden);
        }
        let designated = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM project_management_assignments
             WHERE org_id = $1 AND project_id = $2 AND manager_id = $3)",
            org_id,
            request.project_id,
            actor_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if designated != Some(true) {
            return Err(ProjectManagersError::Forbidden);
        }
    }
    let mut intent = request.clone();
    intent.manager_ids.sort_unstable();
    if intent.manager_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProjectManagersError::Duplicate);
    }
    let intent_json = serde_json::to_value(&intent)?;
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
            return Err(ProjectManagersError::ReceiptVersion);
        }
        if receipt.intent != intent_json {
            return Err(ProjectManagersError::RequestConflict);
        }
        let outcome = serde_json::from_value(receipt.result)?;
        tx.commit().await?;
        return Ok(outcome);
    }
    if intent.expected_access_revision != org.access_revision {
        return Err(ProjectManagersError::Stale);
    }
    // The legacy editor holds project UPDATE before requesting organization SHARE.
    // Do not wait for that parent while holding the organization serialization gate.
    let project = sqlx::query!(
        "SELECT id FROM projects WHERE org_id = $1 AND id = $2 FOR KEY SHARE NOWAIT",
        org_id,
        intent.project_id
    )
    .fetch_optional(&mut *tx)
    .await;
    match project {
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("55P03") => {
            tx.rollback().await?;
            return Err(ProjectManagersError::Busy);
        }
        result => {
            result?.ok_or(ProjectManagersError::NotFound)?;
        }
    }
    let existing = sqlx::query_as!(
        Designation,
        "SELECT id, manager_id, revision FROM project_management_assignments
         WHERE org_id = $1 AND project_id = $2 ORDER BY manager_id FOR UPDATE",
        org_id,
        intent.project_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut added = Vec::new();
    for manager_id in &intent.manager_ids {
        if existing
            .binary_search_by_key(manager_id, |row| row.manager_id)
            .is_ok()
        {
            continue;
        }
        let active = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE org_id = $1 AND id = $2 AND active)",
            org_id,
            manager_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if active != Some(true) {
            return Err(ProjectManagersError::Ineligible);
        }
        let target = load_person_permissions(&mut tx, org_id, *manager_id)
            .await?
            .ok_or(ProjectManagersError::Ineligible)?;
        if !target.grants.contains(Permission::ProjectReadManaged)
            && !target.grants.contains(Permission::ProjectReadAll)
        {
            return Err(ProjectManagersError::Ineligible);
        }
        added.push(Designation {
            id: Uuid::now_v7(),
            manager_id: *manager_id,
            revision: 0,
        });
    }
    let removed: Vec<_> = existing
        .into_iter()
        .filter(|row| intent.manager_ids.binary_search(&row.manager_id).is_err())
        .collect();
    let changed = !added.is_empty() || !removed.is_empty();
    let access_revision = if changed {
        org.access_revision
            .checked_add(1)
            .ok_or(ProjectManagersError::RevisionExhausted)?
    } else {
        org.access_revision
    };
    if changed {
        let removed_ids: Vec<_> = removed.iter().map(|row| row.id).collect();
        sqlx::query!(
            "DELETE FROM project_management_assignments WHERE org_id = $1 AND project_id = $2 AND id = ANY($3)",
            org_id, intent.project_id, &removed_ids
        ).execute(&mut *tx).await?;
        for row in &added {
            sqlx::query!(
                "INSERT INTO project_management_assignments (id, org_id, project_id, manager_id)
                 VALUES ($1, $2, $3, $4)",
                row.id,
                org_id,
                intent.project_id,
                row.manager_id
            )
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query!(
            "UPDATE organizations SET access_revision = $2 WHERE id = $1",
            org_id,
            access_revision
        )
        .execute(&mut *tx)
        .await?;
    }
    let result = ProjectManagersOutcome {
        project_id: intent.project_id,
        access_revision,
        changed,
    };
    let audit = ProjectManagersAudit {
        project_id: intent.project_id,
        previous_access_revision: org.access_revision,
        access_revision,
        change: changed.then_some(ScopeChange { added, removed }),
    };
    sqlx::query!(
        "INSERT INTO permission_change_receipts
         (id, org_id, actor_user_id, request_id, format_version, intent, result, audit)
         VALUES ($1, $2, $3, $4, 1, $5, $6, $7)",
        Uuid::now_v7(),
        org_id,
        actor_id,
        intent.request_id,
        intent_json,
        serde_json::to_value(&result)?,
        serde_json::to_value(&audit)?
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result)
}
