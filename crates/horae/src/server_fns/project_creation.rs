//! Transactional creation and creator-owned draft persistence.

use super::*;
#[cfg(feature = "server")]
use crate::db::OrganizationLock;
use crate::models::project_creation::{
    CreationClient, CreationOptions, CreationSearch, CreationSelection, DraftSaved,
    EditableProject, ProjectDraft, ProjectEditRequest, ProjectEditorCatalog,
    ProjectEditorCatalogSearch, ProjectEditorContext, ProjectForm,
};
use crate::models::project_people::{
    ProjectPeopleContext, ProjectPeopleQuery, ProjectPeopleResult,
};

#[cfg(feature = "server")]
use crate::ops::projects::creation;
#[cfg(feature = "server")]
use crate::ops::{
    self, Caller, Done, EventSink, NoEvents, OpError, SnapshotLimits, WriteIsolation, WriteTx,
};
// The tests below reach these through `use super::*`.
#[cfg(all(test, feature = "server"))]
use crate::{
    db::lock_organization, models::project_creation::TaskAccess,
    ops::projects::form::validate_project_form,
};

/// The project editor as its tests call it.
#[cfg(all(test, feature = "server"))]
pub(super) mod editing;

#[cfg(all(test, feature = "server"))]
mod assignment_tests;

#[cfg(all(test, feature = "server"))]
mod client_lifecycle_tests;

#[cfg(all(test, feature = "server"))]
mod import_tests;

#[cfg(all(test, feature = "server"))]
mod locking_tests;

/// Read identity-only teammate choices for the authorized project operation.
#[server]
pub async fn project_people(
    context: ProjectPeopleContext,
    query: ProjectPeopleQuery,
) -> Result<ProjectPeopleResult, ServerFnError> {
    let caller = session_caller_or("Project people are unavailable").await?;
    let state = crate::state::global_state().await;
    Ok(project_people_page(&state.db, caller, context, &query).await?)
}

/// Read catalogs for the canonical editor under its current operation authority.
#[server]
pub async fn project_editor_catalog(
    context: ProjectEditorContext,
    search: ProjectEditorCatalogSearch,
) -> Result<ProjectEditorCatalog, ServerFnError> {
    let caller = session_caller_or("Project catalogs are unavailable").await?;
    let state = crate::state::global_state().await;
    Ok(
        project_editor_catalog_page(&state.db, caller, context, &search, state.mail.is_some())
            .await?,
    )
}

/// Load the existing project into the same form used for creation.
#[server]
pub async fn load_project_editor(project_id: uuid::Uuid) -> Result<EditableProject, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    Ok(project_editor_view(&state.db, caller, project_id).await?)
}

#[server]
pub async fn save_project_editor(request: ProjectEditRequest) -> Result<uuid::Uuid, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let saved = project_editor_save(
        &state.db,
        caller,
        &request,
        state.mail.is_some(),
        &*state.plugins,
    )
    .await?;
    Ok(saved.id)
}

#[cfg(feature = "server")]
pub(crate) async fn project_people_page(
    db: &sqlx::PgPool,
    caller: Caller,
    context: ProjectPeopleContext,
    query: &ProjectPeopleQuery,
) -> Result<ProjectPeopleResult, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = ops::projects::people::read(&mut read.tx, &read.actor, context, query).await;
    read.finish(result).await
}

#[cfg(feature = "server")]
pub(crate) async fn project_editor_catalog_page(
    db: &sqlx::PgPool,
    caller: Caller,
    context: ProjectEditorContext,
    search: &ProjectEditorCatalogSearch,
    email_available: bool,
) -> Result<ProjectEditorCatalog, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = ops::projects::editor::catalog::read(
        &mut read.tx,
        &read.actor,
        context,
        search,
        email_available,
    )
    .await;
    read.finish(result).await
}

/// Every section of the editor comes from one snapshot.
#[cfg(feature = "server")]
pub(crate) async fn project_editor_view(
    db: &sqlx::PgPool,
    caller: Caller,
    project_id: uuid::Uuid,
) -> Result<EditableProject, OpError> {
    let mut read = ops::begin_read_snapshot(db, caller, SnapshotLimits::Short).await?;
    let result = ops::projects::editor::load(&mut read.tx, &read.actor, project_id).await;
    read.finish(result).await
}

/// A save can change managers, so it holds the organization's `AccessChange`
/// gate, and runs at SERIALIZABLE so a lost race is a conflict to reload.
#[cfg(feature = "server")]
pub(crate) async fn project_editor_save(
    db: &sqlx::PgPool,
    caller: Caller,
    request: &ProjectEditRequest,
    email_available: bool,
    events: &dyn EventSink,
) -> Result<Project, OpError> {
    let mut write = ops::begin_gated(
        db,
        caller,
        OrganizationLock::AccessChange,
        WriteIsolation::Serializable,
    )
    .await?;
    let result =
        ops::projects::editor::save(&mut write.tx, &write.actor, request, email_available).await;
    write.commit(result, events).await
}

