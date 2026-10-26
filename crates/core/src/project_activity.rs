//! Inclusive work-date ranges and exact tracked-time series for project reporting.

use chrono::{Datelike, NaiveDate};
use thiserror::Error;

use crate::project::{MonthlyFeeDay, monthly_fee_date};

/// Invalid reporting input or an aggregate that cannot be represented exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActivityError {
    #[error("Choose both reporting dates or all time")]
    IncompleteRange,
    #[error("Invalid reporting date (use YYYY-MM-DD)")]
    InvalidDate,
    #[error("The reporting start date must not follow the end date")]
    ReversedRange,
    #[error("The reporting period is outside the supported date range")]
    DateOutOfRange,
    #[error("The reporting period exceeds the limit of {limit} weeks")]
    TooManyWeeks { limit: usize },
    #[error("Tracked minutes must not be negative")]
    NegativeMinutes,
    #[error("Tracked minutes exceed the supported total")]
    Overflow,
}

/// An inclusive, nonempty interval of work dates, without timezone conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityRange {
    from: NaiveDate,
    to: NaiveDate,
}

impl ActivityRange {
    /// Parse an optional inclusive period. Only two absent bounds mean all time;
    /// an empty, malformed or incomplete range must not broaden a report.
    pub fn parse_optional(
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Option<Self>, ActivityError> {
        match (from, to) {
            (None, None) => Ok(None),
            (Some(from), Some(to)) => Self::new(
                from.parse().map_err(|_| ActivityError::InvalidDate)?,
                to.parse().map_err(|_| ActivityError::InvalidDate)?,
            )
            .map(Some),
            _ => Err(ActivityError::IncompleteRange),
        }
    }

    /// Validate inclusive bounds. Equal bounds represent one day.
    pub fn new(from: NaiveDate, to: NaiveDate) -> Result<Self, ActivityError> {
        if from > to {
            return Err(ActivityError::ReversedRange);
        }
        Ok(Self { from, to })
    }

    /// The complete calendar month containing `date`.
    pub fn month(date: NaiveDate) -> Result<Self, ActivityError> {
        Self::calendar_months(date.year(), date.month(), date.month())
    }

    /// The complete calendar quarter containing `date`.
    pub fn quarter(date: NaiveDate) -> Result<Self, ActivityError> {
        let first_month = date.month0() / 3 * 3 + 1;
        Self::calendar_months(date.year(), first_month, first_month + 2)
    }

    /// The complete calendar year containing `date`.
    pub fn year(date: NaiveDate) -> Result<Self, ActivityError> {
        Self::calendar_months(date.year(), 1, 12)
    }

    fn calendar_months(year: i32, first: u32, last: u32) -> Result<Self, ActivityError> {
        let from = monthly_fee_date(year, first, MonthlyFeeDay::First)
            .map_err(|_| ActivityError::DateOutOfRange)?;
        let to = monthly_fee_date(year, last, MonthlyFeeDay::Last)
            .map_err(|_| ActivityError::DateOutOfRange)?;
        Self::new(from, to)
    }

    /// First included work date.
    pub fn from(self) -> NaiveDate {
        self.from
    }

    /// Last included work date.
    pub fn to(self) -> NaiveDate {
        self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> NaiveDate {
        value.parse().unwrap()
    }

    fn range(from: &str, to: &str) -> ActivityRange {
        ActivityRange::new(date(from), date(to)).unwrap()
    }

    #[test]
    fn absent_dates_mean_all_time() {
        assert_eq!(ActivityRange::parse_optional(None, None), Ok(None));
    }

    #[test]
    fn explicit_dates_are_inclusive() {
        let day = date("2026-09-07");
        assert_eq!(
            ActivityRange::parse_optional(Some("2026-09-07"), Some("2026-09-07")),
            Ok(Some(ActivityRange::new(day, day).unwrap()))
        );
    }

    #[test]
    fn invalid_dates_never_become_all_time() {
        for (from, to, error) in [
            (None, Some("2026-09-07"), ActivityError::IncompleteRange),
            (Some("2026-09-07"), None, ActivityError::IncompleteRange),
            (Some(""), Some(""), ActivityError::InvalidDate),
            (Some("2026-02-30"), Some("2026-09-07"), ActivityError::InvalidDate),
            (Some("2026-09-07"), Some("07/09/2026"), ActivityError::InvalidDate),
            (Some("2026-09-08"), Some("2026-09-07"), ActivityError::ReversedRange),
        ] {
            assert_eq!(
                ActivityRange::parse_optional(from, to),
                Err(error),
                "{from:?} / {to:?}"
            );
        }
    }

    #[test]
    fn rejects_reversed_custom_dates() {
        assert_eq!(
            ActivityRange::new(date("2026-02-02"), date("2026-02-01")),
            Err(ActivityError::ReversedRange)
        );
    }

    #[test]
    fn includes_leap_day_in_february() {
        assert_eq!(
            ActivityRange::month(date("2024-02-29")).unwrap(),
            range("2024-02-01", "2024-02-29")
        );
    }

    #[test]
    fn month_boundaries_do_not_clamp_from_the_anchor_day() {
        assert_eq!(
            ActivityRange::month(date("2026-01-31")).unwrap(),
            range("2026-01-01", "2026-01-31")
        );
    }

    #[test]
    fn quarters_are_calendar_quarters() {
        for (anchor, from, to) in [
            ("2024-02-29", "2024-01-01", "2024-03-31"),
            ("2026-04-30", "2026-04-01", "2026-06-30"),
            ("2026-09-29", "2026-07-01", "2026-09-30"),
            ("2026-12-31", "2026-10-01", "2026-12-31"),
        ] {
            assert_eq!(
                ActivityRange::quarter(date(anchor)).unwrap(),
                range(from, to)
            );
        }
    }

    #[test]
    fn year_includes_both_boundaries() {
        assert_eq!(
            ActivityRange::year(date("2024-02-29")).unwrap(),
            range("2024-01-01", "2024-12-31")
        );
    }

    #[test]
    fn calendar_bounds_support_the_representable_extremes() {
        for date in [NaiveDate::MIN, NaiveDate::MAX] {
            for period in [
                ActivityRange::month(date),
                ActivityRange::quarter(date),
                ActivityRange::year(date),
            ] {
                let period = period.unwrap();
                assert!((period.from()..=period.to()).contains(&date));
            }
        }
    }
}
