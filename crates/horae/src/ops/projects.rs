//! Project operations: the project and task reads, archiving and restoring
//! (FR-011), creating a project ([`creation`]), the project editor
//! ([`editor`]), its managers ([`managers`]), the people its form offers
//! ([`people`]) and the task catalog's writes ([`task_writes`]).
//!
//! Every decision goes through [`Actor::has`], on the actor's stored grants.

use horae_core::permissions::catalog::Permission;
use horae_core::permissions::projects;
use horae_core::types::{BudgetKind, ProjectType};
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::Project;
use crate::models::permission_editor::PermissionRequester;
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, active_transition};

pub(crate) mod budgets;
pub(crate) mod creation;
mod detail;
pub(crate) mod editor;
pub(crate) mod form;
pub(crate) mod managers;
pub(crate) mod people;
mod reads;
mod task_catalog;
pub(crate) mod task_writes;

pub(crate) use budgets::progress as budget_progress;
pub(crate) use detail::view as detail_view;
// Tests pin the date that selects a monthly-reset budget's month.
#[cfg(test)]
pub(crate) use budgets::progress_on as budget_progress_on;
#[cfg(test)]
pub(crate) use detail::view_on as detail_view_on;
pub(crate) use reads::{ProjectRead, TaskRead, list, overview, spend, tags, tasks};
pub(crate) use task_catalog::page as task_catalog;

const NOT_FOUND: &str = "Project not found";
const WRITE_REQUIRED: &str = "Project editing access required";
/// The editor's, its managers' and its people's refusal.
const EDITING_REQUIRED: &str = "Current project editing authority is required";
const RESTORE_CLIENT_FIRST: &str = "Restore the client before restoring this project.";

const REQUESTER_CHANGED: &str = "Project requester has changed";

/// Archives or restores 1 to 100 projects, each once, in ascending id order,
/// so overlapping batches take their row locks in one global order. A project
/// is written with `ProjectWriteAll`, or with `ProjectWriteManaged` when the
/// actor manages it. Restoring needs the project's client active: a client
/// archive locks the same project rows, so the two serialize here.
///
/// Locks: the organization and the person ([`Actor::load`]), then each
/// project `FOR UPDATE`, then its client `FOR SHARE`. A managed-only writer
/// is checked before the lock, so it never locks a project it may not write.
///
/// The returned rows carry the rate and money budget only when the actor may
/// read them, by the rule of the project overview.
pub(crate) async fn set_active(
    tx: &mut Tx,
    actor: &Actor,
    ids: &[Uuid],
    active: bool,
    expected: Option<PermissionRequester>,
) -> Result<Done<Vec<Project>>, OpError> {
    actor.expect_requester(expected, REQUESTER_CHANGED)?;
    let ids = lock_order(ids)?;
    if !actor.is_active() || !projects::writes_any(|permission| actor.has(permission)) {
        return Err(OpError::forbidden(WRITE_REQUIRED));
    }
    let org_id = actor.org_id();
    let mut projects = Vec::with_capacity(ids.len());
    let mut events = Vec::new();
    for project_id in ids {
        let designated = is_designated(tx, actor, project_id).await?;
        if !projects::writes(|permission| actor.has(permission), designated) {
            return Err(refusal(tx, actor, project_id).await?);
        }
        let before = lock(tx, org_id, project_id).await?;
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
        projects.push(redact_money(actor, designated, project));
    }
    let mut done = Done::new(projects);
    done.events = events;
    Ok(done)
}

/// A status batch of 1 to 100 ids, sorted and without repeats: the order in
/// which `set_active` locks them.
fn lock_order(ids: &[Uuid]) -> Result<Vec<Uuid>, OpError> {
    if ids.is_empty() || ids.len() > 100 {
        return Err(OpError::invalid(
            "project_ids",
            "Select between 1 and 100 projects",
        ));
    }
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn event(org_id: Uuid, project: &Project, transition: ActiveTransition) -> AppEvent {
    let occurred_at = chrono::Utc::now();
    let project = crate::plugin::event::project_payload(project);
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

/// Hides the rate and the money budget from an actor without
/// `BillableRateReadAll`, or `BillableRateReadManaged` on a project they manage.
fn redact_money(actor: &Actor, designated: bool, mut project: Project) -> Project {
    if !projects::reads_money(|permission| actor.has(permission), designated) {
        project.rate_cents = None;
        project.budget_amount_cents = None;
    }
    project
}

/// Refuses anyone who may not edit `project_id`: editing needs
/// `ProjectWriteAll`, or `ProjectWriteManaged` on a project the actor
/// manages. It does not check that the project exists.
async fn authorize_write(tx: &mut Tx, actor: &Actor, project_id: Uuid) -> Result<(), OpError> {
    let has = |permission| actor.has(permission);
    // Only a managed-only writer depends on the designation.
    let managed = projects::writes_any(has)
        && !projects::writes(has, false)
        && is_designated(tx, actor, project_id).await?;
    if projects::writes(has, managed) {
        Ok(())
    } else {
        Err(OpError::forbidden(EDITING_REQUIRED))
    }
}

async fn is_designated(tx: &mut Tx, actor: &Actor, project_id: Uuid) -> Result<bool, OpError> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM project_management_assignments
            WHERE org_id = $1 AND project_id = $2 AND manager_id = $3) AS "managed!""#,
        actor.org_id(),
        project_id,
        actor.id(),
    )
    .fetch_one(&mut **tx)
    .await?)
}

/// The refusal for a project the actor may not write: forbidden when they can
/// see that it exists, not found otherwise, so a managed-only writer without
/// `ProjectReadAll` learns nothing about projects outside their own.
async fn refusal(tx: &mut Tx, actor: &Actor, project_id: Uuid) -> Result<OpError, OpError> {
    let visible = actor.has(Permission::ProjectReadAll)
        && sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND org_id = $2) AS "exists!""#,
            project_id,
            actor.org_id(),
        )
        .fetch_one(&mut **tx)
        .await?;
    Ok(if visible {
        OpError::forbidden(WRITE_REQUIRED)
    } else {
        OpError::not_found(NOT_FOUND)
    })
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

#[cfg(test)]
mod tests;
