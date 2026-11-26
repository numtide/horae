use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;

use super::{Cadence, next_due};

fn at(rfc3339: &str) -> DateTime<Utc> {
    rfc3339.parse().unwrap()
}

fn time(hour: u32, minute: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
}

const MADRID: Tz = chrono_tz::Europe::Madrid;

#[test]
fn every_n_seconds_lands_on_multiples_of_n_strictly_after() {
    let every = Cadence::Every(60);
    assert_eq!(
        next_due(&every, MADRID, at("2026-05-01T12:00:30Z")),
        at("2026-05-01T12:01:00Z")
    );
    assert_eq!(
        next_due(&every, MADRID, at("2026-05-01T12:01:00Z")),
        at("2026-05-01T12:02:00Z")
    );
    let every_hour = Cadence::Every(3600);
    assert_eq!(
        next_due(&every_hour, Tz::UTC, at("2026-05-01T12:00:00Z")),
        at("2026-05-01T13:00:00Z")
    );
}

#[test]
fn daily_keeps_the_local_hour_across_both_madrid_changes() {
    let nine = Cadence::Daily(time(9, 0));
    // CET (+1) the day before the spring change, CEST (+2) on it.
    assert_eq!(
        next_due(&nine, MADRID, at("2026-03-28T07:00:00Z")),
        at("2026-03-28T08:00:00Z")
    );
    assert_eq!(
        next_due(&nine, MADRID, at("2026-03-28T08:00:00Z")),
        at("2026-03-29T07:00:00Z")
    );
    // CEST the day before the autumn change, CET on it.
    assert_eq!(
        next_due(&nine, MADRID, at("2026-10-24T07:00:00Z")),
        at("2026-10-25T08:00:00Z")
    );
}

#[test]
fn a_skipped_local_time_fires_at_the_end_of_the_gap() {
    // On 2026-03-29 Madrid jumps from 02:00 to 03:00 CEST (01:00 UTC).
    let half_past_two = Cadence::Daily(time(2, 30));
    let due = next_due(&half_past_two, MADRID, at("2026-03-28T23:00:00Z"));
    assert_eq!(due, at("2026-03-29T01:00:00Z"));
    // And the day after, at 02:30 CEST again.
    assert_eq!(
        next_due(&half_past_two, MADRID, due),
        at("2026-03-30T00:30:00Z")
    );
}

#[test]
fn a_repeated_local_time_fires_once_at_its_first_occurrence() {
    // On 2026-10-25 Madrid goes back from 03:00 CEST to 02:00 CET: 02:30 is
    // 00:30 UTC and again 01:30 UTC.
    let half_past_two = Cadence::Daily(time(2, 30));
    let first = next_due(&half_past_two, MADRID, at("2026-10-24T22:00:00Z"));
    assert_eq!(first, at("2026-10-25T00:30:00Z"));
    // Planned from the first firing, or from between the two, it skips the second.
    assert_eq!(
        next_due(&half_past_two, MADRID, first),
        at("2026-10-26T01:30:00Z")
    );
    assert_eq!(
        next_due(&half_past_two, MADRID, at("2026-10-25T01:00:00Z")),
        at("2026-10-26T01:30:00Z")
    );
}

#[test]
fn weekly_monday_nine_in_new_york_across_its_spring_change() {
    let monday = Cadence::Weekly(Weekday::Mon, time(9, 0));
    let new_york = chrono_tz::America::New_York;
    // Wednesday 2026-02-25: the next Monday is 2026-03-02, still EST (-5).
    let first = next_due(&monday, new_york, at("2026-02-25T12:00:00Z"));
    assert_eq!(first, at("2026-03-02T14:00:00Z"));
    // DST starts on Sunday 2026-03-08, so the next Monday is EDT (-4).
    assert_eq!(
        next_due(&monday, new_york, first),
        at("2026-03-09T13:00:00Z")
    );
    // On a Monday after nine, the next is a week later.
    assert_eq!(
        next_due(&monday, new_york, at("2026-03-09T13:00:01Z")),
        at("2026-03-16T13:00:00Z")
    );
}

#[test]
fn a_persons_zone_gives_their_local_time() {
    // "17:00 in Ana's zone", for a person in Kolkata (+05:30, no DST).
    let five_pm = Cadence::Daily(time(17, 0));
    assert_eq!(
        next_due(
            &five_pm,
            chrono_tz::Asia::Kolkata,
            at("2026-05-01T12:00:00Z")
        ),
        at("2026-05-02T11:30:00Z")
    );
}
