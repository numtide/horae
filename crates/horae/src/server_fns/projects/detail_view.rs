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
    fetch(&state.db, &viewer, project_id, expected_requester).await
}

#[cfg(feature = "server")]
pub(super) async fn fetch(
    pool: &sqlx::PgPool,
    viewer: &User,
    project_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectDetailView, ServerFnError> {
    let requester = project_requester(viewer, expected_requester)?;
    let mut access = read_access::ReadAccess::begin(pool, viewer.org_id, viewer.id)
        .await?
        .ok_or_else(|| forbidden("Project access is unavailable"))?;
    let project = project_details_in_transaction(&mut access, viewer.org_id, viewer.id, project_id)
        .await?
        .ok_or_else(|| not_found("Project not found"))?;
    let context = sqlx::query!(
        r#"SELECT u.org_role as "org_role: OrgRole",
            EXISTS(SELECT 1 FROM project_management_assignments management
                WHERE management.org_id=u.org_id AND management.project_id=$3
                    AND management.manager_id=u.id) AS "managed!"
        FROM users u WHERE u.org_id=$1 AND u.id=$2 AND u.active"#,
        viewer.org_id,
        viewer.id,
        project_id,
    )
    .fetch_one(&mut *access.tx)
    .await
    .map_err(server_err)?;
    let can_edit = if access.legacy() {
        matches!(context.org_role, OrgRole::Admin | OrgRole::Manager)
    } else {
        access.has(Permission::ProjectWriteAll)
            || (access.has(Permission::ProjectWriteManaged) && context.managed)
    };
    let member_ids: Vec<_> = assignment_rows(&mut access, viewer, project_id)
        .await?
        .into_iter()
        .map(|row| row.user_id)
        .collect();
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
    let canonical_permissions = !access.legacy();
    access.tx.commit().await.map_err(server_err)?;
    Ok(ProjectDetailView {
        requester,
        canonical_permissions,
        project,
        can_edit,
        team,
        tasks,
    })
}
