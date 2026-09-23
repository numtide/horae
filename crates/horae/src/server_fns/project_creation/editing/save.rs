use super::super::validation::{ValidatedProject, optional_amount, with_field};
use super::*;
use crate::models::project_creation::ProjectFormField;
use horae_core::project::FeeSchedule;

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
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
        .execute(&mut *tx)
        .await
        .map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let payload = validate_draft_form(&request.form, role == OrgRole::Admin)?;
    let revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM projects WHERE id = $1 AND org_id = $2 FOR UPDATE",
        request.project_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| not_found("Project not found"))?;
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
        if completed.expected_revision != request.expected_revision
            || completed.payload != payload
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
    let before = load_project_form(&mut tx, org_id, request.project_id, role).await?;
    let form = &request.form;
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
    let changed = form != &before.form;
    if changed {
        sqlx::query!(
            "UPDATE projects SET client_id = $3, name = $4, code = $5, starts_on = $6, ends_on = $7,
             project_type = $8, rate_cents = $9, budget_kind = $10, budget_amount_cents = $11, budget_minutes = $12
             WHERE id = $1 AND org_id = $2
               AND (client_id, name, code, starts_on, ends_on, project_type, rate_cents, budget_kind, budget_amount_cents, budget_minutes)
                 IS DISTINCT FROM ($3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            request.project_id, org_id, client_id, form.name.trim(),
            (!form.code.trim().is_empty()).then_some(form.code.trim()),
            parsed.starts_on as Option<chrono::NaiveDate>, parsed.ends_on as Option<chrono::NaiveDate>,
            form.project_type as ProjectType, parsed.rate_cents, parsed.budget_kind as BudgetKind,
            parsed.budget_cents, parsed.budget_minutes,
        ).execute(&mut *tx).await.map_err(storage_error)?;
        if before.configured {
            save_settings(&mut tx, org_id, request.project_id, form, &parsed).await?;
        }
        if role == OrgRole::Admin && form.admin_notes != before.form.admin_notes {
            sqlx::query!(
                "INSERT INTO project_private_settings (id,org_id,project_id,admin_notes) VALUES ($1,$2,$3,$4)
                 ON CONFLICT (project_id) DO UPDATE SET admin_notes = EXCLUDED.admin_notes",
                Uuid::now_v7(), org_id, request.project_id, form.admin_notes.trim(),
            ).execute(&mut *tx).await.map_err(storage_error)?;
        }
        save_tags(&mut tx, org_id, request.project_id, &parsed.tags).await?;
        super::associations::save(&mut tx, org_id, &before, form, role).await?;
        save_milestones(&mut tx, org_id, &before, form, &parsed).await?;
    }
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
    Ok((project, completed_revision != revision))
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
    sqlx::query!(
        "UPDATE project_settings SET rate_mode=$3, budget_scope=$4, monthly_reset=$5, include_nonbillable=$6,
         alert_enabled=$7, alert_threshold=$8, report_visibility=$9, fee_mode=$10, fee_amount_cents=$11,
         monthly_day=$12, terms_days=$13, po_number=$14, discount_bps=$15, tax1_bps=$16, tax2_name=$17, tax2_bps=$18
         WHERE project_id=$1 AND org_id=$2",
        project_id, org_id, parsed.rate_mode, parsed.budget_scope, has_budget && form.budget_monthly,
        has_budget && form.budget_nonbillable, has_budget && form.budget_alert, parsed.alert_threshold, visibility,
        mode, amount, day, parsed.terms_days, parsed.po_number, parsed.discount_bps, parsed.tax1_bps,
        parsed.tax2.as_ref().map(|(name,_)| name.as_str()), parsed.tax2.as_ref().map(|(_,bps)| *bps),
    ).execute(&mut **tx).await.map_err(storage_error)?;
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
