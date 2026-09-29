//! Authorized task/person intersections for the project reporting tabs.

use super::*;
use crate::models::project::{ProjectActivityInterval, ProjectBreakdown};

#[server]
pub async fn get_project_breakdown(
    project_id: String,
    interval: Option<ProjectActivityInterval>,
) -> Result<ProjectBreakdown, ServerFnError> {
    let viewer = require_user().await?;
    let project_id = project_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid project_id"))?;
    let state = crate::state::global_state().await;
    fetch_project_breakdown(&state.db, viewer.org_id, viewer.id, project_id, interval).await
}

#[cfg(feature = "server")]
async fn fetch_project_breakdown(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
    interval: Option<ProjectActivityInterval>,
) -> Result<ProjectBreakdown, ServerFnError> {
    use crate::models::project::ProjectWorkEntity;
    use horae_core::project_activity::{ActivityRange, weekly_activity};
    use horae_core::project_breakdown::{WorkCell, WorkTotals, summarize};

    const MAX_ROWS: usize = 5_000;
    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    let access = sqlx::query!(
        r#"SELECT o.default_currency, o.week_start,
           (u.org_role = 'admin' OR (u.org_role = 'manager' AND NOT EXISTS (
             SELECT 1 FROM time_entries te
             JOIN project_member_costs mc ON mc.project_id = te.project_id
               AND mc.user_id = te.user_id AND mc.org_id = te.org_id
             WHERE te.org_id = p.org_id AND te.project_id = p.id
           ))) as "can_view_costs!"
           FROM projects p JOIN organizations o ON o.id = p.org_id
           JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id
           JOIN users u ON u.id = a.user_id AND u.org_id = a.org_id
           WHERE p.org_id = $1 AND p.id = $2 AND a.user_id = $3 AND a.can_view_progress"#,
        org_id,
        project_id,
        viewer_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))?;
    if let Some(interval) = interval {
        let range = ActivityRange::new(interval.from, interval.to)
            .map_err(|error| err(BAD_REQUEST, error))?;
        let weekday = super::super::organization::configured_weekday(access.week_start)?;
        weekly_activity(range, weekday, &[], 5_200).map_err(|error| err(BAD_REQUEST, error))?;
    }
    let from = interval.map(|value| value.from);
    let to = interval.map(|value| value.to);
    // Never join private cost overrides for an unauthorized viewer. Aggregate
    // actual minutes and round each contributing cost before grouping.
    let rows = sqlx::query!(
        r#"SELECT te.task_id, te.user_id,
                  SUM(te.minutes)::bigint AS "minutes!",
                  COALESCE(SUM(te.minutes) FILTER (WHERE te.billable), 0)::bigint AS "billable_minutes!",
                  CASE WHEN $5 AND COUNT(*) FILTER (WHERE COALESCE(mc.cost_rate_cents, u.cost_rate_cents) IS NULL) = 0
                       THEN SUM(line_amount_cents(COALESCE(mc.cost_rate_cents, u.cost_rate_cents), te.minutes)) FILTER (WHERE $5)::bigint
                       ELSE NULL END AS "cost_cents?"
           FROM time_entries te JOIN users u ON u.id = te.user_id AND u.org_id = te.org_id
           LEFT JOIN project_member_costs mc ON $5 AND mc.org_id = te.org_id
             AND mc.project_id = te.project_id AND mc.user_id = te.user_id
           WHERE te.org_id = $1 AND te.project_id = $2
             AND ($3::date IS NULL OR te.spent_date >= $3)
             AND ($4::date IS NULL OR te.spent_date <= $4)
           GROUP BY te.task_id, te.user_id ORDER BY te.task_id, te.user_id LIMIT $6"#,
        org_id, project_id, from as Option<chrono::NaiveDate>, to as Option<chrono::NaiveDate>,
        access.can_view_costs, MAX_ROWS as i64 + 1,
    ).fetch_all(&mut *tx).await.map_err(server_err)?;
    if rows.len() > MAX_ROWS {
        return Err(err(
            BAD_REQUEST,
            "More than 5000 task/person groups. Select a shorter period; no partial results are shown.",
        ));
    }
    let cells: Vec<_> = rows
        .into_iter()
        .map(|row| WorkCell {
            task_id: row.task_id,
            user_id: row.user_id,
            totals: WorkTotals {
                minutes: row.minutes,
                billable_minutes: row.billable_minutes,
                cost_cents: row.cost_cents,
            },
        })
        .collect();
    let totals = summarize(&cells, access.can_view_costs).map_err(server_err)?;
    let task_ids: Vec<_> = totals.by_task.keys().copied().collect();
    let user_ids: Vec<_> = totals.by_person.keys().copied().collect();
    let tasks = sqlx::query_as!(ProjectWorkEntity,
        r#"SELECT t.id, t.name, t.active,
                  EXISTS (SELECT 1 FROM project_tasks pt WHERE pt.project_id = $2 AND pt.task_id = t.id) AS "current!",
                  false AS "manager!"
           FROM tasks t WHERE t.org_id = $1 AND (t.id = ANY($3::uuid[]) OR EXISTS (
             SELECT 1 FROM project_tasks pt WHERE pt.project_id = $2 AND pt.task_id = t.id))
           ORDER BY t.name, t.id LIMIT $4"#,
        org_id, project_id, &task_ids, MAX_ROWS as i64 + 1,
    ).fetch_all(&mut *tx).await.map_err(server_err)?;
    let people = sqlx::query_as!(
        ProjectWorkEntity,
        r#"SELECT u.id, u.name, u.active, (a.id IS NOT NULL) AS "current!",
                  COALESCE(a.role IN ('lead', 'admin'), false) AS "manager!"
           FROM users u LEFT JOIN assignments a ON a.user_id = u.id AND a.project_id = $2
           WHERE u.org_id = $1 AND (a.id IS NOT NULL OR u.id = ANY($3::uuid[]))
           ORDER BY u.name, u.id LIMIT $4"#,
        org_id,
        project_id,
        &user_ids,
        MAX_ROWS as i64 + 1,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    if tasks.len() > MAX_ROWS || people.len() > MAX_ROWS {
        return Err(err(
            BAD_REQUEST,
            "More than 5000 project contributors. No partial results are shown.",
        ));
    }
    tx.commit().await.map_err(server_err)?;
    Ok(ProjectBreakdown {
        interval,
        cost_currency: access.can_view_costs.then_some(access.default_currency),
        tasks,
        people,
        cells,
        totals,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests;
