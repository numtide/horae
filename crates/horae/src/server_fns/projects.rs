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
#[cfg(feature = "server")]
use crate::plugin::event::task_payload;

mod detail_view;
pub use detail_view::get_project_detail_view;

mod task_catalog;
pub use task_catalog::load_task_catalog;

#[cfg(feature = "server")]
use crate::ops::projects::{ProjectRead, TaskRead};
#[cfg(feature = "server")]
use crate::ops::{self, Caller};
#[cfg(feature = "server")]
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
    let viewer = require_user().await?;
    Caller::session(viewer.org_id, viewer.id)
        .expect_requester(expected_requester, PROJECT_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    fetch_project_tags(&state.db, viewer.org_id, viewer.id).await
}

#[cfg(feature = "server")]
async fn fetch_project_tags(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
) -> Result<Vec<ProjectTagLink>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = ops::projects::tags(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// Load overview rows and labels without requiring an unrelated client directory grant.
#[server]
pub async fn get_project_overview(
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    fetch_project_overview(&state.db, &user, expected_requester).await
}

#[cfg(feature = "server")]
async fn fetch_project_overview(
    pool: &sqlx::PgPool,
    viewer: &User,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectOverview, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(viewer.org_id, viewer.id)).await?;
    let result = ops::projects::overview(&mut read.tx, &read.actor, expected_requester).await;
    Ok(read.finish(result).await?)
}

#[cfg(feature = "server")]
const PROJECT_REQUESTER_CHANGED: &str = "Project requester has changed";
#[cfg(feature = "server")]
const TASK_REQUESTER_CHANGED: &str = "Task requester has changed";

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    include_inactive: bool,
) -> Result<Vec<Project>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_opt_uuid(client_id, "client_id")?;
    projects_for_viewer(
        &state.db,
        &user,
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
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    projects_for_viewer(&state.db, &user, None, true, ProjectRead::Tracking).await
}

#[cfg(feature = "server")]
async fn projects_for_viewer(
    pool: &sqlx::PgPool,
    viewer: &User,
    client_id: Option<uuid::Uuid>,
    include_inactive: bool,
    purpose: ProjectRead,
) -> Result<Vec<Project>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(viewer.org_id, viewer.id)).await?;
    let result = ops::projects::list(
        &mut read.tx,
        &read.actor,
        client_id,
        include_inactive,
        purpose,
    )
    .await;
    Ok(read.finish(result).await?)
}

