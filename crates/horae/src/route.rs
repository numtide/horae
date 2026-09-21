use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::admin_shell::AdminShell;
use crate::components::layout::AppLayout;
use crate::pages::{
    admin::AdminUsers,
    approvals::Approvals,
    clients::{ClientDetail, ClientList},
    gallery::Gallery,
    importers::HarvestImport,
    invoices::{InvoiceDetail, InvoiceList},
    new_project::NewProject,
    projects::{ProjectDetail, ProjectList},
    reports::Reports,
    settings::Settings,
    timesheet::{Anchor, CalSpan, Timesheet, ViewMode},
};

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h1 { style: "font-size: 2rem; color: var(--color-text-muted); text-align: center;", "404" }
                p { style: "text-align: center; color: var(--color-text-secondary);",
                    "Page not found: /{route.join(\"/\")}"
                }
                div { style: "text-align: center; margin-top: 1rem;",
                    Link {
                        to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() },
                        class: "btn btn-primary",
                        "Go to Timesheet"
                    }
                }
            }
        }
    }
}

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    // /auth/* routes are handled by Axum directly (see src/auth/mod.rs).
    // The Dioxus router only manages the authenticated SPA.
    #[layout(AppLayout)]
    // Clean, shareable paths like Harvest (/timesheet/day/2026-08-06); bare "/"
    // lands on this week.
    #[redirect("/", || Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default() })]
    #[route("/timesheet/:view/:date?:span")]
    Timesheet {
        view: ViewMode,
        date: Anchor,
        span: CalSpan,
    },
    #[route("/clients")]
    ClientList {},
    #[route("/clients/:id")]
    ClientDetail { id: Uuid },
    #[route("/projects")]
    ProjectList {},
    #[route("/projects/new")]
    NewProject {},
    #[route("/projects/:id")]
    ProjectDetail { id: Uuid },
    #[route("/approvals")]
    Approvals {},
    #[route("/reports")]
    Reports {},
    #[route("/invoices")]
    InvoiceList {},
    #[route("/invoices/:id")]
    InvoiceDetail { id: Uuid },
    #[layout(AdminShell)]
    #[route("/admin/users")]
    AdminUsers {},
    #[route("/admin/importers")]
    HarvestImport {},
    #[end_layout]
    #[route("/settings")]
    Settings {},
    #[route("/components")]
    Gallery {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

/// Whether `to` names the route the user is currently on, compared by variant
/// only so a parameterized route (e.g. the dated timesheet) stays active across
/// all its parameter values. Call from a component to highlight the current nav
/// link. Runs on both the server and web targets.
pub fn route_is_active(to: &Route) -> bool {
    matches_navigation(to, &use_route::<Route>())
}

fn matches_navigation(to: &Route, current: &Route) -> bool {
    matches!((to, current), (Route::ProjectList {}, Route::NewProject {}))
        || std::mem::discriminant(current) == std::mem::discriminant(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_project_is_static_and_highlights_only_projects() {
        let route: Route = "/projects/new".parse().unwrap();
        assert!(matches!(route, Route::NewProject {}));
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::ClientList {}, &route));
        assert!(!matches_navigation(&Route::InvoiceList {}, &route));
    }
}
