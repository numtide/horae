use super::*;
use crate::models::project_creation::{FeeMode, TaskSource};
use chrono::NaiveDate;
use horae_core::project::{BudgetMode, FeeMilestone, FeeSchedule, Percentage, RateMode};
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

fn sum_budgets<'a>(
    values: impl Iterator<Item = &'a str>,
    money: bool,
) -> Result<Option<i64>, ServerFnError> {
    let mut total = None;
    for value in values {
        let amount = if money {
            optional_amount(value, "Budget")?
        } else {
            optional_hours(value)?
        };
        if let Some(amount) = amount {
            total = Some(
                total
                    .unwrap_or(0_i64)
                    .checked_add(amount)
                    .ok_or_else(|| err(BAD_REQUEST, "Budget total is too large"))?,
            );
        }
    }
    Ok(total)
}

pub(super) fn validate_project_form(
    form: &ProjectForm,
    currency: &str,
    email_available: bool,
) -> Result<ValidatedProject, ServerFnError> {
    let starts_on = date(&form.starts_on, "Start date")?;
    let ends_on = date(&form.ends_on, "End date")?;
    horae_core::project::validate_basics(
        &form.name,
        Some(&form.code),
        currency,
        starts_on,
        ends_on,
    )
    .map_err(|error| err(BAD_REQUEST, error))?;
    form.budget_mode
        .validate_for(form.project_type)
        .map_err(|error| err(BAD_REQUEST, error))?;
    bounded_text(&form.admin_notes, 10000, false, "Private notes")?;

    let is_hourly = form.project_type == ProjectType::TimeAndMaterials;
    let rate_mode = if !is_hourly {
        "person"
    } else {
        match form.rate_mode {
            RateMode::Person => "person",
            RateMode::Task => "task",
            RateMode::Project => "project",
            RateMode::Legacy => {
                return Err(err(BAD_REQUEST, "Select person, task or project rates"));
            }
        }
    };
    let rate_cents = if is_hourly && form.rate_mode == RateMode::Project {
        Some(
            optional_amount(&form.project_rate, "Project rate")?
                .ok_or_else(|| err(BAD_REQUEST, "Project rate is required"))?,
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
            optional_hours(&form.budget_value)?,
        ),
        BudgetMode::TotalFees => (
            BudgetKind::Amount,
            "project",
            optional_amount(&form.budget_value, "Budget")?,
            None,
        ),
        BudgetMode::HoursPerTask => (
            BudgetKind::Hours,
            "task",
            None,
            sum_budgets(form.tasks.iter().map(|task| task.budget.as_str()), false)?,
        ),
        BudgetMode::FeesPerTask => (
            BudgetKind::Amount,
            "task",
            sum_budgets(form.tasks.iter().map(|task| task.budget.as_str()), true)?,
            None,
        ),
        BudgetMode::HoursPerPerson => (
            BudgetKind::Hours,
            "person",
            None,
            sum_budgets(form.team.iter().map(|member| member.budget.as_str()), false)?,
        ),
    };
    let alert_threshold = if form.budget_alert && form.budget_mode != BudgetMode::None {
        if !email_available {
            return Err(err(BAD_REQUEST, "Budget email delivery is not configured"));
        }
        let threshold = form.budget_alert_at.trim().parse::<i16>().map_err(|_| {
            err(
                BAD_REQUEST,
                "Budget alert: enter a whole percentage from 0 to 100",
            )
        })?;
        if !(0..=100).contains(&threshold) {
            return Err(err(
                BAD_REQUEST,
                "Budget alert: enter a whole percentage from 0 to 100",
            ));
        }
        threshold
    } else {
        80
    };

    let fee = if form.project_type == ProjectType::FixedFee {
        let fee = match form.fee_mode {
            FeeMode::Single | FeeMode::Monthly => {
                let amount_cents = optional_amount(&form.fee_amount, "Fixed fee")?
                    .ok_or_else(|| err(BAD_REQUEST, "Fixed fee amount is required"))?;
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
                let mut ids = HashSet::new();
                let mut milestones = Vec::with_capacity(form.milestones.len());
                for item in &form.milestones {
                    if !ids.insert(item.id) {
                        return Err(err(BAD_REQUEST, "Duplicate milestone"));
                    }
                    milestones.push(FeeMilestone {
                        name: item.name.trim().into(),
                        due_on: date(&item.due_on, "Milestone date")?
                            .ok_or_else(|| err(BAD_REQUEST, "Milestone date is required"))?,
                        amount_cents: optional_amount(&item.amount, "Milestone fee")?
                            .ok_or_else(|| err(BAD_REQUEST, "Milestone amount is required"))?,
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
    let terms_days = defaults
        .terms_days
        .trim()
        .parse::<i16>()
        .map_err(|_| err(BAD_REQUEST, "Payment terms: enter 0–365 days"))?;
    if !(0..=365).contains(&terms_days) {
        return Err(err(BAD_REQUEST, "Payment terms: enter 0–365 days"));
    }
    bounded_text(&defaults.po_number, 200, false, "PO number")?;
    let discount_bps = percentage(&defaults.discount, "Discount")?;
    let tax1_bps = percentage(&defaults.tax, "Tax")?;
    let tax2 = defaults
        .second_tax
        .as_ref()
        .map(|tax| {
            bounded_text(&tax.name, 100, true, "Second tax name")?;
            Ok::<_, ServerFnError>((
                tax.name.trim().into(),
                percentage(&tax.percentage, "Second tax")?,
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
        optional_amount(&member.cost_rate, "Cost rate")?;
        if is_hourly && form.rate_mode == RateMode::Person {
            optional_amount(&member.billable_rate, "Person rate")?;
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
                bounded_text(name, 200, true, "Task name")?;
                if !new_task_names.insert(name.trim().to_lowercase()) {
                    return Err(err(BAD_REQUEST, "Duplicate new task name"));
                }
            }
            _ => {}
        }
        if let TaskAccess::Restricted { user_ids } = &task.access
            && user_ids.iter().any(|id| !people.contains(id))
        {
            return Err(err(
                BAD_REQUEST,
                "Task access must refer to selected teammates",
            ));
        }
        if is_hourly && form.rate_mode == RateMode::Task {
            optional_amount(&task.rate, "Task rate")?;
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
