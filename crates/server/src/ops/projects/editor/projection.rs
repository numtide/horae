use super::*;
use crate::models::project_creation::{ProjectEditorAccess, ProjectFieldAccess};
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::{
    ManagementAssignments,
    rates::{
        BillableRateOwner, BillableRateResource, RateAction, billable_rate_access, cost_rate_access,
    },
};

fn field_access(read: bool, write: bool) -> ProjectFieldAccess {
    if !read {
        ProjectFieldAccess::Withheld
    } else if write {
        ProjectFieldAccess::Editable
    } else {
        ProjectFieldAccess::ReadOnly
    }
}

/// Project access is already held; catalog defaults retain their own field owners.
pub(super) async fn redact(
    tx: &mut Tx,
    actor: &Actor,
    project: &mut EditableProject,
) -> Result<(), OpError> {
    let (org_id, actor_id) = (actor.org_id(), actor.id());
    let grants = actor
        .grants()
        .ok_or_else(|| OpError::forbidden(EDITING_REQUIRED))?;
    let managers = super::super::managers::current(tx, actor, project.id).await?;
    apply_manager_flags(&mut project.form, &managers);
    let designated = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM project_management_assignments
         WHERE org_id = $1 AND project_id = $2 AND manager_id = $3)",
        org_id,
        project.id,
        actor_id,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?
        == Some(true);
    let selected_people: Vec<_> = project
        .selection
        .people
        .iter()
        .map(|person| person.id)
        .collect();
    let people = sqlx::query_scalar!(
        "SELECT managed_user_id FROM person_management_assignments
         WHERE org_id=$1 AND manager_id=$2 AND managed_user_id=ANY($3)",
        org_id,
        actor_id,
        &selected_people,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    let identity = actor.identity();
    let assignments = ManagementAssignments {
        actor_id,
        org_id,
        people: &people,
        projects: if designated {
            std::slice::from_ref(&project.id)
        } else {
            &[]
        },
    };
    let billable = |action, owner| {
        billable_rate_access(
            grants,
            action,
            identity,
            &BillableRateResource { org_id, owner },
            &assignments,
        )
    };
    let read_billable = billable(RateAction::Read, BillableRateOwner::Project(project.id));
    let write_billable = billable(RateAction::Write, BillableRateOwner::Project(project.id));
    let read_cost = cost_rate_access(grants, RateAction::Read, identity, org_id);
    let write_cost = cost_rate_access(grants, RateAction::Write, identity, org_id);
    if !read_billable {
        project.form.project_rate.clear();
        for member in &mut project.form.team {
            member.billable_rate.clear();
        }
        for task in &mut project.form.tasks {
            task.rate.clear();
        }
        if matches!(
            project.form.budget_mode,
            BudgetMode::TotalFees | BudgetMode::FeesPerTask
        ) {
            project.form.budget_value.clear();
            project.form.budget_alert = false;
            project.form.budget_alert_at.clear();
            project.form.budget_monthly = false;
            project.form.budget_nonbillable = false;
            for task in &mut project.form.tasks {
                task.budget.clear();
            }
        }
        project.form.fee_mode = FeeMode::Single;
        project.form.fee_amount.clear();
        project.form.monthly_day = MonthlyFeeDay::First;
        project.form.milestones.clear();
        project.form.invoice_defaults = InvoiceDefaultsInput {
            terms_days: String::new(),
            po_number: String::new(),
            tax: String::new(),
            second_tax: None,
            discount: String::new(),
        };
    }
    if !read_cost {
        for member in &mut project.form.team {
            member.cost_rate.clear();
        }
        for person in &mut project.selection.people {
            person.cost_rate_cents = None;
        }
    }
    // Private notes belong to whoever may edit every project.
    let notes = actor.has(Permission::ProjectWriteAll);
    if !notes {
        project.form.admin_notes.clear();
    }
    for task in &mut project.selection.tasks {
        if !billable(RateAction::Read, BillableRateOwner::GlobalTask) {
            task.default_rate_cents = None;
        }
    }
    for person in &mut project.selection.people {
        if !billable(RateAction::Read, BillableRateOwner::Person(person.id)) {
            person.billable_rate_cents = None;
        }
    }
    // Client defaults are not consumed by the existing-project editor.
    project.client.default_rate_cents = None;
    project.access = Some(ProjectEditorAccess {
        managers,
        requester: actor.requester(),
        billable: field_access(read_billable, write_billable),
        costs: field_access(read_cost, write_cost),
        private_notes: field_access(notes, notes),
    });
    Ok(())
}
