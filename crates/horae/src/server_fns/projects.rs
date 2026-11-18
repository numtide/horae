//! Project, task, and assignment server functions.

use super::*;
#[cfg(feature = "server")]
use crate::db::OrganizationLock;
#[cfg(all(test, feature = "server"))]
use crate::db::lock_organization;
use crate::models::ProjectTagLink;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::ProjectOverview;
use crate::models::task::TaskRateEdit;

mod detail_view;
pub use detail_view::get_project_detail_view;

mod task_catalog;
pub use task_catalog::load_task_catalog;

#[cfg(feature = "server")]
use crate::ops::projects::task_writes::{self, TaskDetails};
#[cfg(feature = "server")]
use crate::ops::projects::{ProjectRead, TaskRead};
#[cfg(feature = "server")]
use crate::ops::{self, Caller};
#[cfg(all(test, feature = "server"))]
use horae_core::permissions::catalog::Permission;

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(all(test, feature = "server"))]
mod privacy_tests;

#[cfg(all(test, feature = "server"))]
mod canonical_read_tests;

#[cfg(all(test, feature = "server"))]
mod details_tests;

#[cfg(all(test, feature = "server"))]
mod mutation_tests;

#[cfg(all(test, feature = "server"))]
mod bulk_tests;

#[cfg(all(test, feature = "server"))]
mod client_lifecycle_tests;

#[cfg(all(test, feature = "server"))]
mod task_write_tests;

#[cfg(all(test, feature = "server"))]
mod task_edit_tests;

#[cfg(all(test, feature = "server"))]
mod task_activity_tests;

#[cfg(all(test, feature = "server"))]
mod common_task_tests;

// ── Projects ─────────────────────────────────────────────────────────────────

#[server]
pub async fn list_project_tags(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectTagLink>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    fetch_project_tags(&state.db, caller.org_id, caller.user_id, expected_requester).await
}

#[cfg(feature = "server")]
async fn fetch_project_tags(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectTagLink>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id))
        .await
        .map_err(op_error)?;
    let result = ops::projects::tags(&mut read.tx, &read.actor, expected_requester).await;
    read.finish(result).await.map_err(op_error)
}

/// Load overview rows and labels without requiring an unrelated client directory grant.
#[server]
pub async fn get_project_overview(
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    fetch_project_overview(&state.db, caller, expected_requester).await
}

#[cfg(feature = "server")]
async fn fetch_project_overview(
    pool: &sqlx::PgPool,
    viewer: impl Into<Caller>,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let mut read = ops::begin_read(pool, viewer.into())
        .await
        .map_err(op_error)?;
    let result = ops::projects::overview(&mut read.tx, &read.actor, expected_requester).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    include_inactive: bool,
) -> Result<Vec<Project>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_opt_uuid(client_id, "client_id")?;
    projects_for_viewer(
        &state.db,
        caller,
        client_id,
        include_inactive,
        ProjectRead::Overview,
    )
    .await
}

/// Minimal project identities for starting time and resolving an own history.
/// Reporting visibility must not prevent a teammate from using their timesheet.
#[server]
pub async fn list_tracking_projects() -> Result<Vec<Project>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    projects_for_viewer(&state.db, caller, None, true, ProjectRead::Tracking).await
}

#[cfg(feature = "server")]
async fn projects_for_viewer(
    pool: &sqlx::PgPool,
    viewer: impl Into<Caller>,
    client_id: Option<uuid::Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<Project>, ServerFnError> {
    let mut read = ops::begin_read(pool, viewer.into())
        .await
        .map_err(op_error)?;
    let result = ops::projects::list(
        &mut read.tx,
        &read.actor,
        client_id,
        include_inactive,
        purpose,
    )
    .await;
    read.finish(result).await.map_err(op_error)
}

/// Per-project tracked totals for the overview's Spent column (see
/// `ops::projects::spend`).
#[server]
pub async fn list_project_spend(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;

    fetch_project_spend(&state.db, caller.org_id, caller.user_id, expected_requester).await
}

/// Configured budgets use their own period and scope; tracked totals remain
/// available separately through `list_project_spend`.
#[server]
pub async fn list_project_budget_progress(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<crate::models::ProjectBudgetOverview>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read(&state.db, caller).await.map_err(op_error)?;
    let result =
        ops::projects::budget_progress(&mut read.tx, &read.actor, expected_requester).await;
    read.finish(result).await.map_err(op_error)
}

#[cfg(feature = "server")]
pub(super) async fn fetch_project_spend(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id))
        .await
        .map_err(op_error)?;
    let result = ops::projects::spend(&mut read.tx, &read.actor, expected_requester).await;
    read.finish(result).await.map_err(op_error)
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Project, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let mut projects = set_projects_status(
        &state.db,
        &*state.plugins,
        caller.org_id,
        caller.user_id,
        &[project_id],
        active,
        expected_requester,
    )
    .await?;
    projects
        .pop()
        .ok_or_else(|| server_err("Project status change returned no project"))
}

