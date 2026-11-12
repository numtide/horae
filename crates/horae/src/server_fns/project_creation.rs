//! Transactional creation and creator-owned draft persistence.

use super::*;
#[cfg(feature = "server")]
use crate::db::{OrganizationLock, lock_organization};
#[cfg(feature = "server")]
use crate::models::project_creation::TaskAccess;
use crate::models::project_creation::{
    CreationClient, CreationOptions, CreationSearch, CreationSelection, DraftSaved,
    EditableProject, ProjectDraft, ProjectEditRequest, ProjectEditorCatalog,
    ProjectEditorCatalogSearch, ProjectEditorContext, ProjectForm,
};
use crate::models::project_people::{
    ProjectPeopleContext, ProjectPeopleQuery, ProjectPeopleResult,
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
pub(super) mod editing;
#[cfg(feature = "server")]
mod options;
#[cfg(feature = "server")]
use options::{load_creation_options, load_selected_catalog, load_selected_client};

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
    let actor = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate project people request");
            server_err("Project people are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    permissions::project_people::read(&state.db, actor.org_id, actor.id, context, &query)
        .await
        .map_err(|error| match error {
            permissions::project_people::ProjectPeopleError::Forbidden => {
                forbidden("Current project editing authority is required")
            }
            permissions::project_people::ProjectPeopleError::InvalidInput => {
                err(BAD_REQUEST, "Invalid project people query")
            }
            error => {
                tracing::error!(%error, "Project people read failed");
                server_err("Project people are unavailable")
            }
        })
}

/// Read catalogs for the canonical editor under its current operation authority.
#[server]
pub async fn project_editor_catalog(
    context: ProjectEditorContext,
    search: ProjectEditorCatalogSearch,
) -> Result<ProjectEditorCatalog, ServerFnError> {
    let actor = require_user().await.map_err(|error| match error {
        error @ ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => error,
        error => {
            tracing::error!(%error, "Unable to authenticate project catalog request");
            server_err("Project catalogs are unavailable")
        }
    })?;
    let state = crate::state::global_state().await;
    editing::catalog::read(
        &state.db,
        actor.id,
        actor.org_id,
        context,
        &search,
        state.mail.is_some(),
    )
    .await
}

/// Load the existing project into the same form used for creation.
#[server]
pub async fn load_project_editor(project_id: uuid::Uuid) -> Result<EditableProject, ServerFnError> {
    let actor = require_user().await?;
    let state = crate::state::global_state().await;
    editing::load_editable_project(&state.db, actor.id, actor.org_id, project_id).await
}

#[server]
pub async fn save_project_editor(request: ProjectEditRequest) -> Result<uuid::Uuid, ServerFnError> {
    let actor = require_user().await?;
    let state = crate::state::global_state().await;
    let (project, changed) = editing::save_editable_project(
        &state.db,
        actor.id,
        actor.org_id,
        &request,
        state.mail.is_some(),
    )
    .await?;
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::ProjectUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: actor.org_id,
                project: project_payload(&project),
            });
    }
    Ok(project.id)
}

/// Resolve only the selected active task/person identities, independently of pagination.
#[server]
pub async fn project_creation_selection(
    task_ids: Vec<uuid::Uuid>,
    user_ids: Vec<uuid::Uuid>,
) -> Result<CreationSelection, ServerFnError> {
    let actor = require_user().await?;
    let state = crate::state::global_state().await;
    load_selected_catalog(&state.db, actor.id, actor.org_id, &task_ids, &user_ids).await
}

/// Resolve an existing draft selection without searching or fetching a whole catalog.
#[server]
pub async fn project_creation_client(
    client_id: uuid::Uuid,
) -> Result<Option<CreationClient>, ServerFnError> {
    let actor = require_user().await?;
    let state = crate::state::global_state().await;
    load_selected_client(&state.db, actor.id, actor.org_id, client_id).await
}

#[server]
pub async fn project_creation_options(
    search: CreationSearch,
) -> Result<CreationOptions, ServerFnError> {
    let actor = require_user().await?;
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
    let actor = require_user().await?;
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
    let actor = require_user().await?;
    let state = crate::state::global_state().await;
    load_draft_record(&state.db, actor.id, actor.org_id).await
}

