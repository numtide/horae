use super::*;
use crate::models::permission_editor::PermissionRequester;
use horae_core::permissions::{
    Actor, ManagementAssignments,
    rates::{BillableRateOwner, BillableRateResource, RateAction, billable_rate_access},
};

pub(in crate::server_fns::project_creation) async fn read(
    pool: &sqlx::PgPool,
    actor_id: Uuid,
    org_id: Uuid,
    context: ProjectEditorContext,
    search: &ProjectEditorCatalogSearch,
    email_available: bool,
) -> Result<ProjectEditorCatalog, ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    crate::server_fns::permissions::configure_administration(&mut tx)
        .await
        .map_err(storage_error)?;
    let (_, permissions) = lock_editor_actor(
        &mut tx,
        actor_id,
        org_id,
        context.project_id,
        OrganizationLock::Shared,
    )
    .await?;
    let permissions =
        permissions.ok_or_else(|| forbidden("Current project editing authority is required"))?;
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    if context.requester != requester {
        return Err(ServerFnError::ServerError {
            code: CONFLICT,
            message: "Your session changed. Reload the project before continuing.".into(),
            details: Some(serde_json::json!({
                "reason": crate::models::project_creation::PROJECT_EDITOR_SESSION_CHANGED
            })),
        });
    }
    // All-project scope still requires a real project in the authenticated tenant.
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE org_id=$1 AND id=$2)",
        org_id,
        context.project_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    if exists != Some(true) {
        return Err(forbidden("Current project editing authority is required"));
    }
    for search in [&search.clients, &search.tasks] {
        if search.query.chars().count() > 100
            || search.query.contains('\0')
            || search.offset > 10000
        {
            return Err(err(
                BAD_REQUEST,
                "Narrow the catalog search to at most 100 characters",
            ));
        }
    }
    let actor = Actor {
        id: actor_id,
        org_id,
        active: true,
    };
    let read_task_rates = billable_rate_access(
        &permissions.grants,
        RateAction::Read,
        &actor,
        &BillableRateResource {
            org_id,
            owner: BillableRateOwner::GlobalTask,
        },
        &ManagementAssignments {
            actor_id,
            org_id,
            people: &[],
            projects: &[],
        },
    );
    let organization_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1",
        org_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    let mut clients = sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, NULL::bigint AS default_rate_cents FROM clients
         WHERE org_id=$1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.clients.query.trim(),
        i64::from(search.clients.offset),
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(storage_error)?;
    let mut tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default AS billable,
           CASE WHEN $4 THEN default_rate_cents END AS default_rate_cents,
           CASE WHEN $4 THEN default_rate_currency END AS default_rate_currency
         FROM tasks WHERE org_id=$1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.tasks.query.trim(),
        i64::from(search.tasks.offset),
        read_task_rates,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(storage_error)?;
    let more_clients = clients.len() > 50;
    let more_tasks = tasks.len() > 50;
    clients.truncate(50);
    tasks.truncate(50);
    tx.commit().await.map_err(storage_error)?;
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