/// Set up to 100 projects to the requested status in one organization-scoped transaction.
#[server]
pub async fn set_projects_active(
    project_ids: Vec<String>,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<Project>, ServerFnError> {
    let caller = session_caller().await?;
    let ids = parse_bulk_project_ids(&project_ids)?;
    let state = crate::state::global_state().await;
    set_projects_status(
        &state.db,
        &*state.plugins,
        caller.org_id,
        caller.user_id,
        &ids,
        active,
        expected_requester,
    )
    .await
}

/// Only converts: `ops::projects::set_active` bounds and orders the batch.
#[cfg(feature = "server")]
fn parse_bulk_project_ids(values: &[String]) -> Result<Vec<uuid::Uuid>, ServerFnError> {
    values
        .iter()
        .map(|value| {
            uuid::Uuid::parse_str(value).map_err(|_| err(BAD_REQUEST, "Invalid project ID"))
        })
        .collect()
}

/// Archives or restores `ids` in one transaction through
/// `ops::projects::set_active`, dispatching its events after the commit.
#[cfg(feature = "server")]
async fn set_projects_status(
    db: &sqlx::PgPool,
    events: &dyn crate::ops::EventSink,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    ids: &[uuid::Uuid],
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<Project>, ServerFnError> {
    use crate::ops::{self, Caller};
    let mut write = ops::begin(db, Caller::session(org_id, actor_id))
        .await
        .map_err(op_error)?;
    let result =
        ops::projects::set_active(&mut write.tx, &write.actor, ids, active, expected_requester)
            .await;
    write.commit(result, events).await.map_err(op_error)
}

/// The status write as the tests drive it: each project with the transition
/// its committed event reported.
#[cfg(all(test, feature = "server"))]
async fn set_projects_active_records(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<(Project, Option<crate::plugin::event::ActiveTransition>)>, ServerFnError> {
    use crate::plugin::{AppEvent, event::ActiveTransition};
    let recorder = crate::ops::testing::Recorder::default();
    let projects = set_projects_status(db, &recorder, org_id, actor_id, ids, active, None).await?;
    let events = recorder.take();
    Ok(projects
        .into_iter()
        .map(|project| {
            let transition = events.iter().find_map(|event| match event {
                AppEvent::ProjectReactivated { project: p, .. } if p.id == project.id => {
                    Some(ActiveTransition::Reactivated)
                }
                AppEvent::ProjectDeactivated { project: p, .. } if p.id == project.id => {
                    Some(ActiveTransition::Deactivated)
                }
                _ => None,
            });
            (project, transition)
        })
        .collect())
}

#[cfg(all(test, feature = "server"))]
async fn set_project_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    project_id: uuid::Uuid,
    active: bool,
) -> Result<(Project, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut results =
        set_projects_active_records(db, org_id, actor_id, &[project_id], active).await?;
    results
        .pop()
        .ok_or_else(|| server_err("Project status change returned no project"))
}

// ── Tasks ────────────────────────────────────────────────────────────────────

/// Lists all active org-level tasks.
#[server]
pub async fn list_tasks() -> Result<Vec<Task>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;

    tasks_for_viewer(&state.db, caller, None, TaskRead::Catalog).await
}

/// Rate-free task identities, including archived tasks in the viewer's history.
#[server]
pub async fn list_tracking_tasks() -> Result<Vec<Task>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    tasks_for_viewer(&state.db, caller, None, TaskRead::Tracking).await
}

#[cfg(feature = "server")]
async fn tasks_for_viewer(
    pool: &sqlx::PgPool,
    viewer: impl Into<Caller>,
    project_id: Option<uuid::Uuid>,
    purpose: TaskRead,
) -> Result<Vec<Task>, ServerFnError> {
    let mut read = ops::begin_read(pool, viewer.into())
        .await
        .map_err(op_error)?;
    let result = ops::projects::tasks(&mut read.tx, &read.actor, project_id, purpose).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
    is_default: bool,
    rate: TaskRateEdit,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let details = TaskDetails {
        name,
        billable_default,
        is_default,
        rate,
    };
    task_create(
        &state.db,
        &*state.plugins,
        caller,
        expected_requester,
        &details,
    )
    .await
    .map_err(op_error)
}

/// Begins a task write holding the organization `gate`.
#[cfg(feature = "server")]
async fn begin_task_write(
    db: &sqlx::PgPool,
    caller: Caller,
    gate: OrganizationLock,
) -> Result<ops::WriteTx, ops::OpError> {
    ops::begin_gated(db, caller, gate, ops::WriteIsolation::ReadCommitted).await
}

