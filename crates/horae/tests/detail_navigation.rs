#![cfg(feature = "server")]

//! Exercise the production detail pages through Dioxus routing, with controlled
//! server responses. This checks client resource identity, not HTTP or DB access.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::Navigator;
use dioxus::router::components::HistoryProvider;
use futures_util::FutureExt;
use horae_core::types::{InvoiceStatus, OrgRole, ProjectRole};
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/components/badge.rs"]
pub mod badge;
#[path = "../src/components/combobox.rs"]
pub mod combobox;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/menu.rs"]
pub mod menu;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/table.rs"]
pub mod table;
mod components {
    pub use super::{badge, combobox, controls, form, icons, menu, modal, table};
}
#[path = "../src/models/assignment.rs"]
mod assignment;
#[path = "../src/models/client.rs"]
pub mod client;
#[path = "../src/pages/clients.rs"]
mod clients;
#[path = "../src/models/invoice.rs"]
pub mod invoice;
#[path = "../src/pages/invoices.rs"]
mod invoices;
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
#[path = "../src/models/project.rs"]
mod project;
#[path = "../src/models/project_creation.rs"]
pub mod project_creation;
#[path = "../src/models/project_managers.rs"]
pub mod project_managers;
#[path = "../src/pages/projects.rs"]
mod projects;
#[path = "../src/models/task.rs"]
mod task;
#[path = "../src/models/user.rs"]
pub mod user;
mod models {
    pub use super::{client, invoice, permission_editor, project_creation, project_managers};
    pub use super::{
        client::Client,
        project::{
            Project, ProjectBudgetProgress, ProjectDetails, ProjectTagLink, ProjectTaskRate,
        },
    };
}

type InvoiceResponse = Result<invoice::InvoiceWithLines, ServerFnError>;
type AssignmentResponse = Result<Vec<assignment::Assignment>, ServerFnError>;
type ProjectDetailsResponse = Result<project::ProjectDetails, ServerFnError>;
type ClientDetailsResponse = Result<client::ClientDetails, ServerFnError>;
type ClientInvoicesResponse = Result<Vec<invoice::Invoice>, ServerFnError>;
type ClientProjectsResponse = Result<Vec<project::Project>, ServerFnError>;
type ProjectSpendResponse = Result<Vec<server_fns::ProjectSpend>, ServerFnError>;

#[derive(Clone, Default)]
struct Probe {
    member: bool,
    client_requests: Rc<RefCell<Vec<Uuid>>>,
    client_invoice_requests: Rc<RefCell<Vec<Uuid>>>,
    client_response: Rc<RefCell<Option<oneshot::Receiver<ClientDetailsResponse>>>>,
    client_invoice_response: Rc<RefCell<Option<oneshot::Receiver<ClientInvoicesResponse>>>>,
    client_projects_response: Rc<RefCell<Option<oneshot::Receiver<ClientProjectsResponse>>>>,
    project_spend_response: Rc<RefCell<Option<oneshot::Receiver<ProjectSpendResponse>>>>,
    initial_path: Option<String>,
    requests: Rc<RefCell<Vec<Uuid>>>,
    assignment_requests: Rc<RefCell<Vec<Uuid>>>,
    detail_requests: Rc<RefCell<Vec<Uuid>>>,
    task_requests: Rc<RefCell<Vec<Uuid>>>,
    navigator: Rc<RefCell<Option<Navigator>>>,
    scope: Rc<RefCell<Option<ScopeId>>>,
    response: Rc<RefCell<Option<oneshot::Receiver<InvoiceResponse>>>>,
    assignment_response: Rc<RefCell<Option<oneshot::Receiver<AssignmentResponse>>>>,
    detail_response: Rc<RefCell<Option<oneshot::Receiver<ProjectDetailsResponse>>>>,
}

fn app(probe: Probe) -> Element {
    let initial_path = probe
        .initial_path
        .clone()
        .unwrap_or_else(|| format!("/invoices/{}", Uuid::from_u128(1)));
    use_context_provider(|| probe);
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(initial_path.clone())) as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

