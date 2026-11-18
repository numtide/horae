//! The project editor: loading an existing project into the form used for
//! creation ([`load`]), the catalogs it offers ([`catalog`]) and saving it
//! ([`save`]).
//!
//! Each authorizes with [`authorize_write`]: `ProjectWriteAll`, or
//! `ProjectWriteManaged` on a project the actor manages. What the editor
//! shows and accepts of the rates, costs and private notes then depends on
//! the actor's financial grants ([`projection`]).

use horae_core::duration::format_hhmm;
use horae_core::money::format_cents_plain;
use horae_core::project::{BudgetMode, MonthlyFeeDay, RateMode};
use horae_core::types::{BudgetKind, ProjectRole, ProjectType};
use uuid::Uuid;

use super::form::{internal, storage};
use super::{EDITING_REQUIRED, authorize_write};
use crate::models::project_creation::{
    CreationClient, CreationPerson, CreationSelection, CreationTask, EditableProject, FeeMode,
    InvoiceDefaultsInput, MilestoneInput, ProjectForm, ProjectMemberInput, ProjectTaskInput,
    ReportVisibility, SecondTaxInput, TaskAccess, TaskSource,
};
use crate::ops::{Actor, OpError, Tx};

mod associations;
pub mod catalog;
mod projection;
mod protected;
mod save;
mod task_activity;

pub use save::save;

fn apply_manager_flags(
    form: &mut ProjectForm,
    managers: &crate::models::project_managers::ProjectManagers,
) {
    for member in &mut form.team {
        member.manager = managers
            .managers
            .binary_search_by_key(&member.user_id, |manager| manager.id)
            .is_ok();
    }
}

/// The project as the editor shows it to the actor, with what they may not
/// read withheld. Run it in a snapshot (`ops::begin_read_snapshot`): every
/// section must describe the same committed project, even when another
/// manager saves while the editor loads its associations.
pub async fn load(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
) -> Result<EditableProject, OpError> {
    authorize_write(tx, actor, project_id).await?;
    let mut project = load_project_form(tx, actor.org_id(), project_id).await?;
    projection::redact(tx, actor, &mut project).await?;
    Ok(project)
}

