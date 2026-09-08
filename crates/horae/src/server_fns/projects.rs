//! Project, task, and assignment server functions.

use super::*;

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(all(test, feature = "server"))]
mod mutation_tests;

// ── Projects ─────────────────────────────────────────────────────────────────

#[cfg(feature = "server")]
fn parse_project_rate(value: &str) -> Result<Option<i64>, ServerFnError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let cents = horae_core::money::parse_cents(value)
        .map_err(|_| conflict("Rate must be an amount, e.g. 120 or 120.50"))?;
    if cents < 0 {
        return Err(conflict("Rate cannot be negative"));
    }
    Ok(Some(cents))
}

/// Read a typed budget into the column its kind belongs in: money budgets store
/// minor units, hours budgets store minutes, and the other column stays NULL so
/// the two can never disagree. A blank value clears the budget, which is how a
/// project keeps its kind while its figure is still unknown.
#[cfg(feature = "server")]
fn parse_budget(
    kind: BudgetKind,
    value: &str,
) -> Result<(Option<i64>, Option<i64>), ServerFnError> {
    let value = value.trim();
    if matches!(kind, BudgetKind::None) || value.is_empty() {
        return Ok((None, None));
    }
    match kind {
        BudgetKind::Amount => {
            let cents = horae_core::money::parse_cents(value)
                .map_err(|_| server_err("Budget must be an amount, e.g. 12000 or 12,000.50"))?;
            if cents < 0 {
                return Err(server_err("Budget cannot be negative"));
            }
            Ok((Some(cents), None))
        }
        BudgetKind::Hours => {
            let minutes = horae_core::duration::parse(value)
                .map_err(|_| server_err("Budget must be hours, e.g. 120 or 7:30"))?;
            Ok((None, Some(i64::from(minutes))))
        }
        BudgetKind::None => unreachable!("handled above"),
    }
}

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    include_inactive: bool,
) -> Result<Vec<Project>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let _ = client_id;

    let projects = sqlx::query_as!(
        Project,
        r#"SELECT id, org_id, client_id, code, name,
                project_type as "project_type: ProjectType", currency, rate_cents,
                starts_on as "starts_on: chrono::NaiveDate",
                ends_on as "ends_on: chrono::NaiveDate",
                budget_kind as "budget_kind: BudgetKind",
                budget_amount_cents, budget_minutes, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM projects
         WHERE org_id = $2 AND ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
        user.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(projects)
}

/// Per-project tracked totals for the overview's Spent column: every project's
/// total logged minutes plus its billable amount, with each entry's rate resolved
/// through the FR-024 cascade (task → assignment → project → user default) and summed.
/// Session-gated only: the Projects overview shows Budget/Spent to every signed-in
/// user, and these are per-project aggregates, not per-user time or rates.
#[server]
pub async fn list_project_spend() -> Result<Vec<ProjectSpend>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    fetch_project_spend(&state.db, user.org_id)
        .await
        .map_err(server_err)
}

#[cfg(feature = "server")]
pub(super) async fn fetch_project_spend(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
) -> Result<Vec<ProjectSpend>, sqlx::Error> {
    // Grouped in Postgres, not folded here: the overview needs one number per
    // project, and folding in Rust meant fetching one row per time entry to get
    // there. `COALESCE(pt, a, p, u)` is the FR-024 cascade — exactly what
    // `horae_core::invoice::resolve_rate` does — and `line_amount_cents` is the
    // SQL twin of the Rust function invoicing uses. Attached invoice amounts
    // take priority over live rates. The joins cannot multiply rows: project
    // tasks, assignments, and invoice lines each have a unique pair key.
    let spend = sqlx::query_as!(
        ProjectSpend,
        r#"SELECT
             te.project_id as "project_id!",
             SUM(te.minutes)::bigint as "spent_minutes!",
             COALESCE(SUM(COALESCE(line.amount_cents, line_amount_cents(
                 COALESCE(pt.rate_cents, a.rate_cents, p.rate_cents, u.billable_rate_cents, 0),
                 effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)
               ))) FILTER (WHERE (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default))))), 0)::bigint as "spent_cents!"
           FROM time_entries te
           JOIN projects p ON p.id = te.project_id
           JOIN tasks t ON t.id = te.task_id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           LEFT JOIN invoice_line_items line ON line.invoice_id = te.invoice_id AND line.time_entry_id = te.id
           JOIN users u ON u.id = te.user_id
           JOIN organizations o ON o.id = te.org_id
           WHERE te.org_id = $1
           GROUP BY te.project_id"#,
        org_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(spend)
}

