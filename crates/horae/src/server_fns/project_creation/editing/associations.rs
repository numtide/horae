use super::super::validation::{optional_amount, optional_hours, with_field};
use super::*;
use crate::models::project_creation::{ProjectFormField, ProjectMemberInput};
use std::collections::HashSet;

pub(super) async fn save(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    before: &EditableProject,
    form: &ProjectForm,
    role: OrgRole,
) -> Result<(), ServerFnError> {
    let org_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id=$1 FOR SHARE",
        org_id
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(storage_error)?;
    let mut members: Vec<_> = form.team.iter().collect();
    members.sort_by_key(|member| member.user_id);
    for member in members {
        save_member(tx, org_id, before, form, member, role, &org_currency).await?;
    }
    save_tasks(tx, org_id, before, form).await?;
    // Remove access links before assignments; retained links must never depend
    // on cascading deletion of a still-used teammate.
    for member in &before.form.team {
        if !form.team.iter().any(|item| item.user_id == member.user_id) {
            let history = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM time_entries WHERE project_id=$1 AND user_id=$2)",
                before.id,
                member.user_id
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(storage_error)?
            .unwrap_or(false);
            if history {
                return Err(with_field(
                    conflict("A teammate with recorded time cannot be removed."),
                    ProjectFormField::Person(member.user_id),
                ));
            }
            if role != OrgRole::Admin {
                let private_cost = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM project_member_costs WHERE project_id=$1 AND org_id=$2 AND user_id=$3)", before.id, org_id, member.user_id)
                    .fetch_one(&mut **tx).await.map_err(storage_error)?.unwrap_or(false);
                if private_cost {
                    return Err(forbidden(
                        "An administrator must remove this assignment because it has private cost settings.",
                    ));
                }
            }
            sqlx::query!(
                "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
                before.id,
                member.user_id
            )
            .execute(&mut **tx)
            .await
            .map_err(storage_error)?;
        }
    }
    Ok(())
}

async fn save_member(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    before: &EditableProject,
    form: &ProjectForm,
    member: &ProjectMemberInput,
    role: OrgRole,
    org_currency: &str,
) -> Result<(), ServerFnError> {
    let previous = before
        .form
        .team
        .iter()
        .find(|item| item.user_id == member.user_id);
    let person = sqlx::query!(
        "SELECT active, billable_rate_cents FROM users WHERE id=$1 AND org_id=$2 FOR SHARE",
        member.user_id,
        org_id
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| {
        with_field(
            not_found("Teammate not found"),
            ProjectFormField::Person(member.user_id),
        )
    })?;
    if !person.active && previous != Some(member) {
        return Err(with_field(
            conflict(
                "This teammate is inactive. Keep its existing assignment unchanged or remove it if it has no recorded time.",
            ),
            ProjectFormField::Person(member.user_id),
        ));
    }
    let person_rates = form.rate_mode == RateMode::Legacy
        || (form.project_type == ProjectType::TimeAndMaterials
            && form.rate_mode == RateMode::Person);
    let rate = optional_amount(
        if person_rates {
            &member.billable_rate
        } else {
            previous.map_or("", |item| item.billable_rate.as_str())
        },
        "Person rate",
    )
    .map_err(|error| with_field(error, ProjectFormField::PersonRate(member.user_id)))?;
    let rate_changed = previous.is_none_or(|item| item.billable_rate != member.billable_rate)
        || before.form.rate_mode != form.rate_mode;
    if before.configured
        && person_rates
        && rate_changed
        && rate.is_none()
        && person.billable_rate_cents.is_some()
        && super::save::project_currency(before) != org_currency
    {
        return Err(with_field(
            err(
                BAD_REQUEST,
                "Enter an explicit person rate in the project currency.",
            ),
            ProjectFormField::PersonRate(member.user_id),
        ));
    }
    let manager_changed = previous.is_none_or(|item| item.manager != member.manager);
    let assignment_role = if member.manager {
        ProjectRole::Lead
    } else {
        ProjectRole::Freelancer
    };
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role,rate_cents) VALUES ($1,$2,$3,$4,$5)
         ON CONFLICT (project_id,user_id) DO UPDATE SET rate_cents=EXCLUDED.rate_cents,
           role=CASE WHEN $6 THEN EXCLUDED.role ELSE assignments.role END",
        Uuid::now_v7(),
        before.id,
        member.user_id,
        assignment_role as ProjectRole,
        rate,
        manager_changed,
    )
    .execute(&mut **tx)
    .await
    .map_err(storage_error)?;
    if role == OrgRole::Admin {
        if let Some(cost) = optional_amount(&member.cost_rate, "Cost rate")? {
            sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,$5)
                ON CONFLICT (project_id,user_id) DO UPDATE SET cost_rate_cents=EXCLUDED.cost_rate_cents",
                Uuid::now_v7(), org_id, before.id, member.user_id, cost,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        } else {
            sqlx::query!(
                "DELETE FROM project_member_costs WHERE project_id=$1 AND org_id=$2 AND user_id=$3",
                before.id,
                org_id,
                member.user_id
            )
            .execute(&mut **tx)
            .await
            .map_err(storage_error)?;
        }
    }
    let budget = if form.budget_mode == BudgetMode::HoursPerPerson {
        optional_hours(&member.budget)?
    } else {
        None
    };
    if let Some(minutes) = budget {
        sqlx::query!("INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes) VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT (project_id,user_id) DO UPDATE SET budget_minutes=EXCLUDED.budget_minutes",
            Uuid::now_v7(), org_id, before.id, member.user_id, minutes,
        ).execute(&mut **tx).await.map_err(storage_error)?;
    } else {
        sqlx::query!(
            "DELETE FROM project_member_budgets WHERE project_id=$1 AND org_id=$2 AND user_id=$3",
            before.id,
            org_id,
            member.user_id
        )
        .execute(&mut **tx)
        .await
        .map_err(storage_error)?;
    }
    Ok(())
}

