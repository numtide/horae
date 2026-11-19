//! `GET /users/me/project_assignments` (spec 021 FR-044 item 1): the projects
//! and tasks the caller may track, which is what timer apps build their picker
//! from. The pairs are the Timesheet's own (`ops::time::tracking`, as
//! `/api/v1/me/trackable` serves them), grouped by project.
//!
//! Horae numbers projects and tasks but not the assignments themselves, and an
//! administrator tracks any project without one, so an assignment carries its
//! project's id and a task assignment its task's id. Both are unique within
//! the list, which is all a picker needs.

use std::collections::HashMap;

use horae_types::scoped_time::TimesheetWriteContext;

use super::*;

#[derive(Deserialize)]
pub(super) struct AssignmentQuery {
    page: Option<i64>,
    per_page: Option<i64>,
}

pub(super) async fn mine(
    user: AuthUser,
    State(db): State<PgPool>,
    OriginalUri(uri): OriginalUri,
    Query(query): Query<AssignmentQuery>,
) -> ApiResult<HarvestPagination<HarvestProjectAssignment>> {
    let (page, per_page, offset) = page_window(query.page, query.per_page)?;
    let mut read = begin_read(&db, &user, "Project assignment").await?;
    let result = assignments(&mut read).await;
    let all = finish_read(read, result).await?;
    let total = i64::try_from(all.len()).map_err(internal)?;
    // `page_window` bounds both to small positive numbers.
    let page_items = all
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .collect();
    Ok(Json(
        HarvestPagination::new(
            "project_assignments",
            page_items,
            page,
            per_page,
            total,
            "/harvest/v2/users/me/project_assignments",
        )
        .with_query(uri.query()),
    ))
}

async fn assignments(read: &mut ops::ReadTx) -> Result<Vec<HarvestProjectAssignment>, ApiError> {
    let actor = &read.actor;
    let context = TimesheetWriteContext {
        expected_requester: actor.requester(),
        subject_id: actor.id(),
    };
    let options = ops::time::tracking(&mut read.tx, actor, &context).await?;
    let projects: Vec<Uuid> = options.iter().map(|option| option.project_id).collect();
    let tasks: Vec<Uuid> = options.iter().map(|option| option.task_id).collect();
    let project_rows: HashMap<Uuid, _> = sqlx::query!(
        r#"SELECT p.id, p.harvest_id, p.code, c.harvest_id AS client_id, c.name AS client_name,
                  c.currency AS client_currency,
                  EXISTS (SELECT 1 FROM project_management_assignments m
                          WHERE m.project_id = p.id AND m.manager_id = $2) AS "manager!"
           FROM projects p JOIN clients c ON c.id = p.client_id
           WHERE p.id = ANY($1)"#,
        &projects,
        actor.id(),
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?
    .into_iter()
    .map(|row| (row.id, row))
    .collect();
    let task_numbers: HashMap<Uuid, i64> = sqlx::query!(
        "SELECT id, harvest_id FROM tasks WHERE id = ANY($1)",
        &tasks
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?
    .into_iter()
    .map(|row| (row.id, row.harvest_id))
    .collect();

    // The options come ordered by project, so each project's tasks are adjacent.
    let mut assignments: Vec<(Uuid, HarvestProjectAssignment)> = Vec::new();
    for option in options {
        let task_number = *task_numbers
            .get(&option.task_id)
            .ok_or_else(|| internal("a trackable task has no number"))?;
        let task = HarvestTaskAssignment {
            id: task_number,
            billable: option.billable,
            is_active: true,
            hourly_rate: None,
            budget: None,
            created_at: None,
            updated_at: None,
            task: HarvestRef {
                id: task_number,
                name: option.task_name,
            },
        };
        match assignments.last_mut() {
            Some((project, assignment)) if *project == option.project_id => {
                assignment.task_assignments.push(task);
            }
            _ => {
                let row = project_rows
                    .get(&option.project_id)
                    .ok_or_else(|| internal("a trackable project has no row"))?;
                assignments.push((
                    option.project_id,
                    HarvestProjectAssignment {
                        id: row.harvest_id,
                        is_project_manager: row.manager,
                        is_active: true,
                        use_default_rates: None,
                        budget: None,
                        hourly_rate: None,
                        created_at: None,
                        updated_at: None,
                        project: HarvestProjectRef {
                            id: row.harvest_id,
                            name: option.project_name,
                            code: row.code.clone(),
                        },
                        client: HarvestClientRef {
                            id: row.client_id,
                            name: row.client_name.clone(),
                            currency: row.client_currency.clone(),
                        },
                        task_assignments: vec![task],
                    },
                ));
            }
        }
    }
    Ok(assignments
        .into_iter()
        .map(|(_, assignment)| assignment)
        .collect())
}
