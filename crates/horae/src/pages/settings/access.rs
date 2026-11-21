//! Your access: what the signed-in person may see and do, read from the
//! session's stored permissions. Display only; every operation authorizes
//! afresh on the server.

use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::card::MetricCard;
use crate::components::page_header::PageHeader;
use crate::components::permission_description::permission_description;
use crate::components::screen_state::{ErrorState, LOAD_FAILED, Loading};
use crate::components::session::use_session;
use crate::components::table::DataTable;
use crate::i18n::{Tr, split_tag};
use crate::models::own_permissions::OwnPermissions;
use crate::route::Route;

#[component]
pub fn SettingsAccess() -> Element {
    let tr = crate::i18n::use_tr();
    let session = use_session();
    let retry = use_callback(move |_| session.refresh());
    let own = session.permissions.read();
    let badge = own
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|own| {
            if own.is_administrator {
                (
                    "badge badge-info badge-sm",
                    tr.settings_access_administrator_badge(),
                )
            } else {
                (
                    "badge badge-neutral badge-sm",
                    own.profile
                        .clone()
                        .unwrap_or_else(|| tr.settings_access_individual_badge()),
                )
            }
        });
    rsx! {
        PageHeader { title: tr.settings_access_title(),
            meta: rsx! {
                if let Some((class, label)) = badge { span { class, "{label}" } }
            },
        }
        {access_content(tr, &own, retry)}
    }
}

/// The page body for the session's permission read. Only the first load shows
/// `Loading`: a session refresh keeps the last answer on screen.
fn access_content(
    tr: Tr,
    response: &Option<Result<OwnPermissions, ServerFnError>>,
    retry: Callback<()>,
) -> Element {
    const UNAUTHORIZED: u16 = 401;
    const FORBIDDEN: u16 = 403;
    let own = match response {
        None => return rsx! { Loading { label: tr.settings_access_loading() } },
        Some(Ok(own)) if own.catalog_version == PERMISSION_CATALOG_VERSION => own,
        Some(Err(ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        })) => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: tr.common_session_ended() }
            };
        }
        // No stored permission state: only an administrator can repair it.
        Some(Err(ServerFnError::ServerError {
            code: FORBIDDEN, ..
        })) => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: tr.settings_access_repair() }
            };
        }
        Some(_) => {
            return rsx! {
                ErrorState { title: tr.settings_access_failed(), body: LOAD_FAILED, onretry: retry }
            };
        }
    };
    let rows = access_rows(&own.grants);
    let others: Vec<_> = OTHER
        .iter()
        .copied()
        .filter(|grant| own.grants.contains(grant))
        .collect();
    let administrator = tr.settings_access_administrator();
    let (people_before, people_link, people_after) = split_tag(&administrator, "link");
    rsx! {
        div { class: "flex flex-col gap-5 wrap-anywhere",
            p { class: "text-secondary",
                {tr.settings_access_intro()}
                // Only administrators may read the audit log, and it has no
                // filter by person yet.
                if own.is_administrator {
                    {" "}
                    Link { id: "access-history", to: Route::PermissionAudit {}, {tr.settings_access_history()} }
                }
            }
            if own.is_administrator {
                p { class: "card",
                    "{people_before}"
                    Link { to: Route::AdminUsers {}, "{people_link}" }
                    "{people_after}"
                }
            } else {
                DataTable {
                    table { aria_label: tr.settings_access_table_label(),
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
                                    for (column, cell) in cells.into_iter().enumerate() {
                                        td { key: "{column}", {scope_cell(tr, cell)} }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "text-xs text-muted", {tr.settings_access_legend()} }
                if !others.is_empty() {
                    section { class: "card", aria_labelledby: "access-other-title",
                        h2 { id: "access-other-title", class: "card-title", {tr.settings_access_other_title()} }
                        div { class: "text-sm",
                            for (index, grant) in others.into_iter().enumerate() {
                                div { key: "{grant:?}", class: if index == 0 { "py-2" } else { "py-2 border-t" },
                                    {permission_description(grant)}
                                }
                            }
                        }
                    }
                }
                p { class: "text-secondary", {tr.settings_access_change()} }
            }
            div { class: "grid md:grid-cols-2 gap-3",
                MetricCard { label: tr.settings_access_people_managed(), value: own.managed_person_ids.len().to_string() }
                MetricCard { label: tr.settings_access_projects_managed(), value: own.managed_project_ids.len().to_string() }
            }
        }
    }
}

/// How far a grant reaches, narrowest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Own,
    Managed,
    All,
}

/// A row of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Area {
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
    fn label(self, tr: Tr) -> String {
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
type Cell = Option<Option<Scope>>;

type Grants = &'static [(Permission, Scope)];

/// The table's areas, with the grants behind View, Edit and Approve, each
/// listed widest first. An empty list means the column does not apply.
const AREAS: [(Area, Grants, Grants, Grants); 12] = {
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
const OTHER: [Permission; 8] = {
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

fn access_rows(grants: &[Permission]) -> Vec<(Area, [Cell; 3])> {
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

fn scope_cell(tr: Tr, cell: Cell) -> Element {
    match cell {
        None => {
            rsx! { span { class: "text-muted", aria_label: tr.settings_access_not_applicable_label(), "—" } }
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

#[cfg(all(test, feature = "server"))]
#[path = "access/tests.rs"]
mod tests;
