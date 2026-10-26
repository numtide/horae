//! Inclusive work-date ranges and exact tracked-time series for project reporting.

use chrono::{Datelike, Days, NaiveDate, Weekday};
use thiserror::Error;

use crate::project::{MonthlyFeeDay, monthly_fee_date};
use crate::week::week_start;

/// Invalid reporting input or an aggregate that cannot be represented exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ActivityError {
    #[error("Choose both reporting dates or all time")]
    IncompleteRange,
    /// A bound is not a real date in canonical ISO `%Y-%m-%d` form.
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
    ///
    /// Each bound must be exactly the canonical `%Y-%m-%d` text of its date:
    /// zero-padded four-digit years 0000–9999, and a sign with at least four
    /// digits outside that span (e.g. `-0001-01-01`, `+12026-09-07`).
    pub fn parse_optional(
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Option<Self>, ActivityError> {
        match (from, to) {
            (None, None) => Ok(None),
            (Some(from), Some(to)) => Self::new(parse_date(from)?, parse_date(to)?).map(Some),
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

fn parse_date(value: &str) -> Result<NaiveDate, ActivityError> {
    let date: NaiveDate = value.parse().map_err(|_| ActivityError::InvalidDate)?;
    // chrono also accepts unpadded, whitespace-padded and redundantly signed forms.
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(ActivityError::InvalidDate);
    }
    Ok(date)
}

/// Actual tracked minutes for a work date; no invoice rounding is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyActivity {
    pub date: NaiveDate,
    pub billable_minutes: i64,
    pub non_billable_minutes: i64,
}

/// A week clipped to the selected interval, including weeks with no tracked time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeeklyActivity {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub billable_minutes: i64,
    pub non_billable_minutes: i64,
    /// Tracked minutes from the selected interval's start through this bucket.
    pub cumulative_minutes: i64,
}

