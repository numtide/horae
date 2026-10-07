use super::*;
use crate::models::project_creation::{
    ProjectEditorAccess, ProjectFieldAccess, ProtectedProjectField,
};
use std::collections::HashSet;

/// Saved with the original request so retries reauthorize removed associations
/// and mode changes even after their former state is no longer in the project.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequiredWrites {
    pub billable: bool,
    pub costs: bool,
    pub private_notes: bool,
    pub task_catalog: bool,
}

impl RequiredWrites {
    pub(super) fn authorize(
        &self,
        access: &ProjectEditorAccess,
        task_catalog: bool,
    ) -> Result<(), ServerFnError> {
        if (self.billable && access.billable != ProjectFieldAccess::Editable)
            || (self.costs && access.costs != ProjectFieldAccess::Editable)
            || (self.private_notes && access.private_notes != ProjectFieldAccess::Editable)
            || (self.task_catalog && !task_catalog)
        {
            return Err(forbidden(
                "Current authority for every protected project change is required",
            ));
        }
        Ok(())
    }
}

pub(super) fn validate_intent(request: &ProjectEditRequest) -> Result<(), ServerFnError> {
    if request.unchanged.len() > 1505 {
        return Err(err(BAD_REQUEST, "Too many preserved project fields"));
    }
    let mut seen = HashSet::new();
    for field in &request.unchanged {
        let exists = match *field {
            ProtectedProjectField::TaskRate(id) => {
                request.form.tasks.iter().any(|task| task.id == id)
            }
            ProtectedProjectField::PersonRate(id) | ProtectedProjectField::CostRate(id) => {
                request.form.team.iter().any(|member| member.user_id == id)
            }
            _ => true,
        };
        if !exists || !seen.insert(*field) {
            return Err(err(BAD_REQUEST, "Invalid preserved project field"));
        }
    }
    Ok(())
}

fn require_write(allowed: bool, required: &mut bool) -> Result<(), ServerFnError> {
    if allowed {
        *required = true;
        Ok(())
    } else {
        Err(forbidden(
            "Current authority for every protected project change is required",
        ))
    }
}

pub(super) fn monetary(mode: BudgetMode) -> bool {
    matches!(mode, BudgetMode::TotalFees | BudgetMode::FeesPerTask)
}

/// Authorize original intent before replacing withheld input with current storage.
/// The access value must come from the server's current permission snapshot.
pub(super) fn merge(
    request: &ProjectEditRequest,
    before: &ProjectForm,
    access: &ProjectEditorAccess,
) -> Result<(ProjectForm, RequiredWrites), ServerFnError> {
    use ProtectedProjectField::*;
    let keep = |field| request.unchanged.contains(&field);
    let billable_write = access.billable == ProjectFieldAccess::Editable;
    let cost_write = access.costs == ProjectFieldAccess::Editable;
    let mut form = request.form.clone();
    let mut required = RequiredWrites::default();

    if form.project_type != before.project_type || form.rate_mode != before.rate_mode {
        require_write(billable_write, &mut required.billable)?;
    }
    if keep(ProjectRate) {
        form.project_rate.clone_from(&before.project_rate);
    } else {
        require_write(billable_write, &mut required.billable)?;
    }
    if keep(PrivateNotes) {
        form.admin_notes.clone_from(&before.admin_notes);
    } else {
        require_write(
            access.private_notes == ProjectFieldAccess::Editable,
            &mut required.private_notes,
        )?;
    }
    if keep(InvoiceDefaults) {
        form.invoice_defaults.clone_from(&before.invoice_defaults);
    } else {
        require_write(billable_write, &mut required.billable)?;
    }
    if keep(Fees) {
        form.fee_mode = before.fee_mode;
        form.fee_amount.clone_from(&before.fee_amount);
        form.monthly_day = before.monthly_day;
        form.milestones.clone_from(&before.milestones);
    } else {
        require_write(billable_write, &mut required.billable)?;
    }
    if keep(Budget) {
        form.budget_mode = before.budget_mode;
        form.budget_value.clone_from(&before.budget_value);
        form.budget_alert = before.budget_alert;
        form.budget_alert_at.clone_from(&before.budget_alert_at);
        form.budget_monthly = before.budget_monthly;
        form.budget_nonbillable = before.budget_nonbillable;
    } else if monetary(before.budget_mode) || monetary(form.budget_mode) {
        require_write(billable_write, &mut required.billable)?;
    }
    for member in &mut form.team {
        let previous = before
            .team
            .iter()
            .find(|item| item.user_id == member.user_id);
        if keep(PersonRate(member.user_id)) {
            member.billable_rate =
                previous.map_or_else(String::new, |item| item.billable_rate.clone());
        } else {
            require_write(billable_write, &mut required.billable)?;
        }
        if keep(CostRate(member.user_id)) {
            member.cost_rate = previous.map_or_else(String::new, |item| item.cost_rate.clone());
        } else {
            require_write(cost_write, &mut required.costs)?;
        }
        if keep(Budget) {
            member.budget = previous.map_or_else(String::new, |item| item.budget.clone());
        }
    }
    for task in &mut form.tasks {
        let previous = before.tasks.iter().find(|item| item.source == task.source);
        if keep(TaskRate(task.id)) {
            task.rate = previous.map_or_else(String::new, |item| item.rate.clone());
        } else {
            require_write(billable_write, &mut required.billable)?;
        }
        if keep(Budget) {
            task.budget = previous.map_or_else(String::new, |item| item.budget.clone());
        }
        if previous.is_some_and(|item| item.billable != task.billable) {
            require_write(billable_write, &mut required.billable)?;
        }
    }
    // Removing associations must not bypass protection of their stored overrides.
    for member in &before.team {
        if !form.team.iter().any(|item| item.user_id == member.user_id) {
            if !member.billable_rate.is_empty() {
                require_write(billable_write, &mut required.billable)?;
            }
            if !member.cost_rate.is_empty() {
                require_write(cost_write, &mut required.costs)?;
            }
        }
    }
    for task in &before.tasks {
        if !form.tasks.iter().any(|item| item.source == task.source)
            && (!task.rate.is_empty() || (monetary(before.budget_mode) && !task.budget.is_empty()))
        {
            require_write(billable_write, &mut required.billable)?;
        }
    }
    Ok((form, required))
}
