use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::avatar::Avatar;
use crate::components::icons::NavIcon;
use crate::components::logo::HoraeMark;
use crate::components::session::{Session, use_session};
use crate::components::timer_widget::TimerWidget;
use crate::models::own_permissions::OwnPermissions;
use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;

/// The left rail: brand, a start-timer action, grouped navigation with an active
/// state, and a footer showing the signed-in user. `collapsed` is owned by
/// `AppLayout` so the shell can narrow the content area in step with the rail.
#[component]
pub fn Sidebar(collapsed: Signal<bool>, on_navigate: EventHandler<()>) -> Element {
    // A refresh keeps the last answer, so the rail does not flicker while it runs.
    let session = use_session();
    let is_admin = matches!(&*session.me.read(), Some(Ok(me)) if me.is_admin());
    let sections = nav_sections(nav_access(session.permissions.read().as_ref(), is_admin));
    rsx! {
        aside { id: "app-navigation", class: "app-sidebar", aria_label: "Navigation",
            div { class: "sidebar-brand",
                HoraeMark {}
                span { class: "sidebar-brand-name", "Horae" }
                span { class: "sidebar-brand-dot" }
                button {
                    class: "sidebar-collapse",
                    title: "Collapse sidebar",
                    "aria-label": "Collapse sidebar",
                    onclick: move |_| collapsed.set(!collapsed()),
                    if collapsed() { "»" } else { "«" }
                }
            }

            TimerWidget {}

            for (title, rows) in sections {
                Fragment { key: "{title}",
                    div { class: "sidebar-section", "{title}" }
                    div { class: "sidebar-group",
                        for NavRow { to, icon, label } in rows {
                            SideLink { key: "{label}", to, icon, label, on_navigate }
                        }
                    }
                }
            }

            div { class: "sidebar-spacer" }

            SidebarUser { on_navigate }
        }
    }
}

/// One rail row: a client-side `Link` that auto-marks itself active for its route.
/// The active route keeps its icon (tinted, over a raised surface) rather than
/// swapping it out, matching Harvest's rail.
#[component]
fn SideLink(to: Route, icon: String, label: String, on_navigate: EventHandler<()>) -> Element {
    // Match by route variant, not the exact URL, so a param-carrying route (the
    // timesheet's /timesheet/<view>/<date>) stays highlighted on any view/day.
    let active = crate::route::route_is_active(&to);
    rsx! {
        // The collapsed rail hides the label, so the link carries its own name.
        Link { to, class: if active { "nav-item active" } else { "nav-item" },
            title: "{label}",
            "aria-label": "{label}",
            onclick: move |_| on_navigate.call(()),
            span { class: "nav-item-icon", NavIcon { name: icon } }
            span { class: "nav-item-label", "{label}" }
        }
    }
}

