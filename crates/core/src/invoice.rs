/// FR-024 rate resolution cascade.
///
/// Returns the first non-None rate in priority order:
/// 1. Task rate on the project (`project_tasks.rate_cents`)
/// 2. User's per-project assignment override (`assignments.rate_cents`)
/// 3. User's org-wide default (`users.billable_rate_cents`)
pub fn resolve_rate(
    task_rate_cents: Option<i64>,
    assignment_rate_cents: Option<i64>,
    user_rate_cents: Option<i64>,
) -> Option<i64> {
    task_rate_cents
        .or(assignment_rate_cents)
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
    fn line_amount_accepts_large_representable_amounts() {
        assert_eq!(line_amount_cents(i64::MAX, 60).unwrap(), i64::MAX);
    }

    #[test]
    fn resolve_rate_cascade() {
        // Task rate wins when present
        assert_eq!(resolve_rate(Some(5000), Some(4000), Some(3000)), Some(5000));
        // Falls through to assignment
        assert_eq!(resolve_rate(None, Some(4000), Some(3000)), Some(4000));
        // Falls through to user default
        assert_eq!(resolve_rate(None, None, Some(3000)), Some(3000));
        // All None
        assert_eq!(resolve_rate(None, None, None), None);
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
