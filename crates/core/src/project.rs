//! Validation for project creation and billing settings.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Supported creation currencies share Horae's existing two-decimal precision.
pub const PROJECT_CURRENCIES: [&str; 4] = ["EUR", "CHF", "USD", "GBP"];

/// Legacy precedence is retained for projects without explicit rate settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateMode {
    Legacy,
    Person,
    Task,
    Project,
}

/// A percentage represented in hundredths of one percent, bounded to 0–100%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Percentage(u16);

impl Percentage {
    pub const ZERO: Self = Self(0);

    pub fn basis_points(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for Percentage {
    type Error = ProjectValidationError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value > 10_000 {
            return Err(ProjectValidationError::Percentage);
        }
        Ok(Self(value))
    }
}

impl From<Percentage> for u16 {
    fn from(value: Percentage) -> Self {
        value.0
    }
}

impl std::str::FromStr for Percentage {
    type Err = ProjectValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if !value.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
            return Err(ProjectValidationError::Percentage);
        }
        let basis_points =
            crate::money::parse_cents(value).map_err(|_| ProjectValidationError::Percentage)?;
        Self::try_from(u16::try_from(basis_points).map_err(|_| ProjectValidationError::Percentage)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonthlyFeeDay {
    First,
    Fifteenth,
    Last,
}

/// The handoff's budget choices keep denomination and allocation explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetMode {
    None,
    TotalHours,
    TotalFees,
    HoursPerTask,
    FeesPerTask,
    HoursPerPerson,
}

impl BudgetMode {
    pub fn validate_for(
        self,
        project_type: crate::types::ProjectType,
    ) -> Result<(), ProjectValidationError> {
        use crate::types::ProjectType;
        if project_type == ProjectType::Retainer {
            return Err(ProjectValidationError::ProjectType);
        }
        if matches!(self, Self::TotalFees | Self::FeesPerTask)
            && project_type != ProjectType::TimeAndMaterials
        {
            return Err(ProjectValidationError::BudgetMode);
        }
        Ok(())
    }
}

/// A fixed-fee installment, independent of time-entry duration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeMilestone {
    pub name: String,
    pub due_on: NaiveDate,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeeSchedule {
    Single {
        amount_cents: i64,
    },
    Milestones {
        milestones: Vec<FeeMilestone>,
    },
    Monthly {
        amount_cents: i64,
        day: MonthlyFeeDay,
    },
}

impl FeeSchedule {
    pub fn validate(&self) -> Result<(), ProjectValidationError> {
        match self {
            Self::Single { amount_cents } | Self::Monthly { amount_cents, .. } => {
                if *amount_cents < 0 {
                    return Err(ProjectValidationError::FeeAmount);
                }
            }
            Self::Milestones { milestones } => {
                if !(1..=100).contains(&milestones.len()) {
                    return Err(ProjectValidationError::Milestones);
                }
                let mut total = 0_i64;
                for milestone in milestones {
                    if !(1..=200).contains(&milestone.name.trim().chars().count())
                        || milestone.name.contains('\0')
                    {
                        return Err(ProjectValidationError::Milestones);
                    }
                    if milestone.amount_cents < 0 {
                        return Err(ProjectValidationError::FeeAmount);
                    }
                    total = total
                        .checked_add(milestone.amount_cents)
                        .ok_or(ProjectValidationError::FeeAmount)?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectValidationError {
    #[error("Project name must contain 1–200 characters")]
    Name,
    #[error("Project code must contain at most 100 characters")]
    Code,
    #[error("Choose a supported billing currency")]
    Currency,
    #[error("Project end date must not precede its start date")]
    DateOrder,
    #[error("Enter a percentage from 0 to 100 with at most two decimal places")]
    Percentage,
    #[error("Date is outside the supported range")]
    DateRange,
    #[error("Payment terms must be between 0 and 365 days")]
    PaymentTerms,
    #[error("Choose a budget supported by this project type")]
    BudgetMode,
    #[error("Choose a supported project creation type")]
    ProjectType,
    #[error("Fees must be nonnegative and within the supported monetary range")]
    FeeAmount,
    #[error("Enter 1–100 named milestones, each with at most 200 name characters")]
    Milestones,
}

/// Validate required fields without treating optional planning dates as tracking limits.
pub fn validate_basics(
    name: &str,
    code: Option<&str>,
    currency: &str,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
) -> Result<(), ProjectValidationError> {
    if !(1..=200).contains(&name.trim().chars().count()) || name.contains('\0') {
        return Err(ProjectValidationError::Name);
    }
    if code.is_some_and(|code| code.trim().chars().count() > 100 || code.contains('\0')) {
        return Err(ProjectValidationError::Code);
    }
    if !PROJECT_CURRENCIES.contains(&currency) {
        return Err(ProjectValidationError::Currency);
    }
    if starts_on
        .zip(ends_on)
        .is_some_and(|(start, end)| start > end)
    {
        return Err(ProjectValidationError::DateOrder);
    }
    Ok(())
}

/// Resolve a monthly fee day without approximating months as a fixed duration.
pub fn monthly_fee_date(
    year: i32,
    month: u32,
    day: MonthlyFeeDay,
) -> Result<NaiveDate, ProjectValidationError> {
    match day {
        MonthlyFeeDay::First => NaiveDate::from_ymd_opt(year, month, 1),
        MonthlyFeeDay::Fifteenth => NaiveDate::from_ymd_opt(year, month, 15),
        MonthlyFeeDay::Last => (28..=31)
            .rev()
            .find_map(|day| NaiveDate::from_ymd_opt(year, month, day)),
    }
    .ok_or(ProjectValidationError::DateRange)
}

/// Calculate receipt/Net/custom terms without overflowing the supported calendar.
pub fn payment_due_date(
    issued_on: NaiveDate,
    days: u16,
) -> Result<NaiveDate, ProjectValidationError> {
    if days > 365 {
        return Err(ProjectValidationError::PaymentTerms);
    }
    issued_on
        .checked_add_days(chrono::Days::new(u64::from(days)))
        .ok_or(ProjectValidationError::DateRange)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn monetary_budgets_are_only_available_for_time_and_materials() {
        use crate::types::ProjectType;
        for mode in [BudgetMode::TotalFees, BudgetMode::FeesPerTask] {
            assert!(mode.validate_for(ProjectType::TimeAndMaterials).is_ok());
            for project_type in [ProjectType::FixedFee, ProjectType::NonBillable] {
                assert_eq!(
                    mode.validate_for(project_type),
                    Err(ProjectValidationError::BudgetMode)
                );
            }
        }
        for mode in [
            BudgetMode::None,
            BudgetMode::TotalHours,
            BudgetMode::HoursPerTask,
            BudgetMode::HoursPerPerson,
        ] {
            for project_type in [
                ProjectType::TimeAndMaterials,
                ProjectType::FixedFee,
                ProjectType::NonBillable,
            ] {
                assert!(mode.validate_for(project_type).is_ok());
            }
        }
        assert_eq!(
            BudgetMode::None.validate_for(ProjectType::Retainer),
            Err(ProjectValidationError::ProjectType)
        );
    }

    #[test]
    fn fee_schedules_validate_amounts_and_milestone_total() {
        assert_eq!(
            FeeSchedule::Single { amount_cents: -1 }.validate(),
            Err(ProjectValidationError::FeeAmount)
        );
        assert!(
            FeeSchedule::Monthly {
                amount_cents: 0,
                day: MonthlyFeeDay::Last
            }
            .validate()
            .is_ok()
        );
        let milestone = FeeMilestone {
            name: "Discovery".into(),
            due_on: date(2026, 9, 30),
            amount_cents: i64::MAX,
        };
        assert!(
            FeeSchedule::Milestones {
                milestones: vec![milestone.clone()]
            }
            .validate()
            .is_ok()
        );
        assert_eq!(
            FeeSchedule::Milestones {
                milestones: vec![milestone.clone(), milestone]
            }
            .validate(),
            Err(ProjectValidationError::FeeAmount)
        );
    }

    #[test]
    fn fee_schedules_reject_missing_overlong_and_excessive_milestones() {
        assert_eq!(
            FeeSchedule::Milestones { milestones: vec![] }.validate(),
            Err(ProjectValidationError::Milestones)
        );
        for name in [" ".to_owned(), "a".repeat(201)] {
            let milestone = FeeMilestone {
                name,
                due_on: date(2026, 9, 30),
                amount_cents: 1,
            };
            assert_eq!(
                FeeSchedule::Milestones {
                    milestones: vec![milestone]
                }
                .validate(),
                Err(ProjectValidationError::Milestones)
            );
        }
        let milestone = FeeMilestone {
            name: "Phase".into(),
            due_on: date(2026, 9, 30),
            amount_cents: 1,
        };
        assert_eq!(
            FeeSchedule::Milestones {
                milestones: vec![milestone; 101]
            }
            .validate(),
            Err(ProjectValidationError::Milestones)
        );
    }

    #[test]
    fn percentage_preserves_exact_hundredths() {
        for (input, expected) in [("0", 0), (" 21.50 ", 2150), (".05", 5), ("100", 10000)] {
            assert_eq!(
                input.parse::<Percentage>().unwrap().basis_points(),
                expected
            );
        }
    }

    #[test]
    fn percentage_rejects_invalid_precision_range_and_syntax() {
        for input in [
            "", "-1", "100.01", "1.005", "NaN", "1e2", "1,0", "1 0", "+1", ".",
        ] {
            assert_eq!(
                input.parse::<Percentage>(),
                Err(ProjectValidationError::Percentage),
                "{input}"
            );
        }
        assert_eq!(
            Percentage::try_from(10001),
            Err(ProjectValidationError::Percentage)
        );
    }

    #[test]
    fn basics_accept_supported_currencies_and_optional_dates() {
        for currency in PROJECT_CURRENCIES {
            assert_eq!(
                validate_basics(" Project ", Some("CODE-1"), currency, None, None),
                Ok(())
            );
        }
    }

    #[test]
    fn basics_reject_blank_and_overlong_names() {
        for name in ["   ".to_owned(), "a".repeat(201)] {
            assert_eq!(
                validate_basics(&name, None, "EUR", None, None),
                Err(ProjectValidationError::Name)
            );
        }
        assert!(validate_basics(&"ñ".repeat(200), None, "EUR", None, None).is_ok());
    }

    #[test]
    fn basics_reject_overlong_code_and_unsupported_currency() {
        assert_eq!(
            validate_basics("Project", Some(&"a".repeat(101)), "EUR", None, None),
            Err(ProjectValidationError::Code)
        );
        assert_eq!(
            validate_basics("Project", None, "JPY", None, None),
            Err(ProjectValidationError::Currency)
        );
    }

    #[test]
    fn basics_accept_same_day_but_reject_reversed_dates() {
        let start = date(2026, 9, 21);
        assert!(validate_basics("Project", None, "EUR", Some(start), Some(start)).is_ok());
        assert_eq!(
            validate_basics("Project", None, "EUR", Some(start), Some(date(2026, 9, 20))),
            Err(ProjectValidationError::DateOrder)
        );
    }

    #[test]
    fn monthly_fee_dates_follow_calendar_months() {
        for (year, month, expected_day) in
            [(2024, 2, 29), (2026, 2, 28), (2026, 4, 30), (2026, 12, 31)]
        {
            assert_eq!(
                monthly_fee_date(year, month, MonthlyFeeDay::Last),
                Ok(date(year, month, expected_day))
            );
        }
        assert_eq!(
            monthly_fee_date(2026, 2, MonthlyFeeDay::First),
            Ok(date(2026, 2, 1))
        );
        assert_eq!(
            monthly_fee_date(2026, 2, MonthlyFeeDay::Fifteenth),
            Ok(date(2026, 2, 15))
        );
    }

    #[test]
    fn monthly_fee_dates_reject_invalid_calendar_inputs() {
        for (year, month) in [(2026, 0), (2026, 13), (i32::MAX, 1)] {
            assert_eq!(
                monthly_fee_date(year, month, MonthlyFeeDay::Last),
                Err(ProjectValidationError::DateRange)
            );
        }
    }

    #[test]
    fn payment_terms_support_receipt_and_checked_days() {
        let issued = date(2026, 9, 21);
        assert_eq!(payment_due_date(issued, 0), Ok(issued));
        assert_eq!(payment_due_date(issued, 30), Ok(date(2026, 10, 21)));
        assert_eq!(
            payment_due_date(issued, 366),
            Err(ProjectValidationError::PaymentTerms)
        );
        assert_eq!(
            payment_due_date(NaiveDate::MAX, 1),
            Err(ProjectValidationError::DateRange)
        );
    }
}
