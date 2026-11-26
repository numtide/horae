use std::{sync::Arc, time::Duration};

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use chrono_tz::Tz;
use uuid::Uuid;

use super::{ZoneResolver, fire_due, spawn_ticking};
use crate::{BoxFuture, Cadence, Schedule, ZoneSource, set_schedule_enabled, upsert_schedule};

/// Madrid for the organization, Kolkata for any person.
struct Zones;

impl ZoneResolver for Zones {
    fn zone(&self, _org_id: Uuid, person_id: Option<Uuid>) -> BoxFuture<'_, anyhow::Result<Tz>> {
        Box::pin(async move {
            Ok(match person_id {
                Some(_) => chrono_tz::Asia::Kolkata,
                None => chrono_tz::Europe::Madrid,
            })
        })
    }
}

async fn org(pool: &sqlx::PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Scheduler test')",
        id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn schedule(pool: &sqlx::PgPool, org_id: Uuid, cadence: Cadence, zone: ZoneSource) -> Uuid {
    let mut connection = pool.acquire().await.unwrap();
    upsert_schedule(
        &mut connection,
        Schedule {
            org_id,
            name: "sweep",
            kind: "sweep",
            payload: serde_json::json!({}),
            cadence,
            zone,
            concurrency_key: None,
            priority: -1,
            max_attempts: 1,
        },
    )
    .await
    .unwrap()
}

async fn next_due_at(pool: &sqlx::PgPool, id: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar!(
        r#"SELECT next_due_at AS "next_due_at: DateTime<Utc>" FROM horae_job_schedules WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn set_next_due_at(pool: &sqlx::PgPool, id: Uuid, at: DateTime<Utc>) {
    sqlx::query!(
        "UPDATE horae_job_schedules SET next_due_at = $2 WHERE id = $1",
        id,
        at as _
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn jobs(pool: &sqlx::PgPool) -> Vec<(String, DateTime<Utc>, i16, i32)> {
    sqlx::query!(
        r#"SELECT idempotency_key AS "key!", available_at AS "available_at: DateTime<Utc>", priority, max_attempts
             FROM horae_jobs ORDER BY created_at"#
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.key, row.available_at, row.priority, row.max_attempts))
    .collect()
}

async fn until(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("condition within 10 s");
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn one_of_two_schedulers_leads_and_the_other_takes_over_when_it_dies(pool: sqlx::PgPool) {
    let tick = Duration::from_millis(50);
    let first = spawn_ticking(pool.clone(), Arc::new(Zones), tick);
    let second = spawn_ticking(pool.clone(), Arc::new(Zones), tick);
    until(|| first.is_leader() || second.is_leader()).await;
    for _ in 0..10 {
        tokio::time::sleep(tick).await;
        assert!(
            first.is_leader() != second.is_leader(),
            "exactly one leader"
        );
    }
    let (leader, follower) = if first.is_leader() {
        (first, second)
    } else {
        (second, first)
    };

    // Dropping aborts the task without closing its connection, as a crash would.
    drop(leader);
    until(|| follower.is_leader()).await;

    // A graceful stop hands over too.
    let third = spawn_ticking(pool.clone(), Arc::new(Zones), tick);
    tokio::time::sleep(tick * 4).await;
    assert!(!third.is_leader());
    follower.shutdown(Duration::from_secs(5)).await.unwrap();
    until(|| third.is_leader()).await;
    third.shutdown(Duration::from_secs(5)).await.unwrap();
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn two_schedulers_enqueue_each_firing_once(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = schedule(&pool, org_id, Cadence::Every(60), ZoneSource::Utc).await;
    // Mid-minute, so the next slot is beyond the lookahead.
    let now: DateTime<Utc> = "2026-05-01T12:00:30Z".parse().unwrap();
    set_next_due_at(&pool, id, "2026-05-01T12:00:00Z".parse().unwrap()).await;
    let (a, b) = tokio::join!(fire_due(&pool, &Zones, now), fire_due(&pool, &Zones, now));
    assert_eq!(a.unwrap() + b.unwrap(), 1);
    assert_eq!(jobs(&pool).await.len(), 1);
    assert!(next_due_at(&pool, id).await.unwrap() > now);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_firing_missed_for_three_slots_enqueues_one_job(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = schedule(&pool, org_id, Cadence::Every(60), ZoneSource::Utc).await;
    let now: DateTime<Utc> = "2026-05-01T12:00:30Z".parse().unwrap();
    let missed: DateTime<Utc> = "2026-05-01T11:57:00Z".parse().unwrap();
    set_next_due_at(&pool, id, missed).await;

    assert_eq!(fire_due(&pool, &Zones, now).await.unwrap(), 1);
    let jobs = jobs(&pool).await;
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0],
        (
            format!("schedule:{id}:{}", missed.to_rfc3339()),
            missed,
            -1,
            1
        )
    );
    assert_eq!(
        next_due_at(&pool, id).await,
        Some("2026-05-01T12:01:00Z".parse().unwrap())
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_firing_is_idempotent_by_schedule_and_due_time(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = schedule(&pool, org_id, Cadence::Every(60), ZoneSource::Utc).await;
    let due: DateTime<Utc> = "2026-05-01T12:00:00Z".parse().unwrap();
    let now = due + TimeDelta::seconds(5);
    set_next_due_at(&pool, id, due).await;
    fire_due(&pool, &Zones, now).await.unwrap();
    // A second leader that read the same row before the first advanced it.
    set_next_due_at(&pool, id, due).await;
    fire_due(&pool, &Zones, now).await.unwrap();
    assert_eq!(jobs(&pool).await.len(), 1);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_firing_due_within_the_next_tick_waits_for_its_time(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = schedule(&pool, org_id, Cadence::Every(60), ZoneSource::Utc).await;
    let due: DateTime<Utc> = "2026-05-01T12:01:00Z".parse().unwrap();
    set_next_due_at(&pool, id, due).await;
    let now = due - TimeDelta::seconds(10);
    assert_eq!(fire_due(&pool, &Zones, now).await.unwrap(), 1);
    assert_eq!(jobs(&pool).await[0].1, due);
    assert_eq!(
        next_due_at(&pool, id).await,
        Some(due + TimeDelta::seconds(60))
    );
    // Further ahead than a tick, nothing fires.
    assert_eq!(fire_due(&pool, &Zones, now).await.unwrap(), 0);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_daily_schedule_is_planned_in_its_zone_before_it_fires(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let nine = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
    let id = schedule(
        &pool,
        org_id,
        Cadence::Daily(nine),
        ZoneSource::Organization,
    )
    .await;
    assert_eq!(next_due_at(&pool, id).await, None);

    // Planned, not fired: 09:00 in Madrid (CEST) is 07:00 UTC.
    let now: DateTime<Utc> = "2026-05-01T08:00:00Z".parse().unwrap();
    assert_eq!(fire_due(&pool, &Zones, now).await.unwrap(), 0);
    assert_eq!(
        next_due_at(&pool, id).await,
        Some("2026-05-02T07:00:00Z".parse().unwrap())
    );
    assert!(jobs(&pool).await.is_empty());

    // Storing it again unchanged keeps its plan; a new time replans it.
    schedule(
        &pool,
        org_id,
        Cadence::Daily(nine),
        ZoneSource::Organization,
    )
    .await;
    assert!(next_due_at(&pool, id).await.is_some());
    let ten = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
    schedule(&pool, org_id, Cadence::Daily(ten), ZoneSource::Organization).await;
    assert_eq!(next_due_at(&pool, id).await, None);
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_persons_schedule_fires_in_their_zone(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let person = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'ana@example.com', 'Ana')",
        person,
        org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let five_pm = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
    let id = schedule(
        &pool,
        org_id,
        Cadence::Daily(five_pm),
        ZoneSource::Person(person),
    )
    .await;
    let now: DateTime<Utc> = "2026-05-01T12:00:00Z".parse().unwrap();
    fire_due(&pool, &Zones, now).await.unwrap();
    // 17:00 in Kolkata (+05:30).
    assert_eq!(
        next_due_at(&pool, id).await,
        Some("2026-05-02T11:30:00Z".parse().unwrap())
    );
}

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn a_disabled_schedule_never_fires_and_skips_what_it_missed(pool: sqlx::PgPool) {
    let org_id = org(&pool).await;
    let id = schedule(&pool, org_id, Cadence::Every(60), ZoneSource::Utc).await;
    let mut connection = pool.acquire().await.unwrap();
    set_schedule_enabled(&mut connection, org_id, "sweep", false)
        .await
        .unwrap();
    assert_eq!(fire_due(&pool, &Zones, Utc::now()).await.unwrap(), 0);

    set_schedule_enabled(&mut connection, org_id, "sweep", true)
        .await
        .unwrap();
    assert_eq!(next_due_at(&pool, id).await, None);
    // Mid-minute, so the slot it is planned for is beyond the lookahead.
    let now: DateTime<Utc> = "2026-05-01T12:00:30Z".parse().unwrap();
    assert_eq!(fire_due(&pool, &Zones, now).await.unwrap(), 0);
    assert!(jobs(&pool).await.is_empty());
}
