//! Project operations: archiving and restoring (FR-011).
//!
//! Every decision goes through [`Actor::has`], on the actor's stored grants.

use horae_core::permissions::catalog::Permission;
use horae_core::types::{BudgetKind, ProjectType};
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::Project;
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, active_transition};

const NOT_FOUND: &str = "Project not found";
const WRITE_REQUIRED: &str = "Project editing access required";
const RESTORE_CLIENT_FIRST: &str = "Restore the client before restoring this project.";

/// Whether the actor may write projects at all: everyone's, or the ones they
/// manage. Offered as the status affordance before a project is chosen.
pub(crate) fn writes_any(actor: &Actor) -> bool {
    actor.has(Permission::ProjectWriteAll) || actor.has(Permission::ProjectWriteManaged)
}

/// Archives or restores each project in `sorted_ids`, in that order, so
/// overlapping batches take their row locks in one global order. A project
/// is written with `ProjectWriteAll`, or with `ProjectWriteManaged` when the
/// actor manages it. Restoring needs the project's client active: a client
/// archive locks the same project rows, so the two serialize here.
///
/// Locks: the organization and the person ([`Actor::load`]), then each
/// project `FOR UPDATE`, then its client `FOR SHARE`.
pub(crate) async fn set_active(
    tx: &mut Tx,
    actor: &Actor,
    sorted_ids: &[Uuid],
    active: bool,
) -> Result<Done<Vec<Project>>, OpError> {
    if !actor.is_active() || !writes_any(actor) {
        return Err(OpError::forbidden(WRITE_REQUIRED));
    }
    let org_id = actor.org_id();
    let mut projects = Vec::with_capacity(sorted_ids.len());
    let mut events = Vec::new();
    for &project_id in sorted_ids {
        let before = lock(tx, org_id, project_id).await?;
        if !actor.has(Permission::ProjectWriteAll) && !manages(tx, actor, project_id).await? {
            return Err(OpError::forbidden(WRITE_REQUIRED));
        }
        if active && !before.active {
            let client_active = sqlx::query_scalar!(
                "SELECT active FROM clients WHERE id = $1 AND org_id = $2 FOR SHARE",
                before.client_id,
                org_id,
            )
            .fetch_one(&mut **tx)
            .await?;
            if !client_active {
                return Err(OpError::conflict(RESTORE_CLIENT_FIRST));
            }
        }
        let updated = sqlx::query_as!(
            Project,
            r#"UPDATE projects SET active = $3
              WHERE id = $1 AND org_id = $2 AND active IS DISTINCT FROM $3
             RETURNING id, org_id, client_id, code, name,
                       project_type as "project_type: ProjectType", currency, rate_cents,
                       starts_on as "starts_on: chrono::NaiveDate",
                       ends_on as "ends_on: chrono::NaiveDate",
                       budget_kind as "budget_kind: BudgetKind",
                       budget_amount_cents, budget_minutes, active,
                       created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
            project_id,
            org_id,
            active,
        )
        .fetch_optional(&mut **tx)
        .await?;
        let transition = updated
            .as_ref()
            .and_then(|project| active_transition(Some(before.active), project.active));
        let project = updated.unwrap_or(before);
        if let Some(transition) = transition {
            events.push(event(org_id, &project, transition));
        }
        projects.push(project);
    }
    let mut done = Done::new(projects);
    done.events = events;
    Ok(done)
}

fn event(org_id: Uuid, project: &Project, transition: ActiveTransition) -> AppEvent {
    let occurred_at = chrono::Utc::now();
    let project = crate::server_fns::project_payload(project);
    match transition {
        ActiveTransition::Reactivated => AppEvent::ProjectReactivated {
            occurred_at,
            org_id,
            project,
        },
        ActiveTransition::Deactivated => AppEvent::ProjectDeactivated {
            occurred_at,
            org_id,
            project,
        },
    }
}

async fn manages(tx: &mut Tx, actor: &Actor, project_id: Uuid) -> Result<bool, OpError> {
    Ok(actor.has(Permission::ProjectWriteManaged)
        && sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM project_management_assignments
                WHERE org_id = $1 AND project_id = $2 AND manager_id = $3) AS "managed!""#,
            actor.org_id(),
            project_id,
            actor.id(),
        )
        .fetch_one(&mut **tx)
        .await?)
}

/// Locks the project before comparing, so competing edits and status changes
/// use the latest committed row even when the request looked unchanged.
async fn lock(tx: &mut Tx, org_id: Uuid, project_id: Uuid) -> Result<Project, OpError> {
    sqlx::query_as!(
        Project,
        r#"SELECT id, org_id, client_id, code, name,
                  project_type as "project_type: ProjectType", currency, rate_cents,
                  starts_on as "starts_on: chrono::NaiveDate",
                  ends_on as "ends_on: chrono::NaiveDate",
                  budget_kind as "budget_kind: BudgetKind",
                  budget_amount_cents, budget_minutes, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM projects WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        project_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))
}
