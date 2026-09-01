use dioxus::prelude::*;

use crate::components::icons::NavIcon;
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
            div { class: "adm-shell",
                div { class: "card adm-denied",
                    h1 { class: "page-title", "Admins only" }
                    p { class: "text-secondary", "You need an admin role to manage the workspace." }
                    Link {
                        to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                        class: "btn btn-secondary",
                        "Back to Timesheet"
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "adm-shell",
            Link {
                to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                class: "adm-back",
                span { class: "adm-back-arrow", "←" }
                "Back to Timesheet"
            }
            div { class: "adm-body",
                aside { class: "adm-nav",
                    div { class: "adm-head",
                        span { class: "adm-head-mark", NavIcon { name: "settings" } }
                        div { class: "adm-head-text",
                            div { class: "adm-head-title", "Administration" }
                            span { class: "badge badge-info badge-sm", "Admin" }
                        }
                    }

                    div { class: "adm-group-label", "Workspace" }
                    nav { class: "adm-links",
                        AdmLink { to: Route::AdminUsers {}, label: "People" }
                    }

                    div { class: "adm-group-label", "Data" }
                    nav { class: "adm-links",
                        AdmLink { to: Route::HarvestImport {}, label: "Importers" }
                    }
                }
                div { class: "adm-main",
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
