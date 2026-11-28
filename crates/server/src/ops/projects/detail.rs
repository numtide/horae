//! Project Detail: one project's details, team, tasks and budget headline,
//! read in one snapshot (see `ops::begin_read_snapshot`).

use chrono::NaiveDate;
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::projects;
use horae_core::types::ProjectType;
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use super::{NOT_FOUND, REQUESTER_CHANGED};
use crate::models::ProjectDetails;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project::{ProjectDetailIdentity, ProjectDetailView};

/// [`view_on`] the organization's today, read in the same transaction, as
/// the Projects list does.
pub async fn view(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    expected: Option<PermissionRequester>,
) -> Result<ProjectDetailView, OpError> {
    let today = actor.organization()?.today();
    view_on(tx, actor, project_id, expected, today).await
}

/// The detail page. `today` selects a monthly-reset budget's month. A
/// project the actor may not read is not found.
pub async fn view_on(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
    expected: Option<PermissionRequester>,
    today: NaiveDate,
) -> Result<ProjectDetailView, OpError> {
    let requester = actor.expect_requester(expected, REQUESTER_CHANGED)?;
    if !actor.is_active() {
        return Err(OpError::forbidden("Project access is unavailable"));
    }
    let project = details(tx, actor, project_id)
        .await?
        .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    let context = sqlx::query!(
        r#"SELECT p.client_id, p.project_type as "project_type: ProjectType", p.active,
            EXISTS(SELECT 1 FROM project_management_assignments management
                WHERE management.org_id=p.org_id AND management.project_id=p.id
                    AND management.manager_id=$2) AS "managed!"
        FROM projects p WHERE p.org_id=$1 AND p.id=$3"#,
        actor.org_id(),
        actor.id(),
        project_id,
    )
    .fetch_one(&mut **tx)
    .await?;
    let can_edit = projects::writes(|permission| actor.has(permission), context.managed);
    let member_ids = team_member_ids(tx, actor, project_id).await?;
    let team = sqlx::query_as!(
        ProjectDetailIdentity,
        "SELECT id,name FROM users WHERE org_id=$1 AND id=ANY($2) ORDER BY lower(name),id",
        actor.org_id(),
        &member_ids,
    )
    .fetch_all(&mut **tx)
    .await?;
    let tasks = sqlx::query_as!(
        ProjectDetailIdentity,
        "SELECT t.id,t.name FROM project_tasks pt
         JOIN tasks t ON t.id=pt.task_id AND t.org_id=$1
         JOIN projects p ON p.id=pt.project_id AND p.org_id=$1
         WHERE pt.project_id=$2 AND t.active ORDER BY lower(t.name),t.id",
        actor.org_id(),
        project_id,
    )
    .fetch_all(&mut **tx)
    .await?;
    let can_open_client = crate::ops::clients::reads_catalog(actor);
    let budget = super::budgets::headline(tx, actor, project_id, today).await?;
    Ok(ProjectDetailView {
        requester,
        project,
        client_id: context.client_id,
        project_type: context.project_type,
        active: context.active,
        can_edit,
        can_open_client,
        can_change_status: can_edit,
        team,
        tasks,
        budget,
    })
}

/// The project's saved details, or `None` when the actor may not read it.
async fn details(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
) -> Result<Option<ProjectDetails>, OpError> {
    Ok(sqlx::query_as!(
        ProjectDetails,
        r#"SELECT p.id, p.name, p.code, c.name AS client_name, p.currency,
            CASE WHEN ($6 OR ($7 AND management.id IS NOT NULL))
                AND ($8 OR ($9 AND management.id IS NOT NULL))
              AND p.project_type <> 'non_billable'
              AND (settings.project_id IS NULL
                OR (p.project_type = 'time_and_materials' AND settings.rate_mode = 'task'))
              THEN p.currency END AS task_rate_currency,
            p.starts_on as "starts_on: chrono::NaiveDate",
            p.ends_on as "ends_on: chrono::NaiveDate",
            ARRAY(SELECT t.name FROM project_tag_links l
                JOIN project_tags t ON t.id = l.tag_id AND t.org_id = l.org_id
                WHERE l.project_id = p.id AND l.org_id = p.org_id
                ORDER BY lower(t.name), t.id) as "tags!",
            CASE WHEN $6 THEN private.admin_notes END AS admin_notes
        FROM projects p
        JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
        LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
          AND management.project_id = p.id AND management.manager_id = $2
        LEFT JOIN assignments member ON member.project_id = p.id AND member.user_id = $2
        LEFT JOIN project_private_settings private ON private.project_id = p.id AND private.org_id = p.org_id
        LEFT JOIN project_settings settings ON settings.project_id = p.id AND settings.org_id = p.org_id
        WHERE p.org_id = $1 AND p.id = $3
          AND ($4 OR ($5 AND management.id IS NOT NULL)
            OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility, 'project_members') = 'project_members'))"#,
        actor.org_id(),
        actor.id(),
        project_id,
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
        // Private notes belong to whoever may edit every project.
        actor.has(Permission::ProjectWriteAll),
        actor.has(Permission::ProjectWriteManaged),
        actor.has(Permission::BillableRateWriteAll),
        actor.has(Permission::BillableRateWriteManaged),
    )
    .fetch_optional(&mut **tx)
    .await?)
}

/// The project members the actor may see, in assignment order.
async fn team_member_ids(
    tx: &mut Tx,
    actor: &Actor,
    project_id: Uuid,
) -> Result<Vec<Uuid>, OpError> {
    Ok(sqlx::query_scalar!(
        r#"SELECT a.user_id
         FROM assignments a JOIN projects p ON p.id = a.project_id
         JOIN clients c ON c.id = p.client_id AND c.org_id = p.org_id
         JOIN users member ON member.id = a.user_id AND member.org_id = p.org_id
         LEFT JOIN project_management_assignments management ON management.org_id = p.org_id
           AND management.project_id = p.id AND management.manager_id = $3
         WHERE a.project_id = $1 AND p.org_id = $2
           AND ($4 OR ($5 AND management.id IS NOT NULL) OR a.user_id = $3)
         ORDER BY a.created_at, a.id"#,
        project_id,
        actor.org_id(),
        actor.id(),
        actor.has(Permission::ProjectReadAll),
        actor.has(Permission::ProjectReadManaged),
    )
    .fetch_all(&mut **tx)
    .await?)
}
