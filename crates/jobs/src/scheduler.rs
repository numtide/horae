//! The scheduler: one leader among the server instances enqueues the jobs
//! that stored schedules are due for.
//!
//! Leadership is a session advisory lock held on a connection taken out of the
//! pool, so it ends when that connection closes, however the leader dies. A
//! firing is enqueued with the idempotency key `schedule:<id>:<due>`, so even
//! two leaders (one that lost its connection and does not know yet) enqueue
//! it once.

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use chrono::{DateTime, NaiveTime, Utc, Weekday};
use chrono_tz::Tz;
use sqlx::{PgConnection, PgPool};
use tokio::sync::watch;
use uuid::Uuid;

use crate::{
    BoxFuture, Enqueue, EnqueueError, Enqueued, Worker,
    schedule::{Cadence, next_due},
};

/// How often the leader looks for due schedules, and how far ahead: a firing
/// due within the next tick is enqueued now with its due time as `run_at`.
const TICK: Duration = Duration::from_secs(15);

/// The time zones of an organization and of its people.
pub trait ZoneResolver: Send + Sync {
    /// The person's zone when `person_id` is given, else the organization's.
    fn zone(&self, org_id: Uuid, person_id: Option<Uuid>) -> BoxFuture<'_, anyhow::Result<Tz>>;
}

pub struct Scheduler {
    worker: Worker,
    leader: watch::Receiver<bool>,
}

impl Scheduler {
    /// Whether this instance held the leader's lock at its last tick.
    pub fn is_leader(&self) -> bool {
        *self.leader.borrow()
    }

    pub fn stop_sender(&self) -> watch::Sender<bool> {
        self.worker.stop_sender()
    }

    /// Ends the loop and closes the lock's connection, so another instance
    /// takes over at its next tick.
    pub async fn shutdown(self, grace: Duration) -> anyhow::Result<()> {
        self.worker.shutdown(grace).await
    }
}

pub fn spawn_scheduler(pool: PgPool, zones: Arc<dyn ZoneResolver>) -> Scheduler {
    spawn_ticking(pool, zones, TICK)
}

pub(crate) fn spawn_ticking(
    pool: PgPool,
    zones: Arc<dyn ZoneResolver>,
    tick: Duration,
) -> Scheduler {
    let (stop, mut stopped) = watch::channel(false);
    let (leading, leader) = watch::channel(false);
    let task = tokio::spawn(async move {
        let mut lock: Option<PgConnection> = None;
        let mut leads = false;
        while !*stopped.borrow() && stopped.has_changed().is_ok() {
            leads = lead(&pool, &mut lock, leads).await;
            leading.send_replace(leads);
            if leads && let Err(error) = fire_due(&pool, zones.as_ref(), Utc::now()).await {
                tracing::warn!(%error, "job scheduler tick failed");
            }
            tokio::select! {
                _ = stopped.changed() => break,
                _ = tokio::time::sleep(tick) => {}
            }
        }
        if let Some(connection) = lock {
            // Closing the session releases the lock at once.
            let _ = sqlx::Connection::close(connection).await;
        }
    });
    Scheduler {
        worker: Worker::new(stop, task),
        leader,
    }
}

/// Whether this instance leads after this tick. `lock` is the dedicated
/// connection: the leader checks it still answers, a follower asks for the
/// lock on it again. A connection that fails, or does not answer within a
/// tick, is dropped, ending its session.
async fn lead(pool: &PgPool, lock: &mut Option<PgConnection>, leading: bool) -> bool {
    if lock.is_none() {
        match tokio::time::timeout(TICK, connect(pool)).await {
            Ok(Ok(connection)) => *lock = Some(connection),
            Ok(Err(error)) => {
                tracing::warn!(%error, "job scheduler could not connect");
                return false;
            }
            Err(_) => {
                tracing::warn!("job scheduler timed out connecting");
                return false;
            }
        }
    }
    let connection = lock.as_mut().expect("connected above");
    // Nothing releases the lock but the session's end, so a leader whose
    // connection answers still holds it.
    let check = async {
        if leading {
            sqlx::query!("SELECT 1 AS one")
                .fetch_one(&mut *connection)
                .await
                .map(|_| true)
        } else {
            sqlx::query_scalar!(
                r#"SELECT pg_try_advisory_lock(hashtextextended('horae.job-scheduler', 0)) AS "locked!""#
            )
            .fetch_one(&mut *connection)
            .await
        }
    };
    match tokio::time::timeout(TICK, check).await {
        Ok(Ok(leads)) => leads,
        Ok(Err(error)) => {
            tracing::warn!(%error, "job scheduler lost its connection");
            *lock = None;
            false
        }
        Err(_) => {
            // A half-open connection blocks instead of failing.
            tracing::warn!("job scheduler connection stopped answering");
            *lock = None;
            false
        }
    }
}

