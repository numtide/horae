use thiserror::Error;

#[derive(Debug, Error)]
pub enum DurationError {
    #[error("invalid duration format: {0}")]
    InvalidFormat(String),
    #[error("duration or decimal precision out of range: {0}")]
    OutOfRange(String),
}

/// Parse "H:MM" or decimal hours (e.g. "1:30" or "1.5") into minutes.
/// Non-negative values only, rounded half up to at most `u32::MAX` minutes.
/// Decimals support at most 38 fractional digits and an unscaled numerator
/// up to `i128::MAX`; scientific notation and non-finite values are rejected.
pub fn parse(s: &str) -> Result<u32, DurationError> {
    let s = s.trim();
    if let Some((h, m)) = s.split_once(':') {
        let hours: u32 = h
            .trim()
            .parse()
            .map_err(|_| DurationError::InvalidFormat(s.to_owned()))?;
        let mins: u32 = m
            .trim()
            .parse()
            .map_err(|_| DurationError::InvalidFormat(s.to_owned()))?;
        if mins >= 60 {
            return Err(DurationError::InvalidFormat(s.to_owned()));
        }
        hours
            .checked_mul(60)
            .and_then(|minutes| minutes.checked_add(mins))
            .ok_or_else(|| DurationError::OutOfRange(s.to_owned()))
    } else {
        let minutes = crate::decimal::scale_decimal(s, 60).map_err(|err| match err {
            crate::decimal::DecimalError::OutOfRange(_) => DurationError::OutOfRange(s.to_owned()),
            _ => DurationError::InvalidFormat(s.to_owned()),
        })?;
        u32::try_from(minutes).map_err(|_| DurationError::OutOfRange(s.to_owned()))
    }
}