mod route {
    use super::*;
    use clients::{ClientDetail, ClientList};
    use invoices::{InvoiceDetail, InvoiceList, NewInvoiceForClient};
    use projects::{ProjectDetail, ProjectList, ProjectsForClient};

    #[derive(Clone, PartialEq, Routable)]
    pub enum Route {
        #[layout(Layout)]
        #[route("/clients")]
        ClientList {},
        #[route("/clients/:id")]
        ClientDetail { id: Uuid },
        #[route("/invoices")]
        InvoiceList {},
        #[route("/invoices/new/client/:client")]
        NewInvoiceForClient { client: String },
        #[route("/invoices/:id")]
        InvoiceDetail { id: Uuid },
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
        #[route("/admin/importers")]
        HarvestImport {},
    }

    #[component]
    fn NewProject() -> Element {
        rsx! { h1 { "New project" } }
    }

    #[component]
    fn NewProjectForClient(client: String) -> Element {
        rsx! { h1 { "New project for {client}" } }
    }

    #[component]
    fn EditProject(id: Uuid) -> Element {
        rsx! { h1 { "Edit project {id}" } }
    }

    #[component]
    fn HarvestImport() -> Element {
        rsx! { h1 { "Importers" } }
    }

    #[component]
    fn Layout() -> Element {
        let probe = use_context::<Probe>();
        *probe.navigator.borrow_mut() = Some(use_navigator());
        *probe.scope.borrow_mut() = Some(dioxus::core::current_scope_id());
        rsx! { Outlet::<Route> {} }
    }
}

fn settle(dom: &mut VirtualDom) {
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("detail navigation did not settle");
}

#[tokio::test]
async fn client_detail_navigation_loads_current_identity_billing_and_work() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let probe = Probe {
        initial_path: Some(format!("/clients/{first}")),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in [
        "Client-1",
        "Address-1",
        "Client-project-1",
        "INV-1",
        "EUR 0.00 / h",
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ClientDetail { id: second })
    });
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in ["Client-2", "Address-2", "Client-project-2", "INV-2"] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    for stale in ["Client-1", "Address-1", "Client-project-1", "INV-1"] {
        assert!(!html.contains(stale), "stale {stale}: {html}");
    }
    assert_eq!(*probe.client_requests.borrow(), [first, second]);
    assert_eq!(*probe.client_invoice_requests.borrow(), [first, second]);
    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Client-1"));
    assert_eq!(*probe.client_requests.borrow(), [first, second, first]);
}

#[tokio::test]
async fn client_detail_links_use_current_client_and_preserve_member_boundaries() {
    let id = Uuid::from_u128(1);
    for member in [false, true] {
        let probe = Probe {
            member,
            initial_path: Some(format!("/clients/{id}")),
            ..Probe::default()
        };
        let mut dom = VirtualDom::new_with_props(app, probe);
        dom.rebuild_in_place();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains(&format!("href=\"/projects/client/{id}\"")),
            "{html}"
        );
        for prefix in ["/projects/new/client/", "/invoices/new/client/"] {
            assert_eq!(
                html.contains(&format!("href=\"{prefix}{id}\"")),
                !member,
                "{html}"
            );
        }
    }
}

#[tokio::test]
async fn pending_or_failed_client_navigation_never_shows_previous_identity_or_work() {
    let first = Uuid::from_u128(1);
    let probe = Probe {
        initial_path: Some(format!("/clients/{first}")),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Client-1"));
    let (send, receive) = oneshot::channel();
    *probe.client_response.borrow_mut() = Some(receive);
    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ClientDetail {
            id: Uuid::from_u128(2),
        })
    });
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading client"), "{html}");
    for stale in ["Client-1", "Address-1", "Client-project-1", "INV-1"] {
        assert!(!html.contains(stale), "stale {stale}: {html}");
    }
    send.send(Err(ServerFnError::new("Client unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Retry client"), "{html}");
    assert!(!html.contains("No projects"), "{html}");
    assert_eq!(*probe.client_invoice_requests.borrow(), [first]);
}

