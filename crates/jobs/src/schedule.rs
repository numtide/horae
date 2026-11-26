//! Stored schedules and when they are next due.

use chrono::{
    DateTime, Datelike, Days, NaiveDateTime, NaiveTime, TimeDelta, TimeZone, Timelike, Utc, Weekday,
};
use chrono_tz::Tz;
use sqlx::PgConnection;
use uuid::Uuid;

/// How often a schedule fires. Daily and weekly times are local to the
/// schedule's zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cadence {
    /// Every N seconds (60 to 86400), on multiples of N since the Unix epoch.
    Every(u32),
    Daily(NaiveTime),
    Weekly(Weekday, NaiveTime),
}

/// Whose time zone a daily or weekly time is read in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneSource {
    Utc,
    Organization,
    Person(Uuid),
}

/// A schedule to store, unique per organization by `name`.
pub struct Schedule<'a> {
    pub org_id: Uuid,
    pub name: &'a str,
    pub kind: &'a str,
    /// Stored in the versioned envelope, as for jobs.
    pub payload: serde_json::Value,
    pub cadence: Cadence,
    pub zone: ZoneSource,
    pub concurrency_key: Option<&'a str>,
    pub priority: i16,
    /// Usually the kind's `RetryPolicy::max_attempts`.
    pub max_attempts: i32,
}

/// The first firing of `cadence` in `zone` strictly after `after`.
///
/// A local time that a spring-forward change skips fires at the first instant
/// after the gap; one that an autumn change repeats fires once, at its first
/// occurrence.
pub fn next_due(cadence: &Cadence, zone: Tz, after: DateTime<Utc>) -> DateTime<Utc> {
    let (weekday, time) = match *cadence {
        Cadence::Every(secs) => {
            let secs = i64::from(secs.max(1));
            let next = (after.timestamp().div_euclid(secs) + 1) * secs;
            return DateTime::from_timestamp(next, 0).expect("a timestamp chrono can represent");
        }
        Cadence::Daily(time) => (None, time),
        Cadence::Weekly(weekday, time) => (Some(weekday), time),
    };
    let mut date = after.with_timezone(&zone).date_naive();
    loop {
        if weekday.is_none_or(|weekday| date.weekday() == weekday) {
            let due = earliest_at_or_after(zone, date.and_time(time));
            if due > after {
                return due;
            }
        }
        date = date + Days::new(1);
    }
}

/// The first instant whose local time in `zone` is `local` or, when `local`
/// falls in a gap, the first instant after the gap.
fn earliest_at_or_after(zone: Tz, local: NaiveDateTime) -> DateTime<Utc> {
    let mut candidate = local;
    // Gaps start and end on whole minutes; the longest on record skipped a day.
    for _ in 0..=2 * 24 * 60 {
        if let Some(instant) = zone.from_local_datetime(&candidate).earliest() {
            return instant.with_timezone(&Utc);
        }
        let minute = candidate.with_second(0).unwrap_or(candidate);
        candidate = minute + TimeDelta::minutes(1);
    }
    unreachable!("no valid local time within two days of {local} in {zone}")
}

/// Stores `schedule`, replacing the organization's schedule of the same name.
/// A new schedule, or one whose cadence or zone changed, is planned again by
/// the scheduler: an `Every` fires on its next tick, a daily or weekly one at
/// its next local time. Otherwise its next firing stays.
pub async fn upsert_schedule(
    connection: &mut PgConnection,
    schedule: Schedule<'_>,
) -> anyhow::Result<Uuid> {
    let (cadence, every_secs, local_time, weekday) = match schedule.cadence {
        Cadence::Every(secs) => ("every", Some(i32::try_from(secs)?), None, None),
        Cadence::Daily(time) => ("daily", None, Some(time), None),
        Cadence::Weekly(day, time) => (
            "weekly",
            None,
            Some(time),
            Some(day.number_from_monday() as i16),
        ),
    };
    let (zone_source, person_id) = match schedule.zone {
        ZoneSource::Utc => ("utc", None),
        ZoneSource::Organization => ("organization", None),
        ZoneSource::Person(id) => ("person", Some(id)),
    };
    let payload = crate::encode_payload(&schedule.payload)?;
    let id = sqlx::query_scalar!(
        r#"INSERT INTO horae_job_schedules
                (id, org_id, name, kind, payload, cadence, every_secs, local_time, weekday,
                 zone_source, person_id, concurrency_key, priority, max_attempts,
                 next_due_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                   CASE WHEN $6 = 'every' THEN now() END)
           ON CONFLICT (org_id, name) DO UPDATE
              SET kind = EXCLUDED.kind, payload = EXCLUDED.payload,
                  cadence = EXCLUDED.cadence, every_secs = EXCLUDED.every_secs,
                  local_time = EXCLUDED.local_time, weekday = EXCLUDED.weekday,
                  zone_source = EXCLUDED.zone_source, person_id = EXCLUDED.person_id,
                  concurrency_key = EXCLUDED.concurrency_key, priority = EXCLUDED.priority,
                  max_attempts = EXCLUDED.max_attempts,
                  next_due_at = CASE
                      WHEN (horae_job_schedules.cadence, horae_job_schedules.every_secs,
                            horae_job_schedules.local_time, horae_job_schedules.weekday,
                            horae_job_schedules.zone_source, horae_job_schedules.person_id)
                           IS NOT DISTINCT FROM
                           (EXCLUDED.cadence, EXCLUDED.every_secs, EXCLUDED.local_time,
                            EXCLUDED.weekday, EXCLUDED.zone_source, EXCLUDED.person_id)
                      THEN horae_job_schedules.next_due_at
                      ELSE EXCLUDED.next_due_at END
           RETURNING id"#,
        Uuid::now_v7(),
        schedule.org_id,
        schedule.name,
        schedule.kind,
        payload,
        cadence,
        every_secs,
        // Unchecked, as `enqueue_in`'s `run_at`: the `time` feature changes the expected type.
        local_time as _,
        weekday,
        zone_source,
        person_id,
        schedule.concurrency_key,
        schedule.priority,
        schedule.max_attempts,
    )
    .fetch_one(&mut *connection)
    .await?;
    Ok(id)
}

/// A disabled schedule keeps its row but never fires. Enabling one plans it
/// again, so the firings it missed while disabled are not made up.
pub async fn set_schedule_enabled(
    connection: &mut PgConnection,
    org_id: Uuid,
    name: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"UPDATE horae_job_schedules
              SET enabled = $3,
                  next_due_at = CASE WHEN enabled = $3 THEN next_due_at END
            WHERE org_id = $1 AND name = $2"#,
        org_id,
        name,
        enabled,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub async fn delete_schedule(
    connection: &mut PgConnection,
    org_id: Uuid,
    name: &str,
) -> anyhow::Result<()> {
    sqlx::query!(
        "DELETE FROM horae_job_schedules WHERE org_id = $1 AND name = $2",
        org_id,
        name,
    )
    .execute(&mut *connection)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
