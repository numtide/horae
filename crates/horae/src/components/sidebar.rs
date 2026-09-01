use dioxus::prelude::*;

use crate::components::avatar::Avatar;
use crate::components::icons::NavIcon;
use crate::components::logo::HoraeMark;
use crate::components::timer_widget::TimerWidget;
use crate::pages::timesheet::{Anchor, CalSpan, ViewMode};
use crate::route::Route;
use crate::server_fns;

/// The left rail: brand, a start-timer action, grouped navigation with an active
/// state, and a footer showing the signed-in user. `collapsed` is owned by
/// `AppLayout` so the shell can narrow the content area in step with the rail.
#[component]
pub fn Sidebar(collapsed: Signal<bool>) -> Element {
    rsx! {
        aside { class: "app-sidebar",
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

            div { class: "sidebar-section", "Track" }
            div { class: "sidebar-group",
                SideLink { to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() }, icon: "timesheet", label: "Timesheet" }
            }

            div { class: "sidebar-section", "Organize" }
            div { class: "sidebar-group",
                SideLink { to: Route::ClientList {}, icon: "clients", label: "Clients" }
                SideLink { to: Route::ProjectList {}, icon: "projects", label: "Projects" }
                SideLink { to: Route::InvoiceList {}, icon: "invoices", label: "Invoices" }
            }

            div { class: "sidebar-section", "Review" }
            div { class: "sidebar-group",
                SideLink { to: Route::Approvals {}, icon: "approvals", label: "Approvals" }
                SideLink { to: Route::Reports {}, icon: "reports", label: "Reports" }
            }

            div { class: "sidebar-spacer" }

            SidebarUser {}
        }
    }
}

/// One rail row: a client-side `Link` that auto-marks itself active for its route.
/// The active route keeps its icon (tinted, over a raised surface) rather than
/// swapping it out, matching Harvest's rail.
#[component]
fn SideLink(to: Route, icon: String, label: String) -> Element {
    // Match by route variant, not the exact URL, so a param-carrying route (the
    // timesheet's /timesheet/<view>/<date>) stays highlighted on any view/day.
    let active = std::mem::discriminant(&use_route::<Route>()) == std::mem::discriminant(&to);
    rsx! {
        Link { to, class: if active { "nav-item active" } else { "nav-item" },
            span { class: "nav-item-icon", NavIcon { name: icon } }
            span { class: "nav-item-label", "{label}" }
        }
    }
}

/// The signed-in user: an avatar + name + role row that opens an account popover
/// (settings, an admin section for admins, and sign out). Falls back to a
/// placeholder until `get_me` resolves (or when not authenticated).
#[component]
fn SidebarUser() -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut open = use_signal(|| false);

    let user = me.read();
    let (name, email, role, marks, is_admin) = match &*user {
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
    // The role renders as a compact status pill; admins get the pine variant.
    let role_class = if is_admin {
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
                        Link { to: Route::Settings {}, class: "menu-item", onclick: move |_| open.set(false),
                            span { class: "menu-item-icon", NavIcon { name: "settings" } }
                            "Settings"
                        }
                    }
                    // Org administration, only for admins.
                    if is_admin {
                        div { class: "sidebar-menu-list",
                            div { class: "menu-group", "Admin" }
                            Link { to: Route::AdminUsers {}, class: "menu-item", onclick: move |_| open.set(false),
                                span { class: "menu-item-icon", NavIcon { name: "users" } }
                                "Users"
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
                onclick: move |_| open.set(!open()),
                Avatar { initials: "{marks}" }
                // While the popover is open its header carries the identity, so the
                // footer collapses to just the avatar (no duplicate name/role).
                if !open() {
                    div { class: "sidebar-user",
                        div { class: "sidebar-user-name truncate", "{name}" }
                        if !role.is_empty() {
                            div { class: "sidebar-user-sub", "{role}" }
                        }
                    }
                    span { class: "sidebar-user-caret", "⌄" }
                }
            }
        }
    }
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
