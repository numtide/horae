use dioxus::prelude::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};

use crate::components::admin_shell::first_admin_route;
use crate::components::avatar::Avatar;
use crate::components::icons::NavIcon;
use crate::components::logo::HoraeMark;
use crate::components::session::{Session, use_session};
use crate::components::timer_widget::TimerWidget;
use crate::components::version_info::VersionInfo;
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
    let sections = nav_sections(nav_access(session.permissions.read().as_ref()));
    let version_block = show_version(session.permissions.read().as_ref());
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

            if version_block {
                VersionInfo {}
            }

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

/// The signed-in user: an avatar + name + profile row that opens an account popover
/// (settings, Administration when an admin section is open to them, and sign out). Falls back to a
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
    let (name, email, marks) = match &*user {
        Some(Ok(u)) => (u.name.clone(), u.email.clone(), initials(&u.name)),
        _ => ("Not signed in".to_string(), String::new(), "·".to_string()),
    };
    let administrator = administrator(own.read().as_ref());
    let profile_name = profile(own.read().as_ref());
    let admin_route = first_admin_route(own.read().as_ref());
    let profile_class = if administrator {
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
                        if !profile_name.is_empty() {
                            span { class: "{profile_class}", "{profile_name}" }
                        }
                    }
                    div { class: "sidebar-menu-list",
                        Link { to: Route::Settings {}, class: "menu-item", onclick: move |_| { open.set(false); on_navigate.call(()); },
                            span { class: "menu-item-icon", NavIcon { name: "users" } }
                            "Settings"
                        }
                        // The admin shell's own sub-navigation reaches the other sections.
                        if let Some(to) = admin_route {
                            Link { to, class: "menu-item", onclick: move |_| { open.set(false); on_navigate.call(()); },
                                span { class: "menu-item-icon", NavIcon { name: "settings" } }
                                "Administration"
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
                    if !profile_name.is_empty() {
                        div { class: "sidebar-user-sub", "{profile_name}" }
                    }
                }
                span { class: "sidebar-user-caret", if open() { "⌃" } else { "⌄" } }
            }
        }
    }
}

/// The stored permissions when they loaded under the catalog this bundle knows.
fn current(own: Option<&Result<OwnPermissions, ServerFnError>>) -> Option<&OwnPermissions> {
    match own {
        Some(Ok(access)) if access.catalog_version == PERMISSION_CATALOG_VERSION => Some(access),
        _ => None,
    }
}

/// Administrator authority for display: the stored identity, and nothing while
/// unknown or failed.
fn administrator(own: Option<&Result<OwnPermissions, ServerFnError>>) -> bool {
    current(own).is_some_and(|access| access.is_administrator)
}

/// The name the account menu gives the person's access: their profile or
/// template, and nothing while unknown or failed.
fn profile(own: Option<&Result<OwnPermissions, ServerFnError>>) -> String {
    current(own).map_or_else(String::new, |access| {
        access
            .profile
            .clone()
            .unwrap_or_else(|| "Individual configuration".into())
    })
}

