//! Configured budget alerts: which scopes reached their threshold, and the
//! notifications and mail jobs that announce them. Separate from the legacy
//! lifetime plugin bands.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::ops::projects::budgets::fetch_progress;

/// The stored catalog version the raw-SQL recipient checks accept.
pub(crate) const CATALOG_VERSION: i32 =
    horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION as i32;

/// Atomically record each reached scope/period/threshold/recipient and its
/// pending outbox delivery. No private rates or addresses enter the payload.
///
/// This is a system check with no acting person, so it stays outside `ops/`.
/// Recipients are chosen from their stored permission state: everyone who can
/// read every project, and the project's designated managers who can read the
/// projects they manage. An amount budget's progress is valued at billable
/// rates, so its alerts also need the billable-rate read the budget readers
/// ask for. The mail worker repeats the check under its locks before sending,
/// so a revocation racing this choice still withholds mail.
pub(crate) async fn enqueue_alerts(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    project_id: Uuid,
    date: NaiveDate,
) -> anyhow::Result<usize> {
    let mut tx = pool.begin().await?;
    let Some(settings) = sqlx::query!(
        "SELECT ps.alert_threshold FROM project_settings ps
         JOIN projects p ON p.id = ps.project_id AND p.org_id = ps.org_id
         WHERE ps.org_id = $1 AND ps.project_id = $2 AND ps.alert_enabled AND p.active
         FOR SHARE OF ps, p",
        org_id,
        project_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(0);
    };
    let reached: Vec<_> = configured_progress(&mut tx, org_id, project_id, date)
        .await?
        .into_iter()
        .filter(|row| {
            horae_core::budget::alert_threshold_reached(
                row.consumed,
                row.budget,
                i32::from(settings.alert_threshold),
            )
        })
        .collect();
    if reached.is_empty() {
        return Ok(0);
    }
    // Keep in step with the eligibility check in `notifications::preparation`;
    // a test in `notifications::tests` runs one matrix through both. State from
    // another catalog version reads as no grants, as it does for the readers.
    let recipients = sqlx::query_scalar!(
        "SELECT u.id FROM users u
         JOIN person_permission_states s ON s.org_id = u.org_id AND s.user_id = u.id
         JOIN projects p ON p.org_id = u.org_id AND p.id = $2
         WHERE u.org_id = $1 AND u.active AND s.catalog_version = $3
           AND ('project_read_all' = ANY(s.grants)
             OR ('project_read_managed' = ANY(s.grants) AND EXISTS (
               SELECT 1 FROM project_management_assignments m
               WHERE m.org_id = u.org_id AND m.project_id = p.id AND m.manager_id = u.id)))
           AND (p.budget_kind <> 'amount'
             OR 'billable_rate_read_all' = ANY(s.grants)
             OR ('billable_rate_read_managed' = ANY(s.grants) AND EXISTS (
               SELECT 1 FROM project_management_assignments m
               WHERE m.org_id = u.org_id AND m.project_id = p.id AND m.manager_id = u.id)))
         ORDER BY u.id LIMIT 1001 FOR SHARE OF u",
        org_id,
        project_id,
        CATALOG_VERSION,
    )
    .fetch_all(&mut *tx)
    .await?;
    anyhow::ensure!(
        recipients.len() <= 1000,
        "budget alert recipient limit exceeded"
    );
    let mut enqueued = 0;
    for row in reached {
        let notification_ids: Vec<_> = recipients.iter().map(|_| Uuid::now_v7()).collect();
        let event_ids: Vec<_> = recipients.iter().map(|_| Uuid::now_v7()).collect();
        // Batch recipients per scope so large teams do not require one database
        // round trip per notification. Only newly inserted identities get jobs.
        let result = sqlx::query!(
            "WITH candidates AS (
               SELECT * FROM unnest($1::uuid[], $2::uuid[], $3::uuid[])
                 AS c(notification_id, event_id, recipient_id)
             ), inserted AS (
               INSERT INTO project_budget_notifications
                 (id, org_id, project_id, task_id, user_id, recipient_id, period_key, threshold)
               SELECT notification_id, $4, $5, $6, $7, recipient_id, $8, $9 FROM candidates
               ON CONFLICT DO NOTHING RETURNING id
             )
             INSERT INTO horae_outbox (id, org_id, event_kind, payload)
             SELECT c.event_id, $4, 'budget_email', jsonb_build_object('notification_id', i.id)
             FROM inserted i JOIN candidates c ON c.notification_id = i.id",
            &notification_ids,
            &event_ids,
            &recipients,
            org_id,
            project_id,
            row.task_id,
            row.user_id,
            row.period_key,
            settings.alert_threshold,
        )
        .execute(&mut *tx)
        .await?;
        enqueued += usize::try_from(result.rows_affected())?;
    }
    tx.commit().await?;
    Ok(enqueued)
}

/// Recover checks missed between a committed mutation and its best-effort task,
/// and evaluate a fresh monthly period even when no entry was edited.
/// Check every alerting project, each in its own workspace's period at `now`.
pub(crate) async fn sweep(pool: &sqlx::PgPool, now: DateTime<Utc>) -> anyhow::Result<()> {
    let mut after = Uuid::nil();
    loop {
        let projects = sqlx::query!(
            "SELECT ps.project_id, ps.org_id, o.timezone FROM project_settings ps
             JOIN projects p ON p.id = ps.project_id AND p.org_id = ps.org_id
             JOIN organizations o ON o.id = ps.org_id
             WHERE ps.alert_enabled AND p.active AND ps.project_id > $1
             ORDER BY ps.project_id LIMIT 100",
            after,
        )
        .fetch_all(pool)
        .await?;
        if projects.is_empty() {
            return Ok(());
        }
        for project in projects {
            after = project.project_id;
            let date = crate::timezone::today_in(&project.timezone, now);
            if enqueue_alerts(pool, project.org_id, project.project_id, date)
                .await
                .is_err()
            {
                tracing::warn!(project_id = %project.project_id, "budget alert check failed");
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct BudgetProgress {
    pub task_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub period_key: String,
    pub consumed: i64,
    pub budget: i64,
}

/// Evaluate one configured project's selected period without changing lifetime
/// tracking totals. Task/person budgets remain independent, including empty ones.
pub(crate) async fn configured_progress(
    connection: &mut PgConnection,
    org_id: Uuid,
    project_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<BudgetProgress>, sqlx::Error> {
    Ok(fetch_progress(
        connection,
        org_id,
        Some(project_id),
        None,
        date,
        None,
        false,
    )
    .await?
    .into_iter()
    .filter(|row| row.kind != horae_core::types::BudgetKind::None)
    .filter_map(|row| {
        Some(BudgetProgress {
            task_id: row.task_id,
            user_id: row.user_id,
            period_key: row.period_key,
            consumed: row.consumed,
            budget: row.budget?,
        })
    })
    .collect())
}
