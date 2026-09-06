use chrono::{Datelike, NaiveDate};

/// Monday of the ISO week containing `date` — Horae's weeks start on Monday.
pub fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    date - chrono::Duration::days(date.weekday().num_days_from_monday() as i64)
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
}
