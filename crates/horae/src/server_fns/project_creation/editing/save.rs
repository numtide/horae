use super::super::validation::{ValidatedProject, optional_amount, with_field};
use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project_creation::{
    ProjectFieldAccess, ProjectFormField, ProtectedProjectField,
};
use crate::models::project_managers::{ProjectManagersCommand, ProjectManagersCommandKind};
use crate::server_fns::{permissions::project_management, project_managers::map_error};
use horae_core::permissions::catalog::Permission;
use horae_core::project::FeeSchedule;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalReceipt {
    version: u8,
    intent: serde_json::Value,
    effects: super::protected::RequiredWrites,
}

pub(super) async fn save(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    request: &ProjectEditRequest,
    email_available: bool,
) -> Result<(Project, bool), ServerFnError> {
    if request.id.get_version_num() != 7 || request.expected_revision <= 0 {
        return Err(err(BAD_REQUEST, "Invalid edit identity or revision"));
    }
    let mut tx = pool.begin().await.map_err(storage_error)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE, READ WRITE")
        .execute(&mut *tx)
        .await
        .map_err(storage_error)?;
    crate::server_fns::permissions::configure_transaction_limits(&mut tx)
        .await
        .map_err(storage_error)?;
    let (role, permissions) = lock_editor_actor(
        &mut tx,
        actor_id,
        org_id,
        request.project_id,
        OrganizationLock::AccessChange,
    )
    .await?;
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    if request
        .expected_requester
        .is_some_and(|expected| expected != requester)
        || (permissions.is_some() && request.expected_requester.is_none())
    {
        return Err(ServerFnError::ServerError {
            code: CONFLICT,
            message: "The editing session changed. Reload the project before saving.".into(),
            details: Some(serde_json::json!({
                "reason": crate::models::project_creation::PROJECT_EDITOR_SESSION_CHANGED
            })),
        });
    }
    super::protected::validate_intent(request)?;
    if permissions.is_none() && (!request.unchanged.is_empty() || request.managers.is_some()) {
        return Err(err(
            BAD_REQUEST,
            "Protected-field intent requires the current permission policy",
        ));
    }
    let raw_payload = validate_draft_form(
        &request.form,
        permissions.is_some() || role == OrgRole::Admin,
    )?;
    let managers = if permissions.is_some() {
        let mut selection = request.managers.clone().ok_or_else(|| {
            conflict("Reload the complete project manager selection before saving.")
        })?;
        selection.manager_ids.sort_unstable();
        if selection
            .manager_ids
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(err(BAD_REQUEST, "A manager may only be selected once"));
        }
        if request.form.team.iter().any(|member| {
            member.manager != selection.manager_ids.binary_search(&member.user_id).is_ok()
        }) {
            return Err(err(
                BAD_REQUEST,
                "Team manager controls must match the complete manager selection",
            ));
        }
        Some(selection)
    } else {
        None
    };
    let intent = if permissions.is_some() {
        let mut unchanged = request.unchanged.clone();
        unchanged.sort_unstable();
        serde_json::json!({
            "form": raw_payload,
            "unchanged": unchanged,
            "expected_requester": requester,
            "managers": managers,
        })
    } else {
        raw_payload
    };
    let revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM projects WHERE id = $1 AND org_id = $2 FOR UPDATE",
        request.project_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| not_found("Project not found"))?;
    let mut before = load_project_form(
        &mut tx,
        org_id,
        request.project_id,
        permissions.is_some() || role == OrgRole::Admin,
    )
    .await?;
    let access = if let Some(permissions) = &permissions {
        let mut view = before.clone();
        super::projection::redact(&mut tx, org_id, actor_id, permissions, &mut view).await?;
        Some(
            view.access
                .ok_or_else(|| server_err("Project editor access is unavailable"))?,
        )
    } else {
        None
    };
    if let Some(access) = &access {
        // Validation compares canonical responsibility; persistence separately
        // preserves legacy membership roles, including archived retained rows.
        apply_manager_flags(&mut before.form, &access.managers);
        before.access = Some(access.clone());
    }
    let may_write_tasks = permissions
        .as_ref()
        .is_none_or(|permissions| permissions.grants.contains(Permission::TaskWriteAll));
    let completed = sqlx::query!(
        "SELECT expected_revision, completed_revision, payload FROM project_edit_requests
         WHERE id = $1 AND project_id = $2 AND org_id = $3 AND actor_id = $4",
        request.id,
        request.project_id,
        org_id,
        actor_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?;
    if let Some(completed) = completed {
        let saved_intent = if let Some(access) = &access {
            let receipt: CanonicalReceipt = serde_json::from_value(completed.payload)
                .map_err(|_| conflict("Reload the project before retrying this edit."))?;
            if receipt.version != 1 {
                return Err(conflict("Reload the project before retrying this edit."));
            }
            receipt.effects.authorize(access, may_write_tasks)?;
            receipt.intent
        } else {
            completed.payload
        };
        if completed.expected_revision != request.expected_revision
            || saved_intent != intent
            || completed.completed_revision != revision
        {
            return Err(conflict(
                "This edit was already saved or the project changed. Reload before editing again.",
            ));
        }
        let project = project_record(&mut tx, org_id, request.project_id).await?;
        tx.commit().await.map_err(storage_error)?;
        return Ok((project, false));
    }
    if sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM project_edit_requests WHERE id=$1)",
        request.id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?
    .unwrap_or(false)
    {
        return Err(conflict(
            "This edit request identity has already been used.",
        ));
    }
    if revision != request.expected_revision {
        return Err(conflict(
            "The project changed in another session. Reload before saving.",
        ));
    }
    if managers.is_some()
        && sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM permission_change_receipts
             WHERE org_id=$1 AND actor_user_id=$2 AND request_id=$3)",
            org_id,
            actor_id,
            request.id,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(storage_error)?
        .unwrap_or(false)
    {
        return Err(conflict(
            "This edit request identity has already been used.",
        ));
    }
    let merged;
    let payload;
    let form = if let Some(access) = &access {
        let (form, mut effects) = super::protected::merge(request, &before.form, access)?;
        effects.task_catalog = form
            .tasks
            .iter()
            .any(|task| matches!(task.source, TaskSource::New { .. }));
        let retained_tasks: Vec<_> = form
            .tasks
            .iter()
            .filter_map(|task| match task.source {
                TaskSource::Existing { task_id } => Some(task_id),
                TaskSource::New { .. } => None,
            })
            .collect();
        let keep_budget = request.unchanged.contains(&ProtectedProjectField::Budget);
        // Inspect stored units as well: malformed/stale cents are withheld by
        // the reader, but deleting them is still a monetary change.
        effects.billable |= sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM project_task_settings
             WHERE org_id=$1 AND project_id=$2 AND budget_cents IS NOT NULL
               AND (NOT $3 OR NOT(task_id=ANY($4))))",
            org_id,
            request.project_id,
            keep_budget,
            &retained_tasks,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(storage_error)?
        .unwrap_or(false);
        effects.authorize(access, may_write_tasks)?;
        payload = serde_json::to_value(CanonicalReceipt {
            version: 1,
            intent,
            effects,
        })
        .map_err(server_err)?;
        merged = form;
        &merged
    } else {
        payload = intent;
        &request.form
    };
    if serde_json::to_string_pretty(&payload)
        .map_err(server_err)?
        .len()
        > 256 * 1024
    {
        return Err(err(BAD_REQUEST, "Project edit exceeds 256 KiB"));
    }
    let may_write_costs = access.as_ref().map_or(role == OrgRole::Admin, |access| {
        access.costs == ProjectFieldAccess::Editable
    });
    let may_write_notes = access.as_ref().map_or(role == OrgRole::Admin, |access| {
        access.private_notes == ProjectFieldAccess::Editable
    });
    let unchanged = access.as_ref().map(|_| request.unchanged.as_slice());
    let keep_budget =
        unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::Budget));
    let keep_rate =
        unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::ProjectRate));
    let keep_fees = unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::Fees));
    let client_id = form.client_id.ok_or_else(|| {
        with_field(
            err(BAD_REQUEST, "Client is required"),
            ProjectFormField::Client,
        )
    })?;
    let client = sqlx::query!(
        "SELECT currency, active FROM clients WHERE id = $1 AND org_id = $2 FOR SHARE",
        client_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| with_field(not_found("Client not found"), ProjectFormField::Client))?;
    if !client.active && before.form.client_id != Some(client_id) {
        return Err(with_field(
            err(BAD_REQUEST, "Select an active client"),
            ProjectFormField::Client,
        ));
    }
    let currency = form.currency.as_deref().unwrap_or(&client.currency);
    if !currency
        .trim()
        .eq_ignore_ascii_case(project_currency(&before))
    {
        return Err(with_field(
            conflict(
                "Changing an existing project's currency requires an explicit data migration.",
            ),
            ProjectFormField::Currency,
        ));
    }
    let has_history = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM time_entries WHERE project_id = $1)
          OR EXISTS(SELECT 1 FROM project_fee_occurrences WHERE project_id = $1) as "has_history!""#,
        request.project_id,
    ).fetch_one(&mut *tx).await.map_err(storage_error)?;
    if has_history
        && (form.project_type != before.form.project_type
            || form.client_id != before.form.client_id)
    {
        return Err(conflict(
            "Projects with recorded time or invoice sources must keep their client and billing type.",
        ));
    }
    let parsed = validate_edit(&before, form, email_available)?;
    let reset_task_default = before.configured
        && access.is_some()
        && (form.rate_mode == RateMode::Legacy
            || (form.project_type == ProjectType::TimeAndMaterials
                && form.rate_mode == RateMode::Task))
        && form.tasks.iter().any(|task| {
            task.rate.trim().is_empty()
                && !request
                    .unchanged
                    .contains(&ProtectedProjectField::TaskRate(task.id))
        });
    let changed = form != &before.form || reset_task_default;
    if changed {
        // Keeping row values does not keep a removed row's contribution. Only
        // recompute the active aggregate; unrelated edits must not repair money.
        let removed_task_budget = before.form.tasks.iter().any(|previous| {
            !previous.budget.is_empty()
                && !form.tasks.iter().any(|task| task.source == previous.source)
        });
        let removed_person_budget = before.form.team.iter().any(|previous| {
            !previous.budget.is_empty()
                && !form
                    .team
                    .iter()
                    .any(|person| person.user_id == previous.user_id)
        });
        // Editing hours is not consent to clear an inactive monetary value.
        let keep_inactive_budget_cents = access.is_some()
            && !super::protected::monetary(before.form.budget_mode)
            && !super::protected::monetary(form.budget_mode);
        let keep_budget_cents = (keep_budget
            && !(form.budget_mode == BudgetMode::FeesPerTask && removed_task_budget))
            || keep_inactive_budget_cents;
        let keep_budget_minutes = keep_budget
            && !match form.budget_mode {
                BudgetMode::HoursPerTask => removed_task_budget,
                BudgetMode::HoursPerPerson => removed_person_budget,
                _ => false,
            };
        sqlx::query!(
            "UPDATE projects SET client_id = $3, name = $4, code = $5, starts_on = $6, ends_on = $7,
             project_type = $8, rate_cents = CASE WHEN $14 THEN rate_cents ELSE $9 END,
             budget_kind = CASE WHEN $13 THEN budget_kind ELSE $10 END,
             budget_amount_cents = CASE WHEN $15 THEN budget_amount_cents ELSE $11 END,
             budget_minutes = CASE WHEN $16 THEN budget_minutes ELSE $12 END
             WHERE id = $1 AND org_id = $2
               AND (client_id, name, code, starts_on, ends_on, project_type, rate_cents, budget_kind, budget_amount_cents, budget_minutes)
                 IS DISTINCT FROM ($3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            request.project_id, org_id, client_id, form.name.trim(),
            (!form.code.trim().is_empty()).then_some(form.code.trim()),
            parsed.starts_on as Option<chrono::NaiveDate>, parsed.ends_on as Option<chrono::NaiveDate>,
            form.project_type as ProjectType, parsed.rate_cents, parsed.budget_kind as BudgetKind,
            parsed.budget_cents, parsed.budget_minutes, keep_budget, keep_rate,
            keep_budget_cents, keep_budget_minutes,
        ).execute(&mut *tx).await.map_err(storage_error)?;
        if before.configured {
            save_settings(
                &mut tx,
                org_id,
                request.project_id,
                form,
                &parsed,
                unchanged,
            )
            .await?;
        }
        if may_write_notes && form.admin_notes != before.form.admin_notes {
            sqlx::query!(
                "INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (project_id) DO UPDATE SET admin_notes = EXCLUDED.admin_notes",
                Uuid::now_v7(), org_id, request.project_id, form.admin_notes.trim(),
            ).execute(&mut *tx).await.map_err(storage_error)?;
        }
        save_tags(&mut tx, org_id, request.project_id, &parsed.tags).await?;
        super::associations::save(
            &mut tx,
            org_id,
            &before,
            form,
            may_write_costs,
            may_write_tasks,
            unchanged,
        )
        .await?;
        if !keep_fees {
            save_milestones(&mut tx, org_id, &before, form, &parsed).await?;
        }
    }
    // Run even for an unchanged form: delegation is independent of membership
    // and its access revision must still be checked. Self-removal happens last.
    let managers_changed = if let Some(selection) = managers {
        project_management::execute_in_transaction(
            &mut tx,
            org_id,
            actor_id,
            &ProjectManagersCommand {
                kind: ProjectManagersCommandKind::ReplaceProjectManagers,
                request_id: request.id,
                expected_access_revision: selection.expected_access_revision,
                project_id: request.project_id,
                manager_ids: selection.manager_ids,
            },
        )
        .await
        .map_err(map_error)?
        .changed
    } else {
        false
    };
    let completed_revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM projects WHERE id = $1",
        request.project_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    sqlx::query!(
        "INSERT INTO project_edit_requests (id,org_id,project_id,actor_id,expected_revision,completed_revision,payload)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
        request.id, org_id, request.project_id, actor_id, request.expected_revision, completed_revision, payload,
    ).execute(&mut *tx).await.map_err(storage_error)?;
    let project = project_record(&mut tx, org_id, request.project_id).await?;
    tx.commit().await.map_err(storage_error)?;
    Ok((project, completed_revision != revision || managers_changed))
}

