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
    invoices::{InvoiceDetail, InvoiceList, NewProjectInvoice},
    new_project::{EditProject, NewProject},
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
    #[route("/projects/:id/edit")]
    EditProject { id: Uuid },
    #[route("/projects/:id/invoices/new")]
    NewProjectInvoice { id: Uuid },
    #[route("/projects/:id?:from&:to&:tab")]
    ProjectDetail {
        id: Uuid,
        from: Option<String>,
        to: Option<String>,
        tab: Option<String>,
    },
    #[route("/approvals")]
    Approvals {},
    #[route("/reports?:project_id&:task_id&:user_id&:from&:to&:period")]
    Reports {
        project_id: Option<String>,
        task_id: Option<String>,
        user_id: Option<String>,
        from: Option<String>,
        to: Option<String>,
        period: Option<String>,
    },
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

impl Route {
    pub fn project_detail(id: Uuid) -> Self {
        Self::ProjectDetail {
            id,
            from: None,
            to: None,
            tab: None,
        }
    }

    pub fn reports() -> Self {
        Self::Reports {
            project_id: None,
            task_id: None,
            user_id: None,
            from: None,
            to: None,
            period: None,
        }
    }

    pub fn project_report(
        id: Uuid,
        task: Option<Uuid>,
        person: Option<Uuid>,
        interval: Option<crate::models::project::ProjectActivityInterval>,
    ) -> Self {
        Self::Reports {
            project_id: Some(id.to_string()),
            task_id: task.map(|id| id.to_string()),
            user_id: person.map(|id| id.to_string()),
            from: interval.map(|range| range.from.to_string()),
            to: interval.map(|range| range.to.to_string()),
            period: Some(if interval.is_some() { "custom" } else { "all" }.into()),
        }
    }
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
                | Route::EditProject { .. }
                | Route::ProjectDetail { .. }
                | Route::NewProjectInvoice { .. }
        )
    ) || std::mem::discriminant(current) == std::mem::discriminant(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_routes_preserve_entity_and_period_context_including_invalid_values() {
        let route: Route = "/reports?project_id=invalid&task_id=also-invalid&user_id=person&from=2026-09-01&to=2026-09-30&period=custom".parse().unwrap();
        let path = route.to_string();
        for value in [
            "project_id=invalid",
            "task_id=also-invalid",
            "user_id=person",
            "from=2026-09-01",
            "to=2026-09-30",
            "period=custom",
        ] {
            assert!(path.contains(value), "{value} disappeared from {path}");
        }
    }

    #[test]
    fn project_routes_preserve_reporting_period_and_tab() {
        let id = Uuid::now_v7();
        let route: Route = format!("/projects/{id}?from=2026-09-01&to=2026-09-30&tab=team")
            .parse()
            .unwrap();
        let path = route.to_string();
        for value in ["from=2026-09-01", "to=2026-09-30", "tab=team"] {
            assert!(path.contains(value), "{value} disappeared from {path}");
        }
    }

    #[test]
    fn empty_date_arguments_are_not_dropped_as_absent_bounds() {
        let route: Route = "/reports?from=&to=&period=custom".parse().unwrap();
        assert!(
            matches!(route, Route::Reports { from: Some(from), to: Some(to), .. } if from.is_empty() && to.is_empty())
        );
    }

    #[test]
    fn project_report_links_round_trip_parent_child_and_total_scopes() {
        let project = Uuid::now_v7();
        let task = Uuid::now_v7();
        let person = Uuid::now_v7();
        for (task, person) in [
            (None, None),
            (Some(task), None),
            (None, Some(person)),
            (Some(task), Some(person)),
        ] {
            let route = Route::project_report(project, task, person, None);
            let parsed: Route = route.to_string().parse().unwrap();
            assert!(route == parsed);
            assert!(
                matches!(parsed, Route::Reports { project_id: Some(id), task_id, user_id, from: None, to: None, period: Some(period) } if id == project.to_string() && task_id == task.map(|id| id.to_string()) && user_id == person.map(|id| id.to_string()) && period == "all")
            );
        }
    }

    #[test]
    fn project_invoice_creation_keeps_project_identity_and_navigation() {
        let id = Uuid::now_v7();
        let path = format!("/projects/{id}/invoices/new");
        let route: Route = path.parse().unwrap();
        assert_eq!(route.to_string(), path);
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::InvoiceList {}, &route));
    }

    #[test]
    fn project_detail_highlights_only_projects() {
        let route = Route::project_detail(Uuid::now_v7());
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::ClientList {}, &route));
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
            matches!(format!("/projects/{id}").parse::<Route>().unwrap(), Route::ProjectDetail { id: parsed, .. } if parsed == id)
        );
    }
}
