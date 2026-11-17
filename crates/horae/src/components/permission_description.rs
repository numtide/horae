//! Shared presentation copy; these labels never authorize an operation.

use horae_core::permissions::catalog::Permission;

pub fn permission_description(permission: Permission) -> &'static str {
    use Permission::*;
    match permission {
        TimeReadOwn => "View your own time",
        TimeWriteOwn => "Edit your own time",
        TimeReadManaged => "View time for managed people and projects",
        TimeWriteManaged => "Edit time for managed people and projects",
        TimeApproveManaged => "Approve time for managed people and projects",
        TimeReadAll => "View all time in the workspace",
        TimeWriteAll => "Edit all time in the workspace",
        TimeApproveAll => "Approve all time in the workspace",
        ExpenseReadOwn => "View your own expenses",
        ExpenseWriteOwn => "Edit your own expenses",
        ExpenseReadManaged => "View expenses for managed people and projects",
        ExpenseWriteManaged => "Edit expenses for managed people and projects",
        ExpenseReadAll => "View all expenses in the workspace",
        ExpenseWriteAll => "Edit all expenses in the workspace",
        ProjectReadManaged => "View managed projects",
        ProjectWriteManaged => "Edit managed projects",
        ProjectCreateAll => "Create projects",
        ProjectReadAll => "View all projects in the workspace",
        ProjectWriteAll => "Edit all projects in the workspace",
        ClientReadAll => "View all clients in the workspace",
        ClientWriteAll => "Edit all clients in the workspace",
        TaskReadAll => "View all tasks in the workspace",
        TaskWriteAll => "Edit all tasks in the workspace",
        PeopleReadManaged => "View managed people",
        PeopleWriteManaged => "Edit managed people",
        PeopleReadAll => "View all people in the workspace",
        PeopleWriteAll => "Edit all people in the workspace",
        BillableRateReadManaged => "View billable rates for managed people and projects",
        BillableRateWriteManaged => "Edit billable rates for managed people and projects",
        BillableRateReadAll => "View all billable rates in the workspace",
        BillableRateWriteAll => "Edit all billable rates in the workspace",
        CostRateReadAll => "View all cost rates in the workspace",
        CostRateWriteAll => "Edit all cost rates in the workspace",
        InvoiceReadManaged => "View invoices for managed projects",
        InvoiceDraftWriteManaged => "Create and edit draft invoices for managed projects",
        InvoiceWriteManaged => "Manage invoices for managed projects",
        InvoiceReadAll => "View all invoices in the workspace",
        InvoiceWriteAll => "Manage all invoices in the workspace",
        EstimateReadAll => "View all estimates in the workspace",
        EstimateWriteAll => "Edit all estimates in the workspace",
        ReportProfitabilityRead => "View profitability reports",
        ReportContractorRead => "View contractor reports",
        ReportInvoicingRead => "View invoicing reports",
        SavedReportReadInactive => "View saved reports owned by inactive people",
        SavedReportWriteInactive => "Edit saved reports owned by inactive people",
        ApprovalWithdrawManaged => "Withdraw approvals within managed scope",
        CompanyRead => "View workspace settings",
        CompanyWrite => "Edit workspace settings",
        BillingRead => "View account billing",
        BillingWrite => "Edit account billing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn permission_descriptions_cover_each_grant_distinctly() {
        let descriptions: BTreeSet<_> = Permission::ALL
            .iter()
            .copied()
            .map(permission_description)
            .collect();
        assert_eq!(descriptions.len(), Permission::ALL.len());
        assert!(!descriptions.contains(""));
    }
}
