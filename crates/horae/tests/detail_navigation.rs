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

#[path = "../src/components/combobox.rs"]
pub mod combobox;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/menu.rs"]
pub mod menu;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/table.rs"]
pub mod table;
mod components {
    pub use super::{combobox, form, menu, modal, table};
}
#[path = "../src/models/assignment.rs"]
mod assignment;
#[path = "../src/models/client.rs"]
mod client;
#[path = "../src/models/invoice.rs"]
mod invoice;
#[path = "../src/pages/invoices.rs"]
mod invoices;
#[path = "../src/models/project.rs"]
mod project;
#[path = "../src/pages/projects.rs"]
mod projects;
#[path = "../src/models/task.rs"]
mod task;
#[path = "../src/models/user.rs"]
mod user;
mod models {
    pub use super::{client::Client, project::Project};
}

type InvoiceResponse = Result<invoice::InvoiceWithLines, ServerFnError>;
type AssignmentResponse = Result<Vec<assignment::Assignment>, ServerFnError>;

#[derive(Clone, Default)]
struct Probe {
    initial_path: Option<String>,
    requests: Rc<RefCell<Vec<Uuid>>>,
    assignment_requests: Rc<RefCell<Vec<Uuid>>>,
    task_requests: Rc<RefCell<Vec<Uuid>>>,
    navigator: Rc<RefCell<Option<Navigator>>>,
    scope: Rc<RefCell<Option<ScopeId>>>,
    response: Rc<RefCell<Option<oneshot::Receiver<InvoiceResponse>>>>,
    assignment_response: Rc<RefCell<Option<oneshot::Receiver<AssignmentResponse>>>>,
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
    use invoices::{InvoiceDetail, InvoiceList};
    use projects::{ProjectDetail, ProjectList};

    #[derive(Clone, PartialEq, Routable)]
    pub enum Route {
        #[layout(Layout)]
        #[route("/invoices")]
        InvoiceList {},
        #[route("/invoices/:id")]
        InvoiceDetail { id: Uuid },
        #[route("/projects")]
        ProjectList {},
        #[route("/projects/:id")]
        ProjectDetail { id: Uuid },
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
async fn navigating_between_invoice_ids_loads_the_current_invoice() {
    let probe = Probe::default();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let first = Uuid::from_u128(1);
    let second = Uuid::from_u128(2);
    assert_eq!(*probe.requests.borrow(), [first]);
    assert!(dioxus::ssr::render(&dom).contains("Invoice INV-1"));

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
        html.contains(&format!("Project detail for {second}")),
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
    assert!(!html.contains("User-101"), "rendered: {html}");
    assert!(!html.contains("Task-1"), "rendered: {html}");

    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert_eq!(*probe.assignment_requests.borrow(), [first, second, first]);
    assert_eq!(*probe.task_requests.borrow(), [first, second, first]);
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
fn is_admin(me: &Resource<Result<user::User, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(user)) if user.is_admin())
}

fn is_manager(me: &Resource<Result<user::User, ServerFnError>>) -> bool {
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
    use user::User;

    pub struct ProjectSpend {
        pub project_id: Uuid,
        pub spent_minutes: i64,
        pub spent_cents: i64,
    }

    fn user(id: u128, role: OrgRole) -> User {
        User {
            id: Uuid::from_u128(id),
            org_id: Uuid::nil(),
            email: format!("user-{id}@example.test"),
            name: format!("User-{id}"),
            oidc_subject: None,
            org_role: role,
            cost_rate_cents: None,
            billable_rate_cents: None,
            active: true,
            created_at: chrono::DateTime::UNIX_EPOCH,
        }
    }

    pub async fn get_me() -> Result<User, ServerFnError> {
        Ok(user(300, OrgRole::Admin))
    }

    pub async fn list_users(_archived: bool) -> Result<Vec<User>, ServerFnError> {
        Ok(vec![user(101, OrgRole::Member), user(102, OrgRole::Member)])
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
        _client: Option<String>,
        _archived: bool,
    ) -> Result<Vec<Project>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn list_project_spend() -> Result<Vec<ProjectSpend>, ServerFnError> {
        Ok(Vec::new())
    }
    pub async fn create_project(
        _client: String,
        _name: String,
        _kind: String,
        _currency: String,
        _budget: String,
        _value: String,
        _rate: String,
    ) -> Result<Project, ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn update_project(
        _id: String,
        _name: String,
        _kind: String,
        _currency: String,
        _budget: String,
        _value: String,
        _rate: String,
    ) -> Result<Project, ServerFnError> {
        panic!("unexpected mutation");
    }
    pub async fn set_project_active(_id: String, _active: bool) -> Result<(), ServerFnError> {
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
    pub async fn link_project_task(_project: String, _task: String) -> Result<(), ServerFnError> {
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
    ) -> Result<Invoice, ServerFnError> {
        panic!("navigation must not generate invoices");
    }

    pub async fn update_invoice_status(
        _id: String,
        _status: String,
    ) -> Result<Invoice, ServerFnError> {
        panic!("navigation must not change invoice status");
    }
}
