use chrono::{Datelike, NaiveDate, Weekday};

/// Monday of the ISO week containing `date`.
pub fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64)
}

/// First day of the configured week containing `date`, or `None` if it would
/// precede the representable date range.
pub fn week_start(date: NaiveDate, first_day: Weekday) -> Option<NaiveDate> {
    let offset = (date.weekday().num_days_from_monday() + 7 - first_day.num_days_from_monday()) % 7;
    date.checked_sub_days(chrono::Days::new(offset.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_any_weekday_to_its_monday() {
        let monday = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        let thursday = NaiveDate::from_ymd_opt(2026, 1, 8).unwrap();
        assert_eq!(iso_week_monday(thursday), monday);
        assert_eq!(iso_week_monday(monday), monday);
    }

    #[test]
    fn configured_week_contains_each_day_for_all_seven_start_days() {
        let anchor = NaiveDate::from_ymd_opt(2026, 9, 6).unwrap();
        for start_offset in 0..7 {
            let start = anchor + chrono::Duration::days(start_offset);
            for day_offset in 0..7 {
                let date = start + chrono::Duration::days(day_offset);
                assert_eq!(week_start(date, start.weekday()), Some(start));
            }
        }
    }

    #[test]
    fn configured_week_can_start_in_the_previous_year() {
        assert_eq!(
            week_start("2026-01-01".parse().unwrap(), Weekday::Sun),
            Some("2025-12-28".parse().unwrap())
        );
    }

    #[test]
    fn configured_week_reports_an_unrepresentable_start() {
        assert_eq!(
            week_start(NaiveDate::MIN, NaiveDate::MIN.weekday().succ()),
            None
        );
    }
}
