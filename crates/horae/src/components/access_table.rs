//! The table of areas against View, Edit and Approve that explains a set of
//! grants: Your access shows the session's own, and the audit log shows what
//! a change did to it, limited to the areas that changed.

use dioxus::prelude::*;
use horae_core::permissions::catalog::Permission;

use crate::components::table::DataTable;
use crate::i18n::Tr;

/// How far a grant reaches, narrowest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Own,
    Managed,
    All,
}

/// A row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    Time,
    Expenses,
    Projects,
    Clients,
    Tasks,
    People,
    BillableRates,
    CostRates,
    Invoices,
    Estimates,
    WorkspaceSettings,
    AccountBilling,
}

impl Area {
    pub fn label(self, tr: Tr) -> String {
        match self {
            Area::Time => tr.settings_access_area_time(),
            Area::Expenses => tr.settings_access_area_expenses(),
            Area::Projects => tr.settings_access_area_projects(),
            Area::Clients => tr.settings_access_area_clients(),
            Area::Tasks => tr.settings_access_area_tasks(),
            Area::People => tr.settings_access_area_people(),
            Area::BillableRates => tr.settings_access_area_billable_rates(),
            Area::CostRates => tr.settings_access_area_cost_rates(),
            Area::Invoices => tr.settings_access_area_invoices(),
            Area::Estimates => tr.settings_access_area_estimates(),
            Area::WorkspaceSettings => tr.settings_access_area_workspace_settings(),
            Area::AccountBilling => tr.settings_access_area_account_billing(),
        }
    }
}

/// A cell of the table: `None` when the column does not apply to the area,
/// `Some(None)` when no grant opens it.
pub type Cell = Option<Option<Scope>>;

type Grants = &'static [(Permission, Scope)];

/// The table's areas, with the grants behind View, Edit and Approve, each
/// listed widest first. An empty list means the column does not apply.
pub const AREAS: [(Area, Grants, Grants, Grants); 12] = {
    use Permission::*;
    use Scope::*;
    [
        (
            Area::Time,
            &[
                (TimeReadAll, All),
                (TimeReadManaged, Managed),
                (TimeReadOwn, Own),
            ],
            &[
                (TimeWriteAll, All),
                (TimeWriteManaged, Managed),
                (TimeWriteOwn, Own),
            ],
            &[(TimeApproveAll, All), (TimeApproveManaged, Managed)],
        ),
        (
            Area::Expenses,
            &[
                (ExpenseReadAll, All),
                (ExpenseReadManaged, Managed),
                (ExpenseReadOwn, Own),
            ],
            &[
                (ExpenseWriteAll, All),
                (ExpenseWriteManaged, Managed),
                (ExpenseWriteOwn, Own),
            ],
            &[],
        ),
        (
            Area::Projects,
            &[(ProjectReadAll, All), (ProjectReadManaged, Managed)],
            &[(ProjectWriteAll, All), (ProjectWriteManaged, Managed)],
            &[],
        ),
        (
            Area::Clients,
            &[(ClientReadAll, All)],
            &[(ClientWriteAll, All)],
            &[],
        ),
        (
            Area::Tasks,
            &[(TaskReadAll, All)],
            &[(TaskWriteAll, All)],
            &[],
        ),
        (
            Area::People,
            &[(PeopleReadAll, All), (PeopleReadManaged, Managed)],
            &[(PeopleWriteAll, All), (PeopleWriteManaged, Managed)],
            &[],
        ),
        (
            Area::BillableRates,
            &[
                (BillableRateReadAll, All),
                (BillableRateReadManaged, Managed),
            ],
            &[
                (BillableRateWriteAll, All),
                (BillableRateWriteManaged, Managed),
            ],
            &[],
        ),
        (
            Area::CostRates,
            &[(CostRateReadAll, All)],
            &[(CostRateWriteAll, All)],
            &[],
        ),
        (
            Area::Invoices,
            &[(InvoiceReadAll, All), (InvoiceReadManaged, Managed)],
            &[(InvoiceWriteAll, All), (InvoiceWriteManaged, Managed)],
            &[],
        ),
        (
            Area::Estimates,
            &[(EstimateReadAll, All)],
            &[(EstimateWriteAll, All)],
            &[],
        ),
        (
            Area::WorkspaceSettings,
            &[(CompanyRead, All)],
            &[(CompanyWrite, All)],
            &[],
        ),
        (
            Area::AccountBilling,
            &[(BillingRead, All)],
            &[(BillingWrite, All)],
            &[],
        ),
    ]
};

/// Grants that fit no cell of the table, listed by their description.
pub const OTHER: [Permission; 8] = {
    use Permission::*;
    [
        ProjectCreateAll,
        InvoiceDraftWriteManaged,
        ApprovalWithdrawManaged,
        ReportProfitabilityRead,
        ReportContractorRead,
        ReportInvoicingRead,
        SavedReportReadInactive,
        SavedReportWriteInactive,
    ]
};