/// Resolve only the selected active task/person identities, independently of pagination.
#[server]
pub async fn project_creation_selection(
    task_ids: Vec<uuid::Uuid>,
    user_ids: Vec<uuid::Uuid>,
) -> Result<CreationSelection, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    load_selected_catalog(
        &state.db,
        caller.user_id,
        caller.org_id,
        &task_ids,
        &user_ids,
    )
    .await
}

/// Resolve an existing draft selection without searching or fetching a whole catalog.
#[server]
pub async fn project_creation_client(
    client_id: uuid::Uuid,
) -> Result<Option<CreationClient>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    load_selected_client(&state.db, caller.user_id, caller.org_id, client_id).await
}

#[server]
pub async fn project_creation_options(
    search: CreationSearch,
) -> Result<CreationOptions, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    load_creation_options(
        &state.db,
        caller.user_id,
        caller.org_id,
        &search,
        state.mail.is_some(),
    )
    .await
}

#[server]
pub async fn finalize_project_draft(
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: ProjectForm,
) -> Result<uuid::Uuid, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    finalize_draft_record(
        &state.db,
        caller.user_id,
        caller.org_id,
        draft_id,
        expected_revision,
        &form,
        state.mail.is_some(),
    )
    .await
}

#[server]
pub async fn load_project_draft() -> Result<Option<ProjectDraft>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    load_draft_record(&state.db, caller.user_id, caller.org_id).await
}

#[server]
pub async fn save_project_draft(
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: ProjectForm,
) -> Result<DraftSaved, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    save_draft_record(
        &state.db,
        caller.user_id,
        caller.org_id,
        draft_id,
        expected_revision,
        &form,
    )
    .await
}

#[server]
pub async fn discard_project_draft(
    draft_id: uuid::Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    discard_draft_record(
        &state.db,
        caller.user_id,
        caller.org_id,
        draft_id,
        expected_revision,
    )
    .await
}

/// Begins a creation step holding the organization `gate`: `Shared` for the
/// drafts and catalogs, `AccessChange` to finalize, which designates managers.
#[cfg(feature = "server")]
async fn begin_creation(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    gate: OrganizationLock,
) -> Result<WriteTx, OpError> {
    ops::begin_gated(
        pool,
        Caller::session(org_id, actor_id),
        gate,
        WriteIsolation::ReadCommitted,
    )
    .await
}

/// The project form's client dialog (`clients::create_project_client`) as the
/// creation tests drive it.
#[cfg(all(test, feature = "server"))]
async fn create_client_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    name: &str,
    currency: &str,
    default_rate: &str,
) -> Result<CreationClient, ServerFnError> {
    super::clients::create_project_client_record(
        pool,
        &crate::ops::testing::Recorder::default(),
        crate::ops::Caller::session(org_id, actor_id),
        name,
        currency,
        default_rate,
    )
    .await
}

#[cfg(feature = "server")]
async fn load_selected_catalog(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    task_ids: &[uuid::Uuid],
    user_ids: &[uuid::Uuid],
) -> Result<CreationSelection, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result = creation::selection(&mut write.tx, &write.actor, task_ids, user_ids).await;
    Ok(write.commit(result.map(Done::new), &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn load_selected_client(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
) -> Result<Option<CreationClient>, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result = creation::client(&mut write.tx, &write.actor, client_id).await;
    Ok(write.commit(result.map(Done::new), &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn load_creation_options(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    search: &CreationSearch,
    email_available: bool,
) -> Result<CreationOptions, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result = creation::options(&mut write.tx, &write.actor, search, email_available).await;
    Ok(write.commit(result.map(Done::new), &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn finalize_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: &ProjectForm,
    email_available: bool,
) -> Result<uuid::Uuid, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::AccessChange).await?;
    let result = creation::finalize(
        &mut write.tx,
        &write.actor,
        draft_id,
        expected_revision,
        form,
        email_available,
    )
    .await;
    // `project_created` is already in the outbox: nothing to dispatch here.
    Ok(write.commit(result, &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn load_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<ProjectDraft>, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result = creation::load_draft(&mut write.tx, &write.actor).await;
    Ok(write.commit(result.map(Done::new), &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn save_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: &ProjectForm,
) -> Result<DraftSaved, ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result = creation::save_draft(
        &mut write.tx,
        &write.actor,
        draft_id,
        expected_revision,
        form,
    )
    .await;
    Ok(write.commit(result, &NoEvents).await?)
}

#[cfg(feature = "server")]
async fn discard_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    draft_id: uuid::Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let mut write = begin_creation(pool, actor_id, org_id, OrganizationLock::Shared).await?;
    let result =
        creation::discard_draft(&mut write.tx, &write.actor, draft_id, expected_revision).await;
    Ok(write.commit(result, &NoEvents).await?)
}

#[cfg(all(test, feature = "server"))]
mod tests;
