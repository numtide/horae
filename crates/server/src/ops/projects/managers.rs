//! A project's managers. Designating them changes whose projects a
//! `…Managed` grant covers, never the grants themselves or the team.
//!
//! [`read`] and [`save`] serve the manager editor; the project editor reads
//! and replaces them through [`current`] and [`replace`] in its own save, and
//! finalizing a new project designates its first ones with
//! [`designate_created`].

use horae_core::permissions::catalog::Permission;
use serde::Serialize;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{EDITING_REQUIRED, NOT_FOUND, authorize_write};
use crate::models::permission_editor::PermissionRequester;
use crate::models::project_managers::{
    ProjectManager, ProjectManagers, ProjectManagersCommand, ProjectManagersCommandKind,
    ProjectManagersOutcome,
};
use crate::ops::permissions::{
    PermissionStorageError, REQUEST_CONFLICT, REQUESTER_CHANGED, load_person_permissions,
};
use crate::ops::{Actor, Done, OpError, Tx};

const INELIGIBLE: &str = "The manager selection is not eligible";
const DUPLICATE: &str = "A manager may only be selected once";
const STALE: &str = "Permission state has changed; confirm again";
const BUSY: &str = "The project is being edited; retry the complete request";
/// What the manager editor answers for a fault it does not explain; the cause is logged.
const UNAVAILABLE: &str = "Project managers are unavailable";

fn unavailable(error: impl std::fmt::Display) -> OpError {
    tracing::error!(%error, "Project manager operation failed");
    OpError::Internal(UNAVAILABLE.into())
}

/// The complete current set, including archived people, without
/// revalidating them, for someone who may edit the project.
pub async fn read(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    expected: Option<PermissionRequester>,
) -> Result<ProjectManagers, OpError> {
    actor.expect_requester(expected, REQUESTER_CHANGED)?;
    authorize_write(tx, actor, project_id).await?;
    sqlx::query_scalar!(
        "SELECT id FROM projects WHERE org_id=$1 AND id=$2",
        actor.org_id(),
        project_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    current(tx, actor, project_id).await
}

/// The managers and the access revision they were read at, for an actor
/// already authorized to edit the project. The actor's lock on the
/// organization keeps the revision until the transaction ends.
pub(crate) async fn current(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
) -> Result<ProjectManagers, OpError> {
    let org_id = actor.org_id();
    let access_revision = access_revision(tx, org_id).await?;
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
        requester: actor.requester(),
        project_id,
        access_revision,
        managers,
    })
}

/// Replaces the complete manager set, for the page loaded for `expected`.
/// Unavailable in legacy policy; it does not activate canonical policy.
///
/// Run it under the `AccessChange` organization gate
/// (`ops::begin_gated`), which serializes it with every other access change.
pub async fn save(
    tx: &mut Tx,
    actor: &Actor,
    command: &ProjectManagersCommand,
    expected: PermissionRequester,
) -> Result<Done<ProjectManagersOutcome>, OpError> {
    actor.expect_requester(Some(expected), REQUESTER_CHANGED)?;
    Ok(Done::new(replace(tx, actor, command).await?))
}

/// [`save`] without the requester check, for the project editor, which binds
/// its whole request to one. Runs under the same gate.
pub async fn replace(
    tx: &mut Tx,
    actor: &Actor,
    command: &ProjectManagersCommand,
) -> Result<ProjectManagersOutcome, OpError> {
    let access_revision = access_revision(tx, actor.org_id()).await?;
    authorize_write(tx, actor, command.project_id).await?;
    replace_authorized(tx, actor.org_id(), actor.id(), access_revision, command).await
}

/// Designate the first managers of a project created in this transaction.
/// Creation authority (`ProjectCreateAll`, checked by the caller) stands in for
/// project editing authority, which only a `ProjectWriteAll` holder has before
/// the first designation. The caller holds the AccessChange organization gate.
pub(crate) async fn designate_created(
    tx: &mut Tx,
    org_id: Uuid,
    actor_id: Uuid,
    project_id: Uuid,
    manager_ids: Vec<Uuid>,
) -> Result<ProjectManagersOutcome, OpError> {
    let access_revision = access_revision(tx, org_id).await?;
    let command = ProjectManagersCommand {
        kind: ProjectManagersCommandKind::ReplaceProjectManagers,
        request_id: Uuid::now_v7(),
        expected_access_revision: access_revision,
        project_id,
        manager_ids,
    };
    replace_authorized(tx, org_id, actor_id, access_revision, &command).await
}

async fn access_revision(connection: &mut PgConnection, org_id: Uuid) -> Result<i64, OpError> {
    sqlx::query_scalar!(
        "SELECT access_revision FROM organizations WHERE id = $1",
        org_id
    )
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| OpError::forbidden(EDITING_REQUIRED))
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

/// Eligibility, designation rows, access revision and receipt, once the
/// caller has authorized the actor under the organization gate.
async fn replace_authorized(
    tx: &mut Tx,
    org_id: Uuid,
    actor_id: Uuid,
    previous_access_revision: i64,
    request: &ProjectManagersCommand,
) -> Result<ProjectManagersOutcome, OpError> {
    let mut intent = request.clone();
    intent.manager_ids.sort_unstable();
    if intent.manager_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(OpError::invalid("manager_ids", DUPLICATE));
    }
    let intent_json = serde_json::to_value(&intent).map_err(unavailable)?;
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
            return Err(unavailable("Unsupported stored command format"));
        }
        if receipt.intent != intent_json {
            return Err(OpError::conflict(REQUEST_CONFLICT));
        }
        return serde_json::from_value(receipt.result).map_err(unavailable);
    }
    if intent.expected_access_revision != previous_access_revision {
        return Err(OpError::conflict(STALE));
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
            return Err(OpError::conflict(BUSY));
        }
        result => {
            result?.ok_or_else(|| OpError::not_found(NOT_FOUND))?;
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
            return Err(OpError::invalid("manager_ids", INELIGIBLE));
        }
        let target = load_person_permissions(tx, org_id, *manager_id)
            .await
            .map_err(|error| match error {
                PermissionStorageError::Database(error) => error.into(),
                error => unavailable(error),
            })?
            .ok_or_else(|| OpError::invalid("manager_ids", INELIGIBLE))?;
        if !target.grants.contains(Permission::ProjectReadManaged)
            && !target.grants.contains(Permission::ProjectReadAll)
        {
            return Err(OpError::invalid("manager_ids", INELIGIBLE));
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
        previous_access_revision
            .checked_add(1)
            .ok_or_else(|| unavailable("Permission revision exhausted"))?
    } else {
        previous_access_revision
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
        previous_access_revision,
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
        serde_json::to_value(&result).map_err(unavailable)?,
        serde_json::to_value(&audit).map_err(unavailable)?
    )
    .execute(&mut **tx)
    .await?;
    Ok(result)
}