#[tokio::test]
async fn member_client_detail_never_requests_invoices_or_displays_default_rates() {
    let probe = Probe {
        member: true,
        initial_path: Some(format!("/clients/{}", Uuid::from_u128(1))),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Address-1") && html.contains("Client-project-1"),
        "{html}"
    );
    assert!(
        !html.contains("Default rate") && !html.contains("Invoices"),
        "{html}"
    );
    assert!(probe.client_invoice_requests.borrow().is_empty());
}

#[tokio::test]
async fn client_project_failure_preserves_billing_and_invoices() {
    let probe = Probe {
        initial_path: Some(format!("/clients/{}", Uuid::from_u128(1))),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.client_projects_response.borrow_mut() = Some(receive);
    let mut dom = VirtualDom::new_with_props(app, probe);
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Loading projects"));
    send.send(Err(ServerFnError::new("Projects unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in ["Address-1", "INV-1", "Retry projects"] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    assert!(!html.contains("No projects available"), "{html}");
}

#[tokio::test]
async fn client_project_totals_failure_does_not_invent_zero_or_hide_project_links() {
    let id = Uuid::from_u128(1);
    let probe = Probe {
        initial_path: Some(format!("/clients/{id}")),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.project_spend_response.borrow_mut() = Some(receive);
    let mut dom = VirtualDom::new_with_props(app, probe);
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Loading project totals"));
    send.send(Err(ServerFnError::new("Totals unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in [
        "Client-project-1",
        "Retry totals",
        &format!("href=\"/projects/{id}\""),
        &format!("href=\"/invoices/{id}\""),
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    assert!(html.contains('—'), "{html}");
    assert!(!html.contains("USD 0.00"), "{html}");
}

#[tokio::test]
async fn client_detail_distinguishes_unset_rate_from_known_zero_project_totals() {
    let id = Uuid::from_u128(1);
    let probe = Probe {
        initial_path: Some(format!("/clients/{id}")),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.client_response.borrow_mut() = Some(receive);
    let (send_spend, receive_spend) = oneshot::channel();
    *probe.project_spend_response.borrow_mut() = Some(receive_spend);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let mut detail = dom.in_scope(probe.scope.borrow().unwrap(), || {
        server_fns::get_client_details(id.to_string())
            .now_or_never()
            .unwrap()
            .unwrap()
    });
    detail.billing.as_mut().unwrap().default_rate_cents = None;
    send.send(Ok(detail)).unwrap();
    send_spend
        .send(Ok(vec![server_fns::ProjectSpend {
            project_id: id,
            spent_minutes: 0,
            spent_cents: 0,
        }]))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("USD 0.00"), "{html}");
    assert!(html.contains("Not set"), "{html}");
    assert!(!html.contains("EUR 0.00 / h"), "{html}");
    assert!(!html.contains('—'), "{html}");
}

#[tokio::test]
async fn client_navigation_cancels_previous_pending_invoice_panel() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let probe = Probe {
        initial_path: Some(format!("/clients/{first}")),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.client_invoice_response.borrow_mut() = Some(receive);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Loading invoices"));
    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ClientDetail { id: second })
    });
    settle(&mut dom);
    assert!(
        send.is_closed(),
        "the previous client's request must be cancelled"
    );
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("INV-2"), "{html}");
    assert!(
        !html.contains("INV-1") && !html.contains("Loading invoices"),
        "{html}"
    );
}

#[tokio::test]
async fn failed_client_invoices_keep_identity_and_projects_without_claiming_empty_results() {
    let probe = Probe {
        initial_path: Some(format!("/clients/{}", Uuid::from_u128(1))),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.client_invoice_response.borrow_mut() = Some(receive);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Loading invoices"));
    send.send(Err(ServerFnError::new("Invoice list unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in ["Client-1", "Client-project-1", "Retry invoices"] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    assert!(!html.contains("No invoices"), "{html}");
}

#[tokio::test]
async fn project_creation_links_use_the_static_new_project_route() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert_eq!(
        dioxus::ssr::render(&dom)
            .matches("href=\"/projects/new\"")
            .count(),
        2
    );

    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::NewProject {})
    });
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("New project"));
    assert!(probe.assignment_requests.borrow().is_empty());
    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("No projects yet"));
}

#[tokio::test]
async fn empty_project_list_links_to_the_importer_route() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("No projects yet"), "rendered: {html}");
    assert_eq!(html.matches("href=\"/admin/importers\"").count(), 2);

    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::HarvestImport {})
    });
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Importers"));

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("No projects yet"));
}

