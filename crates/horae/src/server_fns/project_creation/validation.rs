use super::*;
use crate::models::project_creation::{FeeMode, ProjectFormField, TaskSource};
use chrono::NaiveDate;
use horae_core::project::{
    BudgetMode, FeeMilestone, FeeSchedule, Percentage, ProjectValidationError, RateMode,
};
use std::collections::HashSet;

/// Parsed values for the active controls; hidden values remain only in the draft.
pub(super) struct ValidatedProject {
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub rate_mode: &'static str,
    pub rate_cents: Option<i64>,
    pub budget_kind: BudgetKind,
    pub budget_scope: &'static str,
    pub budget_cents: Option<i64>,
    pub budget_minutes: Option<i64>,
    pub alert_threshold: i16,
    pub fee: Option<FeeSchedule>,
    pub terms_days: i16,
    pub po_number: String,
    pub discount_bps: i16,
    pub tax1_bps: i16,
    pub tax2: Option<(String, i16)>,
    pub tags: Vec<String>,
}

pub(super) fn optional_amount(value: &str, field: &str) -> Result<Option<i64>, ServerFnError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let amount = horae_core::money::parse_cents(value.trim()).map_err(|_| {
        err(
            BAD_REQUEST,
            format!("{field}: enter an amount with at most two decimal places"),
        )
    })?;
    if amount < 0 {
        return Err(err(BAD_REQUEST, format!("{field}: cannot be negative")));
    }
    Ok(Some(amount))
}

pub(super) fn optional_hours(value: &str) -> Result<Option<i64>, ServerFnError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    horae_core::duration::parse(value.trim())
        .map(|minutes| Some(i64::from(minutes)))
        .map_err(|_| err(BAD_REQUEST, "Budget: enter valid hours, e.g. 120 or 7:30"))
}

fn date(value: &str, field: &str) -> Result<Option<NaiveDate>, ServerFnError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    // Keep the browser's ISO format; chrono also accepts some unpadded forms.
    let date = value
        .parse::<NaiveDate>()
        .map_err(|_| err(BAD_REQUEST, format!("{field}: use YYYY-MM-DD")))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(err(BAD_REQUEST, format!("{field}: use YYYY-MM-DD")));
    }
    Ok(Some(date))
}

fn percentage(value: &str, field: &str) -> Result<i16, ServerFnError> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    let parsed: Percentage = value
        .parse()
        .map_err(|error| err(BAD_REQUEST, format!("{field}: {error}")))?;
    i16::try_from(parsed.basis_points()).map_err(|_| err(BAD_REQUEST, "Invalid percentage"))
}

fn bounded_text(
    value: &str,
    limit: usize,
    required: bool,
    field: &str,
) -> Result<(), ServerFnError> {
    if value.contains('\0')
        || value.trim().chars().count() > limit
        || (required && value.trim().is_empty())
    {
        return Err(err(
            BAD_REQUEST,
            format!(
                "{field}: enter {}–{limit} characters",
                usize::from(required)
            ),
        ));
    }
    Ok(())
}

pub(super) fn with_field(mut error: ServerFnError, field: ProjectFormField) -> ServerFnError {
    if let ServerFnError::ServerError {
        code: BAD_REQUEST | NOT_FOUND | CONFLICT,
        details,
        ..
    } = &mut error
    {
        *details = Some(serde_json::json!({ "field": field }));
    }
    error
}

fn sum_budgets<'a>(
    values: impl Iterator<Item = (ProjectFormField, &'a str)>,
    money: bool,
) -> Result<Option<i64>, ServerFnError> {
    let mut total = None;
    for (field, value) in values {
        let amount = if money {
            optional_amount(value, "Budget")
        } else {
            optional_hours(value)
        }
        .map_err(|error| with_field(error, field))?;
        if let Some(amount) = amount {
            total =
                Some(total.unwrap_or(0_i64).checked_add(amount).ok_or_else(|| {
                    with_field(err(BAD_REQUEST, "Budget total is too large"), field)
                })?);
        }
    }
    Ok(total)
}