/// The signed-in user: an avatar + name + role row that opens an account popover
/// (settings, an admin section for admins, and sign out). Falls back to a
/// placeholder until `get_me` resolves (or when not authenticated).
#[component]
fn SidebarUser(on_navigate: EventHandler<()>) -> Element {
    let session = use_session();
    let Session {
        me,
        permissions: own,
        ..
    } = session;
    let mut open = use_signal(|| false);

    let user = me.read();
    let (name, email, legacy_role, marks, is_admin) = match &*user {
        Some(Ok(u)) => (
            u.name.clone(),
            u.email.clone(),
            capitalize(&u.org_role.to_string()),
            initials(&u.name),
            u.is_admin(),
        ),
        _ => (
            "Not signed in".to_string(),
            String::new(),
            String::new(),
            "·".to_string(),
            false,
        ),
    };
    let (administrator, role) = match &*own.read() {
        Some(Ok(None)) => (is_admin, legacy_role),
        Some(Ok(Some(access))) if access.catalog_version == PERMISSION_CATALOG_VERSION => (
            access.is_administrator,
            if access.is_administrator {
                "Administrator".into()
            } else {
                String::new()
            },
        ),
        _ => (false, String::new()),
    };
    let role_class = if administrator {
        "badge badge-info badge-sm"
    } else {
        "badge badge-neutral badge-sm"
    };

    rsx! {
        div { class: "sidebar-userbox",
            if open() {
                div { class: "sidebar-menu menu",
                    div { class: "sidebar-menu-head",
                        Avatar { initials: "{marks}" }
                        div { class: "sidebar-user",
                            div { class: "sidebar-user-name truncate", "{name}" }
                            if !email.is_empty() {
                                div { class: "sidebar-user-email truncate", "{email}" }
                            }
                        }
                        if !role.is_empty() {
                            span { class: "{role_class}", "{role}" }
                        }
                    }
                    div { class: "sidebar-menu-list",
                        Link { to: Route::Settings {}, class: "menu-item", onclick: move |_| { open.set(false); on_navigate.call(()); },
                            span { class: "menu-item-icon", NavIcon { name: "settings" } }
                            "Settings"
                        }
                    }
                    // Team and Tasks live in the rail's Organize section.
                    if is_admin {
                        div { class: "sidebar-menu-list",
                            div { class: "menu-group", "Workspace" }
                            Link { to: Route::HarvestImport {}, class: "menu-item", onclick: move |_| { open.set(false); on_navigate.call(()); },
                                span { class: "menu-item-icon", NavIcon { name: "import" } }
                                "Importers"
                            }
                        }
                    }
                    div { class: "sidebar-menu-foot",
                        form { method: "post", action: "/auth/logout",
                            button { class: "menu-item danger", r#type: "submit", "Sign out" }
                        }
                    }
                }
            }

            button {
                class: "sidebar-footer",
                "aria-haspopup": "menu",
                "aria-expanded": "{open()}",
                onclick: move |_| {
                    if !open() { session.refresh(); }
                    open.set(!open());
                },
                Avatar { initials: "{marks}" }
                // The chip keeps the identity while the popover is open (the design
                // shows both); the caret flips to signal the open state.
                div { class: "sidebar-user",
                    div { class: "sidebar-user-name truncate", "{name}" }
                    if !role.is_empty() {
                        div { class: "sidebar-user-sub", "{role}" }
                    }
                }
                span { class: "sidebar-user-caret", if open() { "⌃" } else { "⌄" } }
            }
        }
    }
}

/// The rail rows that depend on the person's permissions. These only hide links;
/// every page authorizes afresh.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct NavAccess {
    people: bool,
    clients: bool,
    tasks: bool,
}

/// Policy zero keeps Clients for everyone and Team for legacy admins; failed or
/// unknown permission state hides every gated row.
fn nav_access(
    own: Option<&Result<Option<OwnPermissions>, ServerFnError>>,
    is_admin: bool,
) -> NavAccess {
    match own {
        Some(Ok(None)) => NavAccess {
            people: is_admin,
            clients: true,
            tasks: false,
        },
        Some(Ok(Some(access))) if access.catalog_version == PERMISSION_CATALOG_VERSION => {
            NavAccess {
                people: access.grants.contains(&Permission::PeopleReadAll)
                    || access.grants.contains(&Permission::PeopleReadManaged),
                clients: access.grants.contains(&Permission::ClientReadAll),
                tasks: access.grants.contains(&Permission::TaskReadAll),
            }
        }
        _ => NavAccess::default(),
    }
}

struct NavRow {
    to: Route,
    icon: &'static str,
    label: &'static str,
}

/// The rail's sections in Harvest's order, keeping only rows the person may open;
/// a section left without rows is dropped so its header does not render.
fn nav_sections(access: NavAccess) -> Vec<(&'static str, Vec<NavRow>)> {
    let row = |to, icon, label| NavRow { to, icon, label };
    let timesheet = Route::Timesheet {
        view: ViewMode::Week,
        date: Anchor::default(),
        span: CalSpan::default(),
        user: String::new(),
    };
    [
        (
            "Track",
            vec![Some(row(timesheet, "timesheet", "Timesheet"))],
        ),
        (
            "Organize",
            vec![
                access
                    .people
                    .then(|| row(Route::AdminUsers {}, "team", "Team")),
                access
                    .clients
                    .then(|| row(Route::ClientList {}, "clients", "Clients")),
                Some(row(Route::ProjectList {}, "projects", "Projects")),
                access
                    .tasks
                    .then(|| row(Route::TaskCatalog {}, "tasks", "Tasks")),
            ],
        ),
        (
            "Bill",
            vec![Some(row(Route::InvoiceList {}, "invoices", "Invoices"))],
        ),
        (
            "Review",
            vec![
                Some(row(
                    Route::Approvals {
                        week: String::new(),
                    },
                    "approvals",
                    "Approvals",
                )),
                Some(row(Route::Reports {}, "reports", "Reports")),
            ],
        ),
    ]
    .into_iter()
    .map(|(title, rows)| (title, rows.into_iter().flatten().collect::<Vec<_>>()))
    .filter(|(_, rows)| !rows.is_empty())
    .collect()
}