/// Build ordered weekly buckets, ignoring dates outside `range`.
///
/// `max_weeks` is a caller-owned resource bound, not a truncation instruction.
/// Duplicate dates are added; input order affects neither the series nor the error.
///
/// # Errors
/// Rejects an unrepresentable first week or more than `max_weeks` buckets, then
/// negative included minutes (checked before any addition), then arithmetic
/// overflow. No partial series is returned on error.
pub fn weekly_activity(
    range: ActivityRange,
    first_day: Weekday,
    days: &[DailyActivity],
    max_weeks: usize,
) -> Result<Vec<WeeklyActivity>, ActivityError> {
    let first_week = week_start(range.from, first_day).ok_or(ActivityError::DateOutOfRange)?;
    let count = usize::try_from((range.to - first_week).num_days() / 7 + 1)
        .map_err(|_| ActivityError::TooManyWeeks { limit: max_weeks })?;
    if count > max_weeks {
        return Err(ActivityError::TooManyWeeks { limit: max_weeks });
    }

    let mut weeks = Vec::with_capacity(count);
    for index in 0..count {
        let start = first_week
            .checked_add_days(Days::new(index as u64 * 7))
            .ok_or(ActivityError::DateOutOfRange)?;
        // Clip before adding so a final partial week at NaiveDate::MAX is valid.
        let included_days = (range.to - start).num_days().min(6) as u64;
        let end = start
            .checked_add_days(Days::new(included_days))
            .ok_or(ActivityError::DateOutOfRange)?;
        weeks.push(WeeklyActivity {
            from: start.max(range.from),
            to: end,
            billable_minutes: 0,
            non_billable_minutes: 0,
            cumulative_minutes: 0,
        });
    }

    let included = || {
        days.iter()
            .filter(|day| (range.from..=range.to).contains(&day.date))
    };
    // Validate every included day first so the error does not depend on input order.
    if included().any(|day| day.billable_minutes < 0 || day.non_billable_minutes < 0) {
        return Err(ActivityError::NegativeMinutes);
    }
    for day in included() {
        let index = ((day.date - first_week).num_days() / 7) as usize;
        let week = &mut weeks[index];
        week.billable_minutes = week
            .billable_minutes
            .checked_add(day.billable_minutes)
            .ok_or(ActivityError::Overflow)?;
        week.non_billable_minutes = week
            .non_billable_minutes
            .checked_add(day.non_billable_minutes)
            .ok_or(ActivityError::Overflow)?;
    }

    let mut cumulative = 0_i64;
    for week in &mut weeks {
        cumulative = cumulative
            .checked_add(week.billable_minutes)
            .and_then(|total| total.checked_add(week.non_billable_minutes))
            .ok_or(ActivityError::Overflow)?;
        week.cumulative_minutes = cumulative;
    }
    Ok(weeks)
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
            (
                Some("2026-02-30"),
                Some("2026-09-07"),
                ActivityError::InvalidDate,
            ),
            (
                Some("2026-09-07"),
                Some("07/09/2026"),
                ActivityError::InvalidDate,
            ),
            (
                Some("2026-09-08"),
                Some("2026-09-07"),
                ActivityError::ReversedRange,
            ),
        ] {
            assert_eq!(
                ActivityRange::parse_optional(from, to),
                Err(error),
                "{from:?} / {to:?}"
            );
        }
    }

    #[test]
    fn rejects_noncanonical_iso_dates_in_either_bound() {
        for value in [
            "26-09-01",
            "2026-9-01",
            "2026-09-1",
            " 2026-09-01",
            "2026-09-01 ",
            "+2026-09-01",
        ] {
            for (from, to) in [(value, "2026-09-30"), ("2026-08-01", value)] {
                assert_eq!(
                    ActivityRange::parse_optional(Some(from), Some(to)),
                    Err(ActivityError::InvalidDate),
                    "{from:?} / {to:?}"
                );
            }
        }
    }

    #[test]
    fn accepts_canonical_signed_and_expanded_years() {
        let (min, max) = (NaiveDate::MIN.to_string(), NaiveDate::MAX.to_string());
        for (value, expected) in [
            ("-0001-01-01", NaiveDate::from_ymd_opt(-1, 1, 1).unwrap()),
            (
                "+12026-09-07",
                NaiveDate::from_ymd_opt(12026, 9, 7).unwrap(),
            ),
            (min.as_str(), NaiveDate::MIN),
            (max.as_str(), NaiveDate::MAX),
        ] {
            assert_eq!(
                ActivityRange::parse_optional(Some(value), Some(value)),
                Ok(Some(ActivityRange::new(expected, expected).unwrap())),
                "{value:?}"
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
    fn non_leap_february_ends_on_the_28th() {
        for (anchor, from, to) in [
            ("2026-02-10", "2026-02-01", "2026-02-28"),
            ("2100-02-15", "2100-02-01", "2100-02-28"),
        ] {
            assert_eq!(ActivityRange::month(date(anchor)).unwrap(), range(from, to));
        }
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

    const WEEKDAYS: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    fn day(value: &str, billable_minutes: i64, non_billable_minutes: i64) -> DailyActivity {
        DailyActivity {
            date: date(value),
            billable_minutes,
            non_billable_minutes,
        }
    }

    #[test]
    fn clips_boundary_weeks_and_preserves_empty_weeks() {
        let weeks =
            weekly_activity(range("2026-09-02", "2026-09-15"), Weekday::Mon, &[], 3).unwrap();
        assert_eq!(
            weeks,
            vec![
                WeeklyActivity {
                    from: date("2026-09-02"),
                    to: date("2026-09-06"),
                    billable_minutes: 0,
                    non_billable_minutes: 0,
                    cumulative_minutes: 0
                },
                WeeklyActivity {
                    from: date("2026-09-07"),
                    to: date("2026-09-13"),
                    billable_minutes: 0,
                    non_billable_minutes: 0,
                    cumulative_minutes: 0
                },
                WeeklyActivity {
                    from: date("2026-09-14"),
                    to: date("2026-09-15"),
                    billable_minutes: 0,
                    non_billable_minutes: 0,
                    cumulative_minutes: 0
                },
            ]
        );
    }

    #[test]
    fn filters_inclusively_and_accumulates_exact_minutes_from_period_start() {
        let entries = [
            day("2026-09-01", 900, 0),
            day("2026-09-02", 61, 2),
            day("2026-09-15", 3, 4),
            day("2026-09-16", 900, 0),
        ];
        let weeks =
            weekly_activity(range("2026-09-02", "2026-09-15"), Weekday::Mon, &entries, 3).unwrap();
        assert_eq!(
            weeks
                .iter()
                .map(|w| (
                    w.billable_minutes,
                    w.non_billable_minutes,
                    w.cumulative_minutes
                ))
                .collect::<Vec<_>>(),
            vec![(61, 2, 63), (0, 0, 63), (3, 4, 70)]
        );
    }

    #[test]
    fn configured_week_start_changes_buckets_at_year_boundary() {
        let entries = [
            day("2025-12-31", 1, 0),
            day("2026-01-03", 2, 0),
            day("2026-01-04", 4, 0),
        ];
        let weeks =
            weekly_activity(range("2025-12-31", "2026-01-04"), Weekday::Sun, &entries, 2).unwrap();
        assert_eq!(
            weeks
                .iter()
                .map(|w| (w.from, w.to, w.billable_minutes))
                .collect::<Vec<_>>(),
            vec![
                (date("2025-12-31"), date("2026-01-03"), 3),
                (date("2026-01-04"), date("2026-01-04"), 4)
            ]
        );
    }

    #[test]
    fn single_day_is_one_bucket_for_every_week_start() {
        for first_day in WEEKDAYS {
            let weeks = weekly_activity(
                range("2026-09-29", "2026-09-29"),
                first_day,
                &[day("2026-09-29", 2, 3)],
                1,
            )
            .unwrap();
            assert_eq!(
                weeks,
                vec![WeeklyActivity {
                    from: date("2026-09-29"),
                    to: date("2026-09-29"),
                    billable_minutes: 2,
                    non_billable_minutes: 3,
                    cumulative_minutes: 5
                }]
            );
        }
    }

    #[test]
    fn duplicate_dates_and_input_order_do_not_change_the_series() {
        let mut entries = [
            day("2026-09-01", 1, 2),
            day("2026-09-08", 3, 4),
            day("2026-09-01", 5, 6),
        ];
        let selected = range("2026-09-01", "2026-09-10");
        let first = weekly_activity(selected, Weekday::Mon, &entries, 2).unwrap();
        entries.reverse();
        assert_eq!(
            first,
            weekly_activity(selected, Weekday::Mon, &entries, 2).unwrap()
        );
        assert_eq!(first.last().unwrap().cumulative_minutes, 21);
    }

    #[test]
    fn refuses_to_truncate_a_period_to_fit_the_bucket_limit() {
        assert_eq!(
            weekly_activity(range("2026-09-01", "2026-09-30"), Weekday::Mon, &[], 4),
            Err(ActivityError::TooManyWeeks { limit: 4 })
        );
    }

    #[test]
    fn zero_bucket_limit_is_an_error_even_without_entries() {
        assert_eq!(
            weekly_activity(range("2026-09-01", "2026-09-01"), Weekday::Mon, &[], 0),
            Err(ActivityError::TooManyWeeks { limit: 0 })
        );
    }

    #[test]
    fn rejects_negative_billable_or_non_billable_minutes() {
        for entry in [day("2026-09-01", -1, 0), day("2026-09-01", 0, -1)] {
            assert_eq!(
                weekly_activity(range("2026-09-01", "2026-09-01"), Weekday::Mon, &[entry], 1),
                Err(ActivityError::NegativeMinutes)
            );
        }
    }

    #[test]
    fn negative_minutes_win_over_overflow_in_any_input_order() {
        let mut entries = [
            day("2026-09-01", i64::MAX, 0),
            day("2026-09-01", 1, 0),
            day("2026-09-01", -1, 0),
        ];
        let selected = range("2026-09-01", "2026-09-01");
        for _ in 0..2 {
            assert_eq!(
                weekly_activity(selected, Weekday::Mon, &entries, 1),
                Err(ActivityError::NegativeMinutes),
                "{entries:?}"
            );
            entries.reverse();
        }
    }

    #[test]
    fn ignores_negative_minutes_outside_the_period() {
        let entries = [
            day("2026-08-31", -1, -1),
            day("2026-09-01", 2, 3),
            day("2026-09-02", -1, 0),
        ];
        let weeks =
            weekly_activity(range("2026-09-01", "2026-09-01"), Weekday::Mon, &entries, 1).unwrap();
        assert_eq!(weeks[0].cumulative_minutes, 5);
    }

    #[test]
    fn rejects_overflow_within_a_bucket() {
        let entries = [day("2026-09-01", i64::MAX, 0), day("2026-09-01", 1, 0)];
        assert_eq!(
            weekly_activity(range("2026-09-01", "2026-09-01"), Weekday::Mon, &entries, 1),
            Err(ActivityError::Overflow)
        );
    }

    #[test]
    fn rejects_overflow_across_the_billable_split() {
        assert_eq!(
            weekly_activity(
                range("2026-09-01", "2026-09-01"),
                Weekday::Mon,
                &[day("2026-09-01", i64::MAX, 1)],
                1
            ),
            Err(ActivityError::Overflow)
        );
    }

    #[test]
    fn rejects_cumulative_overflow_across_weeks() {
        let entries = [day("2026-09-01", i64::MAX, 0), day("2026-09-08", 1, 0)];
        assert_eq!(
            weekly_activity(range("2026-09-01", "2026-09-08"), Weekday::Mon, &entries, 2),
            Err(ActivityError::Overflow)
        );
    }

    #[test]
    fn clips_the_final_week_before_date_overflow() {
        let last = NaiveDate::MAX;
        let weeks = weekly_activity(
            ActivityRange::new(last, last).unwrap(),
            Weekday::Mon,
            &[],
            1,
        )
        .unwrap();
        assert_eq!((weeks[0].from, weeks[0].to), (last, last));
    }

    #[test]
    fn rejects_an_unrepresentable_first_week() {
        let first = NaiveDate::MIN;
        assert_eq!(
            weekly_activity(
                ActivityRange::new(first, first).unwrap(),
                first.weekday().succ(),
                &[],
                1
            ),
            Err(ActivityError::DateOutOfRange)
        );
    }

    #[test]
    fn rejects_a_huge_range_before_allocating_buckets() {
        let selected = ActivityRange::new(NaiveDate::MIN, NaiveDate::MAX).unwrap();
        assert_eq!(
            weekly_activity(selected, NaiveDate::MIN.weekday(), &[], 1040),
            Err(ActivityError::TooManyWeeks { limit: 1040 })
        );
    }

    #[test]
    fn every_day_is_counted_once_for_each_week_start_in_full_calendar_years() {
        for year in [2000, 2024, 2026] {
            let selected =
                ActivityRange::year(NaiveDate::from_ymd_opt(year, 6, 15).unwrap()).unwrap();
            let entries = selected
                .from()
                .iter_days()
                .take_while(|date| *date <= selected.to())
                .enumerate()
                .map(|(index, date)| DailyActivity {
                    date,
                    billable_minutes: (index % 61) as i64,
                    non_billable_minutes: (index % 17) as i64,
                })
                .collect::<Vec<_>>();
            let expected_billable: i64 = entries.iter().map(|day| day.billable_minutes).sum();
            let expected_non_billable: i64 =
                entries.iter().map(|day| day.non_billable_minutes).sum();
            for first_day in WEEKDAYS {
                let weeks = weekly_activity(selected, first_day, &entries, 54).unwrap();
                assert_eq!(
                    weeks.iter().map(|week| week.billable_minutes).sum::<i64>(),
                    expected_billable
                );
                assert_eq!(
                    weeks
                        .iter()
                        .map(|week| week.non_billable_minutes)
                        .sum::<i64>(),
                    expected_non_billable
                );
                assert_eq!(
                    weeks.last().unwrap().cumulative_minutes,
                    expected_billable + expected_non_billable
                );
                assert_eq!(weeks.first().unwrap().from, selected.from());
                assert_eq!(weeks.last().unwrap().to, selected.to());
                for pair in weeks.windows(2) {
                    assert_eq!(pair[0].to.succ_opt(), Some(pair[1].from));
                    // Interior boundaries fall exactly on the configured week start.
                    assert_eq!(pair[1].from.weekday(), first_day);
                    assert_eq!(pair[0].to.weekday(), first_day.pred());
                }
                for week in &weeks {
                    assert!((0..7).contains(&(week.to - week.from).num_days()));
                }
            }
        }
    }

    #[test]
    fn accepts_the_largest_representable_total_without_saturation() {
        let selected = range("2026-09-01", "2026-09-08");
        let entries = [day("2026-09-01", i64::MAX - 1, 0), day("2026-09-08", 0, 1)];
        let weeks = weekly_activity(selected, Weekday::Mon, &entries, 2).unwrap();
        assert_eq!(weeks.last().unwrap().cumulative_minutes, i64::MAX);
    }
}
