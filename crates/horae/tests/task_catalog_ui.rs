#![cfg(feature = "server")]

use dioxus::prelude::*;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use std::{any::Any, cell::RefCell, collections::VecDeque, rc::Rc};
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/modal.rs"]
pub mod modal;
mod components {
    pub use super::{controls, form, modal};
}
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
#[path = "../src/models/task.rs"]
pub mod task;
mod models {
    pub use super::{permission_editor, task};
    pub use task::Task;
}
#[path = "../src/pages/tasks.rs"]
mod page;

use permission_editor::PermissionRequester;
use task::*;
type Response = Result<TaskCatalogPage, ServerFnError>;
type Request = (
    TaskActivity,
    Option<TaskCursor>,
    Option<PermissionRequester>,
);
#[derive(Clone, Default)]
struct Probe {
    replies: Rc<RefCell<VecDeque<oneshot::Receiver<Response>>>>,
    requests: Rc<RefCell<Vec<Request>>>,
    updates: Rc<RefCell<Vec<(String, TaskRateEdit, PermissionRequester)>>>,
    activity: Rc<RefCell<Vec<(String, bool, PermissionRequester)>>>,
    mutation_error: Rc<RefCell<Option<ServerFnError>>>,
}
impl Probe {
    fn reply(&self) -> oneshot::Sender<Response> {
        let (send, receive) = oneshot::channel();
        self.replies.borrow_mut().push_back(receive);
        send
    }
}
fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { page::TaskCatalog {} }
}
struct Harness {
    dom: VirtualDom,
}
impl Harness {
    fn new(probe: &Probe) -> Self {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        let mut harness = Self { dom };
        harness.settle();
        harness
    }
    fn settle(&mut self) {
        for _ in 0..30 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            self.dom.render_immediate_to_vec();
        }
        panic!("task catalog did not settle");
    }
    fn click(&mut self, id: &str) {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        self.dom
            .runtime()
            .handle_event("click", event, self.target(id));
        self.settle();
    }
    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }

    fn form_event(&mut self, id: &str, kind: &str, value: &str) {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
                value.into(),
                vec![],
            )))) as Rc<dyn Any>,
            true,
        );
        self.dom
            .runtime()
            .handle_event(kind, event, self.target(id));
        self.settle();
    }

    fn target(&self, name: &str) -> dioxus::core::ElementId {
        target(&self.dom, self.dom.base_scope().root_node(), name)
            .unwrap_or_else(|| panic!("missing {name}: {}", self.html()))
    }
}

