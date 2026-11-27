use super::*;
use crate::models::project::ProjectDetailView;

/// Every statement reads one snapshot, so the page never mixes states.
#[server]
pub async fn get_project_detail_view(
    project_id: String,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectDetailView, ServerFnError> {
    let caller = unchecked_session_caller().await?;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let state = crate::state::global_state().await;
    let mut read = ops::begin_read_snapshot(&state.db, caller, ops::SnapshotLimits::Short)
        .await
        .map_err(op_error)?;
    let result =
        ops::projects::detail_view(&mut read.tx, &read.actor, project_id, expected_requester).await;
    read.finish(result).await.map_err(op_error)
}

/// The detail page through the same snapshot as the server function, on a
/// pinned `today`.
#[cfg(test)]
pub(super) async fn fetch(
    pool: &sqlx::PgPool,
    viewer: &User,
    project_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
    today: chrono::NaiveDate,
) -> Result<ProjectDetailView, ServerFnError> {
    let mut read = ops::begin_read_snapshot(
        pool,
        Caller::session(viewer.org_id, viewer.id),
        ops::SnapshotLimits::Short,
    )
    .await
    .map_err(op_error)?;
    let result = ops::projects::detail_view_on(
        &mut read.tx,
        &read.actor,
        project_id,
        expected_requester,
        today,
    )
    .await;
    read.finish(result).await.map_err(op_error)
}