/// A connection of the lock's own, outside the pool. Its keepalives let
/// PostgreSQL end the session, and so release the lock, within about a minute
/// of its peer vanishing, instead of the two hours of the default settings.
async fn connect(pool: &PgPool) -> sqlx::Result<PgConnection> {
    let options = pool.connect_options().as_ref().clone().options([
        ("tcp_keepalives_idle", "30"),
        ("tcp_keepalives_interval", "10"),
        ("tcp_keepalives_count", "3"),
    ]);
    sqlx::ConnectOptions::connect(&options).await
}

/// Enqueues every enabled schedule due by `now` plus a tick, each once, and
/// plans its next firing after `now`: a schedule that missed several slots
/// while no instance led fires once. A schedule not planned yet is planned
/// without firing. Returns the jobs it created.
pub async fn fire_due(
    pool: &PgPool,
    zones: &dyn ZoneResolver,
    now: DateTime<Utc>,
) -> anyhow::Result<usize> {
    let until = now + TICK;
    let mut fired = 0;
    // A schedule that failed is left for the next tick instead of being
    // picked again in this one.
    let mut failed: Vec<Uuid> = Vec::new();
    loop {
        let mut tx = pool.begin().await?;
        let Some(row) = sqlx::query!(
            // Typed explicitly: where another crate enables sqlx's `time`
            // feature, the macro would infer its types instead of chrono's.
            r#"SELECT id, org_id, kind, payload, cadence, every_secs,
                      local_time AS "local_time: NaiveTime", weekday,
                      zone_source, person_id, concurrency_key, priority, max_attempts,
                      next_due_at AS "next_due_at: DateTime<Utc>"
                 FROM horae_job_schedules
                WHERE enabled AND (next_due_at IS NULL OR next_due_at <= $1)
                  AND id <> ALL($2)
                ORDER BY next_due_at NULLS FIRST
                LIMIT 1
                  FOR UPDATE SKIP LOCKED"#,
            until as _,
            &failed,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(fired);
        };
        let id = row.id;
        let firing = async {
            let cadence = cadence(&row.cadence, row.every_secs, row.local_time, row.weekday)?;
            let zone = match (cadence, row.zone_source.as_str()) {
                (Cadence::Every(_), _) | (_, "utc") => Tz::UTC,
                _ => zones.zone(row.org_id, row.person_id).await?,
            };
            let next = match row.next_due_at {
                Some(due) => {
                    let key = format!("schedule:{id}:{}", due.to_rfc3339());
                    let enqueued = crate::enqueue_in(
                        &mut tx,
                        Enqueue {
                            org_id: row.org_id,
                            kind: &row.kind,
                            payload: crate::decode_payload(row.payload)?,
                            idempotency_key: Some(&key),
                            concurrency_key: row.concurrency_key.as_deref(),
                            run_at: Some(due),
                            priority: row.priority,
                            max_attempts: row.max_attempts,
                        },
                    )
                    .await;
                    match enqueued {
                        Ok(Enqueued::Created(_)) => fired += 1,
                        // Enqueued before, or folded into the job holding its concurrency key.
                        Ok(Enqueued::Existing(_)) => {}
                        // The schedule's payload changed after this firing was enqueued.
                        Err(EnqueueError::Conflict) => {
                            tracing::warn!(schedule_id = %id, "scheduled firing already enqueued with another payload");
                        }
                        Err(error) => return Err(error.into()),
                    }
                    next_due(&cadence, zone, due.max(now))
                }
                None => next_due(&cadence, zone, now),
            };
            sqlx::query!(
                r#"UPDATE horae_job_schedules
                      SET next_due_at = $2,
                          last_fired_at = CASE WHEN next_due_at IS NULL THEN last_fired_at ELSE $3 END
                    WHERE id = $1"#,
                id,
                next as _,
                now as _,
            )
            .execute(&mut *tx)
            .await?;
            anyhow::Ok(())
        };
        match firing.await {
            Ok(()) => tx.commit().await?,
            Err(error) => {
                tracing::warn!(schedule_id = %id, %error, "scheduled firing failed");
                failed.push(id);
            }
        }
    }
}

fn cadence(
    cadence: &str,
    every_secs: Option<i32>,
    local_time: Option<NaiveTime>,
    weekday: Option<i16>,
) -> anyhow::Result<Cadence> {
    Ok(match (cadence, every_secs, local_time, weekday) {
        ("every", Some(secs), _, _) => Cadence::Every(u32::try_from(secs)?),
        ("daily", _, Some(time), _) => Cadence::Daily(time),
        ("weekly", _, Some(time), Some(day)) => Cadence::Weekly(
            Weekday::try_from(u8::try_from(day - 1)?).context("weekday out of range")?,
            time,
        ),
        _ => anyhow::bail!("invalid schedule cadence {cadence}"),
    })
}

#[cfg(test)]
mod tests;