// Resolve mounted controls, including static IDs stored only in templates.
fn target(dom: &VirtualDom, vnode: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::{AttributeValue, DynamicNode, TemplateAttribute, TemplateNode};
    for (index, path) in vnode.template.attr_paths.iter().enumerate() {
        let mut node = &vnode.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = &children[usize::from(*child)];
        }
        let static_id = matches!(node, TemplateNode::Element { attrs, .. }
            if attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name: "id", value, .. } if *value == name)));
        let dynamic_id = vnode.dynamic_attrs[index].iter().any(|attr| {
            attr.name == "id" && matches!(&attr.value, AttributeValue::Text(value) if value == name)
        });
        if static_id || dynamic_id {
            return vnode.mounted_dynamic_attribute(index, dom);
        }
    }
    for (index, node) in vnode.dynamic_nodes.iter().enumerate() {
        let found = match node {
            DynamicNode::Component(component) => component
                .mounted_scope(index, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .and_then(|node| target(dom, node, name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|node| target(dom, node, name)),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}
fn catalog(edit: bool, rates: bool) -> TaskCatalogPage {
    TaskCatalogPage {
        requester: PermissionRequester {
            org_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        },
        can_edit: edit,
        can_read_rates: rates,
        can_edit_rates: edit && rates,
        rate_currency: (edit && rates).then(|| "EUR".into()),
        tasks: vec![TaskCatalogEntry {
            id: Uuid::now_v7(),
            name: "A private task".into(),
            billable_default: true,
            active: true,
            default_rate_cents: rates.then_some(8000),
            default_rate_currency: rates.then(|| "USD".into()),
        }],
        next_after: None,
    }
}

#[tokio::test]
async fn pending_denied_and_read_only_catalogs_do_not_offer_mutations_or_hidden_rates() {
    let probe = Probe::default();
    let reply = probe.reply();
    let mut ui = Harness::new(&probe);
    assert!(ui.html().contains("Loading tasks"));
    assert!(!ui.html().contains("Task catalog</"));
    reply.send(Ok(catalog(false, false))).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("A private task") && html.contains("read-only"));
    assert!(!html.contains("tasks-edit-") && !html.contains("Default hourly rate"));
    let reply = probe.reply();
    ui.click("tasks-refresh");
    assert!(!ui.html().contains("A private task"));
    reply
        .send(Err(ServerFnError::ServerError {
            code: 403,
            message: "private SQL detail".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Task access is unavailable"));
    assert!(!html.contains("private SQL detail"));
}

#[tokio::test]
async fn refresh_binds_requester_discards_editor_and_rejects_account_switch() {
    let probe = Probe::default();
    let reply = probe.reply();
    let mut ui = Harness::new(&probe);
    let page = catalog(true, true);
    let requester = page.requester;
    let id = page.tasks[0].id;
    reply.send(Ok(page)).unwrap();
    ui.settle();
    ui.click(&format!("tasks-edit-{id}"));
    assert!(ui.html().contains("task-rate-action") && ui.html().contains("USD 80.00"));
    let reply = probe.reply();
    ui.click("tasks-refresh");
    assert!(!ui.html().contains("task-name") && !ui.html().contains("A private task"));
    assert_eq!(probe.requests.borrow().last().unwrap().2, Some(requester));
    reply.send(Ok(catalog(true, true))).unwrap();
    ui.settle();
    assert!(ui.html().contains("Task access is unavailable"));
    assert!(!ui.html().contains("A private task"));
}

#[tokio::test]
async fn archive_requires_confirmation_uses_loaded_identity_and_invalidates_denied_editor() {
    let probe = Probe::default();
    let reply = probe.reply();
    let mut ui = Harness::new(&probe);
    let page = catalog(true, false);
    let requester = page.requester;
    let id = page.tasks[0].id;
    reply.send(Ok(page)).unwrap();
    ui.settle();
    ui.click(&format!("tasks-edit-{id}"));
    assert!(!ui.html().contains("task-rate-action"));
    ui.click("task-activity");
    assert!(probe.activity.borrow().is_empty());
    assert!(ui.html().contains("every project"));
    *probe.mutation_error.borrow_mut() = Some(ServerFnError::ServerError {
        code: 403,
        message: "Access revoked".into(),
        details: None,
    });
    let reload = probe.reply();
    ui.click("task-activity-confirm");
    assert_eq!(
        &*probe.activity.borrow(),
        &[(id.to_string(), false, requester)]
    );
    assert!(!ui.html().contains("task-name"));
    reload.send(Err(ServerFnError::new("unavailable"))).unwrap();
    ui.settle();
    assert!(!ui.html().contains("Task archived."));
}

#[tokio::test]
async fn name_edit_preserves_rates_and_keeps_input_on_validation_failure() {
    let probe = Probe::default();
    let reply = probe.reply();
    let mut ui = Harness::new(&probe);
    let page = catalog(true, false);
    let requester = page.requester;
    let id = page.tasks[0].id;
    reply.send(Ok(page)).unwrap();
    ui.settle();
    ui.click(&format!("tasks-edit-{id}"));
    ui.form_event("task-name", "input", "Changed name");
    *probe.mutation_error.borrow_mut() = Some(ServerFnError::ServerError {
        code: 409,
        message: "Task name already exists".into(),
        details: None,
    });
    ui.form_event("task-edit-form", "submit", "");
    assert_eq!(
        &*probe.updates.borrow(),
        &[(id.to_string(), TaskRateEdit::Preserve {}, requester)]
    );
    assert!(ui.html().contains("Changed name") && ui.html().contains("Task name already exists"));
    assert!(!ui.html().contains("Task updated."));
}

#[tokio::test]
async fn financial_editor_sends_explicit_zero_in_current_currency_and_explicit_clear() {
    let probe = Probe::default();
    let reply = probe.reply();
    let mut ui = Harness::new(&probe);
    let page = catalog(true, true);
    let requester = page.requester;
    let id = page.tasks[0].id;
    reply.send(Ok(page)).unwrap();
    ui.settle();
    ui.click(&format!("tasks-edit-{id}"));
    ui.form_event("task-rate-action", "change", "set");
    ui.form_event("task-rate", "input", "0");
    ui.form_event("task-edit-form", "submit", "");
    assert_eq!(
        probe.updates.borrow()[0],
        (
            id.to_string(),
            TaskRateEdit::Set {
                amount_cents: 0,
                currency: "EUR".into()
            },
            requester
        )
    );
    ui.form_event("task-rate-action", "change", "clear");
    ui.form_event("task-edit-form", "submit", "");
    assert_eq!(
        probe.updates.borrow()[1],
        (id.to_string(), TaskRateEdit::Clear {}, requester)
    );
}

mod server_fns {
    use super::*;
    pub async fn load_task_catalog(
        activity: TaskActivity,
        cursor: Option<TaskCursor>,
        requester: Option<PermissionRequester>,
    ) -> Response {
        let probe = consume_context::<Probe>();
        probe
            .requests
            .borrow_mut()
            .push((activity, cursor, requester));
        let receive = probe
            .replies
            .borrow_mut()
            .pop_front()
            .expect("queued catalog response");
        receive.await.unwrap()
    }
    pub async fn update_task(
        id: String,
        _name: String,
        _billable: bool,
        rate: TaskRateEdit,
        requester: PermissionRequester,
    ) -> Result<Task, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.updates.borrow_mut().push((id, rate, requester));
        Err(probe
            .mutation_error
            .borrow_mut()
            .take()
            .unwrap_or_else(|| ServerFnError::new("Unexpected update")))
    }
    pub async fn set_task_active(
        id: String,
        active: bool,
        requester: PermissionRequester,
    ) -> Result<Task, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.activity.borrow_mut().push((id, active, requester));
        Err(probe
            .mutation_error
            .borrow_mut()
            .take()
            .expect("configured mutation failure"))
    }
}