fn validate_edit(
    before: &EditableProject,
    form: &ProjectForm,
    email_available: bool,
) -> Result<ValidatedProject, ServerFnError> {
    let retained_alert = before.form.budget_alert
        && form.budget_alert
        && before.form.budget_alert_at == form.budget_alert_at;
    if before.configured {
        return validate_project_form(
            form,
            project_currency(before),
            email_available || retained_alert,
        );
    }
    if form.project_type != before.form.project_type || form.rate_mode != RateMode::Legacy {
        return Err(with_field(
            conflict(
                "Legacy billing is preserved; changing its type or rate mode requires an explicit migration.",
            ),
            ProjectFormField::ProjectType,
        ));
    }
    if !matches!(
        form.budget_mode,
        BudgetMode::None | BudgetMode::TotalHours | BudgetMode::TotalFees
    ) || form.budget_alert != before.form.budget_alert
        || form.budget_monthly != before.form.budget_monthly
        || form.budget_nonbillable != before.form.budget_nonbillable
        || form.budget_alert_at != before.form.budget_alert_at
        || form.fee_mode != before.form.fee_mode
        || form.fee_amount != before.form.fee_amount
        || form.monthly_day != before.form.monthly_day
        || form.milestones != before.form.milestones
        || form.invoice_defaults != before.form.invoice_defaults
        || form.report_visibility != before.form.report_visibility
    {
        return Err(conflict(
            "These billing and visibility settings require an explicit migration of this legacy project.",
        ));
    }
    // Validate shared fields without converting legacy currency/type/rate semantics.
    // The actual currency and billing type have already been checked unchanged.
    let mut validation_form = form.clone();
    validation_form.project_type = ProjectType::TimeAndMaterials;
    validation_form.rate_mode = RateMode::Person;
    let mut parsed = validate_project_form(&validation_form, "EUR", email_available)?;
    parsed.rate_cents = optional_amount(&form.project_rate, "Project rate")
        .map_err(|error| with_field(error, ProjectFormField::ProjectRate))?;
    Ok(parsed)
}

