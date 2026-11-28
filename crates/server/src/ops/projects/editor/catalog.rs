//! The clients and tasks the editor offers, searched and paged.

use super::*;
use crate::models::project_creation::{
    PROJECT_EDITOR_SESSION_CHANGED, ProjectEditorCatalog, ProjectEditorCatalogSearch,
    ProjectEditorContext,
};
use horae_core::permissions::{
    ManagementAssignments,
    rates::{BillableRateOwner, BillableRateResource, RateAction, billable_rate_access},
};

/// One page of active clients and tasks for the editor of `context`'s
/// project, bound to the requester the editor was loaded for. Task rates show
/// only to someone who may read the task catalog's billable rates.
pub async fn read(
    tx: &mut Tx,
    actor: &Actor,
    context: ProjectEditorContext,
    search: &ProjectEditorCatalogSearch,
    email_available: bool,
) -> Result<ProjectEditorCatalog, OpError> {
    let org_id = actor.org_id();
    authorize_write(tx, actor, context.project_id).await?;
    let requester = actor
        .expect_requester(Some(context.requester), SESSION_CHANGED)
        .map_err(|_| session_changed())?;
    // All-project scope still requires a real project in the authenticated tenant.
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE org_id=$1 AND id=$2)",
        org_id,
        context.project_id,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if exists != Some(true) {
        return Err(OpError::forbidden(EDITING_REQUIRED));
    }
    for search in [&search.clients, &search.tasks] {
        if search.query.chars().count() > 100
            || search.query.contains('\0')
            || search.offset > 10000
        {
            return Err(OpError::bad_input(
                "Narrow the catalog search to at most 100 characters",
            ));
        }
    }
    let grants = actor
        .grants()
        .ok_or_else(|| OpError::forbidden(EDITING_REQUIRED))?;
    let read_task_rates = billable_rate_access(
        grants,
        RateAction::Read,
        actor.identity(),
        &BillableRateResource {
            org_id,
            owner: BillableRateOwner::GlobalTask,
        },
        &ManagementAssignments {
            actor_id: actor.id(),
            org_id,
            people: &[],
            projects: &[],
        },
    );
    let organization_currency = actor.organization()?.default_currency.clone();
    let mut clients = sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, NULL::bigint AS default_rate_cents FROM clients
         WHERE org_id=$1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.clients.query.trim(),
        i64::from(search.clients.offset),
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let mut tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default AS billable,
           CASE WHEN $4 THEN default_rate_cents END AS default_rate_cents
         FROM tasks WHERE org_id=$1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.tasks.query.trim(),
        i64::from(search.tasks.offset),
        read_task_rates,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let more_clients = clients.len() > 50;
    let more_tasks = tasks.len() > 50;
    clients.truncate(50);
    tasks.truncate(50);
    Ok(ProjectEditorCatalog {
        context: ProjectEditorContext {
            requester,
            project_id: context.project_id,
        },
        organization_currency,
        email_available,
        clients,
        tasks,
        more_clients,
        more_tasks,
    })
}

const SESSION_CHANGED: &str = "Your session changed. Reload the project before continuing.";

/// The catalog was asked for by a page loaded for someone else: a conflict the
/// page answers by reloading, which [`PROJECT_EDITOR_SESSION_CHANGED`] tells it.
fn session_changed() -> OpError {
    OpError::conflict(SESSION_CHANGED).with_reason(PROJECT_EDITOR_SESSION_CHANGED)
}
