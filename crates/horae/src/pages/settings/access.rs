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
use crate::models::own_permissions::OwnPermissions;
use crate::route::Route;

#[component]
pub fn SettingsAccess() -> Element {
    let session = use_session();
    let retry = use_callback(move |_| session.refresh());
    let own = session.permissions.read();
    let badge = own.as_ref().and_then(|result| result.as_ref().ok()).map(|own| {
        if own.is_administrator {
            ("badge badge-info badge-sm", "Administrator".to_string())
        } else {
            (
                "badge badge-neutral badge-sm",
                own.profile
                    .clone()
                    .unwrap_or_else(|| "Individual configuration".into()),
            )
        }
    });
    rsx! {
        PageHeader { title: "Your access",
            meta: rsx! {
                if let Some((class, label)) = badge { span { class, "{label}" } }
            },
        }
        {access_content(&own, retry)}
    }
}

/// The page body for the session's permission read. Only the first load shows
/// `Loading`: a session refresh keeps the last answer on screen.
fn access_content(
    response: &Option<Result<OwnPermissions, ServerFnError>>,
    retry: Callback<()>,
) -> Element {
    const UNAUTHORIZED: u16 = 401;
    const FORBIDDEN: u16 = 403;
    let own = match response {
        None => return rsx! { Loading { label: "Loading your access…" } },
        Some(Ok(own)) if own.catalog_version == PERMISSION_CATALOG_VERSION => own,
        Some(Err(ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        })) => {
            return rsx! {
                ErrorState { title: "Could not load your access", body: "Your session has ended. Sign in again to continue." }
            };
        }
        // No stored permission state: only an administrator can repair it.
        Some(Err(ServerFnError::ServerError {
            code: FORBIDDEN, ..
        })) => {
            return rsx! {
                ErrorState { title: "Could not load your access", body: "Ask an administrator of this workspace to check your access." }
            };
        }
        Some(_) => {
            return rsx! {
                ErrorState { title: "Could not load your access", body: LOAD_FAILED, onretry: retry }
            };
        }
    };
    let rows = access_rows(&own.grants);
    let others: Vec<_> = OTHER
        .iter()
        .copied()
        .filter(|grant| own.grants.contains(grant))
        .collect();
    rsx! {
        div { class: "flex flex-col gap-5 wrap-anywhere",
            p { class: "text-secondary",
                "What you can see and do in this workspace. "
                // Only administrators may read the audit log, and it has no
                // filter by person yet.
                if own.is_administrator {
                    Link { id: "access-history", to: Route::PermissionAudit {}, "See changes to your access" }
                }
            }
            if own.is_administrator {
                p { class: "card",
                    "You can see and change everything here, including other people's access in "
                    Link { to: Route::AdminUsers {}, "People" }
                    "."
                }
            } else {
                DataTable {
                    table { aria_label: "Your access by area",
                        thead { tr {
                            th { scope: "col", "Area" }
                            th { scope: "col", "View" }
                            th { scope: "col", "Edit" }
                            th { scope: "col", "Approve" }
                        } }
                        tbody {
                            for (name, cells) in rows {
                                tr { key: "{name}",
                                    th { scope: "row", class: "font-semibold", "{name}" }
                                    for (column, cell) in cells.into_iter().enumerate() {
                                        td { key: "{column}", {scope_cell(cell)} }
                                    }
                                }
                            }
                        }
                    }
                }
                p { class: "text-xs text-muted",
                    "All: everyone's. Managed: the people and projects you manage. Own: only yours."
                }
                if !others.is_empty() {
                    section { class: "card", aria_labelledby: "access-other-title",
                        h2 { id: "access-other-title", class: "card-title", "You can also" }
                        div { class: "text-sm",
                            for (index, grant) in others.into_iter().enumerate() {
                                div { key: "{grant:?}", class: if index == 0 { "py-2" } else { "py-2 border-t" },
                                    {permission_description(grant)}
                                }
                            }
                        }
                    }
                }
                p { class: "text-secondary", "To change your access, ask an administrator of this workspace." }
            }
            div { class: "grid md:grid-cols-2 gap-3",
                MetricCard { label: "People you manage", value: own.managed_person_ids.len().to_string() }
                MetricCard { label: "Projects you manage", value: own.managed_project_ids.len().to_string() }
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

/// A cell of the table: `None` when the column does not apply to the area,
/// `Some(None)` when no grant opens it.
type Cell = Option<Option<Scope>>;

type Grants = &'static [(Permission, Scope)];

/// The table's areas, with the grants behind View, Edit and Approve, each
/// listed widest first. An empty list means the column does not apply.
const AREAS: [(&str, Grants, Grants, Grants); 12] = {
    use Permission::*;
    use Scope::*;
    [
        (
            "Time",
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
            "Expenses",
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
            "Projects",
            &[(ProjectReadAll, All), (ProjectReadManaged, Managed)],
            &[(ProjectWriteAll, All), (ProjectWriteManaged, Managed)],
            &[],
        ),
        ("Clients", &[(ClientReadAll, All)], &[(ClientWriteAll, All)], &[]),
        ("Tasks", &[(TaskReadAll, All)], &[(TaskWriteAll, All)], &[]),
        (
            "People",
            &[(PeopleReadAll, All), (PeopleReadManaged, Managed)],
            &[(PeopleWriteAll, All), (PeopleWriteManaged, Managed)],
            &[],
        ),
        (
            "Billable rates",
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
            "Cost rates",
            &[(CostRateReadAll, All)],
            &[(CostRateWriteAll, All)],
            &[],
        ),
        (
            "Invoices",
            &[(InvoiceReadAll, All), (InvoiceReadManaged, Managed)],
            &[(InvoiceWriteAll, All), (InvoiceWriteManaged, Managed)],
            &[],
        ),
        (
            "Estimates",
            &[(EstimateReadAll, All)],
            &[(EstimateWriteAll, All)],
            &[],
        ),
        (
            "Workspace settings",
            &[(CompanyRead, All)],
            &[(CompanyWrite, All)],
            &[],
        ),
        (
            "Account billing",
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

fn access_rows(grants: &[Permission]) -> Vec<(&'static str, [Cell; 3])> {
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
        .map(|&(name, view, edit, approve)| (name, [cell(view), cell(edit), cell(approve)]))
        .collect()
}

fn scope_cell(cell: Cell) -> Element {
    match cell {
        None => rsx! { span { class: "text-muted", aria_label: "Does not apply", "—" } },
        Some(None) => rsx! { span { class: "text-muted", "No" } },
        Some(Some(Scope::All)) => rsx! { span { class: "badge badge-success badge-sm", "All" } },
        Some(Some(Scope::Managed)) => rsx! { span { class: "badge badge-info badge-sm", "Managed" } },
        Some(Some(Scope::Own)) => rsx! { span { class: "badge badge-neutral badge-sm", "Own" } },
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "access/tests.rs"]
mod tests;
