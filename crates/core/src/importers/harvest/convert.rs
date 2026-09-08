//! Exact decimal → integer conversions for the importer (FR-005, FR-006).
//!
//! Harvest expresses durations as decimal hours and money as a decimal amount.
//! Horae stores durations as integer minutes and money as integer minor units
//! (cents). These helpers perform the conversion **without binary-float error**:
//! the decimal is parsed digit-by-digit into an exact rational and the scaling
//! (×60 for minutes, ×100 for cents) is done in integer arithmetic, then rounded
//! half up. This is the inverse of the exporter's `hours = minutes / 60` and
//! `rate = cents / 100` (research.md §3, contracts/harvest-api.md §B).
//!
//! Both source adapters feed these the *string* form of the decimal (the API's
//! JSON number rendered back to its text, the CSV cell verbatim) so no `f64`
//! ever sits between Harvest's value and the stored integer.

pub use crate::decimal::DecimalError as ConvertError;
use crate::decimal::scale_decimal;

/// Convert decimal hours to exact whole minutes: `round(hours * 60)`, half up.
///
/// `0.25` → 15, `1.5` → 90, `2` → 120. Rejects negative/unparseable input or
/// a result outside `i64`. Supports at most 38 fractional digits and an
/// unscaled decimal numerator no larger than `i128::MAX`.
pub fn hours_to_minutes(hours: &str) -> Result<i64, ConvertError> {
    scale_decimal(hours, 60)
}

