//! Session-bound project delegation, independent of global permission administration.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project_managers::{
    ProjectManagers, ProjectManagersCommand, ProjectManagersOutcome,
};
#[cfg(feature = "server")]
use crate::ops::{self, Caller, EventSink, OpError, WriteIsolation};
#[cfg(feature = "server")]
use sqlx::PgPool;

/// Load every retained manager under current authority to edit this project.
#[server]
pub async fn load_project_managers(
    project_id: uuid::Uuid,
    expected_requester: Option<PermissionRequester>,
) -> Result<ProjectManagers, ServerFnError> {
    let caller = session_caller_or("Project managers are unavailable").await?;
    let state = crate::state::global_state().await;
    project_managers_view(&state.db, caller, project_id, expected_requester)
        .await
        .map_err(op_error)
}

/// Replace the complete manager set without changing grants or tracking membership.
#[server]
pub async fn save_project_managers(
    command: ProjectManagersCommand,
    expected_requester: PermissionRequester,
) -> Result<ProjectManagersOutcome, ServerFnError> {
    let caller = session_caller_or("Project managers are unavailable").await?;
    let state = crate::state::global_state().await;
    project_managers_save(
        &state.db,
        caller,
        &command,
        expected_requester,
        &*state.plugins,
    )
    .await
    .map_err(op_error)
}

#[cfg(feature = "server")]
pub(crate) async fn project_managers_view(
    db: &PgPool,
    caller: Caller,
    project_id: uuid::Uuid,
    expected: Option<PermissionRequester>,
) -> Result<ProjectManagers, OpError> {
    let mut read = ops::begin_read(db, caller).await?;
    let result =
        ops::projects::managers::read(&mut read.tx, &read.actor, project_id, expected).await;
    read.finish(result).await
}

/// Designating managers changes access, so it holds the organization's
/// `AccessChange` gate: concurrent reads and access changes wait for it.
#[cfg(feature = "server")]
pub(crate) async fn project_managers_save(
    db: &PgPool,
    caller: Caller,
    command: &ProjectManagersCommand,
    expected: PermissionRequester,
    events: &dyn EventSink,
) -> Result<ProjectManagersOutcome, OpError> {
    let mut write = ops::begin_gated(
        db,
        caller,
        crate::db::OrganizationLock::AccessChange,
        WriteIsolation::ReadCommitted,
    )
    .await?;
    let result =
        ops::projects::managers::save(&mut write.tx, &write.actor, command, expected).await;
    write.commit(result, events).await
}

/// The manager editor as the tests call it, as the person a test names: the
/// page is bound to that person.
#[cfg(test)]
pub(crate) mod calls {
    use super::*;
    use crate::ops::testing::Recorder;

    pub(crate) async fn read(
        pool: &PgPool,
        org: uuid::Uuid,
        actor: uuid::Uuid,
        project_id: uuid::Uuid,
    ) -> Result<ProjectManagers, OpError> {
        project_managers_view(pool, Caller::session(org, actor), project_id, None).await
    }

    pub(crate) async fn execute(
        pool: &PgPool,
        org: uuid::Uuid,
        actor: uuid::Uuid,
        command: &ProjectManagersCommand,
    ) -> Result<ProjectManagersOutcome, OpError> {
        let requester = PermissionRequester {
            org_id: org,
            user_id: actor,
        };
        let caller = Caller::session(org, actor);
        project_managers_save(pool, caller, command, requester, &Recorder::default()).await
    }

    /// The project editor's delegation step, inside a transaction that holds
    /// the `AccessChange` gate as the editor's does.
    pub(crate) async fn execute_in_transaction(
        tx: &mut ops::Tx,
        org: uuid::Uuid,
        actor: uuid::Uuid,
        command: &ProjectManagersCommand,
    ) -> Result<ProjectManagersOutcome, OpError> {
        let actor = ops::Actor::load(tx, Caller::session(org, actor)).await?;
        ops::projects::managers::replace(tx, &actor, command).await
    }
}
