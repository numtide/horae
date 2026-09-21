//! Transactional creation and creator-owned draft persistence.

use super::*;
#[cfg(feature = "server")]
use crate::models::project_creation::TaskAccess;
use crate::models::project_creation::{
    CreationClient, CreationOptions, CreationSearch, CreationSelection, DraftSaved, ProjectDraft,
    ProjectForm,
};

#[cfg(feature = "server")]
mod validation;
#[cfg(feature = "server")]
use validation::validate_project_form;
#[cfg(feature = "server")]
mod finalize;
#[cfg(feature = "server")]
use finalize::finalize_draft_record;
#[cfg(feature = "server")]
mod options;
#[cfg(feature = "server")]
use options::{load_creation_options, load_selected_catalog, load_selected_client};

#[cfg(all(test, feature = "server"))]
mod assignment_tests;

#[cfg(all(test, feature = "server"))]
mod import_tests;

/// Resolve only the selected active task/person identities, independently of pagination.
#[server]
pub async fn project_creation_selection(
    task_ids: Vec<uuid::Uuid>,
    user_ids: Vec<uuid::Uuid>,
) -> Result<CreationSelection, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    load_selected_catalog(&state.db, actor.id, actor.org_id, &task_ids, &user_ids).await
}

/// Resolve an existing draft selection without searching or fetching a whole catalog.
#[server]
pub async fn project_creation_client(
    client_id: uuid::Uuid,
) -> Result<Option<CreationClient>, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    load_selected_client(&state.db, actor.id, actor.org_id, client_id).await
}

#[server]
pub async fn project_creation_options(
    search: CreationSearch,
) -> Result<CreationOptions, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    load_creation_options(
        &state.db,
        actor.id,
        actor.org_id,
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
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    finalize_draft_record(
        &state.db,
        actor.id,
        actor.org_id,
        draft_id,
        expected_revision,
        &form,
        state.mail.is_some(),
    )
    .await
}

#[server]
pub async fn load_project_draft() -> Result<Option<ProjectDraft>, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    load_draft_record(&state.db, actor.id, actor.org_id).await
}

#[server]
pub async fn save_project_draft(
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: ProjectForm,
) -> Result<DraftSaved, ServerFnError> {
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    save_draft_record(
        &state.db,
        actor.id,
        actor.org_id,
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
    let actor = require_manager().await?;
    let state = crate::state::global_state().await;
    discard_draft_record(
        &state.db,
        actor.id,
        actor.org_id,
        draft_id,
        expected_revision,
    )
    .await
}

#[cfg(feature = "server")]
fn storage_error(error: sqlx::Error) -> ServerFnError {
    tracing::error!(code = ?error.as_database_error().and_then(|error| error.code()), "Project creation database operation failed");
    server_err("Could not save or load the project. Please retry.")
}

#[cfg(feature = "server")]
pub(super) async fn create_client_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    name: &str,
    currency: &str,
    default_rate: &str,
) -> Result<CreationClient, ServerFnError> {
    if !(1..=200).contains(&name.trim().chars().count()) || name.contains('\0') {
        return Err(err(
            BAD_REQUEST,
            "Client name must contain 1–200 characters",
        ));
    }
    if !horae_core::project::PROJECT_CURRENCIES.contains(&currency) {
        return Err(err(BAD_REQUEST, "Choose a supported client currency"));
    }
    let rate = validation::optional_amount(default_rate, "Client default rate")?;
    let mut tx = pool.begin().await.map_err(storage_error)?;
    lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let client = sqlx::query_as!(
        CreationClient,
        "INSERT INTO clients (id, org_id, name, currency, default_rate_cents)
         VALUES ($1,$2,$3,$4,$5) RETURNING id, name, currency, active, default_rate_cents",
        uuid::Uuid::now_v7(),
        org_id,
        name.trim(),
        currency,
        rate,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(storage_error)?;
    tx.commit().await.map_err(storage_error)?;
    Ok(client)
}

#[cfg(feature = "server")]
async fn load_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<ProjectDraft>, ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let row = sqlx::query!(
        r#"SELECT id, revision, updated_at as "updated_at: chrono::DateTime<chrono::Utc>", payload FROM project_drafts
         WHERE org_id = $1 AND creator_id = $2
           AND completed_project_id IS NULL AND discarded_at IS NULL"#,
        org_id, actor_id,
    ).fetch_optional(&mut *tx).await.map_err(storage_error)?;
    let result = row
        .map(|row| {
            let mut form: ProjectForm = serde_json::from_value(row.payload)
                .map_err(|_| server_err("This project draft cannot be read"))?;
            // A creator may have been demoted since saving private fields.
            if role != OrgRole::Admin {
                form.admin_notes.clear();
                for member in &mut form.team {
                    member.cost_rate.clear();
                }
            }
            Ok(ProjectDraft {
                id: row.id,
                revision: row.revision,
                saved_at: row.updated_at,
                form,
            })
        })
        .transpose();
    tx.commit().await.map_err(storage_error)?;
    result
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
    if expected_revision < 0 || expected_revision == i64::MAX || draft_id.get_version_num() != 7 {
        return Err(err(BAD_REQUEST, "Invalid draft identity or revision"));
    }
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let role = lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let payload = validate_draft_form(form, role == OrgRole::Admin)?;
    if expected_revision == 0 {
        // Both a repeated request ID and competing initial tabs are safe: the
        // ownership/state check below decides which conflict can be retried.
        sqlx::query!(
            "INSERT INTO project_drafts (id, org_id, creator_id, payload)
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            draft_id,
            org_id,
            actor_id,
            payload,
        )
        .execute(&mut *tx)
        .await
        .map_err(storage_error)?;
    }
    let row = sqlx::query!(
        r#"SELECT revision, updated_at as "updated_at: chrono::DateTime<chrono::Utc>", payload, completed_project_id, discarded_at
         FROM project_drafts WHERE id = $1 AND org_id = $2 AND creator_id = $3 FOR UPDATE"#,
        draft_id, org_id, actor_id,
    ).fetch_optional(&mut *tx).await.map_err(storage_error)?
        .ok_or_else(|| if expected_revision == 0 {
            conflict("A draft is already open. Reload the current draft before saving.")
        } else {
            not_found("Project draft not found")
        })?;
    if row.completed_project_id.is_some() || row.discarded_at.is_some() {
        return Err(conflict(
            "This draft has already been completed or discarded",
        ));
    }
    let saved = if row.revision == expected_revision + 1 && row.payload == payload {
        // A response may be lost after COMMIT; repeating that exact write
        // acknowledges the same revision without writing again.
        DraftSaved {
            id: draft_id,
            revision: row.revision,
            saved_at: row.updated_at,
        }
    } else {
        if row.revision != expected_revision {
            return Err(conflict(
                "Draft changed in another tab. Reload before saving.",
            ));
        }
        sqlx::query_as!(DraftSaved,
            r#"UPDATE project_drafts SET payload = $2, revision = revision + 1, updated_at = now()
             WHERE id = $1 RETURNING id, revision, updated_at as "saved_at: chrono::DateTime<chrono::Utc>""#,
            draft_id, payload,
        ).fetch_one(&mut *tx).await.map_err(storage_error)?
    };
    tx.commit().await.map_err(storage_error)?;
    Ok(saved)
}