pub(super) fn project_currency(project: &EditableProject) -> &str {
    project
        .form
        .currency
        .as_deref()
        .unwrap_or(&project.client.currency)
}

async fn project_record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<Project, ServerFnError> {
    sqlx::query_as!(Project,
        r#"SELECT id, org_id, client_id, name, code, currency, active, rate_cents,
           project_type as "project_type: ProjectType", budget_kind as "budget_kind: BudgetKind",
           budget_minutes, budget_amount_cents, starts_on as "starts_on: chrono::NaiveDate",
           ends_on as "ends_on: chrono::NaiveDate", created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM projects WHERE id = $1 AND org_id = $2"#,
        project_id, org_id,
    ).fetch_one(&mut **tx).await.map_err(storage_error)
}

async fn save_settings(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    project_id: Uuid,
    form: &ProjectForm,
    parsed: &ValidatedProject,
    unchanged: Option<&[ProtectedProjectField]>,
) -> Result<(), ServerFnError> {
    let (mode, amount, day) = match &parsed.fee {
        None => (None, None, None),
        Some(FeeSchedule::Single { amount_cents }) => (Some("single"), Some(*amount_cents), None),
        Some(FeeSchedule::Milestones { .. }) => (Some("milestones"), None, None),
        Some(FeeSchedule::Monthly { amount_cents, day }) => (
            Some("monthly"),
            Some(*amount_cents),
            Some(match day {
                MonthlyFeeDay::First => "first",
                MonthlyFeeDay::Fifteenth => "fifteenth",
                MonthlyFeeDay::Last => "last",
            }),
        ),
    };
    let visibility = match form.report_visibility {
        ReportVisibility::Managers => "managers",
        ReportVisibility::ProjectMembers => "project_members",
    };
    let has_budget = form.budget_mode != BudgetMode::None;
    let keep_budget =
        unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::Budget));
    let keep_fees = unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::Fees));
    let keep_invoice =
        unchanged.is_some_and(|fields| fields.contains(&ProtectedProjectField::InvoiceDefaults));
    sqlx::query!(
        "UPDATE project_settings SET rate_mode=$3,
         budget_scope=CASE WHEN $19 THEN budget_scope ELSE $4 END,
         monthly_reset=CASE WHEN $19 THEN monthly_reset ELSE $5 END,
         include_nonbillable=CASE WHEN $19 THEN include_nonbillable ELSE $6 END,
         alert_enabled=CASE WHEN $19 THEN alert_enabled ELSE $7 END,
         alert_threshold=CASE WHEN $19 THEN alert_threshold ELSE $8 END,
         report_visibility=$9,
         fee_mode=CASE WHEN $20 THEN fee_mode ELSE $10 END,
         fee_amount_cents=CASE WHEN $20 THEN fee_amount_cents ELSE $11 END,
         monthly_day=CASE WHEN $20 THEN monthly_day ELSE $12 END,
         terms_days=CASE WHEN $21 THEN terms_days ELSE $13 END,
         po_number=CASE WHEN $21 THEN po_number ELSE $14 END,
         discount_bps=CASE WHEN $21 THEN discount_bps ELSE $15 END,
         tax1_bps=CASE WHEN $21 THEN tax1_bps ELSE $16 END,
         tax2_name=CASE WHEN $21 THEN tax2_name ELSE $17 END,
         tax2_bps=CASE WHEN $21 THEN tax2_bps ELSE $18 END
         WHERE project_id=$1 AND org_id=$2",
        project_id,
        org_id,
        parsed.rate_mode,
        parsed.budget_scope,
        has_budget && form.budget_monthly,
        has_budget && form.budget_nonbillable,
        has_budget && form.budget_alert,
        parsed.alert_threshold,
        visibility,
        mode,
        amount,
        day,
        parsed.terms_days,
        parsed.po_number,
        parsed.discount_bps,
        parsed.tax1_bps,
        parsed.tax2.as_ref().map(|(name, _)| name.as_str()),
        parsed.tax2.as_ref().map(|(_, bps)| *bps),
        keep_budget,
        keep_fees,
        keep_invoice,
    )
    .execute(&mut **tx)
    .await
    .map_err(storage_error)?;
    Ok(())
}

