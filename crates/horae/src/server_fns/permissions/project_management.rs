//! Project delegation changes scope, never grants or tracking membership.

use horae_core::permissions::catalog::Permission;
use serde::Serialize;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{PermissionStorageError, load_person_permissions};

use crate::models::permission_editor::PermissionRequester;
use crate::models::project_managers::{ProjectManager, ProjectManagers};
pub(crate) use crate::models::project_managers::{ProjectManagersCommand, ProjectManagersOutcome};

/// Materialize the complete current set without revalidating retained targets.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
) -> Result<ProjectManagers, ProjectManagersError> {
    let mut tx = pool.begin().await?;
    super::configure_administration(&mut tx).await?;
    let result = read_in_transaction(&mut tx, org_id, actor_id, project_id).await?;
    tx.commit().await?;
    Ok(result)
}

/// Read delegation in the same authorized snapshot as the rest of the editor.
pub(crate) async fn read_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
) -> Result<ProjectManagers, ProjectManagersError> {
    let org = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id=$1 FOR SHARE",
        org_id
    ).fetch_optional(&mut **tx).await?.ok_or(ProjectManagersError::Forbidden)?;
    if org.permission_policy_version != 1 {
        return Err(ProjectManagersError::Forbidden);
    }
    authorize_actor(tx, org_id, actor_id, project_id).await?;
    sqlx::query_scalar!(
        "SELECT id FROM projects WHERE org_id=$1 AND id=$2",
        org_id,
        project_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ProjectManagersError::NotFound)?;
    let managers = sqlx::query_as!(
        ProjectManager,
        "SELECT u.id,u.name,u.active FROM project_management_assignments m
         JOIN users u ON u.org_id=m.org_id AND u.id=m.manager_id
         WHERE m.org_id=$1 AND m.project_id=$2 ORDER BY u.id",
        org_id,
        project_id
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(ProjectManagers {
        requester: PermissionRequester {
            org_id,
            user_id: actor_id,
        },
        project_id,
        access_revision: org.access_revision,
        managers,
    })
}

/// The caller holds the organization gate before checking current project authority.
pub(crate) async fn authorize_actor(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
) -> Result<crate::models::permissions::PersonPermissions, ProjectManagersError> {
    let active = sqlx::query_scalar!(
        "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *connection)
    .await?;
    if active != Some(true) {
        return Err(ProjectManagersError::Forbidden);
    }
    let actor = load_person_permissions(connection, org_id, actor_id)
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
            project_id,
            actor_id
        )
        .fetch_one(connection)
        .await?;
        if designated != Some(true) {
            return Err(ProjectManagersError::Forbidden);
        }
    }
    Ok(actor)
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
/// Unavailable in legacy policy; this operation does not activate canonical policy.
pub(crate) async fn execute(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    request: &ProjectManagersCommand,
) -> Result<ProjectManagersOutcome, ProjectManagersError> {
    let mut tx = pool.begin().await?;
    super::configure_administration(&mut tx).await?;
    match execute_in_transaction(&mut tx, org_id, actor_id, request).await {
        Ok(result) => {
            tx.commit().await?;
            Ok(result)
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}

/// Compose delegation with project edits without committing either independently.
/// The caller configures transaction limits and must roll back on any error.
/// Acquire the AccessChange organization gate before actor/project locks in the
/// caller. Reacquiring that same mode here preserves foreign-key compatibility.
pub(crate) async fn execute_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
    request: &ProjectManagersCommand,
) -> Result<ProjectManagersOutcome, ProjectManagersError> {
    crate::db::lock_organization(tx, org_id, crate::db::OrganizationLock::AccessChange)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => ProjectManagersError::Forbidden,
            error => ProjectManagersError::Database(error),
        })?;
    let org = sqlx::query!(
        "SELECT permission_policy_version, access_revision FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ProjectManagersError::Forbidden)?;
    if org.permission_policy_version != 1 {
        return Err(ProjectManagersError::Forbidden);
    }
    authorize_actor(tx, org_id, actor_id, request.project_id).await?;
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
    .fetch_optional(&mut **tx)
    .await?
    {
        if receipt.format_version != 1 {
            return Err(ProjectManagersError::ReceiptVersion);
        }
        if receipt.intent != intent_json {
            return Err(ProjectManagersError::RequestConflict);
        }
        return Ok(serde_json::from_value(receipt.result)?);
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
    .fetch_optional(&mut **tx)
    .await;
    match project {
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("55P03") => {
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
    .fetch_all(&mut **tx)
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
            "SELECT active FROM users WHERE org_id = $1 AND id = $2 FOR SHARE",
            org_id,
            manager_id
        )
        .fetch_optional(&mut **tx)
        .await?;
        if active != Some(true) {
            return Err(ProjectManagersError::Ineligible);
        }
        let target = load_person_permissions(tx, org_id, *manager_id)
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
        ).execute(&mut **tx).await?;
        for row in &added {
            sqlx::query!(
                "INSERT INTO project_management_assignments (id, org_id, project_id, manager_id)
                 VALUES ($1, $2, $3, $4)",
                row.id,
                org_id,
                intent.project_id,
                row.manager_id
            )
            .execute(&mut **tx)
            .await?;
        }
        sqlx::query!(
            "UPDATE organizations SET access_revision = $2 WHERE id = $1",
            org_id,
            access_revision
        )
        .execute(&mut **tx)
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
    .execute(&mut **tx)
    .await?;
    Ok(result)
}
