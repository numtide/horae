use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;
use crate::server_fns;

#[derive(Clone, Copy, PartialEq)]
enum Section {
    People,
    Tasks,
    Audit,
    Importers,
}

#[derive(Clone, Copy, PartialEq)]
struct WorkspaceAccess {
    people: bool,
    tasks: bool,
    audit: bool,
    importers: bool,
}

impl WorkspaceAccess {
    fn allows(self, section: Section) -> bool {
        match section {
            Section::People => self.people,
            Section::Tasks => self.tasks,
            Section::Audit => self.audit,
            Section::Importers => self.importers,
        }
    }
}

/// The admin area shell: a secondary sub-navigation (Workspace / Data) beside the
/// active admin panel, rendered through an `Outlet`. Layered inside `AppLayout`,
/// so the main rail stays put and this owns only the content panel (per the
/// design's admin settings shell). Each section has its own access boundary.
///
/// Only sections with a real destination are listed — People, Tasks,
/// Importers (Harvest), and the permission audit log. Other sections are deferred
/// until they have a backend.
#[component]
pub fn AdminShell() -> Element {
    let section = match use_route::<Route>() {
        Route::AdminUsers {} => Section::People,
        Route::TaskCatalog {} => Section::Tasks,
        Route::PermissionAudit {} => Section::Audit,
        _ => Section::Importers,
    };
    let mut me = use_resource(use_reactive!(|section| async move {
        let access: Result<WorkspaceAccess, ServerFnError> = async {
            let own = server_fns::get_my_permissions().await.and_then(|own| {
                if own
                    .as_ref()
                    .is_some_and(|own| own.catalog_version != PERMISSION_CATALOG_VERSION)
                {
                    Err(ServerFnError::new("Unsupported permission catalog"))
                } else {
                    Ok(own)
                }
            });
            let legacy_admin = server_fns::get_me().await?.is_admin();
            if section == Section::Importers && own.is_err() {
                return Ok(WorkspaceAccess {
                    people: false,
                    tasks: false,
                    audit: false,
                    importers: legacy_admin,
                });
            }
            // These are navigation hints; every reader/command authorizes afresh.
            Ok(match own? {
                Some(own) => WorkspaceAccess {
                    people: own.grants.contains(&Permission::PeopleReadAll)
                        || own.grants.contains(&Permission::PeopleReadManaged),
                    tasks: own.grants.contains(&Permission::TaskReadAll),
                    audit: own.is_administrator,
                    importers: legacy_admin,
                },
                None => WorkspaceAccess {
                    people: legacy_admin,
                    tasks: false,
                    audit: false,
                    importers: legacy_admin,
                },
            })
        }
        .await;
        (section, access)
    }));

    // Mount the workspace and its outlet only after authorization resolves.
    // Server functions still independently enforce access to every operation.
    let response = me.read();
    let current = response
        .as_ref()
        .filter(|(requested, _)| *requested == section)
        .map(|(_, result)| result);
    match (current, me.state()() == UseResourceState::Ready) {
        (Some(Ok(access)), true) if access.allows(section) => {
            rsx! { AdminWorkspace { access: *access } }
        }
        (Some(Ok(_)), true) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                h1 { class: "page-title", "Access unavailable" }
                p { class: "text-secondary", "Your account cannot view this workspace section." }
                Link {
                    to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() },
                    class: "btn btn-secondary",
                    "Back to Timesheet"
                }
            }
            if section == Section::People { crate::pages::admin::PermissionRecovery { on_saved: move |_| me.restart() } }
        },
        (Some(Err(_)), true) => rsx! {
            div { class: "card flex flex-col items-start gap-3 max-w-md",
                div { class: "alert alert-danger", role: "alert",
                    "Could not verify workspace access. Sign in again or retry."
                }
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    onclick: move |_| me.restart(),
                    "Retry"
                }
            }
            if section == Section::People { crate::pages::admin::PermissionRecovery { on_saved: move |_| me.restart() } }
        },
        _ => rsx! {
            div { class: "text-muted text-sm", role: "status", "Loading…" }
        },
    }
}

#[component]
fn AdminWorkspace(access: WorkspaceAccess) -> Element {
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
                to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() },
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
                            if access.audit { span { class: "badge badge-info badge-sm", "Administrator" } }
                        }
                    }

                    if access.people || access.tasks {
                        div { class: "adm-group-label", "Workspace" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.people { AdmLink { to: Route::AdminUsers {}, label: "People" } }
                            if access.tasks { AdmLink { to: Route::TaskCatalog {}, label: "Tasks" } }
                        }
                    }

                    if access.importers || access.audit {
                        div { class: "adm-group-label", "Data" }
                        nav { class: "flex flex-col gap-1 mb-5",
                            if access.importers { AdmLink { to: Route::HarvestImport {}, label: "Importers" } }
                            if access.audit { AdmLink { to: Route::PermissionAudit {}, label: "Audit log" } }
                        }
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