#[cfg(feature = "server")]
async fn task_create(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    expected: PermissionRequester,
    details: &TaskDetails,
) -> Result<Task, ops::OpError> {
    let mut write = begin_task_write(db, caller, OrganizationLock::Shared).await?;
    let result = task_writes::create(&mut write.tx, &write.actor, expected, details).await;
    write.commit(result, events).await
}

#[server]
pub async fn update_task(
    task_id: String,
    name: String,
    billable_default: bool,
    is_default: bool,
    rate: TaskRateEdit,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let details = TaskDetails {
        name,
        billable_default,
        is_default,
        rate,
    };
    task_update(
        &state.db,
        &*state.plugins,
        caller,
        expected_requester,
        task_id,
        &details,
    )
    .await
    .map_err(op_error)
}

#[cfg(feature = "server")]
async fn task_update(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    expected: PermissionRequester,
    task_id: uuid::Uuid,
    details: &TaskDetails,
) -> Result<Task, ops::OpError> {
    let mut write = begin_task_write(db, caller, OrganizationLock::Shared).await?;
    let result = task_writes::update(&mut write.tx, &write.actor, expected, task_id, details).await;
    write.commit(result, events).await
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(
    task_id: String,
    active: bool,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    task_set_active(
        &state.db,
        &*state.plugins,
        caller,
        expected_requester,
        task_id,
        active,
    )
    .await
    .map_err(op_error)
}

/// Holds the `AccessChange` gate: tracking writers hold the shared gate
/// through entry insertion.
#[cfg(feature = "server")]
async fn task_set_active(
    db: &sqlx::PgPool,
    events: &dyn ops::EventSink,
    caller: Caller,
    expected: PermissionRequester,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<Task, ops::OpError> {
    let mut write = begin_task_write(db, caller, OrganizationLock::AccessChange).await?;
    let result =
        task_writes::set_active(&mut write.tx, &write.actor, expected, task_id, active).await;
    write.commit(result, events).await
}

/// The task writes as the tests drive them: as the person `actor_id`, bound
/// to their own session, with the payload of the event each committed.
#[cfg(all(test, feature = "server"))]
mod task_records {
    use super::*;
    use crate::ops::testing::Recorder;
    use crate::plugin::AppEvent;
    use crate::plugin::event::TaskPayload;

    fn session(org_id: uuid::Uuid, actor_id: uuid::Uuid) -> (Caller, PermissionRequester) {
        let requester = PermissionRequester {
            org_id,
            user_id: actor_id,
        };
        (Caller::session(org_id, actor_id), requester)
    }

    fn details(
        name: &str,
        billable_default: bool,
        is_default: bool,
        rate: &TaskRateEdit,
    ) -> TaskDetails {
        TaskDetails {
            name: name.into(),
            billable_default,
            is_default,
            rate: rate.clone(),
        }
    }

    pub(in crate::server_fns) async fn create_catalog_task(
        db: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        name: &str,
        billable_default: bool,
        is_default: bool,
        rate: &TaskRateEdit,
    ) -> Result<(Task, TaskPayload), ServerFnError> {
        let events = Recorder::default();
        let (caller, requester) = session(org_id, actor_id);
        let details = details(name, billable_default, is_default, rate);
        let task = task_create(db, &events, caller, requester, &details)
            .await
            .map_err(op_error)?;
        let payload = events.take().into_iter().find_map(|event| match event {
            AppEvent::TaskCreated { task, .. } => Some(task),
            _ => None,
        });
        Ok((
            task,
            payload.expect("a created task announces task_created"),
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::server_fns) async fn update_task_record(
        db: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        task_id: uuid::Uuid,
        name: &str,
        billable_default: bool,
        is_default: bool,
        rate: &TaskRateEdit,
    ) -> Result<(Task, Option<TaskPayload>), ServerFnError> {
        let events = Recorder::default();
        let (caller, requester) = session(org_id, actor_id);
        let details = details(name, billable_default, is_default, rate);
        let task = task_update(db, &events, caller, requester, task_id, &details)
            .await
            .map_err(op_error)?;
        let payload = events.take().into_iter().find_map(|event| match event {
            AppEvent::TaskUpdated { task, .. } => Some(task),
            _ => None,
        });
        Ok((task, payload))
    }

    pub(in crate::server_fns) async fn set_task_active_record(
        db: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        task_id: uuid::Uuid,
        active: bool,
    ) -> Result<(Task, Option<AppEvent>), ServerFnError> {
        let events = Recorder::default();
        let (caller, requester) = session(org_id, actor_id);
        let task = task_set_active(db, &events, caller, requester, task_id, active)
            .await
            .map_err(op_error)?;
        Ok((task, events.take().pop()))
    }
}
#[cfg(all(test, feature = "server"))]
use task_records::{create_catalog_task, set_task_active_record, update_task_record};
