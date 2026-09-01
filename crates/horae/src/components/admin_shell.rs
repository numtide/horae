use dioxus::prelude::*;

use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;
use crate::server_fns;

/// The admin area shell: a secondary sub-navigation (Workspace / Data) beside the
/// active admin panel, rendered through an `Outlet`. Layered inside `AppLayout`,
/// so the main rail stays put and this owns only the content panel (per the
/// design's admin settings shell). Admin-only; non-admins get a short notice.
///
/// Only sections with a real destination are listed — People (user management)
/// and Importers (Harvest). Roles & permissions, General, Export & backups, and
/// Audit log from the design are deferred until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });

    // Gate on the resolved user; server fns enforce this too, but a non-admin
    // should never see the admin chrome.
    if let Some(Ok(user)) = &*me.read()
        && !user.is_admin()
    {
        return rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                h1 { class: "page-title", "Admins only" }
                p { class: "text-secondary", "You need an admin role to manage the workspace." }
                Link {
                    to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                    class: "btn btn-secondary",
                    "Back to Timesheet"
                }
            }
        };
    }

    // The workspace's real name for the header chip (no slug — the schema has no
    // such field, so we show the name only rather than inventing a URL).
    let org = use_resource(|| async move { server_fns::get_org_name().await });
    let org_name = match &*org.read() {
        Some(Ok(name)) => name.clone(),
        _ => "Workspace".to_string(),
    };
    let org_initial = org_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "·".to_string());

    rsx! {
        div {
            Link {
                to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                class: "adm-back inline-flex items-center gap-2 text-sm text-secondary mb-5",
                span { "←" }
                "Back to Timesheet"
            }
            div { class: "flex items-start gap-12",
                aside { class: "adm-nav",
                    div { class: "flex items-center gap-3 border-b pb-5 mb-4",
                        span { class: "adm-head-mark", "{org_initial}" }
                        div { class: "min-w-0",
                            div { class: "text-sm font-semibold truncate mb-1", "{org_name}" }
                            span { class: "badge badge-info badge-sm", "Admin" }
                        }
                    }

                    div { class: "adm-group-label", "Workspace" }
                    nav { class: "flex flex-col gap-1 mb-5",
                        AdmLink { to: Route::AdminUsers {}, label: "People" }
                    }

                    div { class: "adm-group-label", "Data" }
                    nav { class: "flex flex-col gap-1 mb-5",
                        AdmLink { to: Route::HarvestImport {}, label: "Importers" }
                    }
                }
                div { class: "adm-main flex-1 min-w-0",
                    Outlet::<Route> {}
                }
            }
        }
    }
}

/// One sub-nav row: a client-side `Link` that marks itself active for its route
/// variant (matching the rail's `SideLink` behaviour).
#[component]
fn AdmLink(to: Route, label: String) -> Element {
    let active = std::mem::discriminant(&use_route::<Route>()) == std::mem::discriminant(&to);
    rsx! {
        Link { to, class: if active { "adm-link active" } else { "adm-link" }, "{label}" }
    }
}