/// Per-project tracked totals for the overview's Spent column (see
/// `ops::projects::spend`).
#[server]
pub async fn list_project_spend(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let user = require_user().await?;
    Caller::session(user.org_id, user.id)
        .expect_requester(expected_requester, PROJECT_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;

    fetch_project_spend(&state.db, user.org_id, user.id).await
}

/// Configured budgets use their own period and scope; tracked totals remain
/// available separately through `list_project_spend`.
#[server]
pub async fn list_project_budget_progress(
    expected_requester: Option<PermissionRequester>,
) -> Result<Vec<crate::models::ProjectBudgetOverview>, ServerFnError> {
    let user = require_user().await?;
    Caller::session(user.org_id, user.id)
        .expect_requester(expected_requester, PROJECT_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    // Budget months are the workspace's, so every viewer sees the same period.
    let today = crate::timezone::today_for_org(&state.db, user.org_id)
        .await
        .map_err(server_err)?;
    super::budgets::progress_for_viewer(&state.db, user.org_id, user.id, today).await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_project_spend(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
) -> Result<Vec<ProjectSpend>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = ops::projects::spend(&mut read.tx, &read.actor).await;
    Ok(read.finish(result).await?)
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
    expected_requester: Option<PermissionRequester>,
) -> Result<Project, ServerFnError> {
    let user = require_user().await?;
    Caller::session(user.org_id, user.id)
        .expect_requester(expected_requester, PROJECT_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let mut projects = set_projects_status(
        &state.db,
        &*state.plugins,
        user.org_id,
        user.id,
        &[project_id],
        active,
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
    let user = require_user().await?;
    Caller::session(user.org_id, user.id)
        .expect_requester(expected_requester, PROJECT_REQUESTER_CHANGED)?;
    let ids = parse_bulk_project_ids(&project_ids)?;
    let state = crate::state::global_state().await;
    set_projects_status(
        &state.db,
        &*state.plugins,
        user.org_id,
        user.id,
        &ids,
        active,
    )
    .await
}

#[cfg(feature = "server")]
fn parse_bulk_project_ids(values: &[String]) -> Result<Vec<uuid::Uuid>, ServerFnError> {
    if values.is_empty() || values.len() > 100 {
        return Err(err(BAD_REQUEST, "Select between 1 and 100 projects"));
    }
    let mut ids = values
        .iter()
        .map(|value| {
            uuid::Uuid::parse_str(value).map_err(|_| err(BAD_REQUEST, "Invalid project ID"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Overlapping batches acquire their row locks in one global order.
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Archives or restores `sorted_ids` in one transaction through
/// `ops::projects::set_active`, dispatching its events after the commit.
#[cfg(feature = "server")]
async fn set_projects_status(
    db: &sqlx::PgPool,
    events: &dyn crate::ops::EventSink,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    sorted_ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<Project>, ServerFnError> {
    use crate::ops::{self, Caller};
    let mut write = ops::begin(db, Caller::session(org_id, actor_id)).await?;
    let result = ops::projects::set_active(&mut write.tx, &write.actor, sorted_ids, active).await;
    Ok(write.commit(result, events).await?)
}

/// The status write as the tests drive it: each project with the transition
/// its committed event reported.
#[cfg(all(test, feature = "server"))]
async fn set_projects_active_records(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    sorted_ids: &[uuid::Uuid],
    active: bool,
) -> Result<Vec<(Project, Option<crate::plugin::event::ActiveTransition>)>, ServerFnError> {
    use crate::plugin::{AppEvent, event::ActiveTransition};
    let recorder = crate::ops::testing::Recorder::default();
    let projects = set_projects_status(db, &recorder, org_id, actor_id, sorted_ids, active).await?;
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
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    tasks_for_viewer(&state.db, &user, None, TaskRead::Catalog).await
}

/// Rate-free task identities, including archived tasks in the viewer's history.
#[server]
pub async fn list_tracking_tasks() -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    tasks_for_viewer(&state.db, &user, None, TaskRead::Tracking).await
}

#[cfg(feature = "server")]
async fn tasks_for_viewer(
    pool: &sqlx::PgPool,
    viewer: &User,
    project_id: Option<uuid::Uuid>,
    purpose: TaskRead,
) -> Result<Vec<Task>, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(viewer.org_id, viewer.id)).await?;
    let result = ops::projects::tasks(&mut read.tx, &read.actor, project_id, purpose).await;
    Ok(read.finish(result).await?)
}

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
    is_default: bool,
    rate: TaskRateEdit,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let actor = require_user().await?;
    Caller::session(actor.org_id, actor.id)
        .expect_requester(Some(expected_requester), TASK_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    let (task, event) = create_catalog_task(
        &state.db,
        actor.org_id,
        actor.id,
        &name,
        billable_default,
        is_default,
        &rate,
    )
    .await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TaskCreated {
            occurred_at: chrono::Utc::now(),
            org_id: actor.org_id,
            task: event,
        });
    Ok(task)
}

/// Begins a task write holding the organization `gate` and requires
/// `TaskWriteAll`.
#[cfg(feature = "server")]
async fn begin_task_write(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    gate: OrganizationLock,
) -> Result<(crate::ops::Tx, crate::ops::Actor), ServerFnError> {
    let write = crate::ops::begin_gated(
        db,
        crate::ops::Caller::session(org_id, actor_id),
        gate,
        crate::ops::WriteIsolation::ReadCommitted,
    )
    .await?;
    if !write.actor.has(Permission::TaskWriteAll) {
        let refusal = forbidden("Current task editing authority is required");
        return Err(write.refuse(refusal).await);
    }
    Ok((write.tx, write.actor))
}

/// Adds a task to the organization catalog. Projects link tasks through the
/// project editor, as Harvest links them through task assignments.
#[cfg(feature = "server")]
async fn create_catalog_task(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    is_default: bool,
    rate: &TaskRateEdit,
) -> Result<(Task, crate::plugin::event::TaskPayload), ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let (mut tx, actor) = begin_task_write(db, org_id, actor_id, OrganizationLock::Shared).await?;
    let (amount, currency) = authorize_task_rate(&mut tx, org_id, &actor, rate).await?;
    let id = uuid::Uuid::now_v7();
    let mut task = sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents, default_rate_currency, is_default)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        id,
        org_id,
        name,
        billable_default,
        amount,
        currency,
        is_default,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    let event = task_payload(&task);
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    tx.commit().await.map_err(server_err)?;
    Ok((task, event))
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
    let actor = require_user().await?;
    Caller::session(actor.org_id, actor.id)
        .expect_requester(Some(expected_requester), TASK_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, event) = update_task_record(
        &state.db,
        actor.org_id,
        actor.id,
        task_id,
        &name,
        billable_default,
        is_default,
        &rate,
    )
    .await?;
    if let Some(payload) = event {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TaskUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: actor.org_id,
                task: payload,
            });
    }
    Ok(task)
}

#[cfg(feature = "server")]
#[allow(clippy::too_many_arguments)]
async fn update_task_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    task_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    is_default: bool,
    rate: &TaskRateEdit,
) -> Result<(Task, Option<crate::plugin::event::TaskPayload>), ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let (mut tx, actor) = begin_task_write(db, org_id, actor_id, OrganizationLock::Shared).await?;
    let (amount, currency) = authorize_task_rate(&mut tx, org_id, &actor, rate).await?;
    let preserve = matches!(rate, TaskRateEdit::Preserve {});
    let before = lock_task(&mut tx, org_id, task_id).await?;

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks
            SET name = $3, billable_default = $4, is_default = $8,
                default_rate_cents = CASE WHEN $5 THEN default_rate_cents ELSE $6 END,
                default_rate_currency = CASE WHEN $5 THEN default_rate_currency ELSE $7 END
          WHERE id = $1 AND org_id = $2
            AND (name, billable_default, is_default, default_rate_cents, default_rate_currency)
                IS DISTINCT FROM ($3, $4, $8,
                    CASE WHEN $5 THEN default_rate_cents ELSE $6 END,
                    CASE WHEN $5 THEN default_rate_currency ELSE $7 END)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        name,
        billable_default,
        preserve,
        amount,
        currency,
        is_default,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    // Service events retain their committed payload; session responses must not
    // inherit that service authority over a hidden global rate.
    let event = task.as_ref().map(task_payload);
    let mut task = task.unwrap_or(before);
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    tx.commit().await.map_err(server_err)?;
    Ok((task, event))
}

