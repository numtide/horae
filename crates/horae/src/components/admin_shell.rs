use dioxus::prelude::*;
use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;

use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;
use crate::server_fns;

/// The admin area shell: a secondary sub-navigation (Workspace / Data) beside the
/// active admin panel, rendered through an `Outlet`. Layered inside `AppLayout`,
/// so the main rail stays put and this owns only the content panel (per the
/// design's admin settings shell). Admin-only; non-admins get a short notice.
///
/// Only sections with a real destination are listed — People (user management)
/// Importers (Harvest), and the permission audit log. Other sections are deferred
/// until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let audit = matches!(use_route::<Route>(), Route::PermissionAudit {});
    let mut me = use_resource(use_reactive!(|audit| async move {
        // Only this reader has moved to canonical Administrator authority.
        let allowed = if audit {
            server_fns::get_my_permissions().await.map(|own| {
                own.is_some_and(|own| {
                    own.catalog_version == PERMISSION_CATALOG_VERSION && own.is_administrator
                })
            })
        } else {
            server_fns::get_me().await.map(|user| user.is_admin())
        };
        (audit, allowed)
    }));

    // Mount the workspace and its outlet only after authorization resolves.
    // Server functions still independently enforce access to every operation.
    let response = me.read();
    let current = response
        .as_ref()
        .filter(|(requested, _)| *requested == audit)
        .map(|(_, result)| result);
    match (current, me.state()() == UseResourceState::Ready) {
        (Some(Ok(true)), true) => rsx! { AdminWorkspace {} },
        (Some(Ok(false)), true) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                h1 { class: "page-title", "Admins only" }
                p { class: "text-secondary", "You need Administrator access to view this workspace section." }
                Link {
                    to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                    class: "btn btn-secondary",
                    "Back to Timesheet"
                }
            }
        },
        (Some(Err(error)), true) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                div { class: "alert alert-danger", role: "alert",
                    if audit { "Could not verify Administrator access. Sign in again or retry." }
                    else { "{error}" }
                }
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| me.restart(),
                    "Retry"
                }
            }
        },
        _ => rsx! {
            div { class: "text-muted text-sm", role: "status", "Loading…" }
        },
    }
}

#[component]
fn AdminWorkspace() -> Element {
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
            div { class: "flex flex-wrap items-start gap-12",
                aside { class: "adm-nav",
                    div { class: "adm-head flex items-center gap-3",
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
                        AdmLink { to: Route::PermissionAudit {}, label: "Audit log" }
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
    let active = crate::route::route_is_active(&to);
    rsx! {
        Link { to, class: if active { "adm-link active" } else { "adm-link" }, "{label}" }
    }
}
