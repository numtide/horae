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
    invoices::{InvoiceDetail, InvoiceList, NewInvoiceForClient},
    new_project::{EditProject, NewProject, NewProjectForClient},
    permission_audit::PermissionAudit,
    projects::{ProjectDetail, ProjectList, ProjectsForClient},
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
                        to: Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() },
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
    #[redirect("/", || Route::Timesheet { view: ViewMode::Week, date: Anchor::default(), span: CalSpan::default(), user: String::new() })]
    #[route("/timesheet/:view/:date?:span&:user")]
    Timesheet {
        view: ViewMode,
        date: Anchor,
        span: CalSpan,
        user: String,
    },
    #[route("/clients")]
    ClientList {},
    #[route("/clients/:id")]
    ClientDetail { id: Uuid },
    #[route("/projects")]
    ProjectList {},
    #[route("/projects/client/:client")]
    ProjectsForClient { client: String },
    #[route("/projects/new")]
    NewProject {},
    #[route("/projects/new/client/:client")]
    NewProjectForClient { client: String },
    #[route("/projects/:id/edit")]
    EditProject { id: Uuid },
    #[route("/projects/:id")]
    ProjectDetail { id: Uuid },
    #[route("/approvals")]
    Approvals {},
    #[route("/reports")]
    Reports {},
    #[route("/invoices")]
    InvoiceList {},
    #[route("/invoices/new/client/:client")]
    NewInvoiceForClient { client: String },
    #[route("/invoices/:id")]
    InvoiceDetail { id: Uuid },
    #[layout(AdminShell)]
    #[route("/admin/users")]
    AdminUsers {},
    #[route("/admin/importers")]
    HarvestImport {},
    #[route("/admin/audit")]
    PermissionAudit {},
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
    matches!(
        (to, current),
        (
            Route::ProjectList {},
            Route::NewProject {}
                | Route::NewProjectForClient { .. }
                | Route::ProjectsForClient { .. }
                | Route::EditProject { .. }
        ) | (Route::ClientList {}, Route::ClientDetail { .. })
            | (Route::InvoiceList {}, Route::NewInvoiceForClient { .. })
    ) || std::mem::discriminant(current) == std::mem::discriminant(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timesheet_routes_preserve_selected_person_in_every_view() {
        let user = Uuid::now_v7();
        for view in ["day", "week", "calendar"] {
            for span in ["day", "5day", "week"] {
                let url = format!("/timesheet/{view}/2026-09-07?span={span}&user={user}");
                let route: Route = url.parse().unwrap();
                assert_eq!(route.to_string(), url);
            }
        }
    }

    #[test]
    fn invalid_timesheet_person_is_not_silently_removed_from_the_route() {
        let url = "/timesheet/week/2026-09-07?span=week&user=invalid-person";
        assert_eq!(url.parse::<Route>().unwrap().to_string(), url);
    }

    #[test]
    fn existing_timesheet_links_without_a_person_still_select_own_time() {
        for url in [
            "/timesheet/week/2026-09-07",
            "/timesheet/calendar/2026-09-07?span=5day",
        ] {
            assert!(
                matches!(url.parse::<Route>().unwrap(), Route::Timesheet { user, .. } if user.is_empty())
            );
        }
    }

    #[test]
    fn client_workflow_paths_preserve_context_and_navigation_section() {
        let id = Uuid::now_v7();
        for (path, section) in [
            (format!("/projects/new/client/{id}"), Route::ProjectList {}),
            (format!("/projects/client/{id}"), Route::ProjectList {}),
            (format!("/invoices/new/client/{id}"), Route::InvoiceList {}),
        ] {
            let route: Route = path.parse().unwrap();
            assert!(!matches!(route, Route::NotFound { .. }), "{path}");
            assert_eq!(route.to_string(), path);
            assert!(matches_navigation(&section, &route));
            assert!(!matches_navigation(&Route::ClientList {}, &route));
        }
    }

    #[test]
    fn malformed_context_reaches_the_workflow_for_validation_after_recovery() {
        for path in [
            "/projects/new/client/not-a-client",
            "/projects/client/not-a-client",
            "/invoices/new/client/not-a-client",
        ] {
            let route: Route = path.parse().unwrap();
            assert!(!matches!(route, Route::NotFound { .. }), "{path}");
            assert_eq!(route.to_string(), path);
        }
    }

    #[test]
    fn client_detail_preserves_identity_and_highlights_only_clients() {
        let id = Uuid::now_v7();
        let path = format!("/clients/{id}");
        let route: Route = path.parse().unwrap();
        assert!(matches!(route, Route::ClientDetail { id: parsed } if parsed == id));
        assert_eq!(route.to_string(), path);
        assert!(matches_navigation(&Route::ClientList {}, &route));
        assert!(!matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::InvoiceList {}, &route));
    }

    #[test]
    fn new_project_is_static_and_highlights_only_projects() {
        let route: Route = "/projects/new".parse().unwrap();
        assert!(matches!(route, Route::NewProject {}));
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::ClientList {}, &route));
        assert!(!matches_navigation(&Route::InvoiceList {}, &route));
    }

    #[test]
    fn edit_project_preserves_identity_and_highlights_only_projects() {
        let id = Uuid::now_v7();
        let path = format!("/projects/{id}/edit");
        let route: Route = path.parse().unwrap();
        assert!(matches!(route, Route::EditProject { id: parsed } if parsed == id));
        assert_eq!(route.to_string(), path);
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::ClientList {}, &route));
        assert!(
            matches!(format!("/projects/{id}").parse::<Route>().unwrap(), Route::ProjectDetail { id: parsed } if parsed == id)
        );
    }
}
