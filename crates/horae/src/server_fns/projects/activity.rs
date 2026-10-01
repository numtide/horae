//! Read-only tracked activity for the project dashboard.

use super::*;
use crate::models::project::{ProjectActivity, ProjectActivityInterval};

/// Read actual weekly minutes for an inclusive interval, or all recorded work.
/// Requires current project-progress access; never includes private entry data.
#[server]
pub async fn get_project_activity(
    project_id: String,
    interval: Option<ProjectActivityInterval>,
) -> Result<ProjectActivity, ServerFnError> {
    let viewer = require_user().await?;
    let project_id = project_id
        .parse()
        .map_err(|_| err(BAD_REQUEST, "Invalid project_id"))?;
    let state = crate::state::global_state().await;
    fetch_project_activity(&state.db, viewer.org_id, viewer.id, project_id, interval).await
}

#[cfg(feature = "server")]
async fn fetch_project_activity(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
    interval: Option<ProjectActivityInterval>,
) -> Result<ProjectActivity, ServerFnError> {
    use horae_core::project_activity::{ActivityRange, DailyActivity, weekly_activity};

    use crate::models::project::ProjectActivityWeek;

    const MAX_WEEKS: usize = 5_200;

    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    let first_day = sqlx::query_scalar!(
        "SELECT o.week_start FROM projects p
         JOIN organizations o ON o.id = p.org_id
         JOIN project_read_access a ON a.project_id = p.id AND a.org_id = p.org_id
         WHERE p.org_id = $1 AND p.id = $2 AND a.user_id = $3 AND a.can_view_progress",
        org_id,
        project_id,
        viewer_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))?;
    let week_start = super::super::organization::configured_weekday(first_day)?;
    let interval = match interval {
        Some(selected) => Some(selected),
        None => {
            let bounds = sqlx::query!(
                r#"SELECT MIN(spent_date) AS "from: chrono::NaiveDate",
                          MAX(spent_date) AS "to: chrono::NaiveDate"
                   FROM time_entries WHERE org_id = $1 AND project_id = $2"#,
                org_id,
                project_id,
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(server_err)?;
            match (bounds.from, bounds.to) {
                (Some(from), Some(to)) => Some(ProjectActivityInterval { from, to }),
                (None, None) => None,
                _ => return Err(server_err("Incomplete project activity bounds")),
            }
        }
    };
    let Some(interval) = interval else {
        tx.commit().await.map_err(server_err)?;
        return Ok(ProjectActivity {
            interval: None,
            week_start,
            weeks: Vec::new(),
        });
    };
    let range =
        ActivityRange::new(interval.from, interval.to).map_err(|error| err(BAD_REQUEST, error))?;
    // Check the bucket/date bound before loading even the daily aggregate.
    weekly_activity(range, week_start, &[], MAX_WEEKS).map_err(|error| err(BAD_REQUEST, error))?;
    let days = sqlx::query_as!(
        DailyActivity,
        r#"SELECT spent_date AS "date: chrono::NaiveDate",
                  SUM(CASE WHEN billable THEN minutes::bigint ELSE 0 END)::bigint
                    AS "billable_minutes!",
                  SUM(CASE WHEN NOT billable THEN minutes::bigint ELSE 0 END)::bigint
                    AS "non_billable_minutes!"
           FROM time_entries
           WHERE org_id = $1 AND project_id = $2 AND spent_date BETWEEN $3 AND $4
           GROUP BY spent_date ORDER BY spent_date"#,
        org_id,
        project_id,
        interval.from as chrono::NaiveDate,
        interval.to as chrono::NaiveDate,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    let weeks = weekly_activity(range, week_start, &days, MAX_WEEKS)
        .map_err(server_err)?
        .into_iter()
        .map(|week| ProjectActivityWeek {
            from: week.from,
            to: week.to,
            billable_minutes: week.billable_minutes,
            non_billable_minutes: week.non_billable_minutes,
            cumulative_minutes: week.cumulative_minutes,
        })
        .collect();
    tx.commit().await.map_err(server_err)?;
    Ok(ProjectActivity {
        interval: Some(interval),
        week_start,
        weeks,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests;
