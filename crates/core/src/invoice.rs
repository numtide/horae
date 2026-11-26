use crate::project::{Percentage, RateMode};
use crate::types::RoundingMethod;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    #[error("Invoice line amounts must not be negative")]
    NegativeLine,
    #[error("Invoice total exceeds the supported range")]
    Overflow,
}

/// Return each line's contribution after discount and before taxes.
///
/// Allocates the rounded invoice discount proportionally using largest
/// remainders. Equal remainders favor earlier input lines; callers must use
/// stable source order so preview, save and draft edits allocate identically.
/// Zero-subtotal invoices return zero contributions.
///
/// # Errors
/// Rejects negative lines or a subtotal exceeding `i64::MAX`.
pub fn allocate_invoice_discount(
    gross_lines: &[i64],
    discount: Percentage,
    rounding: Rounding,
) -> Result<Vec<i64>, InvoiceAmountError> {
    let subtotal = gross_lines.iter().try_fold(0_i64, |sum, &amount| {
        if amount < 0 {
            return Err(InvoiceAmountError::NegativeLine);
        }
        sum.checked_add(amount).ok_or(InvoiceAmountError::Overflow)
    })?;
    if subtotal == 0 {
        return Ok(vec![0; gross_lines.len()]);
    }
    let discount_cents = invoice_amounts(
        subtotal,
        discount,
        Percentage::ZERO,
        Percentage::ZERO,
        rounding,
    )?
    .discount_cents;
    let mut remaining = discount_cents;
    let mut contributions = Vec::with_capacity(gross_lines.len());
    let mut remainders = Vec::with_capacity(gross_lines.len());
    for (index, &gross) in gross_lines.iter().enumerate() {
        let numerator = i128::from(gross) * i128::from(discount_cents);
        let share = i64::try_from(numerator / i128::from(subtotal))
            .map_err(|_| InvoiceAmountError::Overflow)?;
        contributions.push(gross - share);
        remaining -= share;
        remainders.push((index, numerator % i128::from(subtotal)));
    }
    remainders.sort_unstable_by_key(|&(index, remainder)| (std::cmp::Reverse(remainder), index));
    for (index, _) in remainders {
        if remaining == 0 {
            break;
        }
        contributions[index] -= 1;
        remaining -= 1;
    }
    Ok(contributions)
}

/// How an invoice's discount and taxes round: to the step of its currency, in
/// hundredths (100 for a zero-decimal currency, else 1), by `method`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rounding {
    pub(crate) step: i64,
    pub(crate) method: RoundingMethod,
}

impl Rounding {
    /// `currency`'s step, the one line amounts round to, with `method`.
    pub fn new(currency: &str, method: RoundingMethod) -> Self {
        Self {
            step: amount_step(currency),
            method,
        }
    }

    /// `percentage` of `amount`, rounded to the step. A signed amount rounds
    /// by magnitude, so a negative one mirrors its positive. The SQL twin is
    /// `invoice_portion_cents(numeric, integer, text, rounding_method)`. It
    /// stays in `i128`: rounding up to the step can pass `i64::MAX`.
    fn portion(self, amount: i64, percentage: Percentage) -> i128 {
        let exact = i128::from(amount) * i128::from(percentage.basis_points());
        let unit = 10_000 * i128::from(self.step);
        let bias = match self.method {
            RoundingMethod::Down => 0,
            RoundingMethod::HalfUp => unit / 2,
            RoundingMethod::Up => unit - 1,
        };
        (exact.abs() + bias) / unit * i128::from(self.step) * exact.signum()
    }
}

fn to_cents(amount: i128) -> Result<i64, InvoiceAmountError> {
    i64::try_from(amount).map_err(|_| InvoiceAmountError::Overflow)
}

