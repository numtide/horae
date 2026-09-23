//! Configured budgets, separate from the legacy lifetime plugin bands.

use chrono::NaiveDate;
use sqlx::PgConnection;
use uuid::Uuid;

/// Atomically record each reached scope/period/threshold/recipient and its
/// pending outbox delivery. No private rates or addresses enter the payload.
pub(crate) async fn enqueue_alerts(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    project_id: Uuid,
    date: NaiveDate,
) -> anyhow::Result<usize> {
    let mut tx = pool.begin().await?;
    let Some(settings) = sqlx::query!(
        "SELECT ps.creator_id, ps.alert_threshold FROM project_settings ps
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
    let recipients = sqlx::query_scalar!(
        "SELECT u.id FROM users u WHERE u.org_id = $1 AND u.active
         AND (u.org_role IN ('admin', 'manager') OR EXISTS (
           SELECT 1 FROM assignments a WHERE a.project_id = $2 AND a.user_id = u.id
           AND (a.role IN ('lead', 'admin') OR u.id = $3)))
         ORDER BY u.id LIMIT 1001 FOR SHARE OF u",
        org_id,
        project_id,
        settings.creator_id,
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
pub(crate) async fn sweep(pool: &sqlx::PgPool, date: NaiveDate) -> anyhow::Result<()> {
    let mut after = Uuid::nil();
    loop {
        let projects = sqlx::query!(
            "SELECT ps.project_id, ps.org_id FROM project_settings ps
             JOIN projects p ON p.id = ps.project_id AND p.org_id = ps.org_id
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
pub(super) struct BudgetProgress {
    pub task_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub period_key: String,
    pub consumed: i64,
    pub budget: i64,
}

/// Evaluate one configured project's selected period without changing lifetime
/// tracking totals. Task/person budgets remain independent, including empty ones.
pub(super) async fn configured_progress(
    connection: &mut PgConnection,
    org_id: Uuid,
    project_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<BudgetProgress>, sqlx::Error> {
    Ok(
        fetch_progress(connection, org_id, Some(project_id), None, date)
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
            .collect(),
    )
}

/// Batch overview projection. Authorization is checked against the current
/// database actor, not a role cached in the session or supplied by the caller.
pub(super) async fn progress_for_viewer(
    connection: &mut PgConnection,
    org_id: Uuid,
    viewer_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<crate::models::ProjectBudgetProgress>, sqlx::Error> {
    fetch_progress(connection, org_id, None, Some(viewer_id), date).await
}

async fn fetch_progress(
    connection: &mut PgConnection,
    org_id: Uuid,
    project_id: Option<Uuid>,
    viewer_id: Option<Uuid>,
    date: NaiveDate,
) -> Result<Vec<crate::models::ProjectBudgetProgress>, sqlx::Error> {
    use crate::models::ProjectBudgetProgress;
    use horae_core::types::BudgetKind;

    sqlx::query_as!(
        ProjectBudgetProgress,
        r#"WITH config AS (
             SELECT p.id, p.budget_kind, p.budget_minutes, p.budget_amount_cents,
                    p.project_type, p.currency, p.rate_cents, p.client_id,
                    ps.budget_scope, ps.monthly_reset, ps.include_nonbillable, ps.rate_mode
             FROM projects p JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
             WHERE p.org_id = $1 AND ($2::uuid IS NULL OR p.id = $2)
               AND ($4::uuid IS NULL OR EXISTS (
                 SELECT 1 FROM project_read_access access
                 WHERE access.org_id = p.org_id AND access.project_id = p.id
                   AND access.user_id = $4 AND access.can_view_progress))
           ), scopes AS (
             SELECT c.id AS project_id, NULL::uuid AS task_id, NULL::uuid AS user_id,
                    NULL::text AS label,
                    CASE WHEN c.budget_kind = 'hours' THEN c.budget_minutes ELSE c.budget_amount_cents END AS budget
             FROM config c WHERE c.budget_scope = 'project'
             UNION ALL
             SELECT c.id, pt.task_id, NULL::uuid, t.name,
                    CASE WHEN c.budget_kind = 'hours' THEN s.budget_minutes ELSE s.budget_cents END
             FROM config c JOIN project_tasks pt ON pt.project_id = c.id
             JOIN tasks t ON t.id = pt.task_id AND t.org_id = $1
             LEFT JOIN project_task_settings s ON s.project_id = c.id AND s.org_id = $1 AND s.task_id = pt.task_id
             WHERE c.budget_scope = 'task'
             UNION ALL
             SELECT c.id, NULL::uuid, a.user_id, u.name, s.budget_minutes
             FROM config c JOIN assignments a ON a.project_id = c.id
             JOIN users u ON u.id = a.user_id AND u.org_id = $1
             LEFT JOIN project_member_budgets s ON s.project_id = c.id AND s.org_id = $1 AND s.user_id = a.user_id
             WHERE c.budget_scope = 'person' AND c.budget_kind = 'hours'
           ), entries AS (
             SELECT c.id AS project_id, te.task_id, te.user_id,
                    CASE WHEN c.budget_kind = 'hours'
                      THEN effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)::bigint
                      ELSE COALESCE(line.amount_cents, line_amount_cents(
                        COALESCE(resolve_project_rate(c.rate_mode, pt.rate_cents, a.rate_cents, c.rate_cents,
                          CASE WHEN c.currency = o.default_currency THEN u.billable_rate_cents END,
                          CASE WHEN c.currency = cl.currency THEN cl.default_rate_cents END), 0),
                        effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir)))
                    END AS consumed
             FROM config c JOIN time_entries te ON te.project_id = c.id AND te.org_id = $1
             JOIN organizations o ON o.id = te.org_id
             JOIN users u ON u.id = te.user_id AND u.org_id = te.org_id
             JOIN clients cl ON cl.id = c.client_id AND cl.org_id = te.org_id
             JOIN tasks t ON t.id = te.task_id AND t.org_id = te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id = c.id AND pt.task_id = te.task_id
             LEFT JOIN assignments a ON a.project_id = c.id AND a.user_id = te.user_id
             LEFT JOIN invoice_line_items line ON line.invoice_id = te.invoice_id AND line.time_entry_id = te.id
             WHERE c.budget_kind <> 'none' AND (NOT c.monthly_reset OR (
                      te.spent_date >= date_trunc('month', $3::date)::date
                      AND te.spent_date < (date_trunc('month', $3::date) + interval '1 month')::date))
               AND (c.include_nonbillable OR c.project_type = 'non_billable'
                    OR (te.billable AND (te.invoice_id IS NOT NULL OR COALESCE(pt.billable, t.billable_default))))
           )
           SELECT c.id AS "project_id!", s.task_id AS "task_id?", s.user_id AS "user_id?",
                  s.label AS "label?", c.budget_scope AS "scope!", c.currency AS "currency!",
                  c.budget_kind AS "kind!: BudgetKind", s.budget AS "budget?",
                  CASE WHEN c.monthly_reset THEN to_char($3::date, 'YYYY-MM') ELSE 'lifetime' END AS "period_key!",
                  COALESCE(SUM(e.consumed), 0)::bigint AS "consumed!"
           FROM config c LEFT JOIN scopes s ON s.project_id = c.id
           LEFT JOIN entries e ON e.project_id = c.id
                             AND (s.task_id IS NULL OR e.task_id = s.task_id)
                             AND (s.user_id IS NULL OR e.user_id = s.user_id)
           GROUP BY c.id, c.budget_kind, c.budget_scope, c.currency, c.monthly_reset,
                    s.task_id, s.user_id, s.label, s.budget
           ORDER BY c.id, s.label, s.task_id, s.user_id"#,
        org_id,
        project_id,
        date as NaiveDate,
        viewer_id,
    )
    .fetch_all(connection)
    .await
}
