use super::validation::{optional_amount, optional_hours};
use super::*;
use crate::models::project_creation::{ReportVisibility, TaskSource};
use horae_core::project::{BudgetMode, FeeSchedule, MonthlyFeeDay, RateMode};
use std::collections::HashSet;
use uuid::Uuid;

pub(super) async fn finalize_draft_record(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    draft_id: Uuid,
    expected_revision: i64,
    form: &ProjectForm,
    email_available: bool,
) -> Result<Uuid, ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let draft = sqlx::query!(
        "SELECT revision, completed_project_id, discarded_at FROM project_drafts
         WHERE id = $1 AND org_id = $2 AND creator_id = $3 FOR UPDATE",
        draft_id,
        org_id,
        actor_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| not_found("Project draft not found"))?;
    if let Some(project_id) = draft.completed_project_id {
        tx.commit().await.map_err(storage_error)?;
        return Ok(project_id);
    }
    if draft.discarded_at.is_some() || draft.revision != expected_revision {
        return Err(conflict(
            "Draft changed or was discarded. Reload before creating the project.",
        ));
    }
    let payload = validate_draft_form(form, role == OrgRole::Admin)?;
    let client_id = form
        .client_id
        .ok_or_else(|| err(BAD_REQUEST, "Client is required"))?;
    let client = lock_creation_client(&mut tx, client_id, org_id).await?;
    let currency = form.currency.as_deref().unwrap_or(&client.currency);
    let org_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1 FOR SHARE",
        org_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    let settings = validate_project_form(form, currency, email_available)?;
    let project_id = Uuid::now_v7();
    let code = (!form.code.trim().is_empty()).then(|| form.code.trim());
    let project = sqlx::query_as!(Project,
        r#"INSERT INTO projects (id, org_id, client_id, name, code, project_type, currency,
             starts_on, ends_on, rate_cents, budget_kind, budget_amount_cents, budget_minutes)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
           RETURNING id, org_id, client_id, name, code, project_type as "project_type: ProjectType",
             currency, starts_on as "starts_on: chrono::NaiveDate", ends_on as "ends_on: chrono::NaiveDate",
             rate_cents, budget_kind as "budget_kind: BudgetKind", budget_amount_cents, budget_minutes,
             active, created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        project_id, org_id, client_id, form.name.trim(), code, form.project_type as ProjectType,
        currency, settings.starts_on as Option<chrono::NaiveDate>, settings.ends_on as Option<chrono::NaiveDate>, settings.rate_cents,
        settings.budget_kind as BudgetKind, settings.budget_cents, settings.budget_minutes,
    ).fetch_one(&mut *tx).await.map_err(storage_error)?;

    let (fee_mode, fee_amount, monthly_day) = match &settings.fee {
        None => (None, None, None),
        Some(FeeSchedule::Single { amount_cents }) => (Some("single"), Some(*amount_cents), None),
        Some(FeeSchedule::Milestones { .. }) => (Some("milestones"), None, None),
        Some(FeeSchedule::Monthly { amount_cents, day }) => {
            let day = match day {
                MonthlyFeeDay::First => "first",
                MonthlyFeeDay::Fifteenth => "fifteenth",
                MonthlyFeeDay::Last => "last",
            };
            (Some("monthly"), Some(*amount_cents), Some(day))
        }
    };
    let visibility = match form.report_visibility {
        ReportVisibility::Managers => "managers",
        ReportVisibility::ProjectMembers => "project_members",
    };
    let has_budget = form.budget_mode != BudgetMode::None;
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, budget_scope,
          monthly_reset, include_nonbillable, alert_enabled, alert_threshold, report_visibility,
          fee_mode, fee_amount_cents, monthly_day, terms_days, po_number, discount_bps, tax1_bps, tax2_name, tax2_bps)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
        Uuid::now_v7(), org_id, project_id, actor_id, settings.rate_mode, settings.budget_scope,
        has_budget && form.budget_monthly, has_budget && form.budget_nonbillable,
        has_budget && form.budget_alert, settings.alert_threshold, visibility,
        fee_mode, fee_amount, monthly_day, settings.terms_days, settings.po_number,
        settings.discount_bps, settings.tax1_bps, settings.tax2.as_ref().map(|(name, _)| name.as_str()),
        settings.tax2.as_ref().map(|(_, bps)| *bps),
    ).execute(&mut *tx).await.map_err(storage_error)?;
    if !form.admin_notes.trim().is_empty() {
        sqlx::query!(
            "INSERT INTO project_private_settings (id, org_id, project_id, admin_notes) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(), org_id, project_id, form.admin_notes.trim(),
        ).execute(&mut *tx).await.map_err(storage_error)?;
    }
    if let Some(FeeSchedule::Milestones { milestones }) = &settings.fee {
        for (position, milestone) in milestones.iter().enumerate() {
            let position =
                i16::try_from(position).map_err(|_| err(BAD_REQUEST, "Too many milestones"))?;
            sqlx::query!(
                "INSERT INTO project_fee_milestones (id,org_id,project_id,name,due_on,amount_cents,position)
                 VALUES ($1,$2,$3,$4,$5,$6,$7)",
                Uuid::now_v7(), org_id, project_id, milestone.name, milestone.due_on as chrono::NaiveDate, milestone.amount_cents, position,
            ).execute(&mut *tx).await.map_err(storage_error)?;
        }
    }
    // Consistent ordering avoids inverted row locks when projects reuse tags.
    let mut tags = settings.tags;
    tags.sort_by_cached_key(|name| name.to_lowercase());
    for name in tags {
        let tag_id = sqlx::query_scalar!(
            "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,$3)
             ON CONFLICT (org_id, lower(btrim(name))) DO UPDATE SET name = project_tags.name RETURNING id",
            Uuid::now_v7(), org_id, name,
        ).fetch_one(&mut *tx).await.map_err(storage_error)?;
        sqlx::query!(
            "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(),
            org_id,
            project_id,
            tag_id,
        )
        .execute(&mut *tx)
        .await
        .map_err(storage_error)?;
    }
    save_members(&mut tx, org_id, project_id, form, currency == org_currency).await?;
    save_tasks(&mut tx, org_id, project_id, form, currency == org_currency).await?;

    let event = crate::plugin::AppEvent::ProjectCreated {
        occurred_at: project.created_at,
        org_id,
        project: project_payload(&project),
    };
    crate::jobs::enqueue_outbox(
        &mut tx,
        org_id,
        "project_created",
        serde_json::to_value(event).map_err(|_| server_err("Could not record project creation"))?,
    )
    .await
    .map_err(|_| server_err("Could not record project creation"))?;
    sqlx::query!(
        "UPDATE project_drafts SET payload = $2, completed_project_id = $3, updated_at = now() WHERE id = $1",
        draft_id, payload, project_id,
    ).execute(&mut *tx).await.map_err(storage_error)?;
    tx.commit().await.map_err(storage_error)?;
    Ok(project_id)
}

