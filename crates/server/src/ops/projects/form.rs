//! The project form's validation, shared by creating a project and editing
//! one: field formats, limits, and the values a project row stores.

use crate::models::project_creation::{
    FeeMode, ProjectForm, ProjectFormField, TaskAccess, TaskSource,
};
use crate::ops::OpError;
use chrono::NaiveDate;
use horae_core::project::{
    BudgetMode, FeeMilestone, FeeSchedule, Percentage, ProjectValidationError, RateMode,
};
use horae_core::types::{BudgetKind, ProjectType};
use std::collections::HashSet;

/// Parsed values for the active controls; hidden values remain only in the draft.
pub struct ValidatedProject {
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

pub(crate) fn optional_amount(value: &str, field: &str) -> Result<Option<i64>, OpError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let amount = horae_core::money::parse_cents(value.trim()).map_err(|_| {
        OpError::bad_input(format!(
            "{field}: enter an amount with at most two decimal places"
        ))
    })?;
    if amount < 0 {
        return Err(OpError::bad_input(format!("{field}: cannot be negative")));
    }
    Ok(Some(amount))
}

/// Refuses a money budget that is not a whole number of `currency`'s ISO 4217
/// minor unit (JPY 10.50). A budget is an amount, served exactly in minor
/// units by `/api/v1`, and the database checks the same step.
pub(crate) fn budget_step(amount: Option<i64>, currency: &str) -> Result<Option<i64>, OpError> {
    use horae_core::money::{MinorUnitsError, to_iso_minor};
    match amount.map(|cents| to_iso_minor(cents, currency)) {
        Some(Err(MinorUnitsError::Inexact)) => Err(OpError::bad_input(format!(
            "Budget: {} amounts have no decimal places",
            currency.trim()
        ))),
        Some(Err(MinorUnitsError::Overflow)) => Err(OpError::bad_input("Budget is too large")),
        _ => Ok(amount),
    }
}

pub(crate) fn optional_hours(value: &str) -> Result<Option<i64>, OpError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    horae_core::duration::parse(value.trim())
        .map(|minutes| Some(i64::from(minutes)))
        .map_err(|_| OpError::bad_input("Budget: enter valid hours, e.g. 120 or 7:30"))
}

fn date(value: &str, field: &str) -> Result<Option<NaiveDate>, OpError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    // Keep the browser's ISO format; chrono also accepts some unpadded forms.
    let date = value
        .parse::<NaiveDate>()
        .map_err(|_| OpError::bad_input(format!("{field}: use YYYY-MM-DD")))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(OpError::bad_input(format!("{field}: use YYYY-MM-DD")));
    }
    Ok(Some(date))
}

fn percentage(value: &str, field: &str) -> Result<i16, OpError> {
    if value.trim().is_empty() {
        return Ok(0);
    }
    let parsed: Percentage = value
        .parse()
        .map_err(|error| OpError::bad_input(format!("{field}: {error}")))?;
    i16::try_from(parsed.basis_points()).map_err(|_| OpError::bad_input("Invalid percentage"))
}

fn bounded_text(value: &str, limit: usize, required: bool, field: &str) -> Result<(), OpError> {
    if value.contains('\0')
        || value.trim().chars().count() > limit
        || (required && value.trim().is_empty())
    {
        return Err(OpError::bad_input(format!(
            "{field}: enter {}–{limit} characters",
            usize::from(required)
        )));
    }
    Ok(())
}

/// The form as stored JSON, within the size and content limits shared by
/// drafts and edits.
pub(crate) fn form_payload(form: &ProjectForm) -> Result<serde_json::Value, OpError> {
    if form.tasks.len() > 500
        || form.team.len() > 500
        || form.tags.len() > 50
        || form.milestones.len() > 100
        || form.tasks.iter().any(|task| matches!(&task.access, TaskAccess::Restricted { user_ids } if user_ids.len() > 500))
    {
        return Err(OpError::bad_input("Too many project tasks, people, tags or milestones"));
    }
    let payload = serde_json::to_value(form).map_err(internal)?;
    fn contains_nul(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(text) => text.contains('\0'),
            serde_json::Value::Array(values) => values.iter().any(contains_nul),
            serde_json::Value::Object(values) => values.values().any(contains_nul),
            _ => false,
        }
    }
    if contains_nul(&payload) {
        return Err(OpError::bad_input(
            "Project fields cannot contain a null character",
        ));
    }
    // The indented representation is a conservative bound for PostgreSQL's
    // jsonb text size, which also includes spaces after keys and separators.
    if serde_json::to_string_pretty(&payload)
        .map_err(internal)?
        .len()
        > 256 * 1024
    {
        return Err(OpError::bad_input("Project draft exceeds 256 KiB"));
    }
    Ok(payload)
}