#[cfg(feature = "server")]
async fn discard_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    draft_id: uuid::Uuid,
    expected_revision: i64,
) -> Result<(), ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    lock_creation_actor(&mut tx, actor_id, org_id).await?;
    let affected = sqlx::query!(
        "UPDATE project_drafts SET discarded_at = now(), updated_at = now()
         WHERE id = $1 AND org_id = $2 AND creator_id = $3 AND revision = $4
           AND completed_project_id IS NULL AND discarded_at IS NULL",
        draft_id,
        org_id,
        actor_id,
        expected_revision,
    )
    .execute(&mut *tx)
    .await
    .map_err(storage_error)?
    .rows_affected();
    if affected != 1 {
        let owned = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM project_drafts WHERE id = $1 AND org_id = $2 AND creator_id = $3) as "owned!""#,
            draft_id, org_id, actor_id,
        ).fetch_one(&mut *tx).await.map_err(storage_error)?;
        if !owned {
            return Err(not_found("Project draft not found"));
        }
        return Err(conflict(
            "Draft is unavailable or changed. Reload before discarding.",
        ));
    }
    tx.commit().await.map_err(storage_error)?;
    Ok(())
}

#[cfg(feature = "server")]
fn validate_draft_form(
    form: &ProjectForm,
    actor_is_admin: bool,
) -> Result<serde_json::Value, ServerFnError> {
    if !actor_is_admin
        && (!form.admin_notes.is_empty()
            || form.team.iter().any(|member| !member.cost_rate.is_empty()))
    {
        return Err(forbidden(
            "Only administrators can save private notes or cost rates",
        ));
    }
    if form.tasks.len() > 500
        || form.team.len() > 500
        || form.tags.len() > 50
        || form.milestones.len() > 100
        || form.tasks.iter().any(|task| matches!(&task.access, TaskAccess::Restricted { user_ids } if user_ids.len() > 500))
    {
        return Err(err(BAD_REQUEST, "Too many project tasks, people, tags or milestones"));
    }
    let payload = serde_json::to_value(form).map_err(server_err)?;
    fn contains_nul(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::String(text) => text.contains('\0'),
            serde_json::Value::Array(values) => values.iter().any(contains_nul),
            serde_json::Value::Object(values) => values.values().any(contains_nul),
            _ => false,
        }
    }
    if contains_nul(&payload) {
        return Err(err(
            BAD_REQUEST,
            "Project fields cannot contain a null character",
        ));
    }
    // The indented representation is a conservative bound for PostgreSQL's
    // jsonb text size, which also includes spaces after keys and separators.
    if serde_json::to_string_pretty(&payload)
        .map_err(server_err)?
        .len()
        > 256 * 1024
    {
        return Err(err(BAD_REQUEST, "Project draft exceeds 256 KiB"));
    }
    Ok(payload)
}

#[cfg(feature = "server")]
async fn lock_creation_actor(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<OrgRole, ServerFnError> {
    let role = sqlx::query_scalar!(
        r#"SELECT org_role as "org_role: OrgRole" FROM users
           WHERE id = $1 AND org_id = $2 AND active FOR SHARE"#,
        actor_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| forbidden("Active manager access required"))?;
    if !role.is_manager_or_above() {
        return Err(forbidden("Manager access required"));
    }
    Ok(role)
}

#[cfg(feature = "server")]
async fn lock_creation_client(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    client_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<CreationClient, ServerFnError> {
    sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, default_rate_cents FROM clients
         WHERE id = $1 AND org_id = $2 AND active FOR SHARE",
        client_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage_error)?
    .ok_or_else(|| not_found("Select an active client in this organization"))
}

#[cfg(all(test, feature = "server"))]
mod tests;