pub fn access_rows(grants: &[Permission]) -> Vec<(Area, [Cell; 3])> {
    let cell = |column: Grants| -> Cell {
        (!column.is_empty()).then(|| {
            column
                .iter()
                .find(|(grant, _)| grants.contains(grant))
                .map(|&(_, scope)| scope)
        })
    };
    AREAS
        .iter()
        .map(|&(area, view, edit, approve)| (area, [cell(view), cell(edit), cell(approve)]))
        .collect()
}

/// A cell as shown: what it is now, and what it was when a change moved it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessCell {
    pub now: Cell,
    pub was: Option<Cell>,
}

/// The rows of a change: only the areas where some cell moved, with each moved
/// cell carrying its old value.
pub fn changed_rows(before: &[Permission], after: &[Permission]) -> Vec<(Area, [AccessCell; 3])> {
    access_rows(before)
        .into_iter()
        .zip(access_rows(after))
        .filter(|((_, was), (_, now))| was != now)
        .map(|((area, was), (_, now))| {
            let cells = [0, 1, 2].map(|column| AccessCell {
                now: now[column],
                was: (was[column] != now[column]).then_some(was[column]),
            });
            (area, cells)
        })
        .collect()
}

/// The table itself. Moved cells read "before → after", tinted by whether the
/// access widened or narrowed.
#[component]
pub fn AccessTable(label: String, rows: Vec<(Area, [AccessCell; 3])>) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        DataTable {
            table { aria_label: label,
                thead { tr {
                    th { scope: "col", {tr.settings_access_column_area()} }
                    th { scope: "col", {tr.settings_access_column_view()} }
                    th { scope: "col", {tr.settings_access_column_edit()} }
                    th { scope: "col", {tr.settings_access_column_approve()} }
                } }
                tbody {
                    for (area, cells) in rows {
                        tr { key: "{area:?}",
                            th { scope: "row", class: "font-semibold", {area.label(tr)} }
                            for (column, cell) in ["view", "edit", "approve"].into_iter().zip(cells) {
                                td { key: "{column}", {access_cell(tr, cell)} }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn access_cell(tr: Tr, cell: AccessCell) -> Element {
    let Some(was) = cell.was else {
        return scope_cell(tr, cell.now);
    };
    let class = if cell.now.flatten() > was.flatten() {
        "access-change access-change-up"
    } else {
        "access-change access-change-down"
    };
    rsx! {
        span { class, {tr.audit_scope_change(scope_word(tr, cell.now), scope_word(tr, was))} }
    }
}

fn scope_word(tr: Tr, cell: Cell) -> String {
    match cell.flatten() {
        None => tr.settings_access_scope_none(),
        Some(Scope::Own) => tr.settings_access_scope_own(),
        Some(Scope::Managed) => tr.settings_access_scope_managed(),
        Some(Scope::All) => tr.settings_access_scope_all(),
    }
}

pub fn scope_cell(tr: Tr, cell: Cell) -> Element {
    match cell {
        None => {
            // `aria-label` on a plain span is ignored, so the dash is hidden
            // and the words are read instead.
            rsx! {
                span { class: "text-muted", aria_hidden: "true", "—" }
                span { class: "sr-only", {tr.settings_access_not_applicable_label()} }
            }
        }
        Some(None) => rsx! { span { class: "text-muted", {tr.settings_access_scope_none()} } },
        Some(Some(Scope::All)) => {
            rsx! { span { class: "badge badge-success badge-sm", {tr.settings_access_scope_all()} } }
        }
        Some(Some(Scope::Managed)) => {
            rsx! { span { class: "badge badge-info badge-sm", {tr.settings_access_scope_managed()} } }
        }
        Some(Some(Scope::Own)) => {
            rsx! { span { class: "badge badge-neutral badge-sm", {tr.settings_access_scope_own()} } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_keeps_only_the_areas_that_moved_and_marks_the_moved_cells() {
        use Permission::*;
        let rows = changed_rows(
            &[TimeReadAll, TimeWriteAll, ClientReadAll],
            &[
                TimeReadAll,
                TimeWriteManaged,
                TimeApproveManaged,
                ClientReadAll,
            ],
        );
        assert_eq!(rows.len(), 1);
        let (area, [view, edit, approve]) = rows[0];
        assert_eq!(area, Area::Time);
        assert_eq!(view.was, None);
        assert_eq!(edit.was, Some(Some(Some(Scope::All))));
        assert_eq!(edit.now, Some(Some(Scope::Managed)));
        assert_eq!(approve.was, Some(Some(None)));
    }
}