async fn save_members(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    project_id: Uuid,
    form: &ProjectForm,
    uses_org_currency: bool,
) -> Result<(), ServerFnError> {
    let person_rates =
        form.project_type == ProjectType::TimeAndMaterials && form.rate_mode == RateMode::Person;
    for member in &form.team {
        let person = sqlx::query!(
            "SELECT billable_rate_cents FROM users WHERE id = $1 AND org_id = $2 AND active FOR SHARE",
            member.user_id, org_id,
        ).fetch_optional(&mut **tx).await.map_err(storage_error)?
            .ok_or_else(|| not_found("Select active teammates in this organization"))?;
        let rate = if person_rates {
            optional_amount(&member.billable_rate, "Person rate")?
        } else {
            None
        };
        if person_rates
            && rate.is_none()
            && person.billable_rate_cents.is_some()
            && !uses_org_currency
        {
            return Err(err(
                BAD_REQUEST,
                "Person rate: enter an explicit rate in the project currency",
            ));
        }
        let project_role = if member.manager {
            ProjectRole::Lead
        } else {
            ProjectRole::Freelancer
        };
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id,role,rate_cents) VALUES ($1,$2,$3,$4,$5)",
            Uuid::now_v7(), project_id, member.user_id, project_role as ProjectRole, rate,
        ).execute(&mut **tx).await.map_err(storage_error)?;
        if let Some(cost) = optional_amount(&member.cost_rate, "Cost rate")? {
            sqlx::query!(
                "INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,$5)",
                Uuid::now_v7(), org_id, project_id, member.user_id, cost,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        }
        if form.budget_mode == BudgetMode::HoursPerPerson
            && let Some(minutes) = optional_hours(&member.budget)?
        {
            sqlx::query!(
                    "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes) VALUES ($1,$2,$3,$4,$5)",
                    Uuid::now_v7(), org_id, project_id, member.user_id, minutes,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        }
    }
    Ok(())
}

