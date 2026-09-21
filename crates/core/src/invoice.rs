use crate::project::{Percentage, RateMode};

/// Resolve an explicitly selected billing mode without changing legacy precedence.
pub fn resolve_project_rate(
    mode: RateMode,
    task_rate_cents: Option<i64>,
    assignment_rate_cents: Option<i64>,
    project_rate_cents: Option<i64>,
    user_rate_cents: Option<i64>,
    client_rate_cents: Option<i64>,
) -> Option<i64> {
    match mode {
        RateMode::Legacy => resolve_rate(
            task_rate_cents,
            assignment_rate_cents,
            project_rate_cents,
            user_rate_cents,
        ),
        RateMode::Person => assignment_rate_cents
            .or(user_rate_cents)
            .or(client_rate_cents),
        RateMode::Task => task_rate_cents.or(client_rate_cents),
        RateMode::Project => project_rate_cents,
    }
}

/// Invoice-owned components after discount and non-compounding taxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvoiceAmounts {
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax1_cents: i64,
    pub tax2_cents: i64,
    pub total_cents: i64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvoiceAmountError {
    #[error("Invoice subtotal must not be negative")]
    NegativeSubtotal,
    #[error("Invoice total exceeds the supported range")]
    Overflow,
}

/// Apply the discount before independently rounded taxes, using exact minor units.
pub fn invoice_amounts(
    subtotal_cents: i64,
    discount: Percentage,
    tax1: Percentage,
    tax2: Percentage,
) -> Result<InvoiceAmounts, InvoiceAmountError> {
    if subtotal_cents < 0 {
        return Err(InvoiceAmountError::NegativeSubtotal);
    }
    let portion = |amount: i64, percentage: Percentage| {
        i64::try_from((i128::from(amount) * i128::from(percentage.basis_points()) + 5_000) / 10_000)
            .map_err(|_| InvoiceAmountError::Overflow)
    };
    let discount_cents = portion(subtotal_cents, discount)?;
    let taxable_cents = subtotal_cents - discount_cents;
    let tax1_cents = portion(taxable_cents, tax1)?;
    let tax2_cents = portion(taxable_cents, tax2)?;
    let total_cents = taxable_cents
        .checked_add(tax1_cents)
        .and_then(|total| total.checked_add(tax2_cents))
        .ok_or(InvoiceAmountError::Overflow)?;
    Ok(InvoiceAmounts {
        subtotal_cents,
        discount_cents,
        tax1_cents,
        tax2_cents,
        total_cents,
    })
}

/// FR-024 rate resolution cascade.
///
/// Returns the first non-None rate in priority order:
/// 1. Task rate on the project (`project_tasks.rate_cents`)
/// 2. User's per-project assignment override (`assignments.rate_cents`)
/// 3. Project rate (`projects.rate_cents`)
/// 4. User's org-wide default (`users.billable_rate_cents`)
pub fn resolve_rate(
    task_rate_cents: Option<i64>,
    assignment_rate_cents: Option<i64>,
    project_rate_cents: Option<i64>,
    user_rate_cents: Option<i64>,
) -> Option<i64> {
    task_rate_cents
        .or(assignment_rate_cents)
        .or(project_rate_cents)
        .or(user_rate_cents)
}