/// Whether to show the version block: to administrators, and also under a
/// catalog version this bundle does not know, since that means the bundle is
/// older than the server and the block is what says so. The server still
/// refuses the version to anyone else.
fn show_version(own: Option<&Result<OwnPermissions, ServerFnError>>) -> bool {
    match own {
        Some(Ok(access)) if access.catalog_version != PERMISSION_CATALOG_VERSION => true,
        own => administrator(own),
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

/// Each gated row follows its read grant. Failed or unknown permission state
/// hides every gated row.
fn nav_access(own: Option<&Result<OwnPermissions, ServerFnError>>) -> NavAccess {
    current(own).map_or_else(NavAccess::default, |access| NavAccess {
        people: access.grants.contains(&Permission::PeopleReadAll)
            || access.grants.contains(&Permission::PeopleReadManaged),
        clients: access.grants.contains(&Permission::ClientReadAll),
        tasks: access.grants.contains(&Permission::TaskReadAll),
    })
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
            profile: None,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        }
    }

    fn granted(grants: Vec<Permission>) -> NavAccess {
        nav_access(Some(&Ok(canonical(grants, PERMISSION_CATALOG_VERSION))))
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
    fn grants_open_their_own_rows() {
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
    fn the_administrator_identity_opens_no_row_by_itself() {
        let admin = OwnPermissions {
            is_administrator: true,
            ..canonical(Vec::new(), PERMISSION_CATALOG_VERSION)
        };
        assert_eq!(nav_access(Some(&Ok(admin))), NavAccess::default());
    }

    #[test]
    fn unknown_permission_state_hides_every_gated_row() {
        let everything = vec![
            Permission::PeopleReadAll,
            Permission::ClientReadAll,
            Permission::TaskReadAll,
        ];
        assert_eq!(nav_access(None), NavAccess::default(), "loading");
        assert_eq!(
            nav_access(Some(&Err(ServerFnError::new("unavailable")))),
            NavAccess::default()
        );
        assert_eq!(
            nav_access(Some(&Ok(canonical(
                everything,
                PERMISSION_CATALOG_VERSION + 1
            )))),
            NavAccess::default(),
            "newer catalog"
        );
    }

    /// The previous, next and largest catalog versions never show more rows
    /// than no grants at all, for every subset of the gating grants and
    /// whether or not the person is an administrator.
    #[test]
    fn other_catalog_versions_never_open_a_gated_row() {
        let gated = [
            Permission::PeopleReadAll,
            Permission::PeopleReadManaged,
            Permission::ClientReadAll,
            Permission::TaskReadAll,
        ];
        let versions = [
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
                    assert_eq!(
                        nav_access(Some(&Ok(own))),
                        NavAccess::default(),
                        "version {version}, grants {mask:#06b}"
                    );
                }
            }
        }
    }

    #[test]
    fn administrator_follows_the_stored_identity() {
        let admin = OwnPermissions {
            is_administrator: true,
            ..canonical(Vec::new(), PERMISSION_CATALOG_VERSION)
        };
        assert!(administrator(Some(&Ok(admin.clone()))));
        // Every grant without the identity is not an administrator.
        assert!(!administrator(Some(&Ok(canonical(
            Permission::ALL.to_vec(),
            PERMISSION_CATALOG_VERSION
        )))));
        let future = OwnPermissions {
            catalog_version: PERMISSION_CATALOG_VERSION + 1,
            ..admin
        };
        assert!(!administrator(Some(&Ok(future))));
        assert!(!administrator(None));
        assert!(!administrator(Some(&Err(ServerFnError::new(
            "unavailable"
        )))));
    }

    #[test]
    fn version_block_follows_administrator_authority() {
        let admin = OwnPermissions {
            is_administrator: true,
            ..canonical(Vec::new(), PERMISSION_CATALOG_VERSION)
        };
        assert!(show_version(Some(&Ok(admin))));
        assert!(!show_version(Some(&Ok(canonical(
            Vec::new(),
            PERMISSION_CATALOG_VERSION
        )))));
        assert!(!show_version(None));
        assert!(!show_version(Some(&Err(ServerFnError::new("unavailable")))));
    }

    /// A deploy that changes the catalog leaves open tabs on the old one, and
    /// those are the tabs that need the stale-bundle warning. The server still
    /// refuses the version to anyone who is not an administrator.
    #[test]
    fn version_block_shows_under_another_catalog_version() {
        for is_administrator in [false, true] {
            let own = OwnPermissions {
                is_administrator,
                ..canonical(Vec::new(), PERMISSION_CATALOG_VERSION + 1)
            };
            assert!(show_version(Some(&Ok(own))));
        }
    }

    #[test]
    fn the_account_menu_names_the_profile_or_template() {
        let named = |profile: Option<&str>| OwnPermissions {
            profile: profile.map(str::to_owned),
            ..canonical(Vec::new(), PERMISSION_CATALOG_VERSION)
        };
        assert_eq!(
            profile(Some(&Ok(named(Some("Project Manager"))))),
            "Project Manager"
        );
        assert_eq!(
            profile(Some(&Ok(named(Some("Night shift"))))),
            "Night shift"
        );
        assert_eq!(profile(Some(&Ok(named(None)))), "Individual configuration");
        assert_eq!(profile(None), "");
        assert_eq!(profile(Some(&Err(ServerFnError::new("unavailable")))), "");
        let future = OwnPermissions {
            catalog_version: PERMISSION_CATALOG_VERSION + 1,
            ..named(Some("Administrator"))
        };
        assert_eq!(profile(Some(&Ok(future))), "");
    }
}
