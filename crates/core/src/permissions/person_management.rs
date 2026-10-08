//! Person-management prerequisites, not authorization to change relationships.
//!
//! Callers must independently verify current administrator authority, tenancy,
//! active identities, revisions and confirmation, and commit changes with audit.

use uuid::Uuid;

use super::catalog::{Permission, PermissionSelection};

/// Whether existing grants can use person-management scope.
///
/// Check the receiving manager's grants before adding relationships and after
/// editing permissions to determine retention. No prior assignment or general
/// people-directory grant is required. This does not grant assignment authority
/// or automatically remove/restore relationships.
#[must_use]
pub fn has_person_management_grant(selection: &PermissionSelection) -> bool {
    use Permission::*;

    selection.iter().any(|permission| match permission {
        TimeReadManaged
        | TimeWriteManaged
        | TimeApproveManaged
        | TimeReadAll
        | TimeWriteAll
        | TimeApproveAll
        | ExpenseReadManaged
        | ExpenseWriteManaged
        | ExpenseReadAll
        | ExpenseWriteAll
        | PeopleReadManaged
        | PeopleWriteManaged
        | PeopleReadAll
        | PeopleWriteAll
        | BillableRateReadManaged
        | BillableRateWriteManaged
        | BillableRateReadAll
        | BillableRateWriteAll
        | ApprovalWithdrawManaged => true,
        TimeReadOwn
        | TimeWriteOwn
        | ExpenseReadOwn
        | ExpenseWriteOwn
        | ProjectReadManaged
        | ProjectWriteManaged
        | ProjectCreateAll
        | ProjectReadAll
        | ProjectWriteAll
        | ClientReadAll
        | ClientWriteAll
        | TaskReadAll
        | TaskWriteAll
        | CostRateReadAll
        | CostRateWriteAll
        | InvoiceReadManaged
        | InvoiceDraftWriteManaged
        | InvoiceWriteManaged
        | InvoiceReadAll
        | InvoiceWriteAll
        | EstimateReadAll
        | EstimateWriteAll
        | ReportProfitabilityRead
        | ReportContractorRead
        | ReportInvoicingRead
        | SavedReportReadInactive
        | SavedReportWriteInactive
        | CompanyRead
        | CompanyWrite
        | BillingRead
        | BillingWrite => false,
    })
}

/// A proposed managed-person set contains its own responsible person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("A person cannot be assigned as their own managed person")]
pub struct SelfManagement;

/// Checks the complete proposed set for self-management without filtering it.
///
/// `manager_id` identifies the receiving responsible person, not the acting
/// administrator. An empty set is valid here; removing relationships does not
/// require compatible grants. Success establishes neither authority nor tenancy.
///
/// # Errors
/// Returns [`SelfManagement`] if any proposed person is the responsible person.
pub fn validate_no_self_management(
    manager_id: Uuid,
    proposed_people: &[Uuid],
) -> Result<(), SelfManagement> {
    if proposed_people.contains(&manager_id) {
        return Err(SelfManagement);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