async fn save_tasks(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    project_id: Uuid,
    form: &ProjectForm,
    uses_org_currency: bool,
) -> Result<(), ServerFnError> {
    let task_rates =
        form.project_type == ProjectType::TimeAndMaterials && form.rate_mode == RateMode::Task;
    let mut attached = HashSet::new();
    for task in &form.tasks {
        let (task_id, default_rate) = match &task.source {
            TaskSource::Existing { task_id } => {
                let row = sqlx::query!(
                    "SELECT id, default_rate_cents FROM tasks WHERE id = $1 AND org_id = $2 AND active FOR SHARE",
                    task_id, org_id,
                ).fetch_optional(&mut **tx).await.map_err(storage_error)?
                    .ok_or_else(|| not_found("Select active tasks in this organization"))?;
                (row.id, row.default_rate_cents)
            }
            TaskSource::New { name } => {
                let existing = sqlx::query!(
                    "SELECT id, active, default_rate_cents FROM tasks WHERE org_id = $1 AND lower(btrim(name)) = lower(btrim($2)) ORDER BY id LIMIT 1 FOR SHARE",
                    org_id, name,
                ).fetch_optional(&mut **tx).await.map_err(storage_error)?;
                if let Some(row) = existing {
                    if !row.active {
                        return Err(conflict(
                            "A task with this name is archived. Reactivate it or choose another name.",
                        ));
                    }
                    (row.id, row.default_rate_cents)
                } else {
                    let id = Uuid::now_v7();
                    sqlx::query!(
                        "INSERT INTO tasks (id,org_id,name,billable_default) VALUES ($1,$2,$3,$4)",
                        id,
                        org_id,
                        name.trim(),
                        task.billable && form.project_type != ProjectType::NonBillable,
                    )
                    .execute(&mut **tx)
                    .await
                    .map_err(storage_error)?;
                    (id, None)
                }
            }
        };
        if !attached.insert(task_id) {
            return Err(err(BAD_REQUEST, "A task can only be selected once"));
        }
        let rate = if task_rates {
            let explicit = optional_amount(&task.rate, "Task rate")?;
            if explicit.is_none() && default_rate.is_some() && !uses_org_currency {
                return Err(err(
                    BAD_REQUEST,
                    "Task rate: enter an explicit rate in the project currency",
                ));
            }
            explicit.or(default_rate)
        } else {
            None
        };
        sqlx::query!(
            "INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,$3,$4)",
            project_id, task_id, task.billable && form.project_type != ProjectType::NonBillable, rate,
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
        sqlx::query!(
            "INSERT INTO project_task_settings (id,org_id,project_id,task_id,restricted,budget_minutes,budget_cents) VALUES ($1,$2,$3,$4,$5,$6,$7)",
            Uuid::now_v7(), org_id, project_id, task_id, matches!(task.access, TaskAccess::Restricted { .. }), minutes, cents,
        ).execute(&mut **tx).await.map_err(storage_error)?;
        if let TaskAccess::Restricted { user_ids } = &task.access {
            let mut allowed = HashSet::new();
            for user_id in user_ids {
                if !allowed.insert(*user_id) {
                    continue;
                }
                sqlx::query!(
                    "INSERT INTO project_task_members (id,org_id,project_id,task_id,user_id) VALUES ($1,$2,$3,$4,$5)",
                    Uuid::now_v7(), org_id, project_id, task_id, user_id,
                ).execute(&mut **tx).await.map_err(storage_error)?;
            }
        }
    }
    Ok(())
}
