//! Session-derived permission administration; legacy policy remains unavailable.

use super::*;
use crate::models::permission_editor::{
    PermissionEditor, PermissionRequester, PermissionSubjectPage, ProfileCommand, ProfileDraft,
    ProfileOutcome, ProfilePreview, TemplateCommand, TemplateDeletionPreview, TemplateOutcome,
};

#[cfg(feature = "server")]
async fn editor_user() -> Result<User, ServerFnError> {
    require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate permission editor request");
            server_err("Permission editor is unavailable")
        }
    })
}

#[cfg(feature = "server")]
async fn expected_editor_user(expected: PermissionRequester) -> Result<User, ServerFnError> {
    let user = editor_user().await?;
    if user.org_id != expected.org_id || user.id != expected.user_id {
        return Err(forbidden(
            "Sign in as the original requester before retrying",
        ));
    }
    Ok(user)
}

#[cfg(feature = "server")]
fn profile_error(error: permissions::profiles::ProfileCommandError) -> ServerFnError {
    use permissions::profiles::ProfileCommandError as Error;
    match error {
        Error::Forbidden => forbidden("Current administrator authority is required"),
        Error::NotFound => not_found("Person or profile not found"),
        error @ (Error::Stale
        | Error::RequestConflict
        | Error::Confirmation
        | Error::LastAdministrator) => conflict(error),
        error @ (Error::AdministratorSelection | Error::Selection(_)) => err(BAD_REQUEST, error),
        error => {
            tracing::error!(%error, "Permission editor operation failed");
            server_err("Permission editor is unavailable")
        }
    }
}

#[cfg(feature = "server")]
fn template_error(error: permissions::templates::TemplateCommandError) -> ServerFnError {
    use permissions::templates::TemplateCommandError as Error;
    match error {
        Error::Forbidden => forbidden("Current administrator authority is required"),
        Error::NotFound => not_found("Profile not found"),
        error @ (Error::Stale | Error::RequestConflict | Error::NameConflict | Error::Limit) => {
            conflict(error)
        }
        error @ (Error::Name(_) | Error::Selection(_)) => err(BAD_REQUEST, error),
        error => {
            tracing::error!(%error, "Permission template operation failed");
            server_err("Permission editor is unavailable")
        }
    }
}

/// Discover local permission-editor subjects under current Administrator authority.
#[server]
pub async fn list_permission_subjects(
    after: Option<uuid::Uuid>,
) -> Result<PermissionSubjectPage, ServerFnError> {
    let user = editor_user().await?;
    let state = crate::state::global_state().await;
    permissions::editor::subjects(&state.db, user.org_id, user.id, after)
        .await
        .map_err(profile_error)
}

/// Load current person permissions and reusable choices without inferring a profile.
#[server]
pub async fn load_permission_editor(
    user_id: uuid::Uuid,
) -> Result<PermissionEditor, ServerFnError> {
    let user = editor_user().await?;
    let state = crate::state::global_state().await;
    permissions::editor::load(&state.db, user.org_id, user.id, user_id)
        .await
        .map_err(profile_error)
}

/// Compute exact effects; this read-only proposal never authorizes its later save.
#[server]
pub async fn preview_person_permissions(
    draft: ProfileDraft,
) -> Result<ProfilePreview, ServerFnError> {
    let user = editor_user().await?;
    let state = crate::state::global_state().await;
    permissions::editor::preview(&state.db, user.org_id, user.id, &draft)
        .await
        .map_err(profile_error)
}

/// Revalidate and commit the explicit final proposal and confirmed relationship losses.
#[server]
pub async fn save_person_permissions(
    command: ProfileCommand,
    expected_requester: PermissionRequester,
) -> Result<ProfileOutcome, ServerFnError> {
    let user = expected_editor_user(expected_requester).await?;
    let state = crate::state::global_state().await;
    permissions::profiles::execute(&state.db, user.org_id, user.id, &command)
        .await
        .map_err(profile_error)
}

/// Show the current affected people; deletion preserves their grants and identity.
#[server]
pub async fn preview_permission_template_deletion(
    template_id: uuid::Uuid,
    expected_access_revision: i64,
    expected_template_revision: i64,
) -> Result<TemplateDeletionPreview, ServerFnError> {
    let user = editor_user().await?;
    let state = crate::state::global_state().await;
    permissions::editor::preview_template_deletion(
        &state.db,
        user.org_id,
        user.id,
        template_id,
        expected_access_revision,
        expected_template_revision,
    )
    .await
    .map_err(profile_error)
}

/// Create a reusable selection or detach and delete a previously reviewed template.
#[server]
pub async fn save_permission_template(
    command: TemplateCommand,
    expected_requester: PermissionRequester,
) -> Result<TemplateOutcome, ServerFnError> {
    let user = expected_editor_user(expected_requester).await?;
    let state = crate::state::global_state().await;
    permissions::templates::execute(&state.db, user.org_id, user.id, &command)
        .await
        .map_err(template_error)
}