#[server]
pub async fn create_project(
    client_id: String,
    name: String,
    project_type: String,
    currency: String,
    budget_kind: String,
    budget_value: String,
    rate_value: String,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let client_id = parse_uuid(&client_id, "client_id")?;
    let pt: ProjectType = parse_enum(&project_type, "project_type")?;
    let bk: BudgetKind = parse_enum(&budget_kind, "budget_kind")?;
    let (budget_amount_cents, budget_minutes) = parse_budget(bk, &budget_value)?;
    let rate_cents = parse_project_rate(&rate_value)?;
    let project = sqlx::query_as!(
        Project,
        r#"INSERT INTO projects
             (id, org_id, client_id, name, project_type, currency,
              budget_kind, budget_amount_cents, budget_minutes, rate_cents)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id, org_id, client_id, code, name,
                   project_type as "project_type: ProjectType", currency, rate_cents,
                   starts_on as "starts_on: chrono::NaiveDate",
                   ends_on as "ends_on: chrono::NaiveDate",
                   budget_kind as "budget_kind: BudgetKind",
                   budget_amount_cents, budget_minutes, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        client_id,
        name,
        pt as ProjectType,
        currency,
        bk as BudgetKind,
        budget_amount_cents,
        budget_minutes,
        rate_cents,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::ProjectCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            project: project_payload(&project),
        });
    Ok(project)
}

#[server]
pub async fn update_project(
    project_id: String,
    name: String,
    project_type: String,
    currency: String,
    budget_kind: String,
    budget_value: String,
    rate_value: String,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let (project, changed) = update_project_record(
        &state.db,
        manager.org_id,
        project_id,
        &ProjectEdit {
            name: &name,
            project_type: &project_type,
            currency: &currency,
            budget_kind: &budget_kind,
            budget_value: &budget_value,
            rate_value: &rate_value,
        },
    )
    .await?;
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::ProjectUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                project: project_payload(&project),
            });
    }
    Ok(project)
}

#[cfg(feature = "server")]
struct ProjectEdit<'a> {
    name: &'a str,
    project_type: &'a str,
    currency: &'a str,
    budget_kind: &'a str,
    budget_value: &'a str,
    rate_value: &'a str,
}

#[cfg(feature = "server")]
async fn update_project_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    edit: &ProjectEdit<'_>,
) -> Result<(Project, bool), ServerFnError> {
    let pt: ProjectType = parse_enum(edit.project_type, "project_type")?;
    let bk: BudgetKind = parse_enum(edit.budget_kind, "budget_kind")?;
    let (budget_amount_cents, budget_minutes) = parse_budget(bk, edit.budget_value)?;
    let rate_cents = parse_project_rate(edit.rate_value)?;
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_project(&mut tx, org_id, project_id).await?;

    let project = sqlx::query_as!(
        Project,
        r#"UPDATE projects
            SET name = $3, project_type = $4, currency = $5, budget_kind = $6,
                budget_amount_cents = $7, budget_minutes = $8, rate_cents = $9
          WHERE id = $1 AND org_id = $2
            AND (name, project_type, currency, budget_kind, budget_amount_cents, budget_minutes, rate_cents)
              IS DISTINCT FROM ($3, $4, $5, $6, $7, $8, $9)
         RETURNING id, org_id, client_id, code, name,
                   project_type as "project_type: ProjectType", currency, rate_cents,
                   starts_on as "starts_on: chrono::NaiveDate",
                   ends_on as "ends_on: chrono::NaiveDate",
                   budget_kind as "budget_kind: BudgetKind",
                   budget_amount_cents, budget_minutes, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        project_id,
        org_id,
        edit.name,
        pt as ProjectType,
        edit.currency,
        bk as BudgetKind,
        budget_amount_cents,
        budget_minutes,
        rate_cents,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let changed = project.is_some();
    tx.commit().await.map_err(server_err)?;
    Ok((project.unwrap_or(before), changed))
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let (project, transition) =
        set_project_active_record(&state.db, manager.org_id, project_id, active).await?;
    if let Some(t) = transition {
        let occurred_at = chrono::Utc::now();
        let project = project_payload(&project);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::ProjectReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    project,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::ProjectDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    project,
                }
            }
        });
    }
    Ok(project)
}