async fn save_tags(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    project_id: Uuid,
    tags: &[String],
) -> Result<(), ServerFnError> {
    let mut names: Vec<_> = tags.iter().collect();
    names.sort_by_cached_key(|name| name.to_lowercase());
    let mut ids = Vec::with_capacity(names.len());
    for name in names {
        let id = sqlx::query_scalar!(
            "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,$3)
             ON CONFLICT (org_id, lower(btrim(name))) DO UPDATE SET name = project_tags.name RETURNING id",
            Uuid::now_v7(), org_id, name,
        ).fetch_one(&mut **tx).await.map_err(storage_error)?;
        ids.push(id);
        sqlx::query!("INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4) ON CONFLICT (project_id,tag_id) DO NOTHING",
            Uuid::now_v7(), org_id, project_id, id,
        ).execute(&mut **tx).await.map_err(storage_error)?;
    }
    sqlx::query!(
        "DELETE FROM project_tag_links WHERE project_id=$1 AND org_id=$2 AND NOT(tag_id=ANY($3))",
        project_id,
        org_id,
        &ids
    )
    .execute(&mut **tx)
    .await
    .map_err(storage_error)?;
    Ok(())
}

async fn save_milestones(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    before: &EditableProject,
    form: &ProjectForm,
    parsed: &ValidatedProject,
) -> Result<(), ServerFnError> {
    let schedules_changed = before.form.fee_mode != form.fee_mode
        || before.form.fee_amount != form.fee_amount
        || before.form.monthly_day != form.monthly_day
        || before.form.project_type != form.project_type;
    if schedules_changed {
        let materialized = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM project_fee_occurrences WHERE project_id=$1 AND org_id=$2)", before.id, org_id)
            .fetch_one(&mut **tx).await.map_err(storage_error)?.unwrap_or(false);
        if materialized {
            return Err(conflict(
                "This fee schedule has invoice sources. Existing charges cannot be reinterpreted.",
            ));
        }
    }
    let milestones = match &parsed.fee {
        Some(FeeSchedule::Milestones { milestones }) => milestones.as_slice(),
        _ => &[],
    };
    let ids: Vec<_> = form
        .milestones
        .iter()
        .take(milestones.len())
        .map(|item| item.id)
        .collect();
    for previous in &before.form.milestones {
        let current = form
            .milestones
            .iter()
            .find(|item| item.id == previous.id)
            .filter(|_| !milestones.is_empty());
        if current != Some(previous) {
            let materialized = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM project_fee_occurrences WHERE milestone_id=$1 AND org_id=$2)", previous.id, org_id)
                .fetch_one(&mut **tx).await.map_err(storage_error)?.unwrap_or(false);
            if materialized {
                return Err(conflict(
                    "A milestone with an invoice source cannot be changed or removed.",
                ));
            }
        }
    }
    sqlx::query!(
        "DELETE FROM project_fee_milestones WHERE project_id=$1 AND org_id=$2 AND NOT(id=ANY($3))",
        before.id,
        org_id,
        &ids
    )
    .execute(&mut **tx)
    .await
    .map_err(storage_error)?;
    for (position, (input, parsed)) in form.milestones.iter().zip(milestones).enumerate() {
        let position =
            i16::try_from(position).map_err(|_| err(BAD_REQUEST, "Too many milestones"))?;
        if before
            .form
            .milestones
            .iter()
            .any(|item| item.id == input.id)
        {
            sqlx::query!("UPDATE project_fee_milestones SET name=$4, due_on=$5, amount_cents=$6, position=$7 WHERE id=$1 AND project_id=$2 AND org_id=$3",
                input.id, before.id, org_id, parsed.name, parsed.due_on as chrono::NaiveDate, parsed.amount_cents, position,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        } else {
            if input.id.get_version_num() != 7 {
                return Err(err(BAD_REQUEST, "Invalid milestone identity"));
            }
            sqlx::query!("INSERT INTO project_fee_milestones (id,org_id,project_id,name,due_on,amount_cents,position) VALUES ($1,$2,$3,$4,$5,$6,$7)",
                input.id, org_id, before.id, parsed.name, parsed.due_on as chrono::NaiveDate, parsed.amount_cents, position,
            ).execute(&mut **tx).await.map_err(storage_error)?;
        }
    }
    Ok(())
}