/// A fault the project form answers with its own text, as it always has.
pub(crate) fn internal(error: impl std::fmt::Display) -> OpError {
    OpError::Internal(error.to_string())
}

/// A storage failure while loading, saving or creating a project: a lost
/// race (serialization failure or deadlock) is a conflict to reload and retry.
pub(crate) fn storage(error: sqlx::Error) -> OpError {
    if error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| matches!(code.as_ref(), "40001" | "40P01"))
    {
        return OpError::conflict("The project changed while saving. Reload before retrying.");
    }
    tracing::error!(code = ?error.as_database_error().and_then(|error| error.code()), "Project creation database operation failed");
    OpError::Internal("Could not save or load the project. Please retry.".into())
}

/// Names the form field to blame (see [`OpError::with_field`]).
pub(crate) fn with_field(error: OpError, field: ProjectFormField) -> OpError {
    error.with_field(field)
}

/// The total of per-task or per-person budgets: money in the given currency,
/// or hours when `money_in` is `None`.
fn sum_budgets<'a>(
    values: impl Iterator<Item = (ProjectFormField, &'a str)>,
    money_in: Option<&str>,
) -> Result<Option<i64>, OpError> {
    let mut total = None;
    for (field, value) in values {
        let amount = match money_in {
            Some(currency) => {
                optional_amount(value, "Budget").and_then(|amount| budget_step(amount, currency))
            }
            None => optional_hours(value),
        }
        .map_err(|error| with_field(error, field))?;
        if let Some(amount) = amount {
            total = Some(total.unwrap_or(0_i64).checked_add(amount).ok_or_else(|| {
                with_field(OpError::bad_input("Budget total is too large"), field)
            })?);
        }
    }
    Ok(total)
}

