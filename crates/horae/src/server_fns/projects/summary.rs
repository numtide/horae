//! Read-only lifetime work, current budgets and authorized internal costs.

use super::*;
use crate::models::project::ProjectSummary;

#[server]
pub async fn get_project_summary(project_id: String) -> Result<ProjectSummary, ServerFnError> {
    let viewer = require_user().await?;
    let project_id = project_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid project_id"))?;
    let state = crate::state::global_state().await;
    fetch_project_summary(
        &state.db,
        viewer.org_id,
        viewer.id,
        project_id,
        chrono::Utc::now().date_naive(),
    )
    .await
}

#[cfg(feature = "server")]
async fn fetch_project_summary(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
    date: chrono::NaiveDate,
) -> Result<ProjectSummary, ServerFnError> {
    use crate::models::project::{ProjectBudgetProgress, ProjectCostSummary};

    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    let project = sqlx::query!(
        r#"SELECT p.currency, p.budget_kind as "budget_kind: BudgetKind",
                  p.budget_minutes, p.budget_amount_cents, o.default_currency,
                  (ps.project_id IS NOT NULL) as "configured_budget!",
                  (u.org_role = 'admin' OR (u.org_role = 'manager' AND NOT EXISTS (
                    SELECT 1 FROM time_entries te
                    JOIN project_member_costs mc ON mc.project_id = te.project_id
                      AND mc.user_id = te.user_id AND mc.org_id = te.org_id
                    WHERE te.org_id = p.org_id AND te.project_id = p.id
                  ))) as "can_view_costs!"
           FROM projects p
           JOIN organizations o ON o.id = p.org_id
           JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id
           JOIN users u ON u.id = a.user_id AND u.org_id = a.org_id
           LEFT JOIN project_settings ps ON ps.project_id = p.id AND ps.org_id = p.org_id
           WHERE p.org_id = $1 AND p.id = $2 AND a.user_id = $3 AND a.can_view_progress"#,
        org_id,
        project_id,
        viewer_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))?;
    let hours = sqlx::query!(
        r#"SELECT COALESCE(SUM(minutes), 0)::bigint as "total_minutes!",
                  COALESCE(SUM(minutes) FILTER (WHERE billable), 0)::bigint as "billable_minutes!",
                  COALESCE(SUM(minutes) FILTER (WHERE NOT billable), 0)::bigint as "non_billable_minutes!"
           FROM time_entries WHERE org_id = $1 AND project_id = $2"#,
        org_id, project_id,
    ).fetch_one(&mut *tx).await.map_err(server_err)?;
    let budgets = if project.configured_budget {
        super::super::budgets::project_progress_for_viewer(
            &mut tx, org_id, viewer_id, project_id, date,
        )
        .await
        .map_err(server_err)?
    } else {
        let (budget, consumed) = match project.budget_kind {
            BudgetKind::Hours => (project.budget_minutes, hours.total_minutes),
            BudgetKind::Amount => {
                let spend = project_spend_for_viewer(&mut tx, org_id, viewer_id, Some(project_id))
                    .await
                    .map_err(server_err)?;
                (
                    project.budget_amount_cents,
                    spend.first().map_or(0, |row| row.spent_cents),
                )
            }
            BudgetKind::None => (None, 0),
        };
        vec![ProjectBudgetProgress {
            project_id,
            task_id: None,
            user_id: None,
            scope: "project".into(),
            label: None,
            kind: project.budget_kind,
            currency: project.currency,
            period_key: "lifetime".into(),
            budget,
            consumed,
        }]
    };
    let budget_totals =
        horae_core::budget::summarize_scopes(budgets.iter().map(|row| (row.budget, row.consumed)))
            .map_err(server_err)?;
    // Never calculate or return a partial cost for a viewer lacking a contributing rate.
    let internal_costs = if project.can_view_costs {
        let cost = sqlx::query!(
            r#"WITH costs AS (
                 SELECT te.minutes, COALESCE(mc.cost_rate_cents, u.cost_rate_cents) AS rate
                 FROM time_entries te
                 JOIN users u ON u.id = te.user_id AND u.org_id = te.org_id
                 LEFT JOIN project_member_costs mc ON mc.project_id = te.project_id
                   AND mc.user_id = te.user_id AND mc.org_id = te.org_id
                 WHERE te.org_id = $1 AND te.project_id = $2
               )
               SELECT CASE WHEN COUNT(*) FILTER (WHERE rate IS NULL) > 0 THEN NULL
                      ELSE COALESCE(SUM(line_amount_cents(rate, minutes)), 0)::bigint END as "total_cents?",
                      COALESCE(SUM(minutes) FILTER (WHERE rate IS NULL), 0)::bigint as "missing_rate_minutes!"
               FROM costs"#,
            org_id, project_id,
        ).fetch_one(&mut *tx).await.map_err(server_err)?;
        Some(ProjectCostSummary {
            currency: project.default_currency,
            total_cents: cost.total_cents,
            missing_rate_minutes: cost.missing_rate_minutes,
        })
    } else {
        None
    };
    tx.commit().await.map_err(server_err)?;
    Ok(ProjectSummary {
        total_minutes: hours.total_minutes,
        billable_minutes: hours.billable_minutes,
        non_billable_minutes: hours.non_billable_minutes,
        configured_budget: project.configured_budget,
        budgets,
        budget_totals,
        internal_costs,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests;