/// The stored project as the form shows it, without authorizing or
/// withholding anything: [`load`] and [`save`] authorize first, and tests
/// compare what a save stored.
pub async fn load_project_form(
    tx: &mut Tx,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<EditableProject, OpError> {
    let project = sqlx::query!(
        r#"SELECT client_id, name, code, currency, active, rate_cents, edit_revision,
           project_type as "project_type: ProjectType", budget_kind as "budget_kind: BudgetKind",
           budget_minutes, budget_amount_cents,
           starts_on as "starts_on: chrono::NaiveDate", ends_on as "ends_on: chrono::NaiveDate"
           FROM projects WHERE id = $1 AND org_id = $2"#,
        project_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or_else(|| OpError::not_found("Project not found"))?;
    let settings = sqlx::query!(
        "SELECT rate_mode, budget_scope, monthly_reset, include_nonbillable,
           alert_enabled, alert_threshold, report_visibility, fee_mode, fee_amount_cents,
           monthly_day, terms_days, po_number, discount_bps, tax1_bps, tax2_name, tax2_bps
         FROM project_settings WHERE project_id = $1 AND org_id = $2",
        project_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let configured = settings.is_some();
    let scope = settings
        .as_ref()
        .map_or("project", |s| s.budget_scope.as_str());
    let budget_mode = match (project.budget_kind, scope) {
        (BudgetKind::None, _) => BudgetMode::None,
        (BudgetKind::Hours, "task") => BudgetMode::HoursPerTask,
        (BudgetKind::Amount, "task") => BudgetMode::FeesPerTask,
        (BudgetKind::Hours, "person") => BudgetMode::HoursPerPerson,
        (BudgetKind::Hours, "project") => BudgetMode::TotalHours,
        (BudgetKind::Amount, "project") => BudgetMode::TotalFees,
        _ => return Err(internal("Project budget settings cannot be read")),
    };
    let budget_value = match budget_mode {
        BudgetMode::TotalHours => project.budget_minutes.map(format_hhmm).unwrap_or_default(),
        BudgetMode::TotalFees => project
            .budget_amount_cents
            .map(format_cents_plain)
            .unwrap_or_default(),
        _ => String::new(),
    };
    let mut form = ProjectForm {
        client_id: Some(project.client_id),
        name: project.name,
        code: project.code.unwrap_or_default(),
        currency: Some(project.currency),
        starts_on: project
            .starts_on
            .map(|date| date.to_string())
            .unwrap_or_default(),
        ends_on: project
            .ends_on
            .map(|date| date.to_string())
            .unwrap_or_default(),
        project_type: project.project_type,
        rate_mode: RateMode::Legacy,
        // The legacy read boundary includes assigned members, and its hourly
        // overview counts all tracked minutes rather than only billable time.
        report_visibility: ReportVisibility::ProjectMembers,
        budget_nonbillable: project.budget_kind == BudgetKind::Hours,
        project_rate: project
            .rate_cents
            .map(format_cents_plain)
            .unwrap_or_default(),
        budget_mode,
        budget_value,
        ..ProjectForm::default()
    };
    if let Some(settings) = settings {
        form.rate_mode = match settings.rate_mode.as_str() {
            "person" => RateMode::Person,
            "task" => RateMode::Task,
            "project" => RateMode::Project,
            _ => return Err(internal("Project rate settings cannot be read")),
        };
        form.budget_monthly = settings.monthly_reset;
        form.budget_nonbillable = settings.include_nonbillable;
        form.budget_alert = settings.alert_enabled;
        form.budget_alert_at = settings.alert_threshold.to_string();
        form.report_visibility = match settings.report_visibility.as_str() {
            "managers" => ReportVisibility::Managers,
            "project_members" => ReportVisibility::ProjectMembers,
            _ => return Err(internal("Project visibility settings cannot be read")),
        };
        form.fee_mode = match settings.fee_mode.as_deref() {
            None | Some("single") => FeeMode::Single,
            Some("milestones") => FeeMode::Milestones,
            Some("monthly") => FeeMode::Monthly,
            _ => return Err(internal("Project fee settings cannot be read")),
        };
        form.fee_amount = settings
            .fee_amount_cents
            .map(format_cents_plain)
            .unwrap_or_default();
        form.monthly_day = match settings.monthly_day.as_deref() {
            None | Some("first") => MonthlyFeeDay::First,
            Some("fifteenth") => MonthlyFeeDay::Fifteenth,
            Some("last") => MonthlyFeeDay::Last,
            _ => return Err(internal("Project monthly fee settings cannot be read")),
        };
        form.invoice_defaults = InvoiceDefaultsInput {
            terms_days: settings.terms_days.to_string(),
            po_number: settings.po_number,
            tax: format_cents_plain(i64::from(settings.tax1_bps)),
            second_tax: settings
                .tax2_name
                .zip(settings.tax2_bps)
                .map(|(name, bps)| SecondTaxInput {
                    name,
                    percentage: format_cents_plain(i64::from(bps)),
                }),
            discount: format_cents_plain(i64::from(settings.discount_bps)),
        };
    }
    form.admin_notes = sqlx::query_scalar!(
        "SELECT admin_notes FROM project_private_settings WHERE project_id = $1 AND org_id = $2",
        project_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .unwrap_or_default();
    form.tags = sqlx::query_scalar!(
        "SELECT t.name FROM project_tags t JOIN project_tag_links l ON l.tag_id = t.id AND l.org_id = t.org_id
         WHERE l.project_id = $1 AND l.org_id = $2 ORDER BY lower(t.name), t.id",
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?;
    form.milestones = sqlx::query!(
        r#"SELECT id, name, due_on as "due_on: chrono::NaiveDate", amount_cents
           FROM project_fee_milestones WHERE project_id = $1 AND org_id = $2 ORDER BY position, id"#,
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?.into_iter().map(|row| MilestoneInput {
        id: row.id, name: row.name, due_on: row.due_on.to_string(), amount: format_cents_plain(row.amount_cents),
    }).collect();
    form.team = load_members(tx, org_id, project_id).await?;
    form.tasks = load_tasks(tx, org_id, project_id, budget_mode).await?;
    let client = sqlx::query_as!(CreationClient,
        "SELECT id, name, currency, active, default_rate_cents FROM clients WHERE id = $1 AND org_id = $2",
        project.client_id, org_id,
    ).fetch_optional(&mut **tx).await.map_err(storage)?
        .ok_or_else(|| internal("Project client cannot be read"))?;
    // Resolve assigned identities, including archived records, independently
    // of the active catalog's search and pagination. They are not new choices.
    let tasks = sqlx::query!(
        "SELECT t.id, t.name, t.active, pt.active AS link_active, t.billable_default, t.default_rate_cents, t.default_rate_currency
         FROM tasks t JOIN project_tasks pt ON pt.task_id = t.id
         WHERE pt.project_id = $1 AND t.org_id = $2 ORDER BY t.id",
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?;
    let inactive_task_ids = tasks
        .iter()
        .filter(|task| !task.active)
        .map(|task| task.id)
        .collect();
    let archived_task_ids = tasks
        .iter()
        .filter(|task| !task.link_active)
        .map(|task| task.id)
        .collect();
    let tasks = tasks
        .into_iter()
        .map(|task| CreationTask {
            id: task.id,
            name: task.name,
            billable: task.billable_default,
            default_rate_cents: task.default_rate_cents,
            default_rate_currency: task.default_rate_currency,
        })
        .collect();
    let people = sqlx::query!(
        "SELECT u.id, u.name, u.active, u.billable_rate_cents, u.cost_rate_cents
         FROM users u JOIN assignments a ON a.user_id = u.id
         WHERE a.project_id = $1 AND u.org_id = $2 ORDER BY u.id",
        project_id,
        org_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let inactive_user_ids = people
        .iter()
        .filter(|person| !person.active)
        .map(|person| person.id)
        .collect();
    let people = people
        .into_iter()
        .map(|person| CreationPerson {
            id: person.id,
            name: person.name,
            billable_rate_cents: person.billable_rate_cents,
            cost_rate_cents: person.cost_rate_cents,
        })
        .collect();
    Ok(EditableProject {
        id: project_id,
        revision: project.edit_revision,
        active: project.active,
        configured,
        form,
        client,
        selection: CreationSelection { tasks, people },
        inactive_task_ids,
        archived_task_ids,
        inactive_user_ids,
        access: None,
    })
}

async fn load_members(
    tx: &mut Tx,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<ProjectMemberInput>, OpError> {
    let rows = sqlx::query!(
        r#"SELECT a.user_id, a.role as "role: ProjectRole", a.rate_cents,
           c.cost_rate_cents as "cost_rate_cents?", b.budget_minutes as "budget_minutes?"
           FROM assignments a JOIN users u ON u.id = a.user_id AND u.org_id = $2
           LEFT JOIN project_member_costs c ON c.project_id = a.project_id AND c.user_id = a.user_id AND c.org_id = $2
           LEFT JOIN project_member_budgets b ON b.project_id = a.project_id AND b.user_id = a.user_id AND b.org_id = $2
           WHERE a.project_id = $1 ORDER BY a.user_id"#,
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?;
    Ok(rows
        .into_iter()
        .map(|row| ProjectMemberInput {
            user_id: row.user_id,
            manager: matches!(row.role, ProjectRole::Lead | ProjectRole::Admin),
            billable_rate: row.rate_cents.map(format_cents_plain).unwrap_or_default(),
            cost_rate: row
                .cost_rate_cents
                .map(format_cents_plain)
                .unwrap_or_default(),
            budget: row.budget_minutes.map(format_hhmm).unwrap_or_default(),
        })
        .collect())
}

async fn load_tasks(
    tx: &mut Tx,
    org_id: Uuid,
    project_id: Uuid,
    budget_mode: BudgetMode,
) -> Result<Vec<ProjectTaskInput>, OpError> {
    let rows = sqlx::query!(
        r#"SELECT pt.task_id, pt.billable, pt.rate_cents, s.restricted as "restricted?", s.budget_minutes, s.budget_cents
         FROM project_tasks pt JOIN tasks t ON t.id = pt.task_id AND t.org_id = $2
         LEFT JOIN project_task_settings s ON s.project_id = pt.project_id AND s.task_id = pt.task_id AND s.org_id = $2
         WHERE pt.project_id = $1 ORDER BY pt.task_id"#,
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?;
    let access = sqlx::query!(
        "SELECT task_id, user_id FROM project_task_members WHERE project_id = $1 AND org_id = $2 ORDER BY task_id, user_id",
        project_id, org_id,
    ).fetch_all(&mut **tx).await.map_err(storage)?;
    Ok(rows
        .into_iter()
        .map(|row| ProjectTaskInput {
            id: row.task_id,
            source: TaskSource::Existing {
                task_id: row.task_id,
            },
            billable: row.billable,
            rate: row.rate_cents.map(format_cents_plain).unwrap_or_default(),
            // A stale monetary setting must not become a visible hours budget.
            budget: match budget_mode {
                BudgetMode::HoursPerTask => row.budget_minutes.map(format_hhmm),
                BudgetMode::FeesPerTask => row.budget_cents.map(format_cents_plain),
                _ => None,
            }
            .unwrap_or_default(),
            access: if row.restricted.unwrap_or(false) {
                TaskAccess::Restricted {
                    user_ids: access
                        .iter()
                        .filter(|entry| entry.task_id == row.task_id)
                        .map(|entry| entry.user_id)
                        .collect(),
                }
            } else {
                TaskAccess::Everyone
            },
        })
        .collect())
}