#[cfg(feature = "server")]
async fn set_project_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    active: bool,
) -> Result<(Project, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_project(&mut tx, org_id, project_id).await?;

    let project = sqlx::query_as!(
        Project,
        r#"UPDATE projects SET active = $3
          WHERE id = $1 AND org_id = $2
            AND active IS DISTINCT FROM $3
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
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let transition = project.as_ref().and_then(|updated| {
        crate::plugin::event::active_transition(Some(before.active), updated.active)
    });
    tx.commit().await.map_err(server_err)?;
    Ok((project.unwrap_or(before), transition))
}

#[cfg(feature = "server")]
async fn lock_project(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<Project, ServerFnError> {
    // Lock before comparing so competing edits and activation changes use
    // the latest committed row even when the request initially looked unchanged.
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
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))
}

// ── Tasks ────────────────────────────────────────────────────────────────────

/// Lists all active org-level tasks.
#[server]
pub async fn list_tasks() -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;

    let tasks = sqlx::query_as!(
        Task,
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks
         WHERE active = true AND org_id = $1
         ORDER BY name ASC",
        user.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(tasks)
}

/// Lists tasks linked to a specific project via the `project_tasks` join table.
#[server]
pub async fn list_project_tasks(project_id: String) -> Result<Vec<Task>, ServerFnError> {
    let user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;

    sqlx::query_as!(
        Task,
        "SELECT t.id, t.org_id, t.name, t.billable_default, t.default_rate_cents, t.active
         FROM tasks t
         JOIN project_tasks pt ON t.id = pt.task_id
         JOIN projects p ON p.id = pt.project_id
         WHERE pt.project_id = $1 AND t.active = true AND t.org_id = $2 AND p.org_id = $2
         ORDER BY t.name",
        project_id,
        user.org_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
    project_id: Option<String>,
) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_opt_uuid(project_id, "project_id")?;
    let task = create_task_for_project(
        &state.db,
        manager.org_id,
        &name,
        billable_default,
        project_id,
    )
    .await?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TaskCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            task: task_payload(&task),
        });
    Ok(task)
}

#[cfg(feature = "server")]
async fn create_task_for_project(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    project_id: Option<uuid::Uuid>,
) -> Result<Task, ServerFnError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(conflict("Task name cannot be empty"));
    }
    let mut tx = db.begin().await.map_err(server_err)?;
    let id = uuid::Uuid::now_v7();
    let task = sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default)
         VALUES ($1, $2, $3, $4)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        id,
        org_id,
        name,
        billable_default,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;

    if let Some(project_id) = project_id {
        enable_project_task(&mut tx, org_id, project_id, task.id).await?;
    }
    tx.commit().await.map_err(server_err)?;
    Ok(task)
}

