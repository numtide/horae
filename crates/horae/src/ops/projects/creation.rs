//! Creating a project: the creator's draft ([`load_draft`], [`save_draft`],
//! [`discard_draft`]), the catalogs the form offers ([`options`], [`client`],
//! [`selection`]) and turning the draft into a project ([`finalize`]).
//!
//! Each needs `ProjectCreateAll`. Run them in `ops::begin_gated`: the
//! organization's `Shared` gate for everything but [`finalize`], which takes
//! `AccessChange` because it designates the new project's managers.

use horae_core::permissions::catalog::Permission;
use horae_core::project::BudgetMode;
use horae_core::types::ProjectType;
use uuid::Uuid;

use super::form::{form_payload, storage};
use crate::models::project_creation::{
    CreationClient, DraftSaved, InvoiceDefaultsInput, ProjectDraft, ProjectForm,
};
use crate::ops::{Actor, Done, OpError, Tx};

mod finalize;
mod options;

pub(crate) use finalize::finalize;
pub(crate) use options::{client, options, selection};

const CREATE_REQUIRED: &str = "Project creation access required";

/// Refuses anyone without `ProjectCreateAll`, and answers what the creator
/// may write in the form's protected fields.
fn require_creator(actor: &Actor) -> Result<PrivateFields, OpError> {
    if !actor.has(Permission::ProjectCreateAll) {
        return Err(OpError::forbidden(CREATE_REQUIRED));
    }
    Ok(PrivateFields::of(actor))
}

/// The creator's open draft, without the protected fields they may no
/// longer write.
pub(crate) async fn load_draft(
    tx: &mut Tx,
    actor: &Actor,
) -> Result<Option<ProjectDraft>, OpError> {
    let private = require_creator(actor)?;
    let row = sqlx::query!(
        r#"SELECT id, revision, updated_at as "updated_at: chrono::DateTime<chrono::Utc>", payload FROM project_drafts
         WHERE org_id = $1 AND creator_id = $2
           AND completed_project_id IS NULL AND discarded_at IS NULL"#,
        actor.org_id(), actor.id(),
    ).fetch_optional(&mut **tx).await.map_err(storage)?;
    row.map(|row| {
        let mut form: ProjectForm = serde_json::from_value(row.payload)
            .map_err(|_| OpError::Internal("This project draft cannot be read".into()))?;
        // A creator may have lost a grant since saving private fields.
        private.prune(&mut form);
        Ok(ProjectDraft {
            id: row.id,
            revision: row.revision,
            saved_at: row.updated_at,
            form,
        })
    })
    .transpose()
}

/// Saves the creator's draft at `expected_revision + 1`. Revision 0 opens a
/// new draft under the client's `draft_id`. Repeating the last save answers
/// the same revision without writing again.
pub(crate) async fn save_draft(
    tx: &mut Tx,
    actor: &Actor,
    draft_id: Uuid,
    expected_revision: i64,
    form: &ProjectForm,
) -> Result<Done<DraftSaved>, OpError> {
    if expected_revision < 0 || expected_revision == i64::MAX || draft_id.get_version_num() != 7 {
        return Err(OpError::bad_input("Invalid draft identity or revision"));
    }
    let payload = validate_draft_form(form, require_creator(actor)?)?;
    let (org_id, actor_id) = (actor.org_id(), actor.id());
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
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    }
    let row = sqlx::query!(
        r#"SELECT revision, updated_at as "updated_at: chrono::DateTime<chrono::Utc>", payload, completed_project_id, discarded_at
         FROM project_drafts WHERE id = $1 AND org_id = $2 AND creator_id = $3 FOR UPDATE"#,
        draft_id, org_id, actor_id,
    ).fetch_optional(&mut **tx).await.map_err(storage)?
        .ok_or_else(|| if expected_revision == 0 {
            OpError::conflict("A draft is already open. Reload the current draft before saving.")
        } else {
            OpError::not_found("Project draft not found")
        })?;
    if row.completed_project_id.is_some() || row.discarded_at.is_some() {
        return Err(OpError::conflict(
            "This draft has already been completed or discarded",
        ));
    }
    if row.revision == expected_revision + 1 && row.payload == payload {
        // A response may be lost after COMMIT; repeating that exact write
        // acknowledges the same revision without writing again.
        return Ok(Done::new(DraftSaved {
            id: draft_id,
            revision: row.revision,
            saved_at: row.updated_at,
        }));
    }
    if row.revision != expected_revision {
        return Err(OpError::conflict(
            "Draft changed in another tab. Reload before saving.",
        ));
    }
    let saved = sqlx::query_as!(DraftSaved,
        r#"UPDATE project_drafts SET payload = $2, revision = revision + 1, updated_at = now()
         WHERE id = $1 RETURNING id, revision, updated_at as "saved_at: chrono::DateTime<chrono::Utc>""#,
        draft_id, payload,
    ).fetch_one(&mut **tx).await.map_err(storage)?;
    Ok(Done::new(saved))
}

