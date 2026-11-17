use super::*;
use crate::models::project_creation::{
    CreationOptions, CreationPerson, CreationSearch, CreationSelection, CreationTask,
};

/// The selected active tasks and people, at most 500 of each, independently
/// of the catalog's pages.
pub(crate) async fn selection(
    tx: &mut Tx,
    actor: &Actor,
    task_ids: &[Uuid],
    user_ids: &[Uuid],
) -> Result<CreationSelection, OpError> {
    if task_ids.len() > 500 || user_ids.len() > 500 {
        return Err(OpError::bad_input(
            "Select at most 500 tasks and 500 people",
        ));
    }
    let private = require_creator(actor)?;
    let org_id = actor.org_id();
    let tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default as billable,
                CASE WHEN $3 THEN default_rate_cents END as default_rate_cents,
                CASE WHEN $3 THEN default_rate_currency END as default_rate_currency FROM tasks
         WHERE org_id = $1 AND active AND id = ANY($2) ORDER BY lower(name), id",
        org_id,
        task_ids,
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let people = sqlx::query_as!(
        CreationPerson,
        "SELECT id, name, CASE WHEN $4 THEN billable_rate_cents END as billable_rate_cents,
                CASE WHEN $3 THEN cost_rate_cents END as cost_rate_cents FROM users
         WHERE org_id = $1 AND active AND id = ANY($2) ORDER BY lower(name), id",
        org_id,
        user_ids,
        private.read_costs,
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(CreationSelection { tasks, people })
}

/// A draft's client, active or archived, without searching the catalog.
pub(crate) async fn client(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
) -> Result<Option<CreationClient>, OpError> {
    let private = require_creator(actor)?;
    sqlx::query_as!(CreationClient,
        "SELECT id, name, currency, active, CASE WHEN $3 THEN default_rate_cents END as default_rate_cents
         FROM clients WHERE org_id = $1 AND id = $2",
        actor.org_id(), client_id, private.read_billable,
    ).fetch_optional(&mut **tx).await.map_err(storage)
}

/// One page of each catalog the form searches, with the organization's
/// currency and the code that follows the last numeric one.
pub(crate) async fn options(
    tx: &mut Tx,
    actor: &Actor,
    search: &CreationSearch,
    email_available: bool,
) -> Result<CreationOptions, OpError> {
    for catalog in [&search.clients, &search.tasks, &search.people] {
        if catalog.query.chars().count() > 100
            || catalog.query.contains('\0')
            || catalog.offset > 10000
        {
            return Err(OpError::bad_input(
                "Narrow the catalog search to at most 100 characters",
            ));
        }
    }
    let private = require_creator(actor)?;
    let org_id = actor.org_id();
    let organization_currency = sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id = $1",
        org_id,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    // Read one extra item to distinguish a complete catalog from a page.
    // Archived clients are explicitly labelled rather than silently replaced.
    let mut clients = sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, CASE WHEN $4 THEN default_rate_cents END as default_rate_cents
         FROM clients
         WHERE org_id = $1 AND strpos(lower(name), lower($2)) > 0
         ORDER BY active DESC, lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.clients.query.trim(),
        i64::from(search.clients.offset),
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let mut tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default as billable,
                CASE WHEN $4 THEN default_rate_cents END as default_rate_cents,
                CASE WHEN $4 THEN default_rate_currency END as default_rate_currency FROM tasks
         WHERE org_id = $1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.tasks.query.trim(),
        i64::from(search.tasks.offset),
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    // A project holds at most 500 tasks, so that bounds the common ones too.
    let common_tasks = sqlx::query_as!(
        CreationTask,
        "SELECT id, name, billable_default as billable,
                CASE WHEN $2 THEN default_rate_cents END as default_rate_cents,
                CASE WHEN $2 THEN default_rate_currency END as default_rate_currency FROM tasks
         WHERE org_id = $1 AND active AND is_default
         ORDER BY lower(name), id LIMIT 500",
        org_id,
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let mut people = sqlx::query_as!(
        CreationPerson,
        "SELECT id, name, CASE WHEN $5 THEN billable_rate_cents END as billable_rate_cents,
                CASE WHEN $4 THEN cost_rate_cents END as cost_rate_cents FROM users
         WHERE org_id = $1 AND active AND strpos(lower(name), lower($2)) > 0
         ORDER BY lower(name), id LIMIT 51 OFFSET $3",
        org_id,
        search.people.query.trim(),
        i64::from(search.people.offset),
        private.read_costs,
        private.read_billable,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let previous_code = sqlx::query_scalar!(
        "SELECT code FROM projects WHERE org_id = $1 AND code IS NOT NULL AND btrim(code) <> ''
         ORDER BY created_at DESC, id DESC LIMIT 1",
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
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
    Ok(CreationOptions {
        organization_currency,
        can_edit_cost_rates: private.write_costs,
        can_edit_private_notes: private.write_notes,
        can_edit_billable_settings: private.write_billable,
        email_available,
        clients,
        tasks,
        common_tasks,
        people,
        more_clients,
        more_tasks,
        more_people,
        previous_code,
        suggested_code,
    })
}
