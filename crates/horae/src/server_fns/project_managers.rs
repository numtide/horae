//! Session-bound project delegation, independent of global permission administration.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project_managers::{
    ProjectManagers, ProjectManagersCommand, ProjectManagersOutcome,
};

#[cfg(feature = "server")]
async fn requester(expected: Option<PermissionRequester>) -> Result<User, ServerFnError> {
    let user = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate project manager request");
            server_err("Project managers are unavailable")
        }
    })?;
    if expected
        .is_some_and(|expected| expected.org_id != user.org_id || expected.user_id != user.id)
    {
        return Err(forbidden(
            "Sign in as the original requester before retrying",
        ));
    }
    Ok(user)
}

#[cfg(feature = "server")]
fn map_error(error: permissions::project_management::ProjectManagersError) -> ServerFnError {
    use permissions::project_management::ProjectManagersError as Error;
    match error {
        Error::Forbidden => forbidden("Current project editing authority is required"),
        Error::NotFound => not_found("Project not found"),
        error @ (Error::Duplicate | Error::Ineligible) => err(BAD_REQUEST, error),
        error @ (Error::Stale | Error::Busy | Error::RequestConflict) => conflict(error),
        error => {
            tracing::error!(%error, "Project manager operation failed");
            server_err("Project managers are unavailable")
        }
    }
}

/// Load every retained manager under current authority to edit this project.
#[server]
pub async fn load_project_managers(
    project_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectManagers, ServerFnError> {
    let user = requester(expected_requester).await?;
    let state = crate::state::global_state().await;
    permissions::project_management::read(&state.db, user.org_id, user.id, project_id)
        .await
        .map_err(map_error)
}

/// Replace the complete manager set without changing grants or tracking membership.
#[server]
pub async fn save_project_managers(
    command: ProjectManagersCommand,
    expected_requester: PermissionRequester,
) -> Result<ProjectManagersOutcome, ServerFnError> {
    let user = requester(Some(expected_requester)).await?;
    let state = crate::state::global_state().await;
    permissions::project_management::execute(&state.db, user.org_id, user.id, &command)
        .await
        .map_err(map_error)
}