pub fn validate_project_form(
    form: &ProjectForm,
    currency: &str,
    email_available: bool,
) -> Result<ValidatedProject, OpError> {
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
            _ => return OpError::bad_input(error),
        };
        with_field(OpError::bad_input(error), field)
    })?;
    form.budget_mode
        .validate_for(form.project_type)
        .map_err(|error| {
            let field = if error == ProjectValidationError::ProjectType {
                ProjectFormField::ProjectType
            } else {
                ProjectFormField::BudgetMode
            };
            with_field(OpError::bad_input(error), field)
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
                    OpError::bad_input("Select person, task or project rates"),
                    ProjectFormField::RateMode,
                ));
            }
        }
    };
    let rate_cents = if is_hourly && form.rate_mode == RateMode::Project {
        Some(
            optional_amount(&form.project_rate, "Project rate")
                .and_then(|value| {
                    value.ok_or_else(|| OpError::bad_input("Project rate is required"))
                })
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
                .and_then(|amount| budget_step(amount, currency))
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
                None,
            )?,
        ),
        BudgetMode::FeesPerTask => (
            BudgetKind::Amount,
            "task",
            sum_budgets(
                form.tasks
                    .iter()
                    .map(|task| (ProjectFormField::TaskBudget(task.id), task.budget.as_str())),
                Some(currency),
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
                None,
            )?,
        ),
    };
    let alert_threshold = if form.budget_alert && form.budget_mode != BudgetMode::None {
        if !email_available {
            return Err(with_field(
                OpError::bad_input("Budget email delivery is not configured"),
                ProjectFormField::BudgetAlert,
            ));
        }
        let threshold = form.budget_alert_at.trim().parse::<i16>().map_err(|_| {
            with_field(
                OpError::bad_input("Budget alert: enter a whole percentage from 0 to 100"),
                ProjectFormField::BudgetAlertAt,
            )
        })?;
        if !(0..=100).contains(&threshold) {
            return Err(with_field(
                OpError::bad_input("Budget alert: enter a whole percentage from 0 to 100"),
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
                        value.ok_or_else(|| OpError::bad_input("Fixed fee amount is required"))
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
                        OpError::bad_input(ProjectValidationError::Milestones),
                        ProjectFormField::Milestones,
                    ));
                }
                let mut ids = HashSet::new();
                let mut milestones = Vec::with_capacity(form.milestones.len());
                let mut total = 0_i64;
                for item in &form.milestones {
                    if !ids.insert(item.id) {
                        return Err(OpError::bad_input("Duplicate milestone"));
                    }
                    bounded_text(&item.name, 200, true, "Milestone name").map_err(|error| {
                        with_field(error, ProjectFormField::MilestoneName(item.id))
                    })?;
                    let due_on = date(&item.due_on, "Milestone date")
                        .and_then(|value| {
                            value.ok_or_else(|| OpError::bad_input("Milestone date is required"))
                        })
                        .map_err(|error| {
                            with_field(error, ProjectFormField::MilestoneDate(item.id))
                        })?;
                    let amount_cents = optional_amount(&item.amount, "Milestone fee")
                        .and_then(|value| {
                            value.ok_or_else(|| OpError::bad_input("Milestone amount is required"))
                        })
                        .map_err(|error| {
                            with_field(error, ProjectFormField::MilestoneAmount(item.id))
                        })?;
                    total = total.checked_add(amount_cents).ok_or_else(|| {
                        with_field(
                            OpError::bad_input("Milestone total is too large"),
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
        fee.validate().map_err(OpError::bad_input)?;
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
            OpError::bad_input("Payment terms: enter 0–365 days"),
            ProjectFormField::PaymentTerms,
        )
    })?;
    if !(0..=365).contains(&terms_days) {
        return Err(with_field(
            OpError::bad_input("Payment terms: enter 0–365 days"),
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
            Ok::<_, OpError>((
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
            return Err(OpError::bad_input("A person can only be assigned once"));
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
            return Err(OpError::bad_input("Duplicate task row"));
        }
        match &task.source {
            TaskSource::Existing { task_id } if !catalog_tasks.insert(*task_id) => {
                return Err(OpError::bad_input("A task can only be selected once"));
            }
            TaskSource::New { name } => {
                bounded_text(name, 200, true, "Task name")
                    .map_err(|error| with_field(error, ProjectFormField::TaskName(task.id)))?;
                if !new_task_names.insert(name.trim().to_lowercase()) {
                    return Err(with_field(
                        OpError::bad_input("Duplicate new task name"),
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
                OpError::bad_input("Task access must refer to selected teammates"),
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
    fn rates_distinguish_inheritance_zero_and_exact_amounts() {
        assert_eq!(optional_amount("  ", "Rate").unwrap(), None);
        assert_eq!(optional_amount("0", "Rate").unwrap(), Some(0));
        assert_eq!(optional_amount(" 120.50 ", "Rate").unwrap(), Some(12050));
        assert_eq!(
            optional_amount("92233720368547758.07", "Rate").unwrap(),
            Some(i64::MAX)
        );
        for invalid in ["-1", "NaN", "inf", "12.345", "92233720368547758.08"] {
            assert!(
                optional_amount(invalid, "Rate").is_err(),
                "accepted {invalid}"
            );
        }
    }

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

    /// A money budget is a whole number of the currency's minor unit: JPY
    /// takes no decimals. The form's own currencies all have two, so this
    /// matters for projects kept in another currency (legacy or imported).
    #[test]
    fn money_budgets_follow_the_currencys_minor_unit() {
        for (amount, currency) in [
            (None, "JPY"),
            (Some(800_000), "JPY"),
            (Some(8_050), "EUR"),
            (Some(8_050), "BHD"),
        ] {
            assert_eq!(budget_step(amount, currency).unwrap(), amount, "{currency}");
        }
        let error = budget_step(Some(8_050), " JPY").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Budget: JPY amounts have no decimal places"
        );
    }

    fn assert_rejection_field(form: &ProjectForm, email: bool, expected: serde_json::Value) {
        assert_eq!(field_rejection(form, "EUR", email).1, expected);
    }

    /// The message and blamed field of an input error that carries only a
    /// field, which is what the adapters answer as 400 with that field.
    fn field_rejection(
        form: &ProjectForm,
        currency: &str,
        email: bool,
    ) -> (String, serde_json::Value) {
        let Err(error) = validate_project_form(form, currency, email) else {
            panic!("Expected a field rejection");
        };
        assert!(matches!(error.kind(), OpError::Invalid { .. }), "{error:?}");
        let detail = error.detail().expect("the rejection names a field");
        assert_eq!(detail.reason, None);
        let field = detail.field.clone().expect("the rejection names a field");
        (error.to_string(), field)
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
            let (message, rejected) = field_rejection(&form, &currency, false);
            assert_eq!(rejected, serde_json::json!(field));
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
            let (message, rejected) = field_rejection(&form, "EUR", false);
            assert_eq!(rejected, serde_json::json!(field));
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