/// Capitalize the first character (roles are stored lower-case: "admin" → "Admin").
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Up to two leading initials from a display name, uppercased.
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(grants: Vec<Permission>, catalog_version: u32) -> OwnPermissions {
        OwnPermissions {
            catalog_version,
            grants,
            is_administrator: false,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        }
    }

    fn access(own: Result<Option<OwnPermissions>, ServerFnError>, is_admin: bool) -> NavAccess {
        nav_access(Some(&own), is_admin)
    }

    fn granted(grants: Vec<Permission>) -> NavAccess {
        access(
            Ok(Some(canonical(grants, PERMISSION_CATALOG_VERSION))),
            false,
        )
    }

    fn labels(access: NavAccess) -> Vec<(&'static str, Vec<&'static str>)> {
        nav_sections(access)
            .into_iter()
            .map(|(title, rows)| (title, rows.iter().map(|row| row.label).collect()))
            .collect()
    }

    #[test]
    fn full_access_lists_harvest_sections_in_order() {
        let all = NavAccess {
            people: true,
            clients: true,
            tasks: true,
        };
        assert_eq!(
            labels(all),
            [
                ("Track", vec!["Timesheet"]),
                ("Organize", vec!["Team", "Clients", "Projects", "Tasks"]),
                ("Bill", vec!["Invoices"]),
                ("Review", vec!["Approvals", "Reports"]),
            ]
        );
    }

    #[test]
    fn gated_rows_are_left_out_without_access() {
        assert_eq!(
            labels(NavAccess::default()),
            [
                ("Track", vec!["Timesheet"]),
                ("Organize", vec!["Projects"]),
                ("Bill", vec!["Invoices"]),
                ("Review", vec!["Approvals", "Reports"]),
            ]
        );
    }

    #[test]
    fn policy_zero_offers_clients_to_everyone_and_team_to_admins() {
        let member = NavAccess {
            people: false,
            clients: true,
            tasks: false,
        };
        assert_eq!(access(Ok(None), false), member);
        assert_eq!(
            access(Ok(None), true),
            NavAccess {
                people: true,
                ..member
            }
        );
    }

    #[test]
    fn canonical_grants_open_their_own_rows() {
        let only = |people, clients, tasks| NavAccess {
            people,
            clients,
            tasks,
        };
        assert_eq!(
            granted(vec![Permission::PeopleReadAll]),
            only(true, false, false)
        );
        assert_eq!(
            granted(vec![Permission::PeopleReadManaged]),
            only(true, false, false)
        );
        assert_eq!(
            granted(vec![Permission::ClientReadAll]),
            only(false, true, false)
        );
        assert_eq!(
            granted(vec![Permission::TaskReadAll]),
            only(false, false, true)
        );
        let unrelated = vec![
            Permission::ProjectReadAll,
            Permission::InvoiceReadAll,
            Permission::BillableRateReadAll,
            Permission::ReportProfitabilityRead,
        ];
        assert_eq!(granted(unrelated), NavAccess::default());
    }

    #[test]
    fn unknown_permission_state_hides_every_gated_row() {
        let everything = vec![
            Permission::PeopleReadAll,
            Permission::ClientReadAll,
            Permission::TaskReadAll,
        ];
        assert_eq!(nav_access(None, true), NavAccess::default(), "loading");
        assert_eq!(
            access(Err(ServerFnError::new("unavailable")), true),
            NavAccess::default()
        );
        assert_eq!(
            access(
                Ok(Some(canonical(everything, PERMISSION_CATALOG_VERSION + 1))),
                true
            ),
            NavAccess::default(),
            "newer catalog"
        );
    }

    /// A stale or newer catalog never shows more rows than no grants at all,
    /// whatever its grants and whether the person is a legacy admin.
    #[test]
    fn other_catalog_versions_never_open_a_gated_row() {
        let gated = [
            Permission::PeopleReadAll,
            Permission::PeopleReadManaged,
            Permission::ClientReadAll,
            Permission::TaskReadAll,
        ];
        let versions = [
            0,
            PERMISSION_CATALOG_VERSION - 1,
            PERMISSION_CATALOG_VERSION + 1,
            u32::MAX,
        ];
        for version in versions {
            for mask in 0..1u8 << gated.len() {
                let grants: Vec<_> = (0..gated.len())
                    .filter(|i| mask >> i & 1 == 1)
                    .map(|i| gated[i])
                    .collect();
                for is_administrator in [false, true] {
                    let own = OwnPermissions {
                        is_administrator,
                        ..canonical(grants.clone(), version)
                    };
                    for is_admin in [false, true] {
                        assert_eq!(
                            access(Ok(Some(own.clone())), is_admin),
                            NavAccess::default(),
                            "version {version}, grants {mask:#06b}"
                        );
                    }
                }
            }
        }
    }
}
