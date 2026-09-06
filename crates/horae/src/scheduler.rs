//! Periodic detection of forgotten (long-running) timers.
//!
//! Unlike every other plugin event, `timer_running_too_long` is time-based:
//! nothing mutates when a timer simply keeps running, so it is found by polling
//! rather than by a post-write dispatch. Each overrun is announced at most once,
//! guarded by `time_entries.notified_long_running_at` (cleared on stop).

use crate::state::AppState;

const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Spawn the background poller. Call once at server startup.
pub fn spawn(state: &'static AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep(state).await {
                tracing::warn!("long-timer scheduler tick failed: {e}");
            }
        }
    });
}

/// One poll: claim every running timer past its org's limit that has not been
/// announced yet — marking it so it is not announced again until it stops — and
/// announce each claimed one.
async fn sweep(state: &AppState) -> anyhow::Result<()> {
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
    )
    .fetch_all(&state.db)
    .await?;

    for r in rows {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TimerRunningTooLong {
                occurred_at: chrono::Utc::now(),
                org_id: r.org_id,
                running_minutes: r.running_minutes,
                time_entry: crate::plugin::event::TimeEntryPayload {
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
            });
    }
    Ok(())
}