#[server]
pub async fn save_project_draft(
    draft_id: uuid::Uuid,
    expected_revision: i64,
    form: ProjectForm,
) -> Result<DraftSaved, ServerFnError> {
    let actor = require_user().await?;
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
    let actor = require_user().await?;
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
    if error
        .as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| matches!(code.as_ref(), "40001" | "40P01"))
    {
        return conflict("The project changed while saving. Reload before retrying.");
    }
    tracing::error!(code = ?error.as_database_error().and_then(|error| error.code()), "Project creation database operation failed");
    server_err("Could not save or load the project. Please retry.")
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
async fn load_draft_record(
    pool: &sqlx::PgPool,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<ProjectDraft>, ServerFnError> {
    let mut tx = pool.begin().await.map_err(storage_error)?;
    let actor = lock_creation_actor(&mut tx, actor_id, org_id, OrganizationLock::Shared).await?;
    let private = PrivateFields::of(&actor);
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
            // A creator may have lost a grant since saving private fields.
            private.prune(&mut form);
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
    let actor = lock_creation_actor(&mut tx, actor_id, org_id, OrganizationLock::Shared).await?;
    let payload = validate_draft_form(form, PrivateFields::of(&actor))?;
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
    lock_creation_actor(&mut tx, actor_id, org_id, OrganizationLock::Shared).await?;
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
    private: PrivateFields,
) -> Result<serde_json::Value, ServerFnError> {
    if !private.write_notes && !form.admin_notes.is_empty() {
        return Err(forbidden(
            "Saving private notes requires permission to edit every project",
        ));
    }
    if !private.write_costs && form.team.iter().any(|member| !member.cost_rate.is_empty()) {
        return Err(forbidden(
            "Saving cost rates requires permission to edit cost rates",
        ));
    }
    if !private.write_billable && PrivateFields::has_billable_settings(form) {
        return Err(forbidden(
            "Setting rates, fees, money budgets or invoice defaults requires permission to edit billable rates",
        ));
    }
    form_payload(form)
}

/// The form as stored JSON, within the size and content limits shared by
/// drafts and edits.
#[cfg(feature = "server")]
fn form_payload(form: &ProjectForm) -> Result<serde_json::Value, ServerFnError> {
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

/// What a creator may put in a draft's protected fields: cost rates with
/// `CostRateWriteAll`, private notes with `ProjectWriteAll`, and billable
/// settings (see [`Self::has_billable_settings`]) with `BillableRateWriteAll`.
/// Blank rates inherit and untouched invoice defaults are the form's own, so
/// creating a time-and-materials or non-billable project needs no such grant.
///
/// The catalogs the form offers show the people's, clients' and tasks'
/// billable rates only with `BillableRateReadAll`: a project being created has
/// no managers yet, so `BillableRateReadManaged` does not apply.
#[cfg(feature = "server")]
#[derive(Clone, Copy)]
struct PrivateFields {
    read_costs: bool,
    read_billable: bool,
    write_costs: bool,
    write_notes: bool,
    write_billable: bool,
}

#[cfg(feature = "server")]
impl PrivateFields {
    fn of(actor: &crate::ops::Actor) -> Self {
        use horae_core::permissions::catalog::Permission;
        Self {
            read_costs: actor.has(Permission::CostRateReadAll),
            read_billable: actor.has(Permission::BillableRateReadAll),
            write_costs: actor.has(Permission::CostRateWriteAll),
            write_notes: actor.has(Permission::ProjectWriteAll),
            write_billable: actor.has(Permission::BillableRateWriteAll),
        }
    }

    /// What the editor protects as `billable`, except the project type and
    /// rate mode a new project must choose: rates, a fixed fee, a budget in
    /// money and invoice defaults that differ from the organization's.
    fn has_billable_settings(form: &ProjectForm) -> bool {
        use horae_core::project::BudgetMode;
        !form.project_rate.is_empty()
            || form
                .team
                .iter()
                .any(|member| !member.billable_rate.is_empty())
            || form.tasks.iter().any(|task| !task.rate.is_empty())
            || (form.project_type == ProjectType::FixedFee
                && (!form.fee_amount.is_empty() || !form.milestones.is_empty()))
            || matches!(
                form.budget_mode,
                BudgetMode::TotalFees | BudgetMode::FeesPerTask
            )
            || form.invoice_defaults
                != crate::models::project_creation::InvoiceDefaultsInput::default()
    }

    /// Clears what the creator may no longer write, as when a saved draft is
    /// reopened after its creator lost a grant.
    fn prune(self, form: &mut ProjectForm) {
        if !self.write_notes {
            form.admin_notes.clear();
        }
        if !self.write_costs {
            for member in &mut form.team {
                member.cost_rate.clear();
            }
        }
        if !self.write_billable {
            use horae_core::project::BudgetMode;
            form.project_rate.clear();
            for member in &mut form.team {
                member.billable_rate.clear();
            }
            for task in &mut form.tasks {
                task.rate.clear();
            }
            form.fee_amount.clear();
            form.milestones.clear();
            if matches!(
                form.budget_mode,
                BudgetMode::TotalFees | BudgetMode::FeesPerTask
            ) {
                form.budget_mode = BudgetMode::None;
                form.budget_value.clear();
                for task in &mut form.tasks {
                    task.budget.clear();
                }
            }
            form.invoice_defaults = Default::default();
        }
    }
}

/// Takes the organization `gate`, then loads the actor (whose own locks must
/// come after the gate: a gate is never upgraded) and requires
/// `ProjectCreateAll`. The gate is why creation authorizes here rather than
/// through `ops::begin`.
#[cfg(feature = "server")]
pub(super) async fn lock_creation_actor(
    tx: &mut crate::ops::Tx,
    actor_id: uuid::Uuid,
    org_id: uuid::Uuid,
    gate: OrganizationLock,
) -> Result<crate::ops::Actor, ServerFnError> {
    use horae_core::permissions::catalog::Permission;
    lock_organization(tx, org_id, gate)
        .await
        .map_err(storage_error)?;
    let actor = crate::ops::Actor::load(tx, crate::ops::Caller::session(org_id, actor_id)).await?;
    if !actor.has(Permission::ProjectCreateAll) {
        return Err(forbidden("Project creation access required"));
    }
    Ok(actor)
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