/// Discards the creator's draft at `expected_revision`.
pub(crate) async fn discard_draft(
    tx: &mut Tx,
    actor: &Actor,
    draft_id: Uuid,
    expected_revision: i64,
) -> Result<Done<()>, OpError> {
    require_creator(actor)?;
    let (org_id, actor_id) = (actor.org_id(), actor.id());
    let affected = sqlx::query!(
        "UPDATE project_drafts SET discarded_at = now(), updated_at = now()
         WHERE id = $1 AND org_id = $2 AND creator_id = $3 AND revision = $4
           AND completed_project_id IS NULL AND discarded_at IS NULL",
        draft_id,
        org_id,
        actor_id,
        expected_revision,
    )
    .execute(&mut **tx)
    .await
    .map_err(storage)?
    .rows_affected();
    if affected != 1 {
        let owned = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM project_drafts WHERE id = $1 AND org_id = $2 AND creator_id = $3) as "owned!""#,
            draft_id, org_id, actor_id,
        ).fetch_one(&mut **tx).await.map_err(storage)?;
        if !owned {
            return Err(OpError::not_found("Project draft not found"));
        }
        return Err(OpError::conflict(
            "Draft is unavailable or changed. Reload before discarding.",
        ));
    }
    Ok(Done::new(()))
}

/// The draft as stored, refused when it fills a protected field the creator
/// may not write.
pub(crate) fn validate_draft_form(
    form: &ProjectForm,
    private: PrivateFields,
) -> Result<serde_json::Value, OpError> {
    if !private.write_notes && !form.admin_notes.is_empty() {
        return Err(OpError::forbidden(
            "Saving private notes requires permission to edit every project",
        ));
    }
    if !private.write_costs && form.team.iter().any(|member| !member.cost_rate.is_empty()) {
        return Err(OpError::forbidden(
            "Saving cost rates requires permission to edit cost rates",
        ));
    }
    if !private.write_billable && PrivateFields::has_billable_settings(form) {
        return Err(OpError::forbidden(
            "Setting rates, fees, money budgets or invoice defaults requires permission to edit billable rates",
        ));
    }
    form_payload(form)
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
#[derive(Clone, Copy)]
pub(crate) struct PrivateFields {
    pub read_costs: bool,
    pub read_billable: bool,
    pub write_costs: bool,
    pub write_notes: bool,
    pub write_billable: bool,
}

impl PrivateFields {
    fn of(actor: &Actor) -> Self {
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
            || form.invoice_defaults != InvoiceDefaultsInput::default()
    }

    /// Clears what the creator may no longer write, as when a saved draft is
    /// reopened after its creator lost a grant.
    pub fn prune(self, form: &mut ProjectForm) {
        if !self.write_notes {
            form.admin_notes.clear();
        }
        if !self.write_costs {
            for member in &mut form.team {
                member.cost_rate.clear();
            }
        }
        if !self.write_billable {
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

/// The project's client: active and in the organization, held `FOR SHARE`
/// until the project is created.
pub(crate) async fn lock_client(
    tx: &mut Tx,
    client_id: Uuid,
    org_id: Uuid,
) -> Result<CreationClient, OpError> {
    sqlx::query_as!(
        CreationClient,
        "SELECT id, name, currency, active, default_rate_cents FROM clients
         WHERE id = $1 AND org_id = $2 AND active FOR SHARE",
        client_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?
    .ok_or_else(|| OpError::not_found("Select an active client in this organization"))
}