/// Apply the discount before independently rounded taxes, using exact minor
/// units. The discount and each tax round to the currency's step by the
/// rounding method; the total is the exact sum of the rounded parts. A
/// discount never exceeds the subtotal, which a subtotal off the step (a
/// zero-decimal invoice with a fractional fee) could otherwise round it past.
pub fn invoice_amounts(
    subtotal_cents: i64,
    discount: Percentage,
    tax1: Percentage,
    tax2: Percentage,
    rounding: Rounding,
) -> Result<InvoiceAmounts, InvoiceAmountError> {
    if subtotal_cents < 0 {
        return Err(InvoiceAmountError::NegativeSubtotal);
    }
    // Clamped before narrowing, so a full discount of any subtotal fits.
    let discount_cents = to_cents(
        rounding
            .portion(subtotal_cents, discount)
            .min(i128::from(subtotal_cents)),
    )?;
    let taxable_cents = subtotal_cents - discount_cents;
    let tax1_cents = to_cents(rounding.portion(taxable_cents, tax1))?;
    let tax2_cents = to_cents(rounding.portion(taxable_cents, tax2))?;
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

/// Whether an invoice is overdue on `today`: sent, still unpaid, and past its
/// due date. Overdue is computed, never stored as a status.
#[must_use]
pub fn is_overdue(
    status: crate::types::InvoiceStatus,
    due_on: chrono::NaiveDate,
    today: chrono::NaiveDate,
) -> bool {
    status == crate::types::InvoiceStatus::Sent && due_on < today
}

/// Compute a line amount in hundredths, rounded to the currency's precision.
///
/// `rate_cents` is an hourly rate in hundredths; `minutes` is the duration.
/// The exact value `rate × minutes ÷ 60` is rounded once, half up, to a whole
/// unit for a zero-decimal currency (a multiple of 100) and to a hundredth for
/// every other currency, so a total that sums these amounts stays in the
/// currency's precision. For the non-negative rates/durations stored by Horae,
/// a half step rounds up. Signed inputs retain truncation toward zero,
/// matching the SQL helper `line_amount_cents(bigint, integer, text)`.
///
/// # Errors
/// Returns an error if the final amount cannot fit in `i64`. The intermediate
/// product of an `i64` rate and `i32` duration always fits in `i128`.
pub fn line_amount_cents(
    rate_cents: i64,
    minutes: i32,
    currency: &str,
) -> Result<i64, std::num::TryFromIntError> {
    line_amount_in_step(rate_cents, minutes, i128::from(amount_step(currency)))
}

/// The step of `currency`'s amounts, in hundredths: 100 (a whole unit) for a
/// zero-decimal currency, else 1. The SQL twin is `currency_amount_step(text)`.
pub(crate) fn amount_step(currency: &str) -> i64 {
    if crate::money::currency_decimals(currency) == 0 {
        100
    } else {
        1
    }
}

/// [`line_amount_cents`] once the currency's step is known, in hundredths:
/// 1, or 100 for a zero-decimal currency.
pub(crate) fn line_amount_in_step(
    rate_cents: i64,
    minutes: i32,
    step: i128,
) -> Result<i64, std::num::TryFromIntError> {
    ((i128::from(rate_cents) * i128::from(minutes) + 30 * step) / (60 * step) * step).try_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EUR: Rounding = Rounding {
        step: 1,
        method: RoundingMethod::HalfUp,
    };

    #[test]
    fn only_a_sent_invoice_past_its_due_date_is_overdue() {
        use crate::types::InvoiceStatus::{Draft, Paid, Sent, Void};
        let due = chrono::NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let after = due.succ_opt().unwrap();
        assert!(is_overdue(Sent, due, after));
        assert!(!is_overdue(Sent, due, due), "due today is not overdue yet");
        for status in [Draft, Paid, Void] {
            assert!(!is_overdue(status, due, after), "{status:?}");
        }
    }

    #[test]
    fn discounted_fee_contribution_leaves_discount_available() {
        let net = allocate_invoice_discount(&[100_000], "10".parse().unwrap(), EUR).unwrap();
        assert_eq!(net, [90_000]);
        assert_eq!(100_000 - net[0], 10_000);
    }

    #[test]
    fn discount_allocation_uses_largest_remainder_then_stable_source_order() {
        assert_eq!(
            allocate_invoice_discount(&[1, 3, 1], "20".parse().unwrap(), EUR).unwrap(),
            [1, 2, 1]
        );
        assert_eq!(
            allocate_invoice_discount(&[1, 1, 1], "50".parse().unwrap(), EUR).unwrap(),
            [0, 0, 1]
        );
    }

    #[test]
    fn discount_allocation_handles_empty_zero_and_full_discounts() {
        let full = "100".parse().unwrap();
        assert!(
            allocate_invoice_discount(&[], full, EUR)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            allocate_invoice_discount(&[0, 0], full, EUR).unwrap(),
            [0, 0]
        );
        assert_eq!(
            allocate_invoice_discount(&[1, 99, 0], Percentage::ZERO, EUR).unwrap(),
            [1, 99, 0]
        );
        assert_eq!(
            allocate_invoice_discount(&[1, 99, 0], full, EUR).unwrap(),
            [0, 0, 0]
        );
    }

    #[test]
    fn discount_allocation_rejects_negative_lines_and_subtotal_overflow() {
        assert!(allocate_invoice_discount(&[100, -1], Percentage::ZERO, EUR).is_err());
        assert_eq!(
            allocate_invoice_discount(&[i64::MAX, 1], Percentage::ZERO, EUR),
            Err(InvoiceAmountError::Overflow)
        );
    }

    #[test]
    fn discount_allocation_supports_maximum_representable_subtotal() {
        let half = "50".parse().unwrap();
        assert_eq!(
            allocate_invoice_discount(&[i64::MAX - 1, 1], half, EUR).unwrap(),
            [i64::MAX / 2, 0]
        );
    }

    #[test]
    fn discount_allocation_conserves_header_amount_and_bounds_each_line() {
        for a in 0..=6 {
            for b in 0..=6 {
                for c in 0..=6 {
                    let gross = [a, b, c];
                    for bps in [0, 1, 2500, 3333, 5000, 9999, 10_000] {
                        let discount = Percentage::try_from(bps).unwrap();
                        let net = allocate_invoice_discount(&gross, discount, EUR).unwrap();
                        let amounts = invoice_amounts(
                            a + b + c,
                            discount,
                            Percentage::ZERO,
                            Percentage::ZERO,
                            EUR,
                        )
                        .unwrap();
                        assert_eq!(net.iter().sum::<i64>(), amounts.total_cents);
                        assert!(
                            net.iter()
                                .zip(gross)
                                .all(|(&net, gross)| (0..=gross).contains(&net))
                        );
                    }
                }
            }
        }
    }

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
            EUR,
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
        let amounts = invoice_amounts(1, Percentage::ZERO, half, half, EUR).unwrap();
        assert_eq!(
            (amounts.tax1_cents, amounts.tax2_cents, amounts.total_cents),
            (1, 1, 3)
        );
        assert_eq!(
            invoice_amounts(1, half, half, half, EUR)
                .unwrap()
                .total_cents,
            0
        );
    }

    /// (discount, tax1, total) of `subtotal` with a 10% discount and 8% tax.
    fn components(subtotal: i64, currency: &str, method: RoundingMethod) -> (i64, i64, i64) {
        let amounts = invoice_amounts(
            subtotal,
            "10".parse().unwrap(),
            "8".parse().unwrap(),
            Percentage::ZERO,
            Rounding::new(currency, method),
        )
        .unwrap();
        (
            amounts.discount_cents,
            amounts.tax1_cents,
            amounts.total_cents,
        )
    }

    #[test]
    fn a_zero_decimal_invoice_rounds_discount_and_taxes_to_whole_units() {
        use RoundingMethod::{Down, HalfUp, Up};
        // ¥12,345: discount ¥1,234.50, so taxable ¥11,110 (tax ¥888.80) or
        // ¥11,111 (tax ¥888.88).
        assert_eq!(
            components(1_234_500, "JPY", HalfUp),
            (123_500, 88_900, 1_199_900)
        );
        assert_eq!(
            components(1_234_500, "JPY", Down),
            (123_400, 88_800, 1_199_900)
        );
        assert_eq!(
            components(1_234_500, "JPY", Up),
            (123_500, 88_900, 1_199_900)
        );
    }

    #[test]
    fn two_and_three_decimal_invoices_round_to_hundredths() {
        use RoundingMethod::{Down, HalfUp, Up};
        // 123.45: discount 12.345, taxable 111.10 or 111.11, tax ≈ 8.888.
        for currency in ["EUR", "BHD"] {
            assert_eq!(components(12_345, currency, HalfUp), (1_235, 889, 11_999));
            assert_eq!(components(12_345, currency, Down), (1_234, 888, 11_999));
            assert_eq!(components(12_345, currency, Up), (1_235, 889, 11_999));
        }
    }

    #[test]
    fn rounding_methods_differ_on_a_half_step() {
        use RoundingMethod::{Down, HalfUp, Up};
        let half = |method| Rounding::new("JPY", method).portion(100, "50".parse().unwrap());
        assert_eq!([half(HalfUp), half(Down), half(Up)], [100, 0, 100]);
        let below_half = |method| Rounding::new("JPY", method).portion(98, "50".parse().unwrap());
        assert_eq!(
            [below_half(HalfUp), below_half(Down), below_half(Up)],
            [0, 0, 100]
        );
    }

    #[test]
    fn negative_amounts_round_as_the_mirror_of_positive_ones() {
        for currency in ["JPY", "EUR", "BHD"] {
            for method in [
                RoundingMethod::HalfUp,
                RoundingMethod::Down,
                RoundingMethod::Up,
            ] {
                let rounding = Rounding::new(currency, method);
                for amount in [0, 1, 49, 50, 51, 99, 150, 12_345, 1_234_550] {
                    for bps in [0_u16, 1, 800, 1_000, 5_000, 10_000] {
                        let percentage = Percentage::try_from(bps).unwrap();
                        assert_eq!(
                            rounding.portion(-amount, percentage),
                            -rounding.portion(amount, percentage),
                            "{currency} {method:?} {amount} {bps}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_discount_never_rounds_past_an_off_step_subtotal() {
        // A fractional-yen fee makes ¥1.50; a full discount rounds up to ¥2.
        let amounts = invoice_amounts(
            150,
            "100".parse().unwrap(),
            "10".parse().unwrap(),
            Percentage::ZERO,
            Rounding::new("JPY", RoundingMethod::Up),
        )
        .unwrap();
        assert_eq!((amounts.discount_cents, amounts.total_cents), (150, 0));
    }

    #[test]
    fn invoice_adjustments_reject_negative_subtotal_and_overflow() {
        let zero = Percentage::ZERO;
        assert_eq!(
            invoice_amounts(-1, zero, zero, zero, EUR),
            Err(InvoiceAmountError::NegativeSubtotal)
        );
        assert_eq!(
            invoice_amounts(i64::MAX, zero, "1".parse().unwrap(), zero, EUR),
            Err(InvoiceAmountError::Overflow)
        );
        assert_eq!(
            invoice_amounts(i64::MAX, zero, zero, zero, EUR)
                .unwrap()
                .total_cents,
            i64::MAX
        );
        assert_eq!(
            invoice_amounts(i64::MAX, "100".parse().unwrap(), zero, zero, EUR)
                .unwrap()
                .total_cents,
            0
        );
        // Rounded up to whole yen, the full discount of `i64::MAX` passes it;
        // it is clamped to the subtotal before it has to fit in `i64`.
        let jpy_up = Rounding::new("JPY", RoundingMethod::Up);
        assert_eq!(
            invoice_amounts(i64::MAX, "100".parse().unwrap(), zero, zero, jpy_up)
                .map(|a| (a.discount_cents, a.total_cents)),
            Ok((i64::MAX, 0))
        );
    }

    #[test]
    fn line_amount_accepts_large_representable_amounts() {
        assert_eq!(line_amount_cents(i64::MAX, 60, "EUR").unwrap(), i64::MAX);
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
        assert_eq!(line_amount_cents(10000, 60, "EUR").unwrap(), 10000);
    }

    #[test]
    fn line_amount_half_hour() {
        // $100/hr for 30 minutes = $50.00
        assert_eq!(line_amount_cents(10000, 30, "EUR").unwrap(), 5000);
    }

    #[test]
    fn line_amount_90_minutes() {
        // $100/hr for 90 minutes = $150.00
        assert_eq!(line_amount_cents(10000, 90, "EUR").unwrap(), 15000);
    }

    #[test]
    fn line_amount_zero_minutes() {
        assert_eq!(line_amount_cents(10000, 0, "EUR").unwrap(), 0);
    }

    #[test]
    fn line_amount_zero_rate() {
        assert_eq!(line_amount_cents(0, 60, "EUR").unwrap(), 0);
    }

    #[test]
    fn line_amount_odd_minutes() {
        // $120/hr (12000 cents) for 25 minutes = 12000 * 25 / 60 = 5000
        assert_eq!(line_amount_cents(12000, 25, "EUR").unwrap(), 5000);
    }

    #[test]
    fn line_amount_rejects_unrepresentable_results() {
        assert!(line_amount_cents(i64::MAX, 61, "EUR").is_err());
        assert!(line_amount_cents(i64::MIN, 61, "EUR").is_err());
        assert!(line_amount_cents(i64::MAX, i32::MAX, "EUR").is_err());
        assert!(line_amount_cents(i64::MAX, 61, "JPY").is_err());
    }

    #[test]
    fn line_amount_rounds_half_cents_up_not_to_even() {
        assert_eq!(line_amount_cents(1, 30, "EUR").unwrap(), 1);
        assert_eq!(line_amount_cents(1, 150, "EUR").unwrap(), 3);
    }

    #[test]
    fn line_amount_rounds_a_zero_decimal_currency_to_whole_units() {
        // 80.50 × 1.5 h = 120.75 → JPY 121; 5.00 × 0.5 h = 2.50 → JPY 3.
        assert_eq!(line_amount_cents(8050, 90, "JPY").unwrap(), 12100);
        assert_eq!(line_amount_cents(500, 30, "JPY").unwrap(), 300);
    }

    #[test]
    fn line_amount_keeps_hundredths_for_two_and_three_decimal_currencies() {
        assert_eq!(line_amount_cents(8050, 90, "EUR").unwrap(), 12075);
        assert_eq!(line_amount_cents(8050, 90, "BHD").unwrap(), 12075);
    }

    #[test]
    fn line_amount_rounds_once_from_the_exact_value() {
        // 150.62 × 48 min = 120.496: straight to units gives 120, while
        // rounding to 120.50 first would give 121.
        assert_eq!(line_amount_cents(15062, 48, "JPY").unwrap(), 12000);
    }
}