/// Compute line amount in minor units (cents).
///
/// `rate_cents` is an hourly rate in cents; `minutes` is the duration.
/// For the non-negative rates/durations stored by Horae, a half-cent rounds up.
/// Signed inputs retain truncation toward zero, matching the SQL helper.
///
/// # Errors
/// Returns an error if the final amount cannot fit in `i64`. The intermediate
/// product of an `i64` rate and `i32` duration always fits in `i128`.
pub fn line_amount_cents(rate_cents: i64, minutes: i32) -> Result<i64, std::num::TryFromIntError> {
    ((i128::from(rate_cents) * i128::from(minutes) + 30) / 60).try_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_rate_mode_ignores_unselected_rate_sources() {
        let rates = |mode| {
            resolve_project_rate(
                mode,
                Some(5000),
                Some(4000),
                Some(3500),
                Some(3000),
                Some(2000),
            )
        };
        assert_eq!(rates(RateMode::Legacy), Some(5000));
        assert_eq!(rates(RateMode::Person), Some(4000));
        assert_eq!(rates(RateMode::Task), Some(5000));
        assert_eq!(rates(RateMode::Project), Some(3500));
    }

    #[test]
    fn selected_rate_keeps_zero_and_explicit_missing_project_rate() {
        assert_eq!(
            resolve_project_rate(
                RateMode::Person,
                Some(9),
                Some(0),
                Some(9),
                Some(8),
                Some(7)
            ),
            Some(0)
        );
        assert_eq!(
            resolve_project_rate(RateMode::Project, Some(9), Some(9), None, Some(8), Some(7)),
            None
        );
        assert_eq!(
            resolve_project_rate(RateMode::Task, None, Some(9), Some(9), Some(8), Some(7)),
            Some(7)
        );
        assert_eq!(
            resolve_project_rate(RateMode::Person, None, None, None, Some(8), Some(7)),
            Some(8)
        );
        assert_eq!(
            resolve_project_rate(RateMode::Legacy, None, None, None, None, Some(7)),
            None
        );
    }

    #[test]
    fn invoice_adjustments_apply_discount_before_non_compounding_taxes() {
        let amounts = invoice_amounts(
            10_000,
            "10".parse().unwrap(),
            "21".parse().unwrap(),
            "5".parse().unwrap(),
        )
        .unwrap();
        assert_eq!(
            amounts,
            InvoiceAmounts {
                subtotal_cents: 10_000,
                discount_cents: 1_000,
                tax1_cents: 1_890,
                tax2_cents: 450,
                total_cents: 11_340,
            }
        );
    }

    #[test]
    fn invoice_adjustments_round_components_half_up() {
        let half = "50".parse().unwrap();
        let amounts = invoice_amounts(1, Percentage::ZERO, half, half).unwrap();
        assert_eq!(
            (amounts.tax1_cents, amounts.tax2_cents, amounts.total_cents),
            (1, 1, 3)
        );
        assert_eq!(invoice_amounts(1, half, half, half).unwrap().total_cents, 0);
    }

    #[test]
    fn invoice_adjustments_reject_negative_subtotal_and_overflow() {
        let zero = Percentage::ZERO;
        assert_eq!(
            invoice_amounts(-1, zero, zero, zero),
            Err(InvoiceAmountError::NegativeSubtotal)
        );
        assert_eq!(
            invoice_amounts(i64::MAX, zero, "1".parse().unwrap(), zero),
            Err(InvoiceAmountError::Overflow)
        );
        assert_eq!(
            invoice_amounts(i64::MAX, zero, zero, zero)
                .unwrap()
                .total_cents,
            i64::MAX
        );
        assert_eq!(
            invoice_amounts(i64::MAX, "100".parse().unwrap(), zero, zero)
                .unwrap()
                .total_cents,
            0
        );
    }

    #[test]
    fn line_amount_accepts_large_representable_amounts() {
        assert_eq!(line_amount_cents(i64::MAX, 60).unwrap(), i64::MAX);
    }

    #[test]
    fn resolve_rate_cascade() {
        // Task rate wins when present
        assert_eq!(
            resolve_rate(Some(5000), Some(4000), Some(3500), Some(3000)),
            Some(5000)
        );
        // Falls through to assignment
        assert_eq!(
            resolve_rate(None, Some(4000), Some(3500), Some(3000)),
            Some(4000)
        );
        assert_eq!(resolve_rate(None, None, Some(3500), Some(3000)), Some(3500));
        assert_eq!(resolve_rate(None, None, Some(0), Some(3000)), Some(0));
        // Falls through to user default
        assert_eq!(resolve_rate(None, None, None, Some(3000)), Some(3000));
        // All None
        assert_eq!(resolve_rate(None, None, None, None), None);
    }

    #[test]
    fn line_amount_exact_hour() {
        // $100/hr for 60 minutes = $100.00
        assert_eq!(line_amount_cents(10000, 60).unwrap(), 10000);
    }

    #[test]
    fn line_amount_half_hour() {
        // $100/hr for 30 minutes = $50.00
        assert_eq!(line_amount_cents(10000, 30).unwrap(), 5000);
    }

    #[test]
    fn line_amount_90_minutes() {
        // $100/hr for 90 minutes = $150.00
        assert_eq!(line_amount_cents(10000, 90).unwrap(), 15000);
    }

    #[test]
    fn line_amount_zero_minutes() {
        assert_eq!(line_amount_cents(10000, 0).unwrap(), 0);
    }

    #[test]
    fn line_amount_zero_rate() {
        assert_eq!(line_amount_cents(0, 60).unwrap(), 0);
    }

    #[test]
    fn line_amount_odd_minutes() {
        // $120/hr (12000 cents) for 25 minutes = 12000 * 25 / 60 = 5000
        assert_eq!(line_amount_cents(12000, 25).unwrap(), 5000);
    }

    #[test]
    fn line_amount_rejects_unrepresentable_results() {
        assert!(line_amount_cents(i64::MAX, 61).is_err());
        assert!(line_amount_cents(i64::MIN, 61).is_err());
        assert!(line_amount_cents(i64::MAX, i32::MAX).is_err());
    }

    #[test]
    fn line_amount_rounds_half_cents_up_not_to_even() {
        assert_eq!(line_amount_cents(1, 30).unwrap(), 1);
        assert_eq!(line_amount_cents(1, 150).unwrap(), 3);
    }
}
