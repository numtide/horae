//! The periodic sweeps: forgotten timers, budget alerts and the removal of
//! expired OAuth codes and tokens. Each organization has one schedule per
//! sweep, every minute, and each job sweeps its own organization.
//!
//! Unlike every other plugin event, `timer_running_too_long` is time-based:
//! nothing mutates when a timer simply keeps running, so it is found by polling
//! rather than by a post-write dispatch. Each overrun is announced at most once,
//! guarded by `time_entries.notified_long_running_at` (cleared on stop).

use horae_jobs::{
    BoxFuture, Cadence, Completed, JobContext, JobError, Registry, RetryPolicy, Schedule,
    ZoneSource,
};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::{
    plugin::{AppEvent, event::TimeEntryPayload},
    state::AppState,
};

/// Each sweep's kind (also its schedule's name), concurrency key and priority.
const SWEEPS: [(&str, &str, i16); 3] = [
    ("long_timer_sweep", "sweep:long_timer", 0),
    ("budget_alert_sweep", "sweep:budget", 0),
    ("oauth_cleanup", "sweep:oauth", -1),
];

/// One attempt: the next firing is the retry.
const POLICY: RetryPolicy = RetryPolicy {
    max_attempts: 1,
    first_delay: std::time::Duration::ZERO,
    multiplier: 1,
    max_delay: std::time::Duration::ZERO,
};

pub(super) fn register(registry: &mut Registry, state: &AppState) {
    for (kind, _, _) in SWEEPS {
        registry.register(Sweep {
            state: state.clone(),
            kind,
        });
    }
}

/// Stores the organization's sweep schedules. Storing them again keeps when
/// they next fire, so a restart does not add a firing.
pub async fn ensure_schedules(connection: &mut PgConnection, org_id: Uuid) -> anyhow::Result<()> {
    for (kind, concurrency_key, priority) in SWEEPS {
        horae_jobs::upsert_schedule(
            connection,
            Schedule {
                org_id,
                name: kind,
                kind,
                payload: serde_json::json!({}),
                cadence: Cadence::Every(60),
                zone: ZoneSource::Utc,
                concurrency_key: Some(concurrency_key),
                priority,
                max_attempts: POLICY.max_attempts,
            },
        )
        .await?;
    }
    Ok(())
}

/// [`ensure_schedules`] for every organization, at server start.
pub async fn ensure_all_schedules(pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let orgs = sqlx::query_scalar!("SELECT id FROM organizations")
        .fetch_all(&mut *tx)
        .await?;
    for org_id in orgs {
        ensure_schedules(&mut tx, org_id).await?;
    }
    tx.commit().await?;
    Ok(())
}

struct Sweep {
    state: AppState,
    kind: &'static str,
}

impl horae_jobs::Handler for Sweep {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn policy(&self) -> RetryPolicy {
        POLICY
    }

    fn run<'a>(
        &'a self,
        ctx: &'a JobContext,
        _payload: serde_json::Value,
    ) -> BoxFuture<'a, Result<Completed, JobError>> {
        Box::pin(async move {
            let (db, org_id) = (&self.state.db, ctx.org_id());
            let processed_count = match self.kind {
                "long_timer_sweep" => {
                    let events = claim_long_timers(db, org_id).await?;
                    let count = events.len();
                    for event in events {
                        self.state.plugins.dispatch(event);
                    }
                    count as i64
                }
                "budget_alert_sweep" => {
                    crate::notifications::budgets::sweep(db, org_id, chrono::Utc::now()).await?;
                    0
                }
                _ => crate::ops::oauth::delete_expired(db, org_id)
                    .await
                    .map_err(anyhow::Error::from)? as i64,
            };
            Ok(Completed {
                report: serde_json::json!({}),
                processed_count,
            })
        })
    }

    /// A sweep's run is worth keeping only when it failed: the succeeded ones,
    /// one a minute, go after a day.
    fn cleanup<'a>(&'a self, pool: &'a PgPool) -> BoxFuture<'a, anyhow::Result<()>> {
        Box::pin(async move {
            sqlx::query!(
                "DELETE FROM horae_jobs
                  WHERE kind = $1 AND status = 'succeeded' AND finished_at < now() - interval '1 day'",
                self.kind,
            )
            .execute(pool)
            .await?;
            Ok(())
        })
    }
}

/// Claims every running timer of the organization past its limit that has not
/// been announced yet, marking it so it is not announced again until it stops,
/// and returns the events announcing them.
async fn claim_long_timers(pool: &PgPool, org_id: Uuid) -> anyhow::Result<Vec<AppEvent>> {
    // Claim and return the due timers in one statement. Selecting first and
    // marking afterwards let two app instances — or one slow dispatch — both
    // see the same timer and announce it twice. Marking up front trades that
    // for the opposite failure: a crash between the UPDATE committing and the
    // dispatch loop finishing drops an announcement. For a "you left a timer
    // running" nudge, a missed one is clearly better than a duplicate.
    let rows = sqlx::query!(
        r#"WITH due AS (
             UPDATE time_entries te
                SET notified_long_running_at = now()
               FROM organizations o
              WHERE o.id = te.org_id
                AND te.org_id = $1
                AND te.is_running = true
                AND te.notified_long_running_at IS NULL
                AND te.started_at < now() - make_interval(mins => o.long_timer_minutes)
           RETURNING te.id, te.org_id, te.user_id, te.project_id, te.task_id,
                     te.spent_date, te.minutes, te.billable, te.notes, te.started_at
           )
           SELECT id, org_id, user_id, project_id, task_id,
                  spent_date as "spent_date: chrono::NaiveDate",
                  minutes, billable, notes,
                  started_at as "started_at!: chrono::DateTime<chrono::Utc>",
                  (EXTRACT(EPOCH FROM (now() - started_at)) / 60)::int as "running_minutes!"
           FROM due"#,
        org_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| AppEvent::TimerRunningTooLong {
            occurred_at: chrono::Utc::now(),
            org_id: r.org_id,
            running_minutes: r.running_minutes,
            time_entry: TimeEntryPayload {
                id: r.id,
                user_id: r.user_id,
                project_id: r.project_id,
                task_id: r.task_id,
                spent_date: r.spent_date,
                minutes: r.minutes,
                billable: r.billable,
                is_running: true,
                notes: r.notes,
                started_at: Some(r.started_at),
            },
        })
        .collect())
}

#[cfg(test)]
mod tests;
