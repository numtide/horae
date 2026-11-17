//! The project editor (`ops::projects::editor`) as its tests call it: through
//! the server functions' adapters, as the person a test names, with the
//! errors a page receives.

use super::*;
// The tests below reach these through `use super::*`.
use crate::models::project_creation::{FeeMode, MilestoneInput};
use crate::ops::testing::Recorder;
use crate::plugin::AppEvent;
use horae_core::project::MonthlyFeeDay;
use uuid::Uuid;

pub(in crate::server_fns) async fn load_editable_project(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<EditableProject, ServerFnError> {
    Ok(project_editor_view(pool, Caller::session(org_id, actor_id), project_id).await?)
}

/// A saved edit and the events it dispatched once committed.
pub(in crate::server_fns) struct SavedEdit {
    pub project: Project,
    pub changed: bool,
    /// The team's events: everything but `project_updated`.
    pub team_events: Vec<AppEvent>,
}

pub(in crate::server_fns) async fn save_project_edit(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    request: &ProjectEditRequest,
    email_available: bool,
) -> Result<SavedEdit, ServerFnError> {
    let events = Recorder::default();
    let project = project_editor_save(
        pool,
        Caller::session(org_id, actor_id),
        request,
        email_available,
        &events,
    )
    .await?;
    // The save announces a change of the project or its managers, and only then.
    let (updated, team_events): (Vec<_>, Vec<_>) = events
        .take()
        .into_iter()
        .partition(|event| matches!(event, AppEvent::ProjectUpdated { .. }));
    Ok(SavedEdit {
        project,
        changed: !updated.is_empty(),
        team_events,
    })
}

/// [`save_project_edit`] as most tests check it: the project and whether it
/// changed.
pub(in crate::server_fns) async fn save_editable_project(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    request: &ProjectEditRequest,
    email_available: bool,
) -> Result<(Project, bool), ServerFnError> {
    let saved = save_project_edit(pool, actor_id, org_id, request, email_available).await?;
    Ok((saved.project, saved.changed))
}

pub(in crate::server_fns) mod catalog {
    use super::*;

    pub(in crate::server_fns) async fn read(
        pool: &sqlx::PgPool,
        actor_id: Uuid,
        org_id: Uuid,
        context: ProjectEditorContext,
        search: &ProjectEditorCatalogSearch,
        email_available: bool,
    ) -> Result<ProjectEditorCatalog, ServerFnError> {
        let caller = Caller::session(org_id, actor_id);
        Ok(project_editor_catalog_page(pool, caller, context, search, email_available).await?)
    }
}

mod tests;