pub(super) fn validate_project_form(
    form: &ProjectForm,
    currency: &str,
    email_available: bool,
) -> Result<ValidatedProject, ServerFnError> {
    let starts_on = date(&form.starts_on, "Start date")
        .map_err(|error| with_field(error, ProjectFormField::StartsOn))?;
    let ends_on = date(&form.ends_on, "End date")
        .map_err(|error| with_field(error, ProjectFormField::EndsOn))?;
    horae_core::project::validate_basics(
        &form.name,
        Some(&form.code),
        currency,
        starts_on,
        ends_on,
    )
    .map_err(|error| {
        let field = match error {
            ProjectValidationError::Name => ProjectFormField::Name,
            ProjectValidationError::Code => ProjectFormField::Code,
            ProjectValidationError::Currency => ProjectFormField::Currency,
            ProjectValidationError::DateOrder => ProjectFormField::EndsOn,
            _ => return err(BAD_REQUEST, error),
        };
        with_field(err(BAD_REQUEST, error), field)
    })?;
    form.budget_mode
        .validate_for(form.project_type)
        .map_err(|error| {
            let field = if error == ProjectValidationError::ProjectType {
                ProjectFormField::ProjectType
            } else {
                ProjectFormField::BudgetMode
            };
            with_field(err(BAD_REQUEST, error), field)
        })?;
    bounded_text(&form.admin_notes, 10000, false, "Private notes")
        .map_err(|error| with_field(error, ProjectFormField::AdminNotes))?;

    let is_hourly = form.project_type == ProjectType::TimeAndMaterials;
    let rate_mode = if !is_hourly {
        "person"
    } else {
        match form.rate_mode {
            RateMode::Person => "person",
            RateMode::Task => "task",
            RateMode::Project => "project",
            RateMode::Legacy => {
                return Err(with_field(
                    err(BAD_REQUEST, "Select person, task or project rates"),
                    ProjectFormField::RateMode,
                ));
            }
        }
    };
    let rate_cents = if is_hourly && form.rate_mode == RateMode::Project {
        Some(
            optional_amount(&form.project_rate, "Project rate")
                .and_then(|value| value.ok_or_else(|| err(BAD_REQUEST, "Project rate is required")))
                .map_err(|error| with_field(error, ProjectFormField::ProjectRate))?,
        )
    } else {
        None
    };

    let (budget_kind, budget_scope, budget_cents, budget_minutes) = match form.budget_mode {
        BudgetMode::None => (BudgetKind::None, "project", None, None),
        BudgetMode::TotalHours => (
            BudgetKind::Hours,
            "project",
            None,
            optional_hours(&form.budget_value)
                .map_err(|error| with_field(error, ProjectFormField::BudgetValue))?,
        ),
        BudgetMode::TotalFees => (
            BudgetKind::Amount,
            "project",
            optional_amount(&form.budget_value, "Budget")
                .map_err(|error| with_field(error, ProjectFormField::BudgetValue))?,
            None,
        ),
        BudgetMode::HoursPerTask => (
            BudgetKind::Hours,
            "task",
            None,
            sum_budgets(
                form.tasks
                    .iter()
                    .map(|task| (ProjectFormField::TaskBudget(task.id), task.budget.as_str())),
                false,
            )?,
        ),
        BudgetMode::FeesPerTask => (
            BudgetKind::Amount,
            "task",
            sum_budgets(
                form.tasks
                    .iter()
                    .map(|task| (ProjectFormField::TaskBudget(task.id), task.budget.as_str())),
                true,
            )?,
            None,
        ),
        BudgetMode::HoursPerPerson => (
            BudgetKind::Hours,
            "person",
            None,
            sum_budgets(
                form.team.iter().map(|member| {
                    (
                        ProjectFormField::PersonBudget(member.user_id),
                        member.budget.as_str(),
                    )
                }),
                false,
            )?,
        ),
    };
    let alert_threshold = if form.budget_alert && form.budget_mode != BudgetMode::None {
        if !email_available {
            return Err(with_field(
                err(BAD_REQUEST, "Budget email delivery is not configured"),
                ProjectFormField::BudgetAlert,
            ));
        }
        let threshold = form.budget_alert_at.trim().parse::<i16>().map_err(|_| {
            with_field(
                err(
                    BAD_REQUEST,
                    "Budget alert: enter a whole percentage from 0 to 100",
                ),
                ProjectFormField::BudgetAlertAt,
            )
        })?;
        if !(0..=100).contains(&threshold) {
            return Err(with_field(
                err(
                    BAD_REQUEST,
                    "Budget alert: enter a whole percentage from 0 to 100",
                ),
                ProjectFormField::BudgetAlertAt,
            ));
        }
        threshold
    } else {
        80
    };

    let fee = if form.project_type == ProjectType::FixedFee {
        let fee = match form.fee_mode {
            FeeMode::Single | FeeMode::Monthly => {
                let amount_cents = optional_amount(&form.fee_amount, "Fixed fee")
                    .and_then(|value| {
                        value.ok_or_else(|| err(BAD_REQUEST, "Fixed fee amount is required"))
                    })
                    .map_err(|error| with_field(error, ProjectFormField::FeeAmount))?;
                if form.fee_mode == FeeMode::Single {
                    FeeSchedule::Single { amount_cents }
                } else {
                    FeeSchedule::Monthly {
                        amount_cents,
                        day: form.monthly_day,
                    }
                }
            }
            FeeMode::Milestones => {
                if form.milestones.is_empty() {
                    return Err(with_field(
                        err(BAD_REQUEST, ProjectValidationError::Milestones),
                        ProjectFormField::Milestones,
                    ));
                }
                let mut ids = HashSet::new();
                let mut milestones = Vec::with_capacity(form.milestones.len());
                let mut total = 0_i64;
                for item in &form.milestones {
                    if !ids.insert(item.id) {
                        return Err(err(BAD_REQUEST, "Duplicate milestone"));
                    }
                    bounded_text(&item.name, 200, true, "Milestone name").map_err(|error| {
                        with_field(error, ProjectFormField::MilestoneName(item.id))
                    })?;
                    let due_on = date(&item.due_on, "Milestone date")
                        .and_then(|value| {
                            value.ok_or_else(|| err(BAD_REQUEST, "Milestone date is required"))
                        })
                        .map_err(|error| {
                            with_field(error, ProjectFormField::MilestoneDate(item.id))
                        })?;
                    let amount_cents = optional_amount(&item.amount, "Milestone fee")
                        .and_then(|value| {
                            value.ok_or_else(|| err(BAD_REQUEST, "Milestone amount is required"))
                        })
                        .map_err(|error| {
                            with_field(error, ProjectFormField::MilestoneAmount(item.id))
                        })?;
                    total = total.checked_add(amount_cents).ok_or_else(|| {
                        with_field(
                            err(BAD_REQUEST, "Milestone total is too large"),
                            ProjectFormField::MilestoneAmount(item.id),
                        )
                    })?;
                    milestones.push(FeeMilestone {
                        name: item.name.trim().into(),
                        due_on,
                        amount_cents,
                    });
                }
                FeeSchedule::Milestones { milestones }
            }
        };
        fee.validate().map_err(|error| err(BAD_REQUEST, error))?;
        Some(fee)
    } else {
        None
    };

    let neutral_defaults = crate::models::project_creation::InvoiceDefaultsInput::default();
    let defaults = if form.project_type == ProjectType::NonBillable {
        &neutral_defaults
    } else {
        &form.invoice_defaults
    };
    let terms_days = defaults.terms_days.trim().parse::<i16>().map_err(|_| {
        with_field(
            err(BAD_REQUEST, "Payment terms: enter 0–365 days"),
            ProjectFormField::PaymentTerms,
        )
    })?;
    if !(0..=365).contains(&terms_days) {
        return Err(with_field(
            err(BAD_REQUEST, "Payment terms: enter 0–365 days"),
            ProjectFormField::PaymentTerms,
        ));
    }
    bounded_text(&defaults.po_number, 200, false, "PO number")
        .map_err(|error| with_field(error, ProjectFormField::PurchaseOrder))?;
    let discount_bps = percentage(&defaults.discount, "Discount")
        .map_err(|error| with_field(error, ProjectFormField::Discount))?;
    let tax1_bps = percentage(&defaults.tax, "Tax")
        .map_err(|error| with_field(error, ProjectFormField::Tax))?;
    let tax2 = defaults
        .second_tax
        .as_ref()
        .map(|tax| {
            bounded_text(&tax.name, 100, true, "Second tax name")
                .map_err(|error| with_field(error, ProjectFormField::SecondTaxName))?;
            Ok::<_, ServerFnError>((
                tax.name.trim().into(),
                percentage(&tax.percentage, "Second tax")
                    .map_err(|error| with_field(error, ProjectFormField::SecondTax))?,
            ))
        })
        .transpose()?;

    let mut tags = Vec::new();
    let mut tag_names = HashSet::new();
    for tag in &form.tags {
        bounded_text(tag, 50, true, "Tag")?;
        if tag_names.insert(tag.trim().to_lowercase()) {
            tags.push(tag.trim().to_owned());
        }
    }
    let mut people = HashSet::new();
    for member in &form.team {
        if !people.insert(member.user_id) {
            return Err(err(BAD_REQUEST, "A person can only be assigned once"));
        }
        optional_amount(&member.cost_rate, "Cost rate")
            .map_err(|error| with_field(error, ProjectFormField::CostRate(member.user_id)))?;
        if is_hourly && form.rate_mode == RateMode::Person {
            optional_amount(&member.billable_rate, "Person rate")
                .map_err(|error| with_field(error, ProjectFormField::PersonRate(member.user_id)))?;
        }
    }
    let mut task_rows = HashSet::new();
    let mut catalog_tasks = HashSet::new();
    let mut new_task_names = HashSet::new();
    for task in &form.tasks {
        if !task_rows.insert(task.id) {
            return Err(err(BAD_REQUEST, "Duplicate task row"));
        }
        match &task.source {
            TaskSource::Existing { task_id } if !catalog_tasks.insert(*task_id) => {
                return Err(err(BAD_REQUEST, "A task can only be selected once"));
            }
            TaskSource::New { name } => {
                bounded_text(name, 200, true, "Task name")
                    .map_err(|error| with_field(error, ProjectFormField::TaskName(task.id)))?;
                if !new_task_names.insert(name.trim().to_lowercase()) {
                    return Err(with_field(
                        err(BAD_REQUEST, "Duplicate new task name"),
                        ProjectFormField::TaskName(task.id),
                    ));
                }
            }
            _ => {}
        }
        if let TaskAccess::Restricted { user_ids } = &task.access
            && user_ids.iter().any(|id| !people.contains(id))
        {
            return Err(with_field(
                err(BAD_REQUEST, "Task access must refer to selected teammates"),
                ProjectFormField::TaskAccess(task.id),
            ));
        }
        if is_hourly && form.rate_mode == RateMode::Task {
            optional_amount(&task.rate, "Task rate")
                .map_err(|error| with_field(error, ProjectFormField::TaskRate(task.id)))?;
        }
    }
    Ok(ValidatedProject {
        starts_on,
        ends_on,
        rate_mode,
        rate_cents,
        budget_kind,
        budget_scope,
        budget_cents,
        budget_minutes,
        alert_threshold,
        fee,
        terms_days,
        po_number: defaults.po_number.trim().to_owned(),
        discount_bps,
        tax1_bps,
        tax2,
        tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project_creation::{InvoiceDefaultsInput, SecondTaxInput};
    use uuid::Uuid;

    #[test]
    fn task_name_and_access_rejections_identify_the_invalid_row() {
        use crate::models::project_creation::ProjectTaskInput;
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        for case in ["empty", "long", "duplicate", "access"] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                tasks: [(first, "First"), (second, "Second")]
                    .map(|(id, name)| ProjectTaskInput {
                        id,
                        source: TaskSource::New { name: name.into() },
                        billable: true,
                        rate: String::new(),
                        budget: String::new(),
                        access: TaskAccess::Everyone,
                    })
                    .to_vec(),
                ..Default::default()
            };
            if case == "access" {
                form.tasks[1].access = TaskAccess::Restricted {
                    user_ids: vec![Uuid::now_v7()],
                };
            } else {
                form.tasks[1].source = TaskSource::New {
                    name: match case {
                        "empty" => "  ".into(),
                        "long" => "é".repeat(201),
                        "duplicate" => " FIRST ".into(),
                        _ => unreachable!(),
                    },
                };
            }
            let field = if case == "access" {
                "task_access"
            } else {
                "task_name"
            };
            assert_rejection_field(&form, false, serde_json::json!({ field: second }));
        }
    }

    #[test]
    fn assignment_rejections_identify_the_invalid_row() {
        use crate::models::project_creation::{ProjectMemberInput, ProjectTaskInput};
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        for (field, value) in [
            ("task_rate", "-1"),
            ("task_rate", "1.001"),
            ("person_rate", "-1"),
            ("person_rate", "1.001"),
            ("cost_rate", "-1"),
            ("cost_rate", "1.001"),
            ("task_budget", "unfinished"),
            ("person_budget", "unfinished"),
            ("task_fees", "-1"),
            ("task_fees", "1.001"),
            ("task_fees", "92233720368547758.07"),
        ] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                tasks: [first, second]
                    .map(|id| ProjectTaskInput {
                        id,
                        source: TaskSource::Existing {
                            task_id: Uuid::now_v7(),
                        },
                        billable: true,
                        rate: "1".into(),
                        budget: "1".into(),
                        access: TaskAccess::Everyone,
                    })
                    .to_vec(),
                team: [first, second]
                    .map(|user_id| ProjectMemberInput {
                        user_id,
                        manager: false,
                        billable_rate: "1".into(),
                        cost_rate: "1".into(),
                        budget: "1".into(),
                    })
                    .to_vec(),
                ..Default::default()
            };
            match field {
                "task_rate" => {
                    form.rate_mode = RateMode::Task;
                    form.tasks[1].rate = value.into();
                }
                "person_rate" => form.team[1].billable_rate = value.into(),
                "cost_rate" => form.team[1].cost_rate = value.into(),
                "task_budget" | "task_fees" => {
                    form.budget_mode = if field == "task_fees" {
                        BudgetMode::FeesPerTask
                    } else {
                        BudgetMode::HoursPerTask
                    };
                    form.tasks[1].budget = value.into();
                }
                "person_budget" => {
                    form.budget_mode = BudgetMode::HoursPerPerson;
                    form.team[1].budget = value.into();
                }
                _ => unreachable!(),
            }
            let expected = if field == "task_fees" {
                "task_budget"
            } else {
                field
            };
            assert_rejection_field(&form, false, serde_json::json!({ expected: second }));
        }
    }

    #[test]
    fn billing_rejections_identify_the_active_control() {
        for (case, field, value, email) in [
            ("rate", "project_rate", "", false),
            ("rate", "project_rate", "-1", false),
            ("rate", "project_rate", "1.001", false),
            ("hours", "budget_value", "unfinished", false),
            ("fees", "budget_value", "-1", false),
            ("single", "fee_amount", "", false),
            ("monthly", "fee_amount", "1.001", false),
            ("alert", "budget_alert_at", "101", true),
            ("alert", "budget_alert_at", "0.5", true),
            ("alert", "budget_alert", "80", false),
            ("budget", "budget_mode", "", false),
            ("legacy", "rate_mode", "", false),
        ] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                ..Default::default()
            };
            match case {
                "rate" => {
                    form.rate_mode = RateMode::Project;
                    form.project_rate = value.into();
                }
                "hours" | "fees" => {
                    form.budget_mode = if case == "hours" {
                        BudgetMode::TotalHours
                    } else {
                        BudgetMode::TotalFees
                    };
                    form.budget_value = value.into();
                }
                "single" | "monthly" => {
                    form.project_type = ProjectType::FixedFee;
                    form.fee_mode = if case == "single" {
                        FeeMode::Single
                    } else {
                        FeeMode::Monthly
                    };
                    form.fee_amount = value.into();
                }
                "alert" => {
                    form.budget_mode = BudgetMode::TotalHours;
                    form.budget_alert = true;
                    form.budget_alert_at = value.into();
                }
                "budget" => {
                    form.project_type = ProjectType::NonBillable;
                    form.budget_mode = BudgetMode::TotalFees;
                }
                "legacy" => form.rate_mode = RateMode::Legacy,
                _ => unreachable!(),
            }
            assert_rejection_field(&form, email, serde_json::json!(field));
        }
    }

    #[test]
    fn milestone_rejections_target_the_invalid_row_by_identity() {
        use crate::models::project_creation::MilestoneInput;
        let first_id = Uuid::now_v7();
        let second_id = Uuid::now_v7();
        for (field, value) in [
            ("milestone_name", ""),
            ("milestone_date", ""),
            ("milestone_date", "2026-9-1"),
            ("milestone_amount", ""),
            ("milestone_amount", "-1"),
            ("milestone_amount", "1.001"),
            ("milestone_amount", "92233720368547758.07"),
        ] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                project_type: ProjectType::FixedFee,
                fee_mode: FeeMode::Milestones,
                milestones: [first_id, second_id]
                    .map(|id| MilestoneInput {
                        id,
                        name: "Delivery".into(),
                        due_on: "2026-09-01".into(),
                        amount: "1.00".into(),
                    })
                    .to_vec(),
                ..Default::default()
            };
            let row = &mut form.milestones[1];
            match field {
                "milestone_name" => row.name = value.into(),
                "milestone_date" => row.due_on = value.into(),
                "milestone_amount" => row.amount = value.into(),
                _ => unreachable!(),
            }
            assert_rejection_field(&form, false, serde_json::json!({ field: second_id }));
        }
        let empty = ProjectForm {
            name: "Valid project".into(),
            project_type: ProjectType::FixedFee,
            fee_mode: FeeMode::Milestones,
            ..Default::default()
        };
        assert_rejection_field(&empty, false, serde_json::json!("milestones"));
    }

    fn assert_rejection_field(form: &ProjectForm, email: bool, expected: serde_json::Value) {
        let Err(ServerFnError::ServerError { code, details, .. }) =
            validate_project_form(form, "EUR", email)
        else {
            panic!("Expected a field rejection for {expected}");
        };
        assert_eq!(code, BAD_REQUEST);
        assert_eq!(details, Some(serde_json::json!({ "field": expected })));
    }

    #[test]
    fn inactive_billing_inputs_remain_in_the_draft_without_blocking_creation() {
        for project_type in [ProjectType::NonBillable, ProjectType::TimeAndMaterials] {
            let form = ProjectForm {
                name: "Valid project".into(),
                project_type,
                project_rate: "unfinished".into(),
                fee_amount: "unfinished".into(),
                budget_value: "unfinished".into(),
                budget_alert: true,
                budget_alert_at: "unfinished".into(),
                ..Default::default()
            };
            let validated = validate_project_form(&form, "EUR", false).unwrap();
            assert!(validated.rate_cents.is_none());
            assert!(validated.budget_cents.is_none());
            assert!(validated.budget_minutes.is_none());
            assert!(validated.fee.is_none());
            assert_eq!(form.project_rate, "unfinished");
            assert_eq!(form.fee_amount, "unfinished");
        }
    }

    #[test]
    fn basic_rejections_identify_the_field_without_echoing_input() {
        for (field, value) in [
            ("name", "private".repeat(29)),
            ("name", String::new()),
            ("code", "private".repeat(15)),
            ("starts_on", "2026-9-1".into()),
            ("ends_on", "not a date".into()),
            ("ends_on", "2026-08-31".into()),
            ("currency", "JPY".into()),
            ("admin_notes", "private".repeat(1429)),
        ] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                starts_on: "2026-09-01".into(),
                ..Default::default()
            };
            let mut currency = "EUR".to_owned();
            match field {
                "name" => form.name = value,
                "code" => form.code = value,
                "starts_on" => form.starts_on = value,
                "ends_on" => form.ends_on = value,
                "currency" => currency = value,
                "admin_notes" => form.admin_notes = value,
                _ => unreachable!(),
            }
            let Err(ServerFnError::ServerError {
                code,
                message,
                details,
            }) = validate_project_form(&form, &currency, false)
            else {
                panic!("Expected a field rejection for {field}");
            };
            assert_eq!(code, BAD_REQUEST);
            assert_eq!(details, Some(serde_json::json!({ "field": field })));
            assert!(!message.contains("private"));
        }
    }

    #[test]
    fn invoice_rejections_identify_the_field_without_echoing_input() {
        for (field, value) in [
            ("payment_terms", "366".to_owned()),
            ("purchase_order", "private".repeat(30)),
            ("discount", "101".to_owned()),
            ("tax", "1.001".to_owned()),
            ("second_tax_name", String::new()),
            ("second_tax", "-1".to_owned()),
        ] {
            let mut form = ProjectForm {
                name: "Valid project".into(),
                ..Default::default()
            };
            let defaults = &mut form.invoice_defaults;
            defaults.second_tax = Some(SecondTaxInput {
                name: "Local".into(),
                percentage: "1.5".into(),
            });
            match field {
                "payment_terms" => defaults.terms_days = value,
                "purchase_order" => defaults.po_number = value,
                "discount" => defaults.discount = value,
                "tax" => defaults.tax = value,
                "second_tax_name" => defaults.second_tax.as_mut().unwrap().name = value,
                "second_tax" => defaults.second_tax.as_mut().unwrap().percentage = value,
                _ => unreachable!(),
            }
            let Err(ServerFnError::ServerError {
                code,
                message,
                details,
            }) = validate_project_form(&form, "EUR", false)
            else {
                panic!("Expected a field rejection for {field}");
            };
            assert_eq!(code, BAD_REQUEST);
            assert_eq!(details, Some(serde_json::json!({ "field": field })));
            assert!(!message.contains("private"));
        }
    }

    #[test]
    fn non_billable_projects_ignore_hidden_invoice_inputs() {
        let form = ProjectForm {
            name: "Internal work".into(),
            project_type: ProjectType::NonBillable,
            invoice_defaults: InvoiceDefaultsInput {
                terms_days: "unfinished".into(),
                po_number: "x".repeat(201),
                tax: "-21".into(),
                discount: "101".into(),
                second_tax: Some(SecondTaxInput {
                    name: String::new(),
                    percentage: "pending".into(),
                }),
            },
            ..Default::default()
        };
        let validated = validate_project_form(&form, "EUR", false).unwrap();
        assert_eq!(validated.terms_days, 30);
        assert!(validated.po_number.is_empty());
        assert_eq!(validated.tax1_bps, 0);
        assert_eq!(validated.discount_bps, 0);
        assert!(validated.tax2.is_none());
        assert_eq!(form.invoice_defaults.terms_days, "unfinished");

        let billable = ProjectForm {
            project_type: ProjectType::TimeAndMaterials,
            ..form
        };
        assert!(validate_project_form(&billable, "EUR", false).is_err());
    }
}