#[tokio::test]
async fn invoice_detail_shows_saved_adjustments_and_payment_metadata() {
    let probe = Probe::default();
    let (send, receive) = oneshot::channel();
    *probe.response.borrow_mut() = Some(receive);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let mut response = dom.in_scope(probe.scope.borrow().unwrap(), || {
        server_fns::get_invoice(Uuid::from_u128(1).to_string())
            .now_or_never()
            .unwrap()
            .unwrap()
    });
    let inv = &mut response.invoice;
    inv.subtotal_cents = 10001;
    inv.discount_bps = 1250;
    inv.discount_cents = 1250;
    inv.tax1_bps = 2100;
    inv.tax1_cents = 1838;
    inv.tax2_name = Some("Local <tax>".into());
    inv.tax2_bps = Some(150);
    inv.tax2_cents = 131;
    inv.total_cents = 10720;
    inv.terms_days = 21;
    inv.po_number = "PO <123>".into();
    send.send(Ok(response)).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    for expected in [
        "Subtotal",
        "100.01",
        "Discount (12.50%)",
        "-12.50",
        "Tax (21.00%)",
        "18.38",
        "Local &#60;tax&#62; (1.50%)",
        "1.31",
        "107.20",
        "21 days",
        "PO &#60;123&#62;",
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
}

#[tokio::test]
async fn navigating_between_invoice_ids_loads_the_current_invoice() {
    let probe = Probe::default();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    assert_eq!(*probe.requests.borrow(), [first]);
    assert!(dioxus::ssr::render(&dom).contains("Invoice INV-1"));
    let html = dioxus::ssr::render(&dom);
    assert!(
        !html.contains("Mark Sent"),
        "SSR must wait for browser recovery before exposing mutations"
    );
    assert!(!html.contains("Edit invoice values"));

    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::InvoiceDetail { id: second })
    });
    assert_eq!(
        dom.in_scope(probe.scope.borrow().unwrap(), || dioxus::history::history()
            .current_route()),
        format!("/invoices/{second}")
    );
    settle(&mut dom);

    let html = dioxus::ssr::render(&dom);
    assert_eq!(
        *probe.requests.borrow(),
        [first, second],
        "rendered: {html}"
    );
    assert!(html.contains("Invoice INV-2"), "rendered: {html}");
    assert!(!html.contains("Invoice INV-1"), "rendered: {html}");

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert_eq!(*probe.requests.borrow(), [first, second, first]);
    assert!(dioxus::ssr::render(&dom).contains("Invoice INV-1"));
}

#[tokio::test]
async fn pending_or_failed_navigation_never_shows_the_previous_invoice() {
    let probe = Probe::default();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Invoice INV-1"));

    let (send, receive) = oneshot::channel();
    *probe.response.borrow_mut() = Some(receive);
    let navigator = probe.navigator.borrow().unwrap();
    let next = route::Route::InvoiceDetail {
        id: Uuid::from_u128(2),
    };
    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.push(next));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading"), "rendered: {html}");
    assert!(!html.contains("Invoice INV-1"), "rendered: {html}");
    assert!(!html.contains("Download PDF"), "rendered: {html}");
    assert!(!html.contains("Mark Sent"), "rendered: {html}");

    send.send(Err(ServerFnError::new("Invoice unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Invoice unavailable"), "rendered: {html}");
    assert!(!html.contains("Invoice INV-1"), "rendered: {html}");

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Invoice INV-1"), "rendered: {html}");
    assert!(!html.contains("Invoice unavailable"), "rendered: {html}");
}

