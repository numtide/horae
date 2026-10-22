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
    tasks::TaskCatalog,
    timesheet::{Anchor, CalSpan, Timesheet, ViewMode},
};
// Route constructors outside the editor name the typed query through this module.
pub use crate::pages::new_project::EditReturn;

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
    // The whole query is the typed `EditReturn`, so only its canonical form survives
    // a reparse; only the project detail sets it.
    #[route("/projects/:id/edit?:..return_to")]
    EditProject { id: Uuid, return_to: EditReturn },
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
    #[route("/admin/tasks")]
    TaskCatalog {},
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
    use crate::pages::new_project::edit_return_route;

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
        assert!(
            matches!(&route, Route::EditProject { id: parsed, return_to } if *parsed == id && *return_to == EditReturn::default())
        );
        // The router may append an empty query delimiter; the route itself must survive.
        assert!([path.clone(), format!("{path}?")].contains(&route.to_string()));
        assert!(route.to_string().parse::<Route>().unwrap() == route);
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        assert!(!matches_navigation(&Route::ClientList {}, &route));
        assert!(
            matches!(format!("/projects/{id}").parse::<Route>().unwrap(), Route::ProjectDetail { id: parsed } if parsed == id)
        );
    }

    /// The editor's exit for a parsed edit URL and the URL the router writes back,
    /// asserting the URL kept its project and that repeated write/parse round
    /// trips change neither the route, its exit nor the written URL.
    fn edit_exit(url: &str, id: Uuid) -> (Route, String) {
        let exit_of = |route: &Route| match route {
            Route::EditProject {
                id: parsed,
                return_to,
            } => {
                assert_eq!(*parsed, id, "{url}");
                edit_return_route(id, *return_to)
            }
            _ => panic!("{url} is not the project editor"),
        };
        let mut route = url.parse::<Route>().unwrap();
        let exit = exit_of(&route);
        let canonical = route.to_string();
        for round in 1..=3 {
            let reparsed: Route = route.to_string().parse().unwrap();
            assert!(
                reparsed == route,
                "{url}: route changed after round trip {round}"
            );
            assert!(
                exit_of(&reparsed) == exit,
                "{url}: exit changed after round trip {round}"
            );
            assert_eq!(reparsed.to_string(), canonical, "{url}: round trip {round}");
            route = reparsed;
        }
        (exit, canonical)
    }

    /// No target, or one that is not lexically the canonical query: the router
    /// writes back only the bare editor URL, and the editor exits to Projects.
    fn assert_bare_list_exit(url: &str, id: Uuid) {
        let written = format!("/projects/{id}/edit?");
        assert!(
            edit_exit(url, id) == (Route::ProjectList {}, written),
            "{url}"
        );
    }

    #[test]
    fn detail_origin_edit_round_trips_and_returns_to_the_same_project() {
        let id = Uuid::now_v7();
        let route = Route::EditProject {
            id,
            return_to: EditReturn::detail(id),
        };
        let url = route.to_string();
        assert_eq!(url, format!("/projects/{id}/edit?return_to=/projects/{id}"));
        assert_eq!(
            url,
            format!(
                "/projects/{id}/edit?return_to={}",
                Route::ProjectDetail { id }
            )
        );
        assert!(url.parse::<Route>().unwrap() == route);
        assert!(edit_exit(&url, id) == (Route::ProjectDetail { id }, url.clone()));
        assert!(matches_navigation(&Route::ProjectList {}, &route));
        // The router decodes the query once, so a standard encoding is the same path.
        let encoded = format!("/projects/{id}/edit?return_to=%2Fprojects%2F{id}");
        assert!(edit_exit(&encoded, id) == (Route::ProjectDetail { id }, url));
    }

    #[test]
    fn list_origin_and_plain_edit_urls_keep_returning_to_projects() {
        let id = Uuid::now_v7();
        let list_origin = Route::EditProject {
            id,
            return_to: EditReturn::default(),
        }
        .to_string();
        for url in [format!("/projects/{id}/edit"), list_origin] {
            assert_bare_list_exit(&url, id);
        }
    }

    #[test]
    fn foreign_unsafe_or_extended_return_targets_fall_back_to_projects() {
        let id = Uuid::now_v7();
        let other = Uuid::now_v7();
        // Another project's canonical target is a valid query, so it is kept as
        // typed and written back unchanged, but it never leaves this editor for
        // that project.
        let foreign = format!("/projects/{id}/edit?return_to=/projects/{other}");
        assert!(
            matches!(foreign.parse::<Route>().unwrap(), Route::EditProject { return_to, .. }
                if return_to == EditReturn::detail(other))
        );
        assert!(edit_exit(&foreign, id) == (Route::ProjectList {}, foreign.clone()));
        // Everything else is not lexically the canonical query and is dropped.
        let upper = id.to_string().to_uppercase();
        let simple = id.simple().to_string();
        let braced = id.braced().to_string();
        let urn = id.urn().to_string();
        for target in [
            String::new(),
            format!("/projects/{upper}"),
            format!("/projects/{simple}"),
            format!("/projects/{braced}"),
            format!("/projects/{urn}"),
            format!("projects/{id}"),
            format!("/projects/{id}/"),
            format!("/projects/{id}/edit"),
            format!("/projects/{id}%3Ftab=team"),
            format!("/projects/{id}%23team"),
            format!("/projects/{id}%20"),
            format!("%20/projects/{id}"),
            format!("/projects/{id}=x"),
            format!("/projects//{id}"),
            format!("//evil.example/projects/{id}"),
            format!("https://evil.example/projects/{id}"),
            format!("%2F%2Fevil.example%2Fprojects%2F{id}"),
            // Decoded once only: a double encoding is not this project's path.
            format!("%252Fprojects%252F{id}"),
            "javascript:alert(1)".into(),
            "/projects/not-a-project".into(),
            format!("/clients/{id}"),
            "/projects".into(),
        ] {
            let url = format!("/projects/{id}/edit?return_to={target}");
            assert_bare_list_exit(&url, id);
        }
    }

    #[test]
    fn extra_encoded_targets_stay_invalid_after_the_router_rewrites_the_url() {
        let id = Uuid::now_v7();
        // After one decode these are not the exact path, so they must never
        // become it, however often the router writes and reparses the URL.
        for target in [
            format!("%252Fprojects%252F{id}"),
            format!("%25252Fprojects%25252F{id}"),
            format!("%2Fprojects%252F{id}"),
            format!("/projects%252F{id}"),
            format!("%252Fprojects/{id}"),
            format!("/projects/{id}%2520"),
            format!("/projects/{id}%26x=1"),
            format!("/projects/{id}%26return_to=/projects/{id}"),
            format!("/projects/{id}%2526x=1"),
        ] {
            let url = format!("/projects/{id}/edit?return_to={target}");
            assert_bare_list_exit(&url, id);
        }
    }

    #[test]
    fn extra_or_repeated_query_parameters_fail_closed() {
        let id = Uuid::now_v7();
        let other = Uuid::now_v7();
        for query in [
            format!("return_to=/projects/{id}&x=1"),
            format!("x=1&return_to=/projects/{id}"),
            format!("return_to=/projects/{id}&return_to=/projects/{id}"),
            format!("return_to=/projects/{other}&return_to=/projects/{id}"),
            format!("return_to=/projects/{id}&"),
            format!("&return_to=/projects/{id}"),
            format!("Return_to=/projects/{id}"),
        ] {
            let url = format!("/projects/{id}/edit?{query}");
            assert_bare_list_exit(&url, id);
        }
    }

    #[test]
    fn return_target_never_selects_a_different_project() {
        let id = Uuid::now_v7();
        let other = Uuid::now_v7();
        let target =
            EditReturn::from(format!("return_to={}", Route::ProjectDetail { id: other }).as_str());
        assert!(target == EditReturn::detail(other));
        assert!(edit_return_route(id, target) == Route::ProjectList {});
        assert!(edit_return_route(other, target) == Route::ProjectDetail { id: other });
        assert!(edit_return_route(id, EditReturn::default()) == Route::ProjectList {});
    }
}
