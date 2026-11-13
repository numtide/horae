use super::*;
#[cfg(feature = "server")]
use crate::models::project::ProjectDetailIdentity;
use crate::models::project::ProjectDetailView;

#[server]
pub async fn get_project_detail_view(
    project_id: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectDetailView, ServerFnError> {
    let viewer = require_user().await?;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let state = crate::state::global_state().await;
    let today = crate::timezone::today_for_org(&state.db, viewer.org_id)
        .await
        .map_err(server_err)?;
    fetch(&state.db, &viewer, project_id, expected_requester, today).await
}

/// `today` selects a monthly-reset budget's month, as the Projects list does.
#[cfg(feature = "server")]
pub(super) async fn fetch(
    pool: &sqlx::PgPool,
    viewer: &User,
    project_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
    today: chrono::NaiveDate,
) -> Result<ProjectDetailView, ServerFnError> {
    let requester = project_requester(viewer, expected_requester)?;
    let mut access = read_access::ReadAccess::begin_snapshot(pool, viewer.org_id, viewer.id)
        .await?
        .ok_or_else(|| forbidden("Project access is unavailable"))?;
    let project = project_details_in_transaction(&mut access, viewer.org_id, viewer.id, project_id)
        .await?
        .ok_or_else(|| not_found("Project not found"))?;
    let context = sqlx::query!(
        r#"SELECT p.client_id, p.project_type as "project_type: ProjectType", p.active,
            EXISTS(SELECT 1 FROM project_management_assignments management
                WHERE management.org_id=p.org_id AND management.project_id=p.id
                    AND management.manager_id=$2) AS "managed!"
        FROM projects p WHERE p.org_id=$1 AND p.id=$3"#,
        viewer.org_id,
        viewer.id,
        project_id,
    )
    .fetch_one(&mut *access.tx)
    .await
    .map_err(server_err)?;
    // The same rule as `ops::projects::set_active`.
    let can_edit = access.has(Permission::ProjectWriteAll)
        || (access.has(Permission::ProjectWriteManaged) && context.managed);
    let member_ids = team_member_ids(&mut access, viewer, project_id).await?;
    let team = sqlx::query_as!(
        ProjectDetailIdentity,
        "SELECT id,name FROM users WHERE org_id=$1 AND id=ANY($2) ORDER BY lower(name),id",
        viewer.org_id,
        &member_ids,
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;
    let tasks = sqlx::query_as!(
        ProjectDetailIdentity,
        "SELECT t.id,t.name FROM project_tasks pt
         JOIN tasks t ON t.id=pt.task_id AND t.org_id=$1
         JOIN projects p ON p.id=pt.project_id AND p.org_id=$1
         WHERE pt.project_id=$2 AND t.active ORDER BY lower(t.name),t.id",
        viewer.org_id,
        project_id,
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;
    // The Clients page's `ClientReadScope::can_read`, applied to this reader.
    let can_open_client = access.has(Permission::ClientReadAll);
    let budget = crate::server_fns::budgets::project_budget(
        &mut access,
        viewer.org_id,
        viewer.id,
        project_id,
        today,
    )
    .await?;
    access.tx.commit().await.map_err(server_err)?;
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