/// Format minutes as "H:MM". Negative values (possible for DB-sourced
/// aggregates) clamp to "0:00".
pub fn format_hhmm(minutes: i64) -> String {
    let minutes = minutes.max(0);
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// Format minutes as trimmed decimal hours (e.g. 90 → "1.5"). Negative values
/// clamp to "0".
pub fn format_decimal(minutes: i64) -> String {
    let decimal = minutes.max(0) as f64 / 60.0;
    if decimal.fract() == 0.0 {
        format!("{}", decimal as i64)
    } else {
        format!("{:.2}", decimal).trim_end_matches('0').to_owned()
    }
}

/// Minutes as fixed two-decimal hours — the report/export convention
/// (e.g. 90 → "1.50"). Integer half-up rounding: whole minutes never land
/// exactly on a half hundredth, and large values keep every digit.
pub fn format_hours2(minutes: i64) -> String {
    let hundredths = (u128::from(minutes.unsigned_abs()) * 100 + 30) / 60;
    let sign = if minutes < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", hundredths / 100, hundredths % 100)
}

/// Whole minutes elapsed between two instants, floored to the minute — the
/// exact tracked duration with no artificial minimum. A non-positive interval
/// (e.g. a clock skew) clamps to 0. Used by the timer stop path so totals stay
/// exact (never inflated).
pub fn minutes_between(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> u32 {
    (end - start).num_minutes().max(0) as u32
}

/// The most one person may track on one day, summed over all their entries:
/// Harvest's 240 hours ("Daily hours cannot exceed 240.").
pub const MAX_DAILY_MINUTES: i64 = 14_400;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("Daily hours cannot exceed 240.")]
pub struct DailyLimitExceeded;

/// Accepts a person's day total (in SQL's `SUM` width) up to [`MAX_DAILY_MINUTES`].
pub fn check_daily_total(total_minutes: i64) -> Result<(), DailyLimitExceeded> {
    if total_minutes > MAX_DAILY_MINUTES {
        Err(DailyLimitExceeded)
    } else {
        Ok(())
    }
}

/// Refuses a change only when it leaves the day over [`MAX_DAILY_MINUTES`] and
/// higher than it was, so a day already over the limit can still be lowered.
pub fn check_daily_change(before: i64, after: i64) -> Result<(), DailyLimitExceeded> {
    if after > before {
        check_daily_total(after)
    } else {
        Ok(())
    }
}

/// Clamps `minutes` to what is left of the day once `others` minutes are
/// tracked, never below 0, so a stopped timer fits the day instead of
/// staying trapped in a running state.
pub fn clamp_to_daily_allowance(minutes: u32, others: i64) -> u32 {
    let remaining = (MAX_DAILY_MINUTES - others).clamp(0, MAX_DAILY_MINUTES);
    minutes.min(remaining as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_total_accepts_exactly_240_hours_and_rejects_one_minute_more() {
        assert_eq!(check_daily_total(0), Ok(()));
        assert_eq!(check_daily_total(14_400), Ok(()));
        assert_eq!(check_daily_total(14_401), Err(DailyLimitExceeded));
        assert_eq!(
            DailyLimitExceeded.to_string(),
            "Daily hours cannot exceed 240."
        );
    }

    #[test]
    fn daily_change_refuses_only_growth_past_the_limit() {
        assert_eq!(check_daily_change(14_000, 14_400), Ok(()));
        assert_eq!(check_daily_change(14_000, 14_401), Err(DailyLimitExceeded));
        // A day already over the limit may shrink or stay, never grow.
        assert_eq!(check_daily_change(20_000, 15_000), Ok(()));
        assert_eq!(check_daily_change(20_000, 20_000), Ok(()));
        assert_eq!(check_daily_change(20_000, 20_001), Err(DailyLimitExceeded));
    }

    #[test]
    fn clamp_keeps_a_stop_within_the_rest_of_the_day() {
        assert_eq!(clamp_to_daily_allowance(90, 0), 90);
        assert_eq!(clamp_to_daily_allowance(90, 14_350), 50);
        assert_eq!(clamp_to_daily_allowance(90, 14_400), 0);
        assert_eq!(clamp_to_daily_allowance(90, 20_000), 0);
        assert_eq!(clamp_to_daily_allowance(u32::MAX, -5), 14_400);
    }

    #[test]
    fn parse_rejects_negative_non_finite_and_malformed_values() {
        for input in [
            "-1", "-0", "NaN", "inf", "-inf", "1e3", "", ".", "1:60", "1:2:3",
        ] {
            assert!(parse(input).is_err(), "accepted {input:?}");
        }
    }

    #[test]
    fn parse_rejects_colon_overflow_without_panicking() {
        assert!(parse("71582789:00").is_err());
    }

    #[test]
    fn parse_preserves_u32_boundary_without_saturating() {
        assert_eq!(parse("71582788:15").unwrap(), u32::MAX);
        assert_eq!(parse("71582788.25").unwrap(), u32::MAX);
        assert!(parse("71582788:16").is_err());
        assert!(parse("71582788.26").is_err());
    }

    #[test]
    fn parse_decimal_rounds_half_minutes_exactly() {
        assert_eq!(
            parse("0.00833333333333333333333333333333333333").unwrap(),
            0
        );
        assert_eq!(
            parse("0.00833333333333333333333333333333333334").unwrap(),
            1
        );
    }

    #[test]
    fn parse_round_trips_each_minute_in_a_day() {
        for minutes in 0..=1440u32 {
            assert_eq!(parse(&format_hhmm(i64::from(minutes))).unwrap(), minutes);
        }
    }

    #[test]
    fn parse_rejects_excessive_decimal_precision_and_numerator() {
        for input in [format!("0.{}1", "0".repeat(38)), format!("{}0", i128::MAX)] {
            assert!(matches!(parse(&input), Err(DurationError::OutOfRange(_))));
        }
    }

    #[test]
    fn parse_colon() {
        assert_eq!(parse("1:30").unwrap(), 90);
        assert_eq!(parse("0:00").unwrap(), 0);
        assert_eq!(parse("2:05").unwrap(), 125);
    }

    #[test]
    fn parse_decimal() {
        assert_eq!(parse("1.5").unwrap(), 90);
        assert_eq!(parse("0.25").unwrap(), 15);
    }

    #[test]
    fn format_round_trip() {
        assert_eq!(format_hhmm(90), "1:30");
        assert_eq!(format_decimal(90), "1.5");
        assert_eq!(format_hours2(90), "1.50");
    }

    #[test]
    fn format_hhmm_pads_minutes() {
        assert_eq!(format_hhmm(65), "1:05");
    }

    #[test]
    fn format_clamps_negative_to_zero() {
        assert_eq!(format_hhmm(-5), "0:00");
        assert_eq!(format_decimal(-5), "0");
    }

    #[test]
    fn format_hours2_keeps_ordinary_output_and_sign() {
        for minutes in -100_000..=100_000_i64 {
            assert_eq!(
                format_hours2(minutes),
                format!("{:.2}", minutes as f64 / 60.0),
                "{minutes}"
            );
        }
        assert_eq!(format_hours2(-30), "-0.50");
        assert_eq!(format_hours2(1), "0.02");
    }

    #[test]
    fn format_hours2_is_exact_where_float_division_is_not() {
        assert_eq!(format_hours2(9_007_199_254_740_991), "150119987579016.52");
        assert_eq!(format_hours2(i64::MAX), "153722867280912930.12");
        assert_eq!(format_hours2(i64::MIN), "-153722867280912930.13");
    }

    #[test]
    fn minutes_between_is_exact_no_minimum() {
        use chrono::{Duration, TimeZone, Utc};
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 9, 0, 0).unwrap();
        // Under a minute records 0 — no artificial 1-minute minimum (exactness).
        assert_eq!(minutes_between(start, start + Duration::seconds(30)), 0);
        assert_eq!(minutes_between(start, start + Duration::seconds(90)), 1);
        assert_eq!(minutes_between(start, start + Duration::minutes(5)), 5);
        // Non-positive intervals clamp to 0.
        assert_eq!(minutes_between(start, start - Duration::minutes(3)), 0);
    }
}