#[tokio::test]
async fn navigating_between_project_ids_loads_current_assignments_and_tasks() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let probe = Probe {
        initial_path: Some(format!("/projects/{first}")),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("User-101"), "rendered: {html}");
    assert!(html.contains("Task-1"), "rendered: {html}");
    assert!(html.contains("Fee-1"), "rendered: {html}");
    assert!(
        html.contains("Over-invoiced: EUR -0.10"),
        "rendered: {html}"
    );
    assert_eq!(*probe.assignment_requests.borrow(), [first]);
    assert_eq!(*probe.task_requests.borrow(), [first]);

    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ProjectDetail { id: second })
    });
    assert_eq!(
        dom.in_scope(probe.scope.borrow().unwrap(), || dioxus::history::history()
            .current_route()),
        format!("/projects/{second}")
    );
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Project-2") && html.contains("CODE-2") && html.contains("Tag-2"),
        "rendered: {html}"
    );
    assert_eq!(
        *probe.assignment_requests.borrow(),
        [first, second],
        "rendered: {html}"
    );
    assert_eq!(
        *probe.task_requests.borrow(),
        [first, second],
        "rendered: {html}"
    );
    assert!(html.contains("User-102"), "rendered: {html}");
    assert!(html.contains("Task-2"), "rendered: {html}");
    assert!(html.contains("Fee-2"), "rendered: {html}");
    assert!(!html.contains("Fee-1"), "rendered: {html}");
    assert!(!html.contains("User-101"), "rendered: {html}");
    assert!(!html.contains("Task-1"), "rendered: {html}");
    assert!(!html.contains("CODE-1"), "rendered: {html}");
    assert!(!html.contains("Tag-1"), "rendered: {html}");
    assert_eq!(*probe.detail_requests.borrow(), [first, second]);

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert_eq!(*probe.assignment_requests.borrow(), [first, second, first]);
    assert_eq!(*probe.task_requests.borrow(), [first, second, first]);
}

#[tokio::test]
async fn pending_or_failed_project_details_never_show_previous_metadata() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let probe = Probe {
        initial_path: Some(format!("/projects/{first}")),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("CODE-1"));
    assert!(dioxus::ssr::render(&dom).contains("Task hourly rate (EUR)"));
    let (send, receive) = oneshot::channel();
    *probe.detail_response.borrow_mut() = Some(receive);
    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ProjectDetail { id: second })
    });
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading project details"), "{html}");
    assert!(!html.contains("project-task-rate"), "{html}");
    assert!(!html.contains("Enable task"), "{html}");
    assert!(
        !html.contains("CODE-1") && !html.contains("Tag-1"),
        "{html}"
    );
    send.send(Err(ServerFnError::new("Metadata unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Metadata unavailable") && html.contains("Retry details"),
        "{html}"
    );
    assert!(!html.contains("project-task-rate"), "{html}");
    assert!(!html.contains("Enable task"), "{html}");
    assert!(
        !html.contains("CODE-1") && !html.contains("Tag-1"),
        "{html}"
    );
}

#[tokio::test]
async fn pending_or_failed_project_assignments_never_show_previous_assignments() {
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    let probe = Probe {
        initial_path: Some(format!("/projects/{first}")),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("User-101"));

    let (send, receive) = oneshot::channel();
    *probe.assignment_response.borrow_mut() = Some(receive);
    let navigator = probe.navigator.borrow().unwrap();
    dom.in_scope(probe.scope.borrow().unwrap(), || {
        navigator.push(route::Route::ProjectDetail { id: second })
    });
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading"), "rendered: {html}");
    assert!(!html.contains("User-101"), "rendered: {html}");
    assert!(!html.contains("Remove"), "rendered: {html}");
    send.send(Err(ServerFnError::new("Assignments unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Assignments unavailable"), "rendered: {html}");
    assert!(!html.contains("User-101"), "rendered: {html}");

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("User-101"), "rendered: {html}");
    assert!(
        !html.contains("Assignments unavailable"),
        "rendered: {html}"
    );
}