#[cfg(feature = "server")]
async fn authorize_task_rate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    actor: &crate::ops::Actor,
    rate: &TaskRateEdit,
) -> Result<(Option<i64>, Option<String>), ServerFnError> {
    use horae_core::permissions::rates::RateEdit;

    let intent = match rate {
        TaskRateEdit::Preserve {} => RateEdit::Unchanged,
        TaskRateEdit::Clear {} => RateEdit::Reset,
        TaskRateEdit::Set { amount_cents, .. } => RateEdit::Set(*amount_cents),
    };
    intent
        .authorize(actor.has(Permission::BillableRateWriteAll))
        .map_err(|_| forbidden("Current global billable-rate editing authority is required"))?;
    if let TaskRateEdit::Set {
        amount_cents,
        currency,
    } = rate
    {
        if *amount_cents < 0 {
            return Err(err(BAD_REQUEST, "Task rate must not be negative"));
        }
        let current = sqlx::query_scalar!(
            "SELECT upper(btrim(default_currency)) AS \"currency!\" FROM organizations WHERE id=$1",
            org_id
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(server_err)?;
        if current.len() != 3
            || !current.bytes().all(|byte| byte.is_ascii_uppercase())
            || !currency.trim().eq_ignore_ascii_case(&current)
        {
            return Err(conflict(
                "Task rate currency changed. Reload the task and enter its rate again",
            ));
        }
        Ok((Some(*amount_cents), Some(current)))
    } else {
        Ok((None, None))
    }
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(
    task_id: String,
    active: bool,
    expected_requester: PermissionRequester,
) -> Result<Task, ServerFnError> {
    let actor = require_user().await?;
    Caller::session(actor.org_id, actor.id)
        .expect_requester(Some(expected_requester), TASK_REQUESTER_CHANGED)?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, event) =
        set_task_active_record(&state.db, actor.org_id, actor.id, task_id, active).await?;
    if let Some(event) = event {
        state.plugins.dispatch(event);
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn set_task_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<(Task, Option<crate::plugin::AppEvent>), ServerFnError> {
    // Tracking writers hold the shared gate through entry insertion.
    let (mut tx, actor) =
        begin_task_write(db, org_id, actor_id, OrganizationLock::AccessChange).await?;
    // The gate stops every access check in the organization: a refusal rolls
    // back before it is answered.
    let result = set_task_active_in(&mut tx, &actor, org_id, task_id, active).await;
    crate::ops::commit_or_roll_back(tx, result, server_err).await
}

#[cfg(feature = "server")]
async fn set_task_active_in(
    tx: &mut sqlx::PgConnection,
    actor: &crate::ops::Actor,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<(Task, Option<crate::plugin::AppEvent>), ServerFnError> {
    if !active {
        // Link updates invalidate editors through their parent project trigger.
        // Take every parent before the task/link locks, in stable order.
        sqlx::query_scalar!(
            "SELECT p.id FROM projects p JOIN project_tasks pt ON pt.project_id=p.id
             JOIN tasks t ON t.id=pt.task_id AND t.org_id=p.org_id
             WHERE p.org_id=$1 AND t.id=$2 ORDER BY p.id FOR NO KEY UPDATE OF p",
            org_id,
            task_id,
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(server_err)?;
    }
    let before = lock_task(tx, org_id, task_id).await?;

    if !active {
        let running = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM time_entries
                WHERE org_id=$1 AND task_id=$2 AND is_running) AS "running!""#,
            org_id,
            task_id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(server_err)?;
        if running {
            // Do not reveal an entry or its owner to a task-only administrator.
            return Err(conflict("A running timer prevents archiving this task"));
        }
    }

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks SET active = $3
          WHERE id = $1 AND org_id = $2
            AND active IS DISTINCT FROM $3
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        active,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let transition = task.as_ref().and_then(|updated| {
        crate::plugin::event::active_transition(Some(before.active), updated.active)
    });
    if !active {
        sqlx::query!(
            "UPDATE project_tasks pt SET active=false FROM projects p
             WHERE pt.project_id=p.id AND p.org_id=$1 AND pt.task_id=$2 AND pt.active",
            org_id,
            task_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    }
    let mut task = task.unwrap_or(before);
    let event = transition.map(|transition| {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        match transition {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::TaskReactivated {
                    occurred_at,
                    org_id,
                    task,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::TaskDeactivated {
                    occurred_at,
                    org_id,
                    task,
                }
            }
        }
    });
    if !actor.has(Permission::BillableRateReadAll) {
        task.default_rate_cents = None;
    }
    Ok((task, event))
}

#[cfg(feature = "server")]
async fn lock_task(
    tx: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
) -> Result<Task, ServerFnError> {
    // Share the row lock between detail edits and activation changes, including no-ops.
    sqlx::query_as!(
        Task,
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks WHERE id = $1 AND org_id = $2 FOR UPDATE",
        task_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Task not found"))
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "server"))]
use crate::models::Assignment;

/// Adds a project member the way a team change does (organization gate, then
/// the project revision), for tests of what a membership change waits for.
/// Production membership changes go through the project editor.
#[cfg(all(test, feature = "server"))]
pub(super) async fn insert_assignment(
    db: &sqlx::PgPool,
    admin: &User,
    project_id: uuid::Uuid,
    user_id: uuid::Uuid,
    role: ProjectRole,
) -> Result<Assignment, ServerFnError> {
    let mut tx = begin_assignment_change(db, admin).await?;
    if !lock_project_revision(&mut tx, admin.org_id, project_id).await? {
        return Err(not_found("Project or person not found"));
    }
    sqlx::query_scalar!(
        "SELECT p.id FROM projects p JOIN users u ON u.org_id = p.org_id
         WHERE p.id = $1 AND u.id = $2 AND p.org_id = $3 FOR SHARE OF u",
        project_id,
        user_id,
        admin.org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project or person not found"))?;
    let id = uuid::Uuid::now_v7();
    let assignment = sqlx::query_as!(
        Assignment,
        r#"INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        project_id,
        user_id,
        role as ProjectRole,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(assignment)
}

/// Removes a project member like [`insert_assignment`] adds one.
#[cfg(all(test, feature = "server"))]
pub(super) async fn remove_assignment(
    db: &sqlx::PgPool,
    admin: &User,
    id: uuid::Uuid,
) -> Result<Option<Assignment>, ServerFnError> {
    let mut tx = begin_assignment_change(db, admin).await?;
    let project_id = sqlx::query_scalar!(
        "SELECT a.project_id FROM assignments a
         JOIN projects p ON p.id=a.project_id JOIN users u ON u.id=a.user_id
         WHERE a.id=$1 AND p.org_id=$2 AND u.org_id=$2",
        id,
        admin.org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    let Some(project_id) = project_id else {
        tx.commit().await.map_err(server_err)?;
        return Ok(None);
    };
    if !lock_project_revision(&mut tx, admin.org_id, project_id).await? {
        tx.commit().await.map_err(server_err)?;
        return Ok(None);
    }
    // Delete and capture the row atomically so the event carries its details
    // and a concurrent delete cannot double-notify.
    let removed = sqlx::query_as!(
        Assignment,
        r#"DELETE FROM assignments a USING projects p, users u
         WHERE a.id = $1 AND a.project_id = p.id AND a.user_id = u.id
           AND p.org_id = $2 AND u.org_id = $2 AND a.project_id = $3
         RETURNING a.id, a.project_id, a.user_id, a.role as "role: ProjectRole", a.rate_cents,
                   a.created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        admin.org_id,
        project_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(removed)
}

#[cfg(all(test, feature = "server"))]
async fn begin_assignment_change<'a>(
    db: &'a sqlx::PgPool,
    admin: &User,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    lock_organization(&mut tx, admin.org_id, OrganizationLock::AccessChange)
        .await
        .map_err(server_err)?;
    Ok(tx)
}

#[cfg(all(test, feature = "server"))]
async fn lock_project_revision(
    db: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<bool, ServerFnError> {
    // Child revision triggers write this parent. Permit entry/invoice FK checks
    // to finish before any dependent membership cascade waits for those writers.
    sqlx::query_scalar!(
        "SELECT id FROM projects WHERE id=$1 AND org_id=$2 FOR NO KEY UPDATE",
        project_id,
        org_id,
    )
    .fetch_optional(db)
    .await
    .map(|row| row.is_some())
    .map_err(server_err)
}
