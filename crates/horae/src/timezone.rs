//! What "today" is for a person or an organization: the calendar date in their
//! IANA zone. A person without a zone of their own follows their organization's.

use chrono::{DateTime, NaiveDate, Offset, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

/// The zone database's name for `name`, if it knows it.
pub(crate) fn parse(name: &str) -> Option<Tz> {
    name.parse().ok()
}

/// The zone names the pickers offer: `Area/City` zones and `UTC`, in the
/// database's order. Legacy aliases (`US/Pacific`, `EST5EDT`) and the
/// sign-inverted `Etc/GMT±N` are left out, though [`parse`] still accepts them.
///
/// shortcut: chrono-tz does not mark backward links, so renamed cities such as
/// `Asia/Calcutta` are still listed; filter them if people pick them by mistake.
pub(crate) fn names() -> Vec<String> {
    const AREAS: [&str; 9] = [
        "Africa/",
        "America/",
        "Antarctica/",
        "Asia/",
        "Atlantic/",
        "Australia/",
        "Europe/",
        "Indian/",
        "Pacific/",
    ];
    chrono_tz::TZ_VARIANTS
        .iter()
        .map(|zone| zone.name())
        .filter(|name| *name == "UTC" || AREAS.iter().any(|area| name.starts_with(area)))
        .map(str::to_owned)
        .collect()
}

/// The calendar date at `now` in `zone`. Stored names were validated on write,
/// so one that no longer parses (removed from a newer zone database) reads as UTC.
pub(crate) fn today_in(zone: &str, now: DateTime<Utc>) -> NaiveDate {
    parse(zone).map_or(now.date_naive(), |tz| now.with_timezone(&tz).date_naive())
}

/// Seconds `zone` is ahead of UTC at `now`, which the browser adds to its own
/// clock so it needs no zone database.
pub(crate) fn utc_offset_seconds(zone: &str, now: DateTime<Utc>) -> i32 {
    parse(zone).map_or(0, |tz| {
        now.with_timezone(&tz).offset().fix().local_minus_utc()
    })
}

/// The zone that decides `user_id`'s today: their own, else their organization's.
pub(crate) async fn person_zone(
    executor: impl sqlx::PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<String> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(u.timezone, o.timezone) AS "zone!"
           FROM users u JOIN organizations o ON o.id = u.org_id WHERE u.id = $1"#,
        user_id
    )
    .fetch_one(executor)
    .await
}

/// `user_id`'s today, in [`person_zone`].
pub(crate) async fn today_for_person(
    executor: impl sqlx::PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<NaiveDate> {
    Ok(today_in(&person_zone(executor, user_id).await?, Utc::now()))
}

/// The organization's today, for workspace-wide periods such as budget months.
pub(crate) async fn today_for_org(
    executor: impl sqlx::PgExecutor<'_>,
    org_id: Uuid,
) -> sqlx::Result<NaiveDate> {
    let zone = sqlx::query_scalar!("SELECT timezone FROM organizations WHERE id = $1", org_id)
        .fetch_one(executor)
        .await?;
    Ok(today_in(&zone, Utc::now()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(instant: &str) -> DateTime<Utc> {
        instant.parse().unwrap()
    }

    #[test]
    fn late_evening_in_los_angeles_stays_on_its_own_day() {
        // 23:30 on 7 Sep in Los Angeles (UTC−7 in summer) is 06:30 on 8 Sep UTC.
        let now = at("2026-09-08T06:30:00Z");
        assert_eq!(
            today_in("America/Los_Angeles", now),
            "2026-09-07".parse::<NaiveDate>().unwrap()
        );
    }

    #[test]
    fn just_after_midnight_in_utc_minus_8_is_the_new_day() {
        // 00:10 on 8 Jan at UTC−8 (Pacific winter time) is 08:10 UTC.
        let now = at("2026-01-08T08:10:00Z");
        assert_eq!(
            today_in("America/Los_Angeles", now),
            "2026-01-08".parse::<NaiveDate>().unwrap()
        );
        assert_eq!(utc_offset_seconds("America/Los_Angeles", now), -8 * 3600);
    }

    #[test]
    fn just_after_midnight_in_utc_plus_2_is_ahead_of_utc() {
        // 00:30 on 8 Sep in Madrid (UTC+2 in summer) is 22:30 on 7 Sep UTC.
        let now = at("2026-09-07T22:30:00Z");
        assert_eq!(
            today_in("Europe/Madrid", now),
            "2026-09-08".parse::<NaiveDate>().unwrap()
        );
        assert_eq!(now.date_naive(), "2026-09-07".parse::<NaiveDate>().unwrap());
        assert_eq!(utc_offset_seconds("Europe/Madrid", now), 2 * 3600);
    }

    #[test]
    fn unknown_zone_reads_as_utc() {
        let now = at("2026-09-07T22:30:00Z");
        assert_eq!(today_in("Mars/Olympus_Mons", now), now.date_naive());
        assert_eq!(utc_offset_seconds("Mars/Olympus_Mons", now), 0);
    }

    #[test]
    fn parse_accepts_only_zone_database_names() {
        assert!(parse("Europe/Madrid").is_some());
        assert!(parse("UTC").is_some());
        assert!(parse("europe/madrid").is_none());
        assert!(parse("+02:00").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn the_picker_offers_cities_and_utc_but_any_name_still_parses() {
        let names = names();
        for offered in ["Europe/Madrid", "America/New_York", "UTC"] {
            assert!(names.iter().any(|name| name == offered), "{offered}");
        }
        for legacy in ["US/Pacific", "EST5EDT", "GMT0", "Etc/GMT+5"] {
            assert!(!names.iter().any(|name| name == legacy), "{legacy}");
            assert!(parse(legacy).is_some(), "{legacy} must stay valid");
        }
    }
}