// Dependency doubles for page helpers and endpoints. The component, data
// models, form/table components, router and resource implementation are real.
fn is_admin(me: &Resource<Result<user::CurrentUser, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(user)) if user.is_admin())
}

fn is_manager(me: &Resource<Result<user::CurrentUser, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(user)) if user.is_manager_or_above())
}

fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(value)) => render(value),
        Some(Err(error)) => rsx! { div { "{error}" } },
        None => rsx! { div { "Loading…" } },
    }
}

fn run_action<O: 'static, T: 'static>(
    _future: impl std::future::Future<Output = Result<O, ServerFnError>> + 'static,
    _resource: Resource<T>,
    _error: Signal<Option<String>>,
    _on_ok: impl FnOnce() + 'static,
) {
    panic!("navigation must not trigger a mutation");
}

mod server_fns {
    use super::*;
    use assignment::Assignment;
    use client::Client;
    use invoice::{Invoice, InvoiceWithLines};
    use project::Project;
    use task::Task;
    use user::CurrentUser;

    #[derive(Debug)]
    pub struct ProjectSpend {
        pub project_id: Uuid,
        pub spent_minutes: i64,
        pub spent_cents: i64,
    }

    fn user(id: u128, role: OrgRole) -> CurrentUser {
        CurrentUser {
            id: Uuid::from_u128(id),
            org_id: Uuid::nil(),
            email: format!("user-{id}@example.test"),
            name: format!("User-{id}"),
            org_role: role,
        }
    }

    pub async fn get_me() -> Result<CurrentUser, ServerFnError> {
        Ok(user(
            300,
            if consume_context::<Probe>().member {
                OrgRole::Member
            } else {
                OrgRole::Admin
            },
        ))
    }

    pub async fn get_client_details(id: String) -> ClientDetailsResponse {
        let id = Uuid::parse_str(&id).unwrap();
        let probe = consume_context::<Probe>();
        probe.client_requests.borrow_mut().push(id);
        let response = probe.client_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        Ok(client::ClientDetails {
            client: Client {
                id,
                org_id: Uuid::nil(),
                name: format!("Client-{}", id.as_u128()),
                currency: "EUR".into(),
                address: Some(format!("Address-{}", id.as_u128())),
                tax_id: None,
                active: true,
                created_at: chrono::DateTime::UNIX_EPOCH,
            },
            billing: (!probe.member).then_some(client::ClientBilling {
                default_rate_cents: Some(0),
            }),
        })
    }

    pub async fn list_client_invoices(id: String) -> ClientInvoicesResponse {
        let id = Uuid::parse_str(&id).unwrap();
        let probe = consume_context::<Probe>();
        assert!(!probe.member, "member must not request invoices");
        probe.client_invoice_requests.borrow_mut().push(id);
        let response = probe.client_invoice_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        let mut invoice = get_invoice(id.to_string()).await?.invoice;
        invoice.client_id = id;
        Ok(vec![invoice])
    }

    pub async fn list_client_summaries() -> Result<Vec<client::ClientSummary>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn create_client_profile(
        _profile: client::ClientProfile,
        _rate: String,
    ) -> ClientDetailsResponse {
        panic!("navigation must not create clients");
    }
    pub async fn update_client_profile(
        _id: String,
        _edit: client::ClientProfileEdit,
    ) -> ClientDetailsResponse {
        panic!("navigation must not edit clients");
    }
    pub async fn set_client_active(_id: String, _active: bool) -> Result<Client, ServerFnError> {
        panic!("navigation must not change client status");
    }