#[server]
pub async fn update_task(
    task_id: String,
    name: String,
    billable_default: bool,
    default_rate_cents: Option<i64>,
) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, changed) = update_task_record(
        &state.db,
        manager.org_id,
        task_id,
        &name,
        billable_default,
        default_rate_cents,
    )
    .await?;
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TaskUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                task: task_payload(&task),
            });
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn update_task_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
    name: &str,
    billable_default: bool,
    default_rate_cents: Option<i64>,
) -> Result<(Task, bool), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_task(&mut tx, org_id, task_id).await?;

    let task = sqlx::query_as!(
        Task,
        "UPDATE tasks
            SET name = $3, billable_default = $4, default_rate_cents = $5
          WHERE id = $1 AND org_id = $2
            AND (name, billable_default, default_rate_cents) IS DISTINCT FROM ($3, $4, $5)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        task_id,
        org_id,
        name,
        billable_default,
        default_rate_cents,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?;

    let changed = task.is_some();
    tx.commit().await.map_err(server_err)?;
    Ok((task.unwrap_or(before), changed))
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(task_id: String, active: bool) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    let (task, transition) =
        set_task_active_record(&state.db, manager.org_id, task_id, active).await?;
    if let Some(t) = transition {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::TaskReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::TaskDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
        });
    }
    Ok(task)
}

#[cfg(feature = "server")]
async fn set_task_active_record(
    db: &sqlx::PgPool,
    org_id: uuid::Uuid,
    task_id: uuid::Uuid,
    active: bool,
) -> Result<(Task, Option<crate::plugin::event::ActiveTransition>), ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let before = lock_task(&mut tx, org_id, task_id).await?;

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
    tx.commit().await.map_err(server_err)?;
    Ok((task.unwrap_or(before), transition))
}

#[cfg(feature = "server")]
async fn lock_task(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Task not found"))
}

/// Enable an org-level task on a project so it becomes loggable there. The
/// project-task link inherits the task's default billable flag; idempotent.
/// Both the project and the task must belong to the manager's organization.
#[server]
pub async fn link_project_task(project_id: String, task_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;

    let mut tx = state.db.begin().await.map_err(server_err)?;
    enable_project_task(&mut tx, manager.org_id, project_id, task_id).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(())
}

#[cfg(feature = "server")]
async fn enable_project_task(
    db: &mut sqlx::PgConnection,
    org_id: uuid::Uuid,
    project_id: uuid::Uuid,
    task_id: uuid::Uuid,
) -> Result<(), ServerFnError> {
    // Validate before the idempotent insert, including already-linked pairs.
    // Hold these rows until commit so archiving cannot race task enablement.
    let task = sqlx::query!(
        "SELECT t.billable_default, t.default_rate_cents
         FROM projects p JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         JOIN tasks t ON t.org_id = p.org_id
         WHERE p.id = $1 AND t.id = $2 AND p.org_id = $3
           AND p.active AND c.active AND t.active
         FOR SHARE OF p, c, t",
        project_id,
        task_id,
        org_id,
    )
    .fetch_optional(&mut *db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Active project and task not found in this organization"))?;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
         VALUES ($1, $2, $3, $4) ON CONFLICT (project_id, task_id) DO NOTHING",
        project_id,
        task_id,
        task.billable_default,
        task.default_rate_cents,
    )
    .execute(db)
    .await
    .map_err(server_err)?;
    Ok(())
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_assignments(project_id: String) -> Result<Vec<Assignment>, ServerFnError> {
    let _user = require_user().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    sqlx::query_as!(
        Assignment,
        r#"SELECT id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM assignments WHERE project_id = $1 ORDER BY created_at"#,
        project_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn create_assignment(
    project_id: String,
    user_id: String,
    role: String,
) -> Result<Assignment, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let project_id = parse_uuid(&project_id, "project_id")?;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let pr: ProjectRole = parse_enum(&role, "role")?;
    let assignment = sqlx::query_as!(
        Assignment,
        r#"INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        project_id,
        user_id,
        pr as ProjectRole,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserAssignedToProject {
            occurred_at: chrono::Utc::now(),
            org_id: admin.org_id,
            assignment: assignment_payload(&assignment),
        });
    Ok(assignment)
}

#[server]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&assignment_id, "assignment_id")?;
    // Delete and capture the row atomically so the event carries its details
    // and a concurrent delete cannot double-notify.
    let removed = sqlx::query_as!(
        Assignment,
        r#"DELETE FROM assignments WHERE id = $1
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    if let Some(a) = removed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::AssignmentRemoved {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                assignment: assignment_payload(&a),
            });
    }
    Ok(())
}
