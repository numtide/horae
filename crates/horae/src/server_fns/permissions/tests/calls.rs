//! The permission editor and history called through their adapters, as the
//! person a test names, and the refusals they answer.

use sqlx::PgPool;
use uuid::Uuid;

use crate::models::permission_editor::PermissionRequester;
use crate::ops::permissions::{
    AUTHORITY_REQUIRED, EDITOR_UNAVAILABLE, REQUEST_CONFLICT, audit::HISTORY_UNAVAILABLE,
    profiles as ops_profiles, templates as ops_templates,
};
use crate::ops::testing::Recorder;
use crate::ops::{Caller, OpError};

fn caller(org: Uuid, actor: Uuid) -> Caller {
    Caller::session(org, actor)
}

/// Saves bind to the requester the page was loaded for: here, the actor.
fn own(org: Uuid, actor: Uuid) -> PermissionRequester {
    PermissionRequester {
        org_id: org,
        user_id: actor,
    }
}

pub(crate) mod editor {
    use super::*;
    use crate::models::permission_editor::{
        PermissionEditor, PermissionSubjectPage, ProfileDraft, ProfilePreview,
        TemplateDeletionPreview,
    };

    pub(crate) async fn subjects(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        after: Option<Uuid>,
    ) -> Result<PermissionSubjectPage, OpError> {
        crate::server_fns::subject_page(pool, caller(org, actor), after).await
    }

    pub(crate) async fn load(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        user: Uuid,
    ) -> Result<PermissionEditor, OpError> {
        crate::server_fns::editor_view(pool, caller(org, actor), user).await
    }

    pub(crate) async fn preview(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        draft: &ProfileDraft,
    ) -> Result<ProfilePreview, OpError> {
        crate::server_fns::profile_preview(pool, caller(org, actor), draft).await
    }

    pub(crate) async fn preview_template_deletion(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        template_id: Uuid,
        expected_access_revision: i64,
        expected_template_revision: i64,
    ) -> Result<TemplateDeletionPreview, OpError> {
        crate::server_fns::template_deletion_preview(
            pool,
            caller(org, actor),
            template_id,
            expected_access_revision,
            expected_template_revision,
        )
        .await
    }
}

pub(crate) mod profiles {
    use super::*;
    use crate::models::permission_editor::ProfileOutcome;
    pub(crate) use crate::models::permission_editor::{ProfileAction, ProfileCommand};
    use crate::plugin::AppEvent;

    pub(crate) async fn execute(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        request: &ProfileCommand,
    ) -> Result<ProfileOutcome, OpError> {
        Ok(save(pool, org, actor, request).await?.0)
    }

    /// The outcome and the plugin event the committed save dispatched.
    pub(crate) async fn save(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        request: &ProfileCommand,
    ) -> Result<(ProfileOutcome, Option<AppEvent>), OpError> {
        let events = Recorder::default();
        let outcome = crate::server_fns::save_profile(
            pool,
            caller(org, actor),
            request,
            own(org, actor),
            &events,
        )
        .await?;
        let mut dispatched = events.take();
        assert!(dispatched.len() <= 1, "one save, one event: {dispatched:?}");
        Ok((outcome, dispatched.pop()))
    }
}

pub(crate) mod templates {
    use super::*;
    use crate::models::permission_editor::TemplateOutcome;
    pub(crate) use crate::models::permission_editor::{TemplateAction, TemplateCommand};

    pub(crate) async fn execute(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        request: &TemplateCommand,
    ) -> Result<TemplateOutcome, OpError> {
        let events = Recorder::default();
        let outcome = crate::server_fns::save_template(
            pool,
            caller(org, actor),
            request,
            own(org, actor),
            &events,
        )
        .await?;
        assert!(
            events.take().is_empty(),
            "template changes announce nothing"
        );
        Ok(outcome)
    }
}

pub(crate) mod audit {
    use super::*;
    use crate::models::permission_audit::{AuditCursor, AuditEntry, AuditPage};
    pub(crate) use crate::ops::permissions::audit::{INVALID_DOCUMENT, UNSUPPORTED_FORMAT, decode};

    pub(crate) async fn page(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        after: Option<&AuditCursor>,
        expected: Option<PermissionRequester>,
    ) -> Result<AuditPage, OpError> {
        crate::server_fns::audit_page(pool, caller(org, actor), after, expected).await
    }

    pub(crate) async fn read(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        receipt: Uuid,
    ) -> Result<Option<AuditEntry>, OpError> {
        let mut read = crate::ops::begin_read(pool, caller(org, actor)).await?;
        let result = crate::ops::permissions::audit::read(&mut read.tx, &read.actor, receipt).await;
        read.finish(result).await
    }
}

pub(crate) fn forbidden() -> OpError {
    OpError::forbidden(AUTHORITY_REQUIRED)
}

/// The actor's own stored state is missing or invalid (`Actor::load`).
pub(crate) fn state_unavailable() -> OpError {
    OpError::forbidden("Permission state is unavailable")
}

/// A fault the editor logs and does not explain: exhausted revisions, an
/// unsupported receipt, or someone else's invalid stored state.
pub(crate) fn unavailable() -> OpError {
    OpError::Internal(EDITOR_UNAVAILABLE.into())
}

pub(crate) fn history_unavailable() -> OpError {
    OpError::Internal(HISTORY_UNAVAILABLE.into())
}

pub(crate) fn request_conflict() -> OpError {
    OpError::conflict(REQUEST_CONFLICT)
}

pub(crate) fn not_found() -> OpError {
    OpError::not_found(ops_profiles::PERSON_NOT_FOUND)
}

pub(crate) fn stale() -> OpError {
    OpError::conflict(ops_profiles::STALE)
}

pub(crate) fn confirmation() -> OpError {
    OpError::conflict(ops_profiles::CONFIRMATION)
}

pub(crate) fn last_administrator() -> OpError {
    OpError::conflict(ops_profiles::LAST_ADMINISTRATOR)
}

pub(crate) fn administrator_selection() -> OpError {
    OpError::invalid("grants", ops_profiles::ADMINISTRATOR_SELECTION)
}

pub(crate) fn template_stale() -> OpError {
    OpError::conflict(ops_templates::STALE)
}

pub(crate) fn template_not_found() -> OpError {
    OpError::not_found(ops_templates::NOT_FOUND)
}

pub(crate) fn name_conflict() -> OpError {
    OpError::conflict(ops_templates::NAME_CONFLICT)
}

pub(crate) fn limit() -> OpError {
    OpError::conflict(ops_templates::LIMIT)
}