    pub async fn list_users(_archived: bool) -> Result<Vec<user::UserListItem>, ServerFnError> {
        Ok([101, 102]
            .into_iter()
            .map(|id| {
                let user = user(id, OrgRole::Member);
                user::UserListItem {
                    id: user.id,
                    name: user.name,
                    email: user.email,
                    org_role: user.org_role,
                    active: true,
                }
            })
            .collect())
    }

    pub async fn list_assignments(id: String) -> AssignmentResponse {
        let id = Uuid::parse_str(&id).unwrap();
        let probe = consume_context::<Probe>();
        probe.assignment_requests.borrow_mut().push(id);
        let response = probe.assignment_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.expect("controlled response was dropped");
        }
        Ok(vec![Assignment {
            id,
            project_id: id,
            user_id: Uuid::from_u128(100 + id.as_u128()),
            role: ProjectRole::Freelancer,
            rate_cents: None,
            created_at: chrono::DateTime::UNIX_EPOCH,
        }])
    }

    pub async fn list_project_tasks(id: String) -> Result<Vec<Task>, ServerFnError> {
        let id = Uuid::parse_str(&id).unwrap();
        consume_context::<Probe>()
            .task_requests
            .borrow_mut()
            .push(id);
        Ok(vec![Task {
            id,
            org_id: Uuid::nil(),
            name: format!("Task-{}", id.as_u128()),
            billable_default: true,
            default_rate_cents: None,
            active: true,
        }])
    }

    pub async fn list_tasks() -> Result<Vec<Task>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn list_projects(
        client: Option<String>,
        _archived: bool,
    ) -> Result<Vec<Project>, ServerFnError> {
        let Some(client) = client else {
            return Ok(Vec::new());
        };
        let response = consume_context::<Probe>()
            .client_projects_response
            .borrow_mut()
            .take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        let id = Uuid::parse_str(&client).unwrap();
        Ok(vec![Project {
            id,
            org_id: Uuid::nil(),
            client_id: id,
            code: None,
            name: format!("Client-project-{}", id.as_u128()),
            project_type: horae_core::types::ProjectType::TimeAndMaterials,
            currency: "USD".into(),
            rate_cents: None,
            starts_on: None,
            ends_on: None,
            budget_kind: horae_core::types::BudgetKind::None,
            budget_amount_cents: None,
            budget_minutes: None,
            active: true,
            created_at: chrono::DateTime::UNIX_EPOCH,
        }])
    }
    pub async fn list_project_tags() -> Result<Vec<project::ProjectTagLink>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn get_project_fee_balances(
        id: String,
        _from: String,
        _to: String,
    ) -> Result<Vec<project::ProjectFeeBalance>, ServerFnError> {
        let id = Uuid::parse_str(&id).unwrap();
        Ok(vec![project::ProjectFeeBalance {
            period_key: "single".into(),
            description: format!("Fee-{}", id.as_u128()),
            currency: "EUR".into(),
            balance: invoice::InvoiceFeeBalance {
                agreed_cents: 100,
                invoiced_cents: 110,
                remaining_cents: -10,
            },
        }])
    }
    pub async fn get_project_details(id: String) -> ProjectDetailsResponse {
        let id = Uuid::parse_str(&id).unwrap();
        let probe = consume_context::<Probe>();
        probe.detail_requests.borrow_mut().push(id);
        let response = probe.detail_response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        Ok(project::ProjectDetails {
            id,
            name: format!("Project-{}", id.as_u128()),
            code: Some(format!("CODE-{}", id.as_u128())),
            client_name: "Client".into(),
            currency: "EUR".into(),
            task_rate_currency: Some("EUR".into()),
            starts_on: None,
            ends_on: None,
            tags: vec![format!("Tag-{}", id.as_u128())],
            admin_notes: None,
        })
    }
    pub async fn list_project_spend() -> Result<Vec<ProjectSpend>, ServerFnError> {
        let response = consume_context::<Probe>()
            .project_spend_response
            .borrow_mut()
            .take();
        if let Some(response) = response {
            return response.await.unwrap();
        }
        Ok(Vec::new())
    }
    pub async fn list_project_budget_progress()
    -> Result<Vec<crate::models::ProjectBudgetProgress>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn set_project_active(_id: String, _active: bool) -> Result<(), ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn set_projects_active(
        _ids: Vec<String>,
        _active: bool,
    ) -> Result<Vec<Project>, ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn create_assignment(
        _project: String,
        _user: String,
        _role: String,
    ) -> Result<Assignment, ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn delete_assignment(_id: String) -> Result<(), ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn link_project_task(
        _project: String,
        _task: String,
        _rate: Option<crate::project::ProjectTaskRate>,
    ) -> Result<(), ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn create_task(
        _name: String,
        _billable: bool,
        _project: Option<String>,
    ) -> Result<Task, ServerFnError> {
        panic!("unexpected mutation");
    }

    pub async fn get_invoice(id: String) -> Result<InvoiceWithLines, ServerFnError> {
        let id = Uuid::parse_str(&id).unwrap();
        let probe = consume_context::<Probe>();
        probe.requests.borrow_mut().push(id);
        let response = probe.response.borrow_mut().take();
        if let Some(response) = response {
            return response.await.expect("controlled response was dropped");
        }
        Ok(InvoiceWithLines {
            invoice: Invoice {
                id,
                org_id: Uuid::nil(),
                client_id: Uuid::nil(),
                number: format!("INV-{}", id.as_u128()),
                status: InvoiceStatus::Draft,
                issued_on: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                due_on: chrono::NaiveDate::from_ymd_opt(2026, 9, 30).unwrap(),
                currency: "EUR".into(),
                total_cents: 0,
                terms_days: 29,
                po_number: String::new(),
                discount_bps: 0,
                tax1_bps: 0,
                tax2_name: None,
                tax2_bps: None,
                subtotal_cents: 0,
                discount_cents: 0,
                tax1_cents: 0,
                tax2_cents: 0,
                notes: None,
                created_at: chrono::DateTime::UNIX_EPOCH,
            },
            lines: Vec::new(),
        })
    }

    pub async fn list_clients(_archived: bool) -> Result<Vec<Client>, ServerFnError> {
        Ok(Vec::new())
    }

    pub async fn list_invoices(_status: Option<String>) -> Result<Vec<Invoice>, ServerFnError> {
        Ok(Vec::new())
    }

    pub async fn generate_invoice(
        _client: String,
        _from: String,
        _to: String,
        _projects: Option<Vec<String>>,
        _overrides: Option<crate::invoice::InvoiceDefaults>,
        _request: crate::invoice::InvoiceGenerationRequest,
    ) -> Result<InvoiceWithLines, ServerFnError> {
        panic!("navigation must not generate invoices");
    }

    pub async fn prepare_invoice(
        _client: String,
        _from: String,
        _to: String,
        _projects: Option<Vec<String>>,
        _overrides: Option<crate::invoice::InvoiceDefaults>,
        _fees: Option<Vec<crate::invoice::InvoiceFeeSelection>>,
    ) -> Result<crate::invoice::InvoicePreparation, ServerFnError> {
        panic!("navigation must not prepare invoices");
    }

    pub async fn get_invoice_editor(
        _id: String,
    ) -> Result<crate::invoice::InvoiceEditor, ServerFnError> {
        panic!("navigation must not load the invoice editor");
    }
    pub async fn review_invoice_edit(
        _id: String,
        _edit: crate::invoice::InvoiceDraftEdit,
    ) -> Result<crate::invoice::InvoiceEditReview, ServerFnError> {
        panic!("navigation must not review invoice edits");
    }
    pub async fn save_invoice_draft(
        _id: String,
        _request: crate::invoice::InvoiceDraftSave,
    ) -> Result<InvoiceWithLines, ServerFnError> {
        panic!("navigation must not save invoice edits");
    }

    pub async fn update_invoice_status(
        _id: String,
        _status: String,
    ) -> Result<Invoice, ServerFnError> {
        panic!("navigation must not change invoice status");
    }
}