/// Convert a decimal money amount to integer minor units: `round(amount * 100)`,
/// half up. `10` → 1000, `10.5` → 1050, `1.005` → 101. Rejects a negative value.
/// Uses the same decimal precision and result limits as [`hours_to_minutes`].
pub fn money_to_cents(amount: &str) -> Result<i64, ConvertError> {
    scale_decimal(amount, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_handles_high_precision_without_intermediate_overflow() {
        assert_eq!(
            hours_to_minutes("1.70141183460469231731687303715884105727"),
            Ok(102)
        );
        assert_eq!(
            money_to_cents("1.70141183460469231731687303715884105727"),
            Ok(170)
        );
    }

    #[test]
    fn conversion_rejects_unrepresentable_scaled_values_without_panicking() {
        assert!(hours_to_minutes(&i128::MAX.to_string()).is_err());
        assert!(money_to_cents(&i128::MAX.to_string()).is_err());
    }

    #[test]
    fn conversion_preserves_the_i64_boundary_and_rejects_rounding_past_it() {
        assert_eq!(money_to_cents("92233720368547758.07"), Ok(i64::MAX));
        assert_eq!(hours_to_minutes("153722867280912930.12"), Ok(i64::MAX));
        for result in [
            money_to_cents("92233720368547758.075"),
            hours_to_minutes("153722867280912930.125"),
        ] {
            assert!(matches!(result, Err(ConvertError::OutOfRange(_))));
        }
    }

    #[test]
    fn high_precision_fraction_straddles_the_half_minute_exactly() {
        assert_eq!(
            hours_to_minutes("0.00833333333333333333333333333333333333"),
            Ok(0)
        );
        assert_eq!(
            hours_to_minutes("0.00833333333333333333333333333333333334"),
            Ok(1)
        );
        assert_eq!(
            hours_to_minutes("0.99999999999999999999999999999999999999"),
            Ok(60)
        );
    }

    #[test]
    fn conversion_reports_precision_and_numerator_limits() {
        for input in [format!("0.{}1", "0".repeat(38)), format!("{}0", i128::MAX)] {
            assert!(matches!(
                money_to_cents(&input),
                Err(ConvertError::OutOfRange(_))
            ));
        }
    }

    #[test]
    fn conversion_matches_exact_scaled_rationals() {
        for numerator in 0..=10_000i64 {
            let decimal = format!("{}.{:03}", numerator / 1000, numerator % 1000);
            assert_eq!(
                hours_to_minutes(&decimal),
                Ok((numerator * 60 + 500) / 1000)
            );
            assert_eq!(money_to_cents(&decimal), Ok((numerator * 100 + 500) / 1000));
        }
    }

    #[test]
    fn hours_common_values_are_exact() {
        assert_eq!(hours_to_minutes("0").unwrap(), 0);
        assert_eq!(hours_to_minutes("0.25").unwrap(), 15);
        assert_eq!(hours_to_minutes("0.5").unwrap(), 30);
        assert_eq!(hours_to_minutes("1").unwrap(), 60);
        assert_eq!(hours_to_minutes("1.5").unwrap(), 90);
        assert_eq!(hours_to_minutes("2.75").unwrap(), 165);
        assert_eq!(hours_to_minutes("8").unwrap(), 480);
    }

    #[test]
    fn hours_round_trip_against_exporter_transform() {
        // The exporter emits `hours = minutes / 60`; importing that string back
        // must recover the exact minutes for every minute in a day.
        for minutes in 0..=1440i64 {
            // Render minutes/60 the way a well-formed source would (enough digits).
            let hours = format!("{:.6}", minutes as f64 / 60.0);
            assert_eq!(
                hours_to_minutes(&hours).unwrap(),
                minutes,
                "minutes={minutes} hours={hours}"
            );
        }
    }

    #[test]
    fn round_half_up_at_the_boundary() {
        // A value exactly at x.5 rounds up (ties toward +inf; non-negative here).
        assert_eq!(scale_decimal("0.5", 1).unwrap(), 1);
        assert_eq!(scale_decimal("1.5", 1).unwrap(), 2);
        assert_eq!(scale_decimal("2.5", 1).unwrap(), 3);
        // Just below/above the tie do not round up / do.
        assert_eq!(scale_decimal("0.49", 1).unwrap(), 0);
        assert_eq!(scale_decimal("0.51", 1).unwrap(), 1);
    }

    #[test]
    fn money_common_values_are_exact() {
        assert_eq!(money_to_cents("0").unwrap(), 0);
        assert_eq!(money_to_cents("10").unwrap(), 1000);
        assert_eq!(money_to_cents("10.5").unwrap(), 1050);
        assert_eq!(money_to_cents("99.99").unwrap(), 9999);
        assert_eq!(money_to_cents("150").unwrap(), 15000);
    }

    #[test]
    fn money_round_trip_against_exporter_transform() {
        for cents in [0i64, 1, 99, 100, 2500, 12345, 100_000, 999_999] {
            let amount = format!("{:.2}", cents as f64 / 100.0);
            assert_eq!(money_to_cents(&amount).unwrap(), cents, "cents={cents}");
        }
    }

    #[test]
    fn money_rounds_third_decimal_half_up() {
        assert_eq!(money_to_cents("1.005").unwrap(), 101); // ties up
        assert_eq!(money_to_cents("1.004").unwrap(), 100);
        assert_eq!(money_to_cents("1.006").unwrap(), 101);
    }

    #[test]
    fn leading_and_trailing_forms_parse() {
        assert_eq!(hours_to_minutes(" 1.5 ").unwrap(), 90);
        assert_eq!(hours_to_minutes("+1.5").unwrap(), 90);
        assert_eq!(hours_to_minutes(".5").unwrap(), 30);
        assert_eq!(hours_to_minutes("2.").unwrap(), 120);
    }

    #[test]
    fn negative_is_rejected() {
        assert_eq!(
            hours_to_minutes("-1"),
            Err(ConvertError::Negative("-1".into()))
        );
        assert_eq!(
            money_to_cents("-0.5"),
            Err(ConvertError::Negative("-0.5".into()))
        );
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(matches!(
            hours_to_minutes("abc"),
            Err(ConvertError::NotANumber(_))
        ));
        assert!(matches!(
            hours_to_minutes("1.2.3"),
            Err(ConvertError::NotANumber(_))
        ));
        assert!(matches!(
            hours_to_minutes(""),
            Err(ConvertError::NotANumber(_))
        ));
        assert!(matches!(
            money_to_cents("1,000"),
            Err(ConvertError::NotANumber(_))
        ));
    }
}
