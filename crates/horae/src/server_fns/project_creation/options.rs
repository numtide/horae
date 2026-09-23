use super::*;
use crate::models::project_creation::{
    CreationOptions, CreationPerson, CreationSearch, CreationSelection, CreationTask,
};

pub(super) async fn load_selected_catalog(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    task_ids: &[uuid::Uuid],
    user_ids: &[uuid::Uuid],
) -> Result<CreationSelection, ServerFnError> {
    if task_ids.len() > 500 || user_ids.len() > 500 {
        return Err(err(BAD_REQUEST, "Select at most 500 tasks and 500 people"));
    }
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default as billable, default_rate_cents, default_rate_currency FROM tasks
         WHERE org_id = $1 AND active AND id = ANY($2) ORDER BY lower(name), id",
        org_id,
        task_ids,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(storage_error)?;
    let people = sqlx::query_as!(CreationPerson,
        "SELECT id, name, billable_rate_cents, CASE WHEN $3 THEN cost_rate_cents END as cost_rate_cents FROM users
         WHERE org_id = $1 AND active AND id = ANY($2) ORDER BY lower(name), id",
        org_id, user_ids, role == OrgRole::Admin,
    ).fetch_all(&mut *tx).await.map_err(storage_error)?;
    tx.commit().await.map_err(storage_error)?;
    Ok(CreationSelection { tasks, people })
}

pub(super) async fn load_selected_client(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
) -> Result<Option<CreationClient>, ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let client = sqlx::query_as!(CreationClient,
        "SELECT id, name, currency, active, default_rate_cents FROM clients WHERE org_id = $1 AND id = $2",
        org_id, client_id,
    ).fetch_optional(&mut *tx).await.map_err(storage_error)?;
    tx.commit().await.map_err(storage_error)?;
    Ok(client)
}

pub(super) async fn load_creation_options(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    search: &CreationSearch,
    email_available: bool,
) -> Result<CreationOptions, ServerFnError> {
    for catalog in [&search.clients, &search.tasks, &search.people] {
        if catalog.query.chars().count() > 100
            || catalog.query.contains('\0')
            || catalog.offset > 10000
        {
            return Err(err(
                BAD_REQUEST,
                "Narrow the catalog search to at most 100 characters",
            ));
        }
    }
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let organization_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1",
        org_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    // Read one extra item to distinguish a complete catalog from a page.
    // Archived clients are explicitly labelled rather than silently replaced.
    let mut clients = sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, default_rate_cents FROM clients
         WHERE org_id = $1 AND strpos(lower(name), lower($2)) > 0
         ORDER BY active DESC, lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.clients.query.trim(),
        i64::from(search.clients.offset),
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(storage_error)?;
    let mut tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default as billable, default_rate_cents, default_rate_currency FROM tasks
         WHERE org_id = $1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.tasks.query.trim(),
        i64::from(search.tasks.offset),
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(storage_error)?;
    let mut people = sqlx::query_as!(CreationPerson,
        "SELECT id, name, billable_rate_cents, CASE WHEN $4 THEN cost_rate_cents END as cost_rate_cents FROM users
         WHERE org_id = $1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id, search.people.query.trim(), i64::from(search.people.offset), role == OrgRole::Admin,
    ).fetch_all(&mut *tx).await.map_err(storage_error)?;
    let previous_code = sqlx::query_scalar!(
        "SELECT code FROM projects WHERE org_id = $1 AND code IS NOT NULL AND btrim(code) <> ''
         ORDER BY created_at DESC, id DESC LIMIT 1",
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage_error)?
    .flatten();
    let suggested_code = previous_code.as_deref().and_then(|code| {
        if code.is_empty() || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let next = code.parse::<u64>().ok()?.checked_add(1)?;
        Some(format!("{next:0width$}", width = code.len()))
    });
    let (more_clients, more_tasks, more_people) =
        (clients.len() > 50, tasks.len() > 50, people.len() > 50);
    clients.truncate(50);
    tasks.truncate(50);
    people.truncate(50);
    tx.commit().await.map_err(storage_error)?;
    Ok(CreationOptions {
        organization_currency,
        can_edit_private_settings: role == OrgRole::Admin,
        email_available,
        clients,
        tasks,
        people,
        more_clients,
        more_tasks,
        more_people,
        previous_code,
        suggested_code,
    })
}
