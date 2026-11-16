//! Permission editor server functions: session adapters over
//! `ops::permissions::{editor, profiles, templates}`.

use super::*;
#[cfg(feature = "server")]
use crate::db::OrganizationLock;
use crate::models::permission_editor::{
    PermissionEditor, PermissionRequester, PermissionSubjectPage, ProfileCommand, ProfileDraft,
    ProfileOutcome, ProfilePreview, TemplateCommand, TemplateDeletionPreview, TemplateOutcome,
};
#[cfg(feature = "server")]
use crate::ops::{self, Caller, EventSink, OpError, WriteIsolation, permissions};
#[cfg(feature = "server")]
use sqlx::PgPool;

#[cfg(feature = "server")]
const UNAVAILABLE: &str = permissions::EDITOR_UNAVAILABLE;

/// Discover local permission-editor subjects under current Administrator authority.
#[server]
pub async fn list_permission_subjects(
    after: Option<uuid::Uuid>,
) -> Result<PermissionSubjectPage, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(subject_page(&state.db, caller, after).await?)
}

/// Load current person permissions and reusable choices without inferring a profile.
#[server]
pub async fn load_permission_editor(
    user_id: uuid::Uuid,
) -> Result<PermissionEditor, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(editor_view(&state.db, caller, user_id).await?)
}

/// Compute exact effects; this read-only proposal never authorizes its later save.
#[server]
pub async fn preview_person_permissions(
    draft: ProfileDraft,
) -> Result<ProfilePreview, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(profile_preview(&state.db, caller, &draft).await?)
}

/// Revalidate and commit the explicit final proposal and confirmed relationship losses.
#[server]
pub async fn save_person_permissions(
    command: ProfileCommand,
    expected_requester: PermissionRequester,
) -> Result<ProfileOutcome, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(save_profile(
        &state.db,
        caller,
        &command,
        expected_requester,
        &*state.plugins,
    )
    .await?)
}

/// Show the current affected people; deletion preserves their grants and identity.
#[server]
pub async fn preview_permission_template_deletion(
    template_id: uuid::Uuid,
    expected_access_revision: i64,
    expected_template_revision: i64,
) -> Result<TemplateDeletionPreview, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(template_deletion_preview(
        &state.db,
        caller,
        template_id,
        expected_access_revision,
        expected_template_revision,
    )
    .await?)
}

/// Create a reusable selection or detach and delete a previously reviewed template.
#[server]
pub async fn save_permission_template(
    command: TemplateCommand,
    expected_requester: PermissionRequester,
) -> Result<TemplateOutcome, ServerFnError> {
    let caller = session_caller_or(UNAVAILABLE).await?;
    let state = crate::state::global_state().await;
    Ok(save_template(
        &state.db,
        caller,
        &command,
        expected_requester,
        &*state.plugins,
    )
    .await?)
}

#[cfg(feature = "server")]
pub(crate) async fn subject_page(
    db: &PgPool,
    caller: Caller,
    after: Option<uuid::Uuid>,
) -> Result<PermissionSubjectPage, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = permissions::editor::subjects(&mut read.tx, &read.actor, after).await;
    read.finish(result).await
}

#[cfg(feature = "server")]
pub(crate) async fn editor_view(
    db: &PgPool,
    caller: Caller,
    user_id: uuid::Uuid,
) -> Result<PermissionEditor, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = permissions::editor::load(&mut read.tx, &read.actor, user_id).await;
    read.finish(result).await
}

#[cfg(feature = "server")]
pub(crate) async fn profile_preview(
    db: &PgPool,
    caller: Caller,
    draft: &ProfileDraft,
) -> Result<ProfilePreview, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = permissions::editor::preview(&mut read.tx, &read.actor, draft).await;
    read.finish(result).await
}

#[cfg(feature = "server")]
pub(crate) async fn template_deletion_preview(
    db: &PgPool,
    caller: Caller,
    template_id: uuid::Uuid,
    expected_access_revision: i64,
    expected_template_revision: i64,
) -> Result<TemplateDeletionPreview, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result = permissions::editor::preview_template_deletion(
        &mut read.tx,
        &read.actor,
        template_id,
        expected_access_revision,
        expected_template_revision,
    )
    .await;
    read.finish(result).await
}

#[cfg(feature = "server")]
pub(crate) async fn save_profile(
    db: &PgPool,
    caller: Caller,
    command: &ProfileCommand,
    expected: PermissionRequester,
    events: &dyn EventSink,
) -> Result<ProfileOutcome, OpError> {
    let mut write = access_change(db, caller).await?;
    let result = permissions::profiles::save(&mut write.tx, &write.actor, command, expected).await;
    write.commit(result, events).await
}

#[cfg(feature = "server")]
pub(crate) async fn save_template(
    db: &PgPool,
    caller: Caller,
    command: &TemplateCommand,
    expected: PermissionRequester,
    events: &dyn EventSink,
) -> Result<TemplateOutcome, OpError> {
    let mut write = access_change(db, caller).await?;
    let result = permissions::templates::save(&mut write.tx, &write.actor, command, expected).await;
    write.commit(result, events).await
}

/// Every editor save changes access, so it holds the organization's
/// `AccessChange` gate: concurrent reads and saves wait for it.
#[cfg(feature = "server")]
async fn access_change(db: &PgPool, caller: Caller) -> Result<ops::WriteTx, OpError> {
    ops::begin_gated(
        db,
        caller,
        OrganizationLock::AccessChange,
        WriteIsolation::ReadCommitted,
    )
    .await
}