async fn save_tasks(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    before: &EditableProject,
    form: &ProjectForm,
) -> Result<(), ServerFnError> {
    let mut selected = HashSet::new();
    for task in &form.tasks {
        let task_id = match &task.source {
            TaskSource::Existing { task_id } => *task_id,
            TaskSource::New { name } => {
                let existing = sqlx::query!("SELECT id, active FROM tasks WHERE org_id=$1 AND lower(btrim(name))=lower(btrim($2)) ORDER BY id LIMIT 1 FOR SHARE", org_id, name)
                    .fetch_optional(&mut **tx).await.map_err(storage_error)?;
                if let Some(existing) = existing {
                    if !existing.active {
                        return Err(with_field(
                            conflict("A task with this name is archived."),
                            ProjectFormField::TaskName(task.id),
                        ));
                    }
                    existing.id
                } else {
                    let id = Uuid::now_v7();
                    sqlx::query!(
                        "INSERT INTO tasks (id,org_id,name,billable_default) VALUES ($1,$2,$3,$4)",
                        id,
                        org_id,
                        name.trim(),
                        task.billable && form.project_type != ProjectType::NonBillable
                    )
                    .execute(&mut **tx)
                    .await
                    .map_err(storage_error)?;
                    id
                }
            }
        };
        if !selected.insert(task_id) {
            return Err(with_field(
                err(BAD_REQUEST, "A task can only be selected once"),
                ProjectFormField::Task(task.id),
            ));
        }
        let previous = before
            .form
            .tasks
            .iter()
            .find(|item| item.source == (TaskSource::Existing { task_id }));
        let catalog = sqlx::query!("SELECT active, default_rate_cents, default_rate_currency FROM tasks WHERE id=$1 AND org_id=$2 FOR SHARE", task_id, org_id)
            .fetch_optional(&mut **tx).await.map_err(storage_error)?
            .ok_or_else(|| with_field(not_found("Task not found"), ProjectFormField::Task(task.id)))?;
        if !catalog.active && previous != Some(task) {
            return Err(with_field(
                conflict(
                    "This task is archived. Keep its existing settings unchanged or remove it if it has no recorded time.",
                ),
                ProjectFormField::Task(task.id),
            ));
        }
        let task_rates = form.rate_mode == RateMode::Legacy
            || (form.project_type == ProjectType::TimeAndMaterials
                && form.rate_mode == RateMode::Task);
        let mut rate = optional_amount(
            if task_rates {
                &task.rate
            } else {
                previous.map_or("", |item| item.rate.as_str())
            },
            "Task rate",
        )
        .map_err(|error| with_field(error, ProjectFormField::TaskRate(task.id)))?;
        let rate_changed = previous.is_none_or(|item| item.rate != task.rate)
            || before.form.rate_mode != form.rate_mode;
        if before.configured && task_rates && rate_changed && rate.is_none() {
            if catalog.default_rate_cents.is_some()
                && !catalog
                    .default_rate_currency
                    .as_deref()
                    .is_some_and(|currency| {
                        currency.eq_ignore_ascii_case(super::save::project_currency(before))
                    })
            {
                return Err(with_field(
                    err(
                        BAD_REQUEST,
                        "Enter an explicit task rate in the project currency.",
                    ),
                    ProjectFormField::TaskRate(task.id),
                ));
            }
            rate = catalog.default_rate_cents;
        }
        sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,$3,$4)
            ON CONFLICT (project_id,task_id) DO UPDATE SET billable=EXCLUDED.billable, rate_cents=EXCLUDED.rate_cents",
            before.id, task_id, task.billable && form.project_type != ProjectType::NonBillable, rate,
        ).execute(&mut **tx).await.map_err(storage_error)?;
        let minutes = if form.budget_mode == BudgetMode::HoursPerTask {
            optional_hours(&task.budget)?
        } else {
            None
        };
        let cents = if form.budget_mode == BudgetMode::FeesPerTask {
            optional_amount(&task.budget, "Task budget")?
        } else {
            None
        };
        // Unchanged legacy links do not need a synthetic settings row.
        if before.configured
            || matches!(task.access, TaskAccess::Restricted { .. })
            || previous.is_some_and(|item| item.access != task.access)
        {
            sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,restricted,budget_minutes,budget_cents) VALUES ($1,$2,$3,$4,$5,$6,$7)
                ON CONFLICT (project_id,task_id) DO UPDATE SET restricted=EXCLUDED.restricted, budget_minutes=EXCLUDED.budget_minutes, budget_cents=EXCLUDED.budget_cents",
                Uuid::now_v7(), org_id, before.id, task_id, matches!(task.access, TaskAccess::Restricted { .. }), minutes, cents,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        }
        let allowed: &[Uuid] = match &task.access {
            TaskAccess::Everyone => &[],
            TaskAccess::Restricted { user_ids } => user_ids,
        };
        sqlx::query!("DELETE FROM project_task_members WHERE project_id=$1 AND org_id=$2 AND task_id=$3 AND NOT(user_id=ANY($4))", before.id, org_id, task_id, allowed)
            .execute(&mut **tx).await.map_err(storage_error)?;
        for user_id in allowed {
            sqlx::query!("INSERT INTO project_task_members (id,org_id,project_id,task_id,user_id) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (project_id,task_id,user_id) DO NOTHING",
                Uuid::now_v7(), org_id, before.id, task_id, user_id,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        }
    }
    for previous in &before.form.tasks {
        let TaskSource::Existing { task_id } = previous.source else {
            continue;
        };
        if !selected.contains(&task_id) {
            let history = sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM time_entries WHERE project_id=$1 AND task_id=$2)",
                before.id,
                task_id
            )
            .fetch_one(&mut **tx)
            .await
            .map_err(storage_error)?
            .unwrap_or(false);
            if history {
                return Err(with_field(
                    conflict("A task with recorded time cannot be removed."),
                    ProjectFormField::Task(previous.id),
                ));
            }
            sqlx::query!(
                "DELETE FROM project_tasks WHERE project_id=$1 AND task_id=$2",
                before.id,
                task_id
            )
            .execute(&mut **tx)
            .await
            .map_err(storage_error)?;
        }
    }
    Ok(())
}
