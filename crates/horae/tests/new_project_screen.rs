#![cfg(feature = "server")]

//! Render the production page with controlled, authorized server responses.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/avatar.rs"]
pub mod avatar;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/date_picker.rs"]
pub mod date_picker;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/select_field.rs"]
pub mod select_field;
mod components {
    pub use super::{avatar, controls, date_picker, form, icons, modal, select_field};
}
#[path = "../src/models/people.rs"]
pub mod people;
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
#[path = "../src/models/project_creation.rs"]
pub mod project_creation;
#[path = "../src/models/project_managers.rs"]
pub mod project_managers;
#[path = "../src/models/project_people.rs"]
pub mod project_people;
mod models {
    pub use super::{
        people, permission_editor, project_creation, project_managers, project_people,
    };
}
#[path = "../src/pages/new_project.rs"]
mod new_project;

use project_creation::{
    CreationClient, CreationOptions, CreationSearch, DraftSaved, ProjectDraft, ProjectForm,
};

#[test]
fn navigation_icons_keep_default_dimensions_and_allow_opt_in_sizing() {
    let default = dioxus::ssr::render_element(rsx! { icons::NavIcon { name: "projects" } });
    assert!(default.contains("width=\"15\""));
    assert!(default.contains("height=\"15\""));
    assert!(!default.contains("class="));

    let information =
        dioxus::ssr::render_element(rsx! { icons::NavIcon { name: "info", class: "size-em" } });
    assert!(information.contains("class=\"size-em\""));
    assert!(information.contains("<circle"));
    assert!(information.contains("<path"));

    let back = dioxus::ssr::render_element(
        rsx! { icons::NavIcon { name: "arrow-left", class: "size-4" } },
    );
    assert!(back.contains("class=\"size-4\""));
    assert!(back.contains("M13 8 H3 M7 4 L3 8 L7 12"));
}

#[derive(Clone)]
struct Probe {
    options: CreationOptions,
    existing: Option<project_creation::EditableProject>,
    other_project: Option<project_creation::EditableProject>,
    draft: Option<ProjectDraft>,
    selected_client: Option<CreationClient>,
    selected_tasks: Vec<project_creation::CreationTask>,
    selected_people: Vec<project_creation::CreationPerson>,
    writes: Rc<Cell<usize>>,
    edits: Rc<RefCell<Vec<project_creation::ProjectEditRequest>>>,
    scripts: Rc<RefCell<Vec<String>>>,
    reads: Rc<RefCell<Vec<&'static str>>>,
    switched_catalog: bool,
    switched_people: Rc<Cell<bool>>,
    switch_people_on_continuation: bool,
    edit_error_code: Rc<Cell<u16>>,
    edit_session_changed: Rc<Cell<bool>>,
    pending_people: Rc<RefCell<Option<tokio::sync::oneshot::Receiver<()>>>>,
    pending_save: Rc<RefCell<Option<tokio::sync::oneshot::Receiver<Uuid>>>>,
}

impl Default for Probe {
    fn default() -> Self {
        Self {
            existing: None,
            other_project: None,
            options: CreationOptions {
                organization_currency: "EUR".into(),
                can_edit_private_settings: false,
                email_available: false,
                clients: vec![],
                tasks: vec![],
                people: vec![],
                more_clients: false,
                more_tasks: false,
                more_people: false,
                previous_code: None,
                suggested_code: None,
            },
            draft: None,
            selected_client: None,
            selected_tasks: vec![],
            selected_people: vec![],
            writes: Rc::new(Cell::new(0)),
            edits: Rc::new(RefCell::new(vec![])),
            scripts: Rc::new(RefCell::new(vec![])),
            reads: Rc::new(RefCell::new(vec![])),
            switched_catalog: false,
            switched_people: Rc::new(Cell::new(false)),
            switch_people_on_continuation: false,
            edit_error_code: Rc::new(Cell::new(500)),
            edit_session_changed: Rc::new(Cell::new(false)),
            pending_people: Rc::new(RefCell::new(None)),
            pending_save: Rc::new(RefCell::new(None)),
        }
    }
}

fn app(probe: Probe) -> Element {
    let initial_path = probe.existing.as_ref().map_or_else(
        || "/projects/new".to_owned(),
        |project| format!("/projects/{}/edit", project.id),
    );
    use_context_provider(|| probe.clone());
    use_context_provider(|| Rc::new(probe.clone()) as Rc<dyn document::Document>);
    let history = Rc::new(MemoryHistory::with_initial_path(initial_path));
    rsx! {
        HistoryProvider { history: move |_| history.clone() as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

impl document::Document for Probe {
    fn eval(&self, script: String) -> document::Eval {
        self.scripts.borrow_mut().push(script);
        struct Unsupported;
        impl document::Evaluator for Unsupported {
            fn poll_join(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<serde_json::Value, document::EvalError>> {
                std::task::Poll::Ready(Err(document::EvalError::Unsupported))
            }
            fn poll_recv(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<serde_json::Value, document::EvalError>> {
                std::task::Poll::Ready(Err(document::EvalError::Unsupported))
            }
            fn send(&self, _: serde_json::Value) -> Result<(), document::EvalError> {
                Err(document::EvalError::Unsupported)
            }
        }
        document::Eval::new(dioxus::core::current_owner().insert(Box::new(Unsupported)))
    }
}

mod route {
    use super::*;
    use new_project::{EditProject, NewProject};

    #[derive(Clone, PartialEq, Routable)]
    #[rustfmt::skip]
    pub enum Route {
        #[layout(TestNavigation)]
        #[route("/projects")]
        ProjectList {},
        #[route("/projects/new")]
        NewProject {},
        #[route("/projects/:id/edit")]
        EditProject { id: Uuid },
        #[route("/projects/:id")]
        ProjectDetail { id: Uuid },
    }
    #[component]
    fn TestNavigation() -> Element {
        let probe = use_context::<Probe>();
        rsx! {
            if let Some(project) = probe.other_project {
                Link {
                    id: "test-other-project",
                    to: Route::EditProject { id: project.id },
                    "Open another project"
                }
            }
            Outlet::<Route> {}
        }
    }
    #[component]
    fn ProjectList() -> Element {
        rsx! { h1 { "Projects" } }
    }
    #[component]
    fn ProjectDetail(id: Uuid) -> Element {
        rsx! { h1 { "Project {id}" } }
    }
}

fn render(probe: Probe) -> String {
    let mut dom = VirtualDom::new_with_props(app, probe);
    dom.rebuild_in_place();
    for _ in 0..30 {
        if dom.wait_for_work().now_or_never().is_none() {
            return dioxus::ssr::render(&dom);
        }
        dom.render_immediate_to_vec();
    }
    panic!("new project render did not settle");
}

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

fn settle(dom: &mut VirtualDom) {
    for _ in 0..30 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("project editor did not settle");
}

fn click(dom: &mut VirtualDom, name: &str) {
    dioxus::html::set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let id = target(dom, dom.base_scope().root_node(), name).expect("missing click target");
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
            as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event("click", event, id);
    settle(dom);
}

fn input(dom: &mut VirtualDom, name: &str, value: &str) {
    dioxus::html::set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
    let id = target(dom, dom.base_scope().root_node(), name).expect("missing input target");
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::new(SerializedFormData::new(
            value.into(),
            vec![],
        )))) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event("input", event, id);
    settle(dom);
}

fn canonical_editor() -> Probe {
    use project_creation::{
        CreationPerson, CreationSelection, EditableProject, ProjectEditorAccess, ProjectFieldAccess,
    };
    let requester = permission_editor::PermissionRequester {
        org_id: Uuid::now_v7(),
        user_id: Uuid::now_v7(),
    };
    let id = Uuid::now_v7();
    let client_id = Uuid::now_v7();
    let mut probe = Probe::default();
    probe.options.people.push(CreationPerson {
        id: Uuid::now_v7(),
        name: "New teammate".into(),
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    probe.existing = Some(EditableProject {
        id,
        revision: 4,
        configured: true,
        active: true,
        form: ProjectForm {
            client_id: Some(client_id),
            name: "Existing project".into(),
            ..Default::default()
        },
        client: CreationClient {
            id: client_id,
            name: "Existing client".into(),
            currency: "EUR".into(),
            active: true,
            default_rate_cents: None,
        },
        selection: CreationSelection {
            tasks: vec![],
            people: vec![],
        },
        inactive_task_ids: vec![],
        inactive_user_ids: vec![],
        access: Some(ProjectEditorAccess {
            requester,
            managers: project_managers::ProjectManagers {
                requester,
                project_id: id,
                access_revision: 7,
                managers: vec![project_managers::ProjectManager {
                    id: Uuid::now_v7(),
                    name: "Retained outside manager".into(),
                    active: false,
                }],
            },
            billable: ProjectFieldAccess::Editable,
            costs: ProjectFieldAccess::Editable,
            private_notes: ProjectFieldAccess::Withheld,
        }),
    });
    probe
}

#[tokio::test]
async fn canonical_editor_searches_and_add_everyone_never_use_creation_catalogs() {
    let probe = canonical_editor();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    assert!(dioxus::ssr::render(&dom).contains("New teammate"));
    let reads = probe.reads.borrow();
    assert_eq!(reads.first(), Some(&"editor"));
    assert!(reads.contains(&"editor catalog"), "{reads:?}");
    assert!(reads.contains(&"project people"), "{reads:?}");
    assert!(!reads.contains(&"creation catalog"), "{reads:?}");
}

#[tokio::test]
async fn canonical_editor_rejects_a_catalog_from_another_requester() {
    let mut probe = canonical_editor();
    probe.switched_catalog = true;
    let html = render(probe);
    assert!(html.contains("Reload the project"), "{html}");
    assert!(!html.contains("id=\"np-name\""), "{html}");
}

#[tokio::test]
async fn canonical_add_everyone_discards_the_editor_after_a_requester_switch() {
    let probe = canonical_editor();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    input(&mut dom, "np-name", "Previous session draft");
    probe.switched_people.set(true);
    click(&mut dom, "np-add-everyone");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Reload the project"), "{html}");
    assert!(!html.contains("id=\"np-name\""), "{html}");
    assert!(!html.contains("Previous session draft"), "{html}");
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn canonical_save_access_loss_discards_pending_intent_until_explicit_reload() {
    for code in [401, 403, 409] {
        let probe = canonical_editor();
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        input(&mut dom, "np-name", "Previous session draft");
        click(&mut dom, "np-save");
        let pending = probe.edits.borrow()[0].clone();
        probe.edit_error_code.set(code);
        probe.edit_session_changed.set(code == 409);
        click(&mut dom, "np-retry");
        assert_eq!(probe.edits.borrow()[1], pending);
        let html = dioxus::ssr::render(&dom);
        assert!(!html.contains("id=\"np-name\""), "{code}: {html}");
        assert!(!html.contains("Previous session draft"), "{code}: {html}");
        assert!(!html.contains("id=\"np-retry\""), "{code}: {html}");
        assert_eq!(
            probe
                .reads
                .borrow()
                .iter()
                .filter(|read| **read == "editor")
                .count(),
            1
        );
        probe.edit_error_code.set(500);
        probe.edit_session_changed.set(false);
        click(&mut dom, "np-editor-reload");
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("id=\"np-name\""), "{html}");
        assert!(!html.contains("Previous session draft"), "{html}");
        input(&mut dom, "np-name", "Fresh session draft");
        click(&mut dom, "np-save");
        let requests = probe.edits.borrow();
        assert_eq!(requests.len(), 3);
        assert_ne!(requests[2].id, pending.id);
        assert_eq!(requests[2].form.name, "Fresh session draft");
    }
}

#[tokio::test]
async fn canonical_save_ordinary_rejections_keep_the_current_form() {
    for code in [400, 409, 500] {
        let probe = canonical_editor();
        probe.edit_error_code.set(code);
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        input(&mut dom, "np-name", "Recoverable project draft");
        click(&mut dom, "np-save");
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("id=\"np-name\""), "{code}: {html}");
        assert!(html.contains("Recoverable project draft"), "{code}: {html}");
        assert!(!html.contains("id=\"np-editor-reload\""), "{code}: {html}");
    }
}

#[tokio::test]
async fn canonical_session_invalidation_cancels_old_people_reads_before_reload() {
    let probe = canonical_editor();
    let (resume, waiting) = tokio::sync::oneshot::channel();
    *probe.pending_people.borrow_mut() = Some(waiting);
    probe.edit_error_code.set(403);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert!(
        probe.pending_people.borrow().is_none(),
        "the old read must actually be waiting"
    );
    assert!(!resume.is_closed());
    input(&mut dom, "np-name", "Old pending draft");
    click(&mut dom, "np-save");
    assert!(
        resume.is_closed(),
        "unmount must cancel the old editor's read"
    );
    probe.edit_error_code.set(500);
    click(&mut dom, "np-editor-reload");
    assert!(resume.send(()).is_err());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("id=\"np-name\""));
    assert!(!html.contains("Old pending draft"));
    assert!(!html.contains("id=\"np-editor-reload\""));
}

#[tokio::test]
async fn canonical_route_change_starts_a_fresh_editor_after_access_loss() {
    let mut probe = canonical_editor();
    let first = probe.existing.as_ref().unwrap();
    let mut second = first.clone();
    second.id = Uuid::now_v7();
    second.form.name = "Second project".into();
    second.access.as_mut().unwrap().managers.project_id = second.id;
    probe.other_project = Some(second);
    probe.edit_error_code.set(403);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    input(&mut dom, "np-name", "Discarded first project draft");
    click(&mut dom, "np-save");
    assert!(!dioxus::ssr::render(&dom).contains("id=\"np-name\""));
    click(&mut dom, "test-other-project");
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("id=\"np-name\""),
        "explicit navigation must open the new project: {html}"
    );
    assert!(html.contains("Second project"));
    assert!(!html.contains("Discarded first project draft"));
}

#[tokio::test]
async fn canonical_invalidation_cancels_delayed_save_success_before_fresh_reload() {
    let probe = canonical_editor();
    let project_id = probe.existing.as_ref().unwrap().id;
    let (people_response, people_waiting) = tokio::sync::oneshot::channel();
    let (save_response, save_waiting) = tokio::sync::oneshot::channel();
    *probe.pending_people.borrow_mut() = Some(people_waiting);
    *probe.pending_save.borrow_mut() = Some(save_waiting);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    input(&mut dom, "np-name", "Uncertain old save");
    click(&mut dom, "np-save");
    assert!(probe.pending_save.borrow().is_none());
    assert!(!save_response.is_closed());
    probe.switched_people.set(true);
    people_response.send(()).unwrap();
    settle(&mut dom);
    assert!(
        save_response.is_closed(),
        "discarding the editor must cancel its pending response handler"
    );
    probe.switched_people.set(false);
    click(&mut dom, "np-editor-reload");
    assert!(save_response.send(project_id).is_err());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("id=\"np-name\""),
        "old success must not navigate the fresh editor away"
    );
    assert!(!html.contains("Uncertain old save"));
}

#[tokio::test]
async fn canonical_route_change_cancels_the_previous_projects_pending_save() {
    let mut probe = canonical_editor();
    let first = probe.existing.as_ref().unwrap();
    let first_id = first.id;
    let mut second = first.clone();
    second.id = Uuid::now_v7();
    second.form.name = "Second project".into();
    second.access.as_mut().unwrap().managers.project_id = second.id;
    probe.other_project = Some(second);
    let (response, waiting) = tokio::sync::oneshot::channel();
    *probe.pending_save.borrow_mut() = Some(waiting);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    input(&mut dom, "np-name", "Pending first project draft");
    click(&mut dom, "np-save");
    assert!(probe.pending_save.borrow().is_none());
    assert!(!response.is_closed());
    click(&mut dom, "test-other-project");
    assert!(
        response.is_closed(),
        "navigation must cancel the old handler"
    );
    assert!(response.send(first_id).is_err());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("id=\"np-name\""), "{html}");
    assert!(html.contains("Second project"), "{html}");
    assert!(!html.contains("Pending first project draft"));
}

fn canonical_team(size: usize) -> Probe {
    let mut probe = canonical_editor();
    probe.options.people = (0..size)
        .map(|index| project_creation::CreationPerson {
            id: Uuid::now_v7(),
            name: format!("Person {index:03}"),
            billable_rate_cents: Some(987654),
            cost_rate_cents: Some(876543),
        })
        .collect();
    probe
}

#[tokio::test]
async fn protected_editor_preserves_unwritable_fields_on_save_and_retry() {
    use project_creation::{ProjectFieldAccess, ProtectedProjectField};
    for access in [ProjectFieldAccess::Withheld, ProjectFieldAccess::ReadOnly] {
        let mut probe = canonical_editor();
        let permissions = probe.existing.as_mut().unwrap().access.as_mut().unwrap();
        permissions.billable = access;
        permissions.costs = access;
        let person = probe.options.people[0].id;
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        click(&mut dom, "np-add-everyone");
        click(&mut dom, "np-save");
        let sent = probe.edits.borrow()[0].clone();
        for field in [
            ProtectedProjectField::ProjectRate,
            ProtectedProjectField::Fees,
            ProtectedProjectField::InvoiceDefaults,
            ProtectedProjectField::PrivateNotes,
            ProtectedProjectField::PersonRate(person),
            ProtectedProjectField::CostRate(person),
        ] {
            assert!(
                sent.unchanged.contains(&field),
                "Missing {field:?} for {access:?}"
            );
        }
        assert!(
            sent.unchanged.contains(&ProtectedProjectField::Budget),
            "untouched budgets stay preserved"
        );
        click(&mut dom, "np-retry");
        assert_eq!(probe.edits.borrow().as_slice(), &[sent.clone(), sent]);
    }
}

#[tokio::test]
async fn protected_editor_cost_read_is_independent_of_private_notes() {
    use project_creation::{CreationPerson, ProjectFieldAccess, ProjectMemberInput};
    let mut probe = canonical_editor();
    let person = probe.options.people[0].clone();
    let project = probe.existing.as_mut().unwrap();
    project.access.as_mut().unwrap().costs = ProjectFieldAccess::ReadOnly;
    project.form.team.push(ProjectMemberInput {
        user_id: person.id,
        manager: false,
        billable_rate: String::new(),
        cost_rate: "123.45".into(),
        budget: String::new(),
    });
    project.selection.people.push(CreationPerson {
        cost_rate_cents: Some(12345),
        ..person.clone()
    });
    let html = render(probe);
    let marker = format!("id=\"np-cost-rate-{}\"", person.id);
    let control = html
        .split('<')
        .find(|tag| tag.contains(&marker))
        .expect("authorized cost reader must see the field");
    assert!(control.contains("disabled") && control.contains("123.45"));
    assert!(!html.contains("id=\"np-notes\""));
}

#[tokio::test]
async fn protected_editor_withholds_finance_controls_but_keeps_hour_budgets() {
    use project_creation::ProjectFieldAccess;
    let mut probe = canonical_editor();
    let project = probe.existing.as_mut().unwrap();
    project.access.as_mut().unwrap().billable = ProjectFieldAccess::Withheld;
    project.form.rate_mode = horae_core::project::RateMode::Project;
    project.form.budget_mode = horae_core::project::BudgetMode::TotalHours;
    project.form.budget_value = "20".into();
    let html = render(probe);
    assert!(!html.contains("id=\"np-project-rate\""));
    assert!(!html.contains("id=\"np-tax\""));
    assert!(html.contains("id=\"np-budget-value\""));
    assert!(!html.contains("value=\"total_fees\""));
}

#[tokio::test]
async fn protected_editor_cost_zero_and_reset_stay_explicit_without_billable_or_notes_writes() {
    use project_creation::{ProjectFieldAccess, ProjectMemberInput, ProtectedProjectField};
    for value in ["0", ""] {
        let mut probe = canonical_editor();
        probe
            .existing
            .as_mut()
            .unwrap()
            .access
            .as_mut()
            .unwrap()
            .billable = ProjectFieldAccess::Withheld;
        let person = probe.options.people[0].id;
        let project = probe.existing.as_mut().unwrap();
        project.form.team.push(ProjectMemberInput {
            user_id: person,
            manager: false,
            billable_rate: String::new(),
            cost_rate: "45.00".into(),
            budget: String::new(),
        });
        project
            .selection
            .people
            .push(probe.options.people[0].clone());
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        input(&mut dom, &format!("np-cost-rate-{person}"), value);
        click(&mut dom, "np-save");
        let sent = probe.edits.borrow()[0].clone();
        assert_eq!(sent.form.team[0].cost_rate, value);
        assert!(
            !sent
                .unchanged
                .contains(&ProtectedProjectField::CostRate(person))
        );
        assert!(
            sent.unchanged
                .contains(&ProtectedProjectField::PersonRate(person))
        );
        assert!(
            sent.unchanged
                .contains(&ProtectedProjectField::PrivateNotes)
        );
        click(&mut dom, "np-retry");
        assert_eq!(probe.edits.borrow().as_slice(), &[sent.clone(), sent]);
    }
}

#[tokio::test]
async fn protected_editor_hour_budget_edit_does_not_require_a_billable_write() {
    use project_creation::{ProjectFieldAccess, ProtectedProjectField};
    let mut probe = canonical_editor();
    let project = probe.existing.as_mut().unwrap();
    project.access.as_mut().unwrap().billable = ProjectFieldAccess::Withheld;
    project.form.budget_mode = horae_core::project::BudgetMode::TotalHours;
    project.form.budget_value = "20".into();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    input(&mut dom, "np-budget-value", "25");
    click(&mut dom, "np-save");
    let edits = probe.edits.borrow();
    assert_eq!(edits[0].form.budget_value, "25");
    assert!(!edits[0].unchanged.contains(&ProtectedProjectField::Budget));
    assert!(
        edits[0]
            .unchanged
            .contains(&ProtectedProjectField::ProjectRate)
    );
}

#[tokio::test]
async fn protected_editor_untouched_editable_task_blank_is_not_a_reset() {
    use project_creation::{
        CreationTask, ProjectTaskInput, ProtectedProjectField, TaskAccess, TaskSource,
    };
    let mut probe = canonical_editor();
    let project = probe.existing.as_mut().unwrap();
    project.form.rate_mode = horae_core::project::RateMode::Task;
    let row = Uuid::now_v7();
    let task = Uuid::now_v7();
    project.form.tasks.push(ProjectTaskInput {
        id: row,
        source: TaskSource::Existing { task_id: task },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    project.selection.tasks.push(CreationTask {
        id: task,
        name: "Existing task".into(),
        billable: true,
        default_rate_cents: Some(12345),
        default_rate_currency: Some("EUR".into()),
    });
    for explicit in [false, true] {
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        input(&mut dom, "np-name", "Renamed project");
        if explicit {
            input(&mut dom, &format!("np-task-rate-{row}"), "");
        }
        click(&mut dom, "np-save");
        let edits = probe.edits.borrow();
        let sent = edits.last().unwrap();
        assert_eq!(sent.form.tasks[0].rate, "");
        assert_eq!(
            sent.unchanged
                .contains(&ProtectedProjectField::TaskRate(row)),
            !explicit
        );
        assert!(sent.unchanged.contains(&ProtectedProjectField::Budget));
    }
}

#[tokio::test]
async fn protected_editor_keeps_monetary_budget_and_task_rates_by_row_identity() {
    use project_creation::{
        ProjectFieldAccess, ProjectTaskInput, ProtectedProjectField, TaskAccess, TaskSource,
    };
    let mut probe = canonical_editor();
    let project = probe.existing.as_mut().unwrap();
    project.access.as_mut().unwrap().billable = ProjectFieldAccess::Withheld;
    project.form.budget_mode = horae_core::project::BudgetMode::FeesPerTask;
    let row = Uuid::now_v7();
    project.form.tasks.push(ProjectTaskInput {
        id: row,
        source: TaskSource::Existing {
            task_id: Uuid::now_v7(),
        },
        billable: true,
        rate: String::new(),
        budget: String::new(),
        access: TaskAccess::Everyone,
    });
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains(&format!("id=\"np-task-budget-{row}\"")));
    assert!(!html.contains(&format!("id=\"np-task-rate-{row}\"")));
    click(&mut dom, "np-save");
    let edits = probe.edits.borrow();
    assert!(edits[0].unchanged.contains(&ProtectedProjectField::Budget));
    assert!(
        edits[0]
            .unchanged
            .contains(&ProtectedProjectField::TaskRate(row))
    );
}

#[tokio::test]
async fn canonical_add_everyone_follows_identity_cursors_without_default_rates() {
    let probe = canonical_team(51);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("51 people"));
    assert!(html.contains("Person 050"));
    assert!(!html.contains("9876.54") && !html.contains("8765.43"));
    click(&mut dom, "np-save");
    let edits = probe.edits.borrow();
    let team = &edits[0].form.team;
    assert_eq!(team.len(), 51);
    assert!(
        team.iter()
            .all(|member| member.billable_rate.is_empty() && member.cost_rate.is_empty())
    );
    assert!(!probe.reads.borrow().contains(&"creation catalog"));
}

#[tokio::test]
async fn canonical_add_everyone_discards_the_editor_if_a_later_page_switches_session() {
    let mut probe = canonical_team(51);
    probe.switch_people_on_continuation = true;
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Reload the project"));
    assert!(!html.contains("id=\"np-name\""));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn canonical_add_everyone_rejects_over_capacity_without_partial_selection() {
    let probe = canonical_team(501);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("at most 500 people"));
    assert!(html.contains("No teammates selected yet"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn manager_only_edit_is_dirty_and_cancel_requires_confirmation() {
    let probe = canonical_editor();
    let person = probe.options.people[0].id;
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    click(&mut dom, &format!("np-person-manager-{person}"));
    click(&mut dom, &format!("np-person-remove-{person}"));
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("data-project-edit-state=\"dirty\""), "{html}");
    assert!(html.contains("Unsaved changes"), "{html}");
    probe.scripts.borrow_mut().clear();
    click(&mut dom, "np-cancel");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Edit project"), "{html}");
    assert!(
        probe
            .scripts
            .borrow()
            .iter()
            .any(|script| script.contains("np-discard-dialog")
                && script.contains("if (true && !dialog.open)")),
        "Cancel must request opening the native discard dialog"
    );
    assert_eq!(probe.writes.get(), 0);
}

fn control_is_disabled(html: &str, id: &str) -> bool {
    let marker = format!("id=\"{id}\"");
    let mut fieldsets = Vec::new();
    for tag in html.split('<') {
        let tag = tag.split('>').next().unwrap();
        if tag.starts_with("/fieldset") {
            fieldsets.pop();
        } else if tag.starts_with("fieldset ") {
            fieldsets.push(tag.contains(" disabled"));
        }
        if tag.contains(&marker) {
            return tag.contains(" disabled") || fieldsets.contains(&true);
        }
    }
    panic!("missing control {id}");
}

#[tokio::test]
async fn retained_manager_outside_the_team_can_be_removed_without_membership_changes() {
    let probe = canonical_editor();
    let project = probe.existing.as_ref().unwrap();
    let manager = &project.access.as_ref().unwrap().managers.managers[0];
    let checkbox = format!("np-person-manager-{}", manager.id);
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains(&manager.name),
        "retained responsibility must be visible"
    );
    assert!(!control_is_disabled(&html, &checkbox));
    click(&mut dom, &checkbox);
    assert!(dioxus::ssr::render(&dom).contains("Unsaved changes"));
    click(&mut dom, &checkbox);
    assert!(dioxus::ssr::render(&dom).contains("No unsaved changes"));
    click(&mut dom, &checkbox);
    click(&mut dom, "np-save");
    let request = probe.edits.borrow()[0].clone();
    assert_eq!(request.form, project.form);
    assert!(request.managers.as_ref().unwrap().manager_ids.is_empty());
    assert_eq!(
        request.managers.as_ref().unwrap().expected_access_revision,
        7
    );
    click(&mut dom, "np-retry");
    assert_eq!(probe.edits.borrow().as_slice(), &[request.clone(), request]);
}

#[tokio::test]
async fn archived_team_manager_can_be_unchecked_without_unlocking_tracking_settings() {
    use project_creation::{CreationPerson, ProjectMemberInput};
    let mut probe = canonical_editor();
    let project = probe.existing.as_mut().unwrap();
    let manager = project.access.as_ref().unwrap().managers.managers[0].clone();
    project.inactive_user_ids.push(manager.id);
    project.form.budget_mode = horae_core::project::BudgetMode::HoursPerPerson;
    project.form.team.push(ProjectMemberInput {
        user_id: manager.id,
        manager: true,
        billable_rate: String::new(),
        cost_rate: "12.34".into(),
        budget: "10".into(),
    });
    project.selection.people.push(CreationPerson {
        id: manager.id,
        name: manager.name,
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    let checkbox = format!("np-person-manager-{}", manager.id);
    assert!(!control_is_disabled(&html, &checkbox));
    for field in ["np-cost-rate", "np-person-budget", "np-person-remove"] {
        assert!(control_is_disabled(
            &html,
            &format!("{field}-{}", manager.id)
        ));
    }
    click(&mut dom, &checkbox);
    click(&mut dom, &checkbox);
    assert!(dioxus::ssr::render(&dom).contains("No unsaved changes"));
    click(&mut dom, &checkbox);
    click(&mut dom, "np-save");
    let request = probe.edits.borrow()[0].clone();
    let mut expected = probe.existing.as_ref().unwrap().form.clone();
    expected.team[0].manager = false;
    assert_eq!(request.form, expected);
    assert!(request.managers.as_ref().unwrap().manager_ids.is_empty());
}

#[tokio::test]
async fn archived_nonmanager_cannot_acquire_a_designation_from_the_team_checkbox() {
    use project_creation::ProjectMemberInput;
    for canonical in [true, false] {
        let mut probe = canonical_editor();
        let person = probe.options.people[0].clone();
        let project = probe.existing.as_mut().unwrap();
        project.inactive_user_ids.push(person.id);
        project.form.team.push(ProjectMemberInput {
            user_id: person.id,
            manager: false,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        });
        project.selection.people.push(person.clone());
        if canonical {
            project.access.as_mut().unwrap().managers.managers.clear();
        } else {
            project.access = None;
        }
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        dom.rebuild_in_place();
        settle(&mut dom);
        let checkbox = format!("np-person-manager-{}", person.id);
        assert!(control_is_disabled(&dioxus::ssr::render(&dom), &checkbox));
        // Even a synthetic event bypassing the native disabled control changes no intent.
        click(&mut dom, &checkbox);
        input(&mut dom, "np-name", "Ordinary project edit");
        click(&mut dom, "np-save");
        let request = probe.edits.borrow()[0].clone();
        assert!(!request.form.team[0].manager);
        assert!(
            request
                .managers
                .as_ref()
                .is_none_or(|selection| selection.manager_ids.is_empty())
        );
    }
}

#[tokio::test]
async fn removed_teammates_manager_control_remains_available_until_save() {
    let probe = canonical_editor();
    let person = probe.options.people[0].id;
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    let checkbox = format!("np-person-manager-{person}");
    click(&mut dom, &checkbox);
    click(&mut dom, &format!("np-person-remove-{person}"));
    click(&mut dom, &checkbox);
    assert!(dioxus::ssr::render(&dom).contains("No unsaved changes"));
    click(&mut dom, &checkbox);
    click(&mut dom, "np-save");
    let request = probe.edits.borrow()[0].clone();
    assert!(request.form.team.is_empty());
    assert!(
        request
            .managers
            .as_ref()
            .unwrap()
            .manager_ids
            .contains(&person)
    );
}

#[tokio::test]
async fn manager_checkbox_save_and_uncertain_retry_preserve_complete_original_intent() {
    let probe = canonical_editor();
    let person = probe.options.people[0].id;
    let original = probe.existing.as_ref().unwrap();
    let access = original.access.as_ref().unwrap();
    let outside = access.managers.managers[0].id;
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    click(&mut dom, "np-add-everyone");
    click(&mut dom, &format!("np-person-manager-{person}"));
    click(&mut dom, &format!("np-person-remove-{person}"));
    click(&mut dom, "np-save");
    let sent = probe.edits.borrow()[0].clone();
    assert_eq!(sent.form, original.form);
    assert_eq!(sent.expected_requester, Some(access.requester));
    let managers = sent.managers.as_ref().unwrap();
    assert_eq!(managers.expected_access_revision, 7);
    assert_eq!(managers.manager_ids.len(), 2);
    assert!(managers.manager_ids.contains(&person));
    assert!(managers.manager_ids.contains(&outside));
    click(&mut dom, "np-retry");
    assert_eq!(probe.edits.borrow().as_slice(), &[sent.clone(), sent]);
}

#[tokio::test]
async fn edit_prefills_legacy_currency_and_type_without_reading_or_saving_a_draft() {
    let client_id = Uuid::now_v7();
    let mut probe = Probe::default();
    probe.options.can_edit_private_settings = true;
    probe.existing = Some(project_creation::EditableProject {
        access: None,
        id: Uuid::now_v7(),
        revision: 9,
        configured: false,
        active: false,
        form: ProjectForm {
            client_id: Some(client_id),
            name: "Imported retainer".into(),
            code: "LEGACY-01".into(),
            currency: Some("JPY".into()),
            project_type: horae_core::types::ProjectType::Retainer,
            rate_mode: horae_core::project::RateMode::Legacy,
            project_rate: "0.00".into(),
            admin_notes: "Existing private notes".into(),
            report_visibility: project_creation::ReportVisibility::ProjectMembers,
            ..Default::default()
        },
        client: CreationClient {
            id: client_id,
            name: "Archived client".into(),
            currency: "JPY".into(),
            active: false,
            default_rate_cents: None,
        },
        selection: project_creation::CreationSelection {
            tasks: vec![],
            people: vec![],
        },
        inactive_task_ids: vec![],
        inactive_user_ids: vec![],
    });
    let writes = probe.writes.clone();
    let html = render(probe);
    for expected in [
        "Edit project",
        "Imported retainer",
        "LEGACY-01",
        "Archived client",
        "JPY",
        "Current type: Retainer",
        "Existing rate hierarchy",
        "Existing private notes",
        "Save changes",
        "No unsaved changes",
    ] {
        assert!(
            html.contains(expected),
            "Missing existing project value: {expected}"
        );
    }
    assert!(!html.contains("Discard draft"));
    assert!(!html.contains("Saving draft"));
    assert_eq!(writes.get(), 0);
}

#[tokio::test]
async fn billing_and_visibility_keep_native_radios_inside_designed_option_cards() {
    let html = render(Probe::default());
    assert_eq!(
        html.matches("data-project-type-icon=").count(),
        3,
        "Missing project type icons"
    );
    assert_eq!(
        html.matches("np-option ").count(),
        5,
        "Three rate options and two visibility options need card chrome"
    );
    for group in ["np-project-type", "np-rate-mode", "np-visibility"] {
        assert!(
            html.contains(&format!("name=\"{group}\"")),
            "Keep native keyboard radio groups: {group}"
        );
    }
}

#[test]
fn shared_input_without_options_keeps_its_original_class_and_external_label() {
    let mut dom = VirtualDom::new(|| rsx! { form::Input { id: "existing-field" } });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("class=\"form-input\""));
    assert!(!html.contains("aria-label="));
    assert!(!html.contains("aria-invalid="));
    assert!(!html.contains("aria-describedby="));
}

#[test]
fn an_input_can_link_its_validation_error_without_changing_its_value_or_class() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            form::Input { id: "tax", value: "101", error_id: "tax-error" }
        }
    });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    for attribute in [
        "class=\"form-input\"",
        "value=\"101\"",
        "aria-invalid=\"true\"",
        "aria-describedby=\"tax-error\"",
    ] {
        assert!(html.contains(attribute), "Missing {attribute}: {html}");
    }
}

#[test]
fn checkbox_error_identity_is_opt_in_without_changing_its_behavior() {
    for error in [false, true] {
        let mut dom = VirtualDom::new_with_props(
            |error: bool| {
                rsx! {
                    controls::Checkbox {
                        checked: true, label: "Budget email", disabled: true,
                        id: if error { "budget-email" } else { "" },
                        error_id: error.then(|| "budget-email-error".to_owned()),
                    }
                }
            },
            error,
        );
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        for attribute in [
            "class=\"choice checked\"",
            "aria-checked=\"true\"",
            "aria-label=\"Budget email\"",
            "disabled",
        ] {
            assert!(html.contains(attribute), "Missing {attribute}");
        }
        assert_eq!(html.contains("id=\"budget-email\""), error);
        assert_eq!(html.contains("aria-invalid=\"true\""), error);
        assert_eq!(
            html.contains("aria-describedby=\"budget-email-error\""),
            error
        );
    }
}

#[test]
fn textarea_and_selector_error_links_are_opt_in_and_preserve_controls() {
    for error in [false, true] {
        let mut dom = VirtualDom::new_with_props(
            |error: bool| {
                rsx! {
                    form::Textarea { id: "notes", value: "Keep this text", error_id: error.then(|| "notes-error".to_owned()) }
                    select_field::SelectField {
                        id: "currency", label: "Currency", selected: "EUR",
                        options: vec![("EUR".into(), "EUR".into())],
                        error_id: error.then(|| "currency-error".to_owned()),
                        onselect: |_| {},
                    }
                }
            },
            error,
        );
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("class=\"form-textarea\""));
        assert!(html.contains("Keep this text</textarea>"));
        assert!(
            html.contains("class=\"form-input flex items-center justify-between gap-2 text-left\"")
        );
        assert!(html.contains("popovertarget=\"currency-options\""));
        assert_eq!(
            html.matches("aria-invalid=\"true\"").count(),
            if error { 2 } else { 0 }
        );
        assert_eq!(html.contains("aria-describedby=\"notes-error\""), error);
        assert_eq!(html.contains("aria-describedby=\"currency-error\""), error);
    }
}

#[test]
fn compact_input_adds_utilities_and_accessible_name_without_losing_native_state() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            form::Input { id: "compact-field", class: "w-24 font-mono text-right", label: "Tax (%)", value: "1.50", disabled: true }
        }
    });
    dom.rebuild_in_place();
    let html = dioxus::ssr::render(&dom);
    for attribute in [
        "class=\"form-input w-24 font-mono text-right\"",
        "aria-label=\"Tax (%)\"",
        "value=\"1.50\"",
        "disabled",
    ] {
        assert!(
            html.contains(attribute),
            "Missing compact input attribute: {attribute}"
        );
    }
}

#[tokio::test]
async fn assignment_controls_reuse_searchable_picker_and_compact_task_entry() {
    let html = render(Probe::default());
    for attribute in [
        "id=\"np-add-person\"",
        "aria-label=\"Choose teammate\"",
        "aria-label=\"Search teammate\"",
        "aria-label=\"Find or create a task\"",
        "placeholder=\"Add a task and press Enter\"",
        "input-group flex-1 basis-assignment-picker min-w-0 h-10",
    ] {
        assert!(
            html.contains(attribute),
            "Missing assignment control: {attribute}"
        );
    }
    assert!(!html.contains("Search teammates</label>"));
}

#[tokio::test]
async fn empty_form_has_labelled_fields_no_demo_data_and_no_spurious_save() {
    let probe = Probe::default();
    let html = render(probe.clone());
    assert!(html.contains("New project"), "{html}");
    assert!(html.contains("No draft saved yet"));
    for id in [
        "np-client",
        "np-name",
        "np-code",
        "np-start",
        "np-end",
        "np-tags",
        "np-currency",
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
        assert!(
            html.contains(&format!("for=\"{id}\"")),
            "unlabelled {id}: {html}"
        );
    }
    assert!(!html.contains("np-notes"));
    assert!(!html.contains("Meridian"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn recovered_client_outside_the_catalog_page_keeps_its_currency_and_name() {
    let client = CreationClient {
        id: Uuid::now_v7(),
        name: "Outside the first page".into(),
        currency: "CHF".into(),
        active: false,
        default_rate_cents: None,
    };
    let mut probe = Probe {
        selected_client: Some(client.clone()),
        ..Default::default()
    };
    probe.draft = Some(ProjectDraft {
        id: Uuid::now_v7(),
        revision: 12,
        saved_at: chrono::Utc::now(),
        form: ProjectForm {
            client_id: Some(client.id),
            name: "Recovered project".into(),
            code: "12-".into(),
            ..Default::default()
        },
    });
    let html = render(probe.clone());
    assert!(html.contains("Outside the first page"), "{html}");
    assert!(html.contains("(archived)"));
    let archived_label = html.rfind("Outside the first page (archived)").unwrap();
    let archived_option = html[..archived_label].rsplit("<button").next().unwrap();
    assert!(
        archived_option.contains("role=\"option\"") && archived_option.contains("disabled=true")
    );
    assert!(html.contains("Client default (CHF)"));
    assert!(html.contains("value=\"Recovered project\""));
    assert!(html.contains("value=\"12-\""));
    assert!(html.contains("Draft saved at"));
    let save_button = html
        .split("Save project")
        .next()
        .unwrap()
        .rsplit("<button")
        .next()
        .unwrap();
    assert!(
        save_button.contains("disabled"),
        "An archived client cannot be used to create a project: {html}"
    );
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn administrative_notes_only_render_for_an_authorized_editor() {
    let mut probe = Probe::default();
    probe.options.can_edit_private_settings = true;
    assert!(render(probe).contains("id=\"np-notes\""));
}

fn with_form(form: ProjectForm) -> Probe {
    Probe {
        draft: Some(ProjectDraft {
            id: Uuid::now_v7(),
            revision: 1,
            saved_at: chrono::Utc::now(),
            form,
        }),
        ..Default::default()
    }
}

#[tokio::test]
async fn a_real_code_suggestion_does_not_replace_recovered_input() {
    let mut probe = with_form(ProjectForm {
        code: "12-".into(),
        ..Default::default()
    });
    probe.options.previous_code = Some("0009".into());
    probe.options.suggested_code = Some("0010".into());
    let html = render(probe.clone());
    assert!(html.contains("value=\"12-\""));
    assert!(html.contains("Last code: ") && html.contains("0009"));
    assert!(html.contains("Use 0010"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn client_and_currency_use_labelled_form_dropdowns() {
    let html = render(Probe::default());
    assert!(
        html.contains("popovertarget=\"np-client-options\""),
        "{html}"
    );
    assert!(html.contains("aria-label=\"Choose Client\""));
    assert!(html.contains("popovertarget=\"np-currency-options\""));
    assert!(html.contains("aria-label=\"Choose Currency\""));
}

#[tokio::test]
async fn recovered_tags_use_the_shared_inline_chip_field_without_saving() {
    let probe = with_form(ProjectForm {
        tags: vec!["Q3".into(), "platform".into()],
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(
        html.contains("chip-input chip-input-neutral np-tags"),
        "{html}"
    );
    assert!(html.contains("class=\"chip-input-field\""));
    assert!(html.contains("aria-label=\"Remove tag platform\""));
    assert!(html.contains("aria-describedby=\"np-tags-hint\""));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn planning_dates_restore_readable_shared_calendar_fields_without_saving() {
    let probe = with_form(ProjectForm {
        starts_on: "2026-09-01".into(),
        ends_on: "2026-10-15".into(),
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(html.contains("01 Sep 2026"), "{html}");
    assert!(html.contains("15 Oct 2026"));
    assert!(html.contains("popovertarget=\"np-start-calendar\""));
    assert!(html.contains("aria-label=\"Choose Start date\""));
    assert!(html.contains("aria-label=\"Clear Start date\""));
    assert!(!html.contains("type=\"date\""));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn time_and_materials_restores_project_rate_and_budget_without_enabling_unconfigured_email() {
    use horae_core::project::{BudgetMode, RateMode};
    let probe = with_form(ProjectForm {
        rate_mode: RateMode::Project,
        project_rate: "0".into(),
        budget_mode: BudgetMode::TotalHours,
        budget_value: "125:30".into(),
        ..Default::default()
    });
    let html = render(probe);
    let text = html.replace("&#38;", "&").replace("&amp;", "&");
    for label in ["Time & Materials", "Fixed Fee", "Non-Billable"] {
        assert!(text.contains(label), "missing {label}: {html}");
    }
    for id in ["np-project-rate", "np-budget-mode", "np-budget-value"] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
    }
    assert!(html.contains("value=\"125:30\""));
    assert!(html.contains("Email delivery is not configured"));
    assert!(html.contains("Invoice defaults"));
}

#[tokio::test]
async fn nonbillable_budget_picker_omits_fee_modes() {
    let html = render(with_form(ProjectForm {
        project_type: horae_core::types::ProjectType::NonBillable,
        ..Default::default()
    }));
    assert!(html.contains("aria-label=\"Choose Budget\""));
    assert!(html.contains("Hours per task"));
    assert!(!html.contains("Total project fees"));
    assert!(!html.contains("Fees per task"));
}

#[tokio::test]
async fn fixed_fee_restores_each_schedule_and_never_shows_hourly_rate_controls() {
    use horae_core::types::ProjectType;
    use project_creation::{FeeMode, MilestoneInput};
    for mode in [FeeMode::Single, FeeMode::Milestones, FeeMode::Monthly] {
        let probe = with_form(ProjectForm {
            project_type: ProjectType::FixedFee,
            fee_mode: mode,
            fee_amount: "800.25".into(),
            milestones: vec![MilestoneInput {
                id: Uuid::now_v7(),
                name: "Research delivery".into(),
                due_on: "2026-11-30".into(),
                amount: "400.25".into(),
            }],
            ..Default::default()
        });
        let html = render(probe);
        assert!(html.contains("Project fee"), "{html}");
        assert!(!html.contains("Billable rates"));
        match mode {
            FeeMode::Single => assert!(html.contains("value=\"800.25\"")),
            FeeMode::Milestones => {
                assert!(html.contains("Research delivery"));
                assert!(html.contains("value=\"2026-11-30\""));
                assert!(html.contains("Add milestone"));
            }
            FeeMode::Monthly => assert!(html.contains("id=\"np-monthly-day\"")),
        }
        assert!(html.contains("Invoice defaults"));
    }
}

#[tokio::test]
async fn nonbillable_hides_fee_rate_and_invoice_controls_without_erasing_the_draft() {
    let probe = with_form(ProjectForm {
        project_type: horae_core::types::ProjectType::NonBillable,
        fee_amount: "800.25".into(),
        project_rate: "95".into(),
        ..Default::default()
    });
    let html = render(probe.clone());
    assert!(html.contains("id=\"np-budget-mode\""), "{html}");
    for hidden in [
        "id=\"np-project-rate\"",
        "id=\"np-fee-amount\"",
        "Invoice defaults",
    ] {
        assert!(!html.contains(hidden));
    }
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn tasks_and_team_restore_scoped_settings_without_exposing_costs_to_managers() {
    use horae_core::project::{BudgetMode, RateMode};
    use project_creation::{
        CreationPerson, CreationTask, ProjectMemberInput, ProjectTaskInput, TaskAccess, TaskSource,
    };
    let user_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();
    let mut probe = with_form(ProjectForm {
        rate_mode: RateMode::Task,
        budget_mode: BudgetMode::HoursPerTask,
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing { task_id },
            billable: true,
            rate: "0".into(),
            budget: "12:30".into(),
            access: TaskAccess::Restricted {
                user_ids: vec![user_id],
            },
        }],
        team: vec![ProjectMemberInput {
            user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    });
    probe.options.tasks.push(CreationTask {
        id: task_id,
        name: "Release preparation".into(),
        billable: true,
        default_rate_cents: Some(8000),
        default_rate_currency: Some("EUR".into()),
    });
    probe.options.people.push(CreationPerson {
        id: user_id,
        name: "Project teammate".into(),
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    let html = render(probe.clone());
    for text in [
        "Tasks",
        "Team",
        "Release preparation",
        "Project teammate",
        "Project manager",
        "avatar-project",
        "Hourly rate for Release preparation",
        "Budget hours for Release preparation",
        "Restricted (1)",
        "Add everyone",
        "Report visibility",
    ] {
        assert!(html.contains(text), "missing {text}: {html}");
    }
    assert!(html.contains("value=\"12:30\""));
    assert!(!html.contains("np-cost-rate"));
    assert_eq!(probe.writes.get(), 0);
    probe.options.can_edit_private_settings = true;
    probe.options.people[0].cost_rate_cents = Some(6234);
    let html = render(probe);
    assert!(html.contains("np-cost-rate"));
    assert!(html.contains("placeholder=\"62.34\""));
    assert!(html.contains("Cost rate for Project teammate"));
}

#[tokio::test]
async fn task_rate_placeholders_use_source_currency_not_workspace_currency() {
    use horae_core::project::RateMode;
    use project_creation::{CreationTask, ProjectTaskInput, TaskAccess, TaskSource};
    let task_id = Uuid::now_v7();
    for (source_currency, amount, placeholder) in [
        (Some("USD"), Some(8000), "80.00"),
        (Some("EUR"), Some(8000), "Enter rate"),
        (None, Some(0), "Enter rate"),
        (None, None, "Inherit"),
    ] {
        let mut probe = with_form(ProjectForm {
            currency: Some("USD".into()),
            rate_mode: RateMode::Task,
            tasks: vec![ProjectTaskInput {
                id: Uuid::now_v7(),
                source: TaskSource::Existing { task_id },
                billable: true,
                rate: "0".into(),
                budget: String::new(),
                access: TaskAccess::Everyone,
            }],
            ..Default::default()
        });
        probe.options.organization_currency = "EUR".into();
        probe.options.tasks.push(CreationTask {
            id: task_id,
            name: "Imported task".into(),
            billable: true,
            default_rate_cents: amount,
            default_rate_currency: source_currency.map(String::from),
        });
        let html = render(probe.clone());
        assert!(
            html.contains(&format!("placeholder=\"{placeholder}\"")),
            "{html}"
        );
        assert!(html.contains("Hourly rate for Imported task (USD)"));
        assert!(html.contains("value=\"0\""));
        assert_eq!(probe.writes.get(), 0);
    }
}

#[tokio::test]
async fn recovered_tasks_and_people_outside_the_catalog_page_keep_their_identity() {
    use project_creation::{
        CreationPerson, CreationTask, ProjectMemberInput, ProjectTaskInput, TaskAccess, TaskSource,
    };
    let task_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    let mut probe = with_form(ProjectForm {
        tasks: vec![ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::Existing { task_id },
            billable: true,
            rate: String::new(),
            budget: String::new(),
            access: TaskAccess::Restricted {
                user_ids: vec![user_id],
            },
        }],
        team: vec![ProjectMemberInput {
            user_id,
            manager: true,
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        }],
        ..Default::default()
    });
    probe.selected_tasks.push(CreationTask {
        id: task_id,
        name: "Selected remote task".into(),
        billable: true,
        default_rate_cents: None,
        default_rate_currency: None,
    });
    probe.selected_people.push(CreationPerson {
        id: user_id,
        name: "Selected remote teammate".into(),
        billable_rate_cents: None,
        cost_rate_cents: None,
    });
    let html = render(probe.clone());
    assert!(html.contains("Selected remote task"), "{html}");
    assert!(html.contains("Selected remote teammate"), "{html}");
    assert!(!html.contains("Unavailable task"));
    assert!(!html.contains("Unavailable teammate"));
    assert_eq!(probe.writes.get(), 0);
}

#[tokio::test]
async fn invoice_defaults_restore_custom_terms_and_named_second_tax() {
    use project_creation::{InvoiceDefaultsInput, SecondTaxInput};
    let probe = with_form(ProjectForm {
        invoice_defaults: InvoiceDefaultsInput {
            terms_days: "37".into(),
            po_number: "PO-actual".into(),
            tax: "7.25".into(),
            second_tax: Some(SecondTaxInput {
                name: "Local tax".into(),
                percentage: "1.50".into(),
            }),
            discount: "2.75".into(),
        },
        ..Default::default()
    });
    let html = render(probe);
    for (id, value) in [
        ("np-terms-days", "37"),
        ("np-po-number", "PO-actual"),
        ("np-tax", "7.25"),
        ("np-second-tax-name", "Local tax"),
        ("np-second-tax", "1.50"),
        ("np-discount", "2.75"),
    ] {
        assert!(
            html.contains(&format!("id=\"{id}\"")),
            "missing {id}: {html}"
        );
        assert!(
            html.contains(&format!("value=\"{value}\"")),
            "missing {value}: {html}"
        );
    }
}

mod server_fns {
    use super::*;

    pub async fn project_creation_options(
        _: CreationSearch,
    ) -> Result<CreationOptions, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.reads.borrow_mut().push("creation catalog");
        Ok(probe.options)
    }
    pub async fn project_editor_catalog(
        context: project_creation::ProjectEditorContext,
        _: project_creation::ProjectEditorCatalogSearch,
    ) -> Result<project_creation::ProjectEditorCatalog, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.reads.borrow_mut().push("editor catalog");
        let mut returned = context;
        if probe.switched_catalog {
            returned.requester.user_id = Uuid::now_v7();
        }
        Ok(project_creation::ProjectEditorCatalog {
            context: returned,
            organization_currency: probe.options.organization_currency,
            email_available: probe.options.email_available,
            clients: probe.options.clients,
            tasks: probe.options.tasks,
            more_clients: probe.options.more_clients,
            more_tasks: probe.options.more_tasks,
        })
    }
    pub async fn project_people(
        context: project_people::ProjectPeopleContext,
        query: project_people::ProjectPeopleQuery,
    ) -> Result<project_people::ProjectPeopleResult, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.reads.borrow_mut().push("project people");
        let waiting = probe.pending_people.borrow_mut().take();
        if let Some(waiting) = waiting {
            waiting
                .await
                .expect("the pending read is either released or cancelled on unmount");
        }
        let project = probe
            .other_project
            .as_ref()
            .filter(|project| {
                context
                    == project_people::ProjectPeopleContext::Edit {
                        project_id: project.id,
                    }
            })
            .or(probe.existing.as_ref())
            .unwrap();
        assert_eq!(
            context,
            project_people::ProjectPeopleContext::Edit {
                project_id: project.id
            }
        );
        let mut requester = project.access.as_ref().unwrap().requester;
        let project_people::ProjectPeopleQuery::Search { query, after } = query else {
            panic!("unexpected selected identity lookup");
        };
        if probe.switched_people.get() || (probe.switch_people_on_continuation && after.is_some()) {
            requester.user_id = Uuid::now_v7();
        }
        let start = after.as_ref().map_or(0, |cursor| {
            probe
                .options
                .people
                .iter()
                .position(|person| person.id == cursor.id)
                .unwrap()
                + 1
        });
        let mut choices: Vec<_> = probe
            .options
            .people
            .into_iter()
            .skip(start)
            .filter(|person| person.name.to_lowercase().contains(&query.to_lowercase()))
            .map(|person| project_people::ProjectPersonChoice {
                id: person.id,
                name: person.name,
            })
            .take(51)
            .collect();
        let more = choices.len() > 50;
        choices.truncate(50);
        let next_after = choices
            .last()
            .filter(|_| more)
            .map(|person| people::PeopleCursor {
                name: person.name.to_lowercase(),
                id: person.id,
            });
        Ok(project_people::ProjectPeopleResult {
            requester,
            people: choices,
            next_after,
        })
    }
    pub async fn project_creation_client(_: Uuid) -> Result<Option<CreationClient>, ServerFnError> {
        Ok(use_context::<Probe>().selected_client)
    }
    pub async fn project_creation_selection(
        task_ids: Vec<Uuid>,
        user_ids: Vec<Uuid>,
    ) -> Result<project_creation::CreationSelection, ServerFnError> {
        let probe = use_context::<Probe>();
        Ok(project_creation::CreationSelection {
            tasks: probe
                .selected_tasks
                .into_iter()
                .filter(|task| task_ids.contains(&task.id))
                .collect(),
            people: probe
                .selected_people
                .into_iter()
                .filter(|person| user_ids.contains(&person.id))
                .collect(),
        })
    }
    pub async fn load_project_draft() -> Result<Option<ProjectDraft>, ServerFnError> {
        let probe = use_context::<Probe>();
        assert!(
            probe.existing.is_none(),
            "editing must not load a creation draft"
        );
        Ok(probe.draft)
    }
    pub async fn load_project_editor(
        id: Uuid,
    ) -> Result<project_creation::EditableProject, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.reads.borrow_mut().push("editor");
        let project = probe
            .other_project
            .filter(|project| project.id == id)
            .or(probe.existing)
            .expect("creation must not load an existing project");
        assert_eq!(project.id, id);
        Ok(project)
    }
    pub async fn save_project_editor(
        request: project_creation::ProjectEditRequest,
    ) -> Result<Uuid, ServerFnError> {
        let probe = use_context::<Probe>();
        assert!(
            probe.existing.is_some(),
            "creation must not update an existing project"
        );
        probe.writes.set(probe.writes.get() + 1);
        probe.edits.borrow_mut().push(request);
        let waiting = probe.pending_save.borrow_mut().take();
        if let Some(waiting) = waiting {
            return Ok(waiting
                .await
                .expect("the pending response is either released or cancelled on unmount"));
        }
        Err(ServerFnError::ServerError {
            code: probe.edit_error_code.get(),
            message: "Response unavailable".into(),
            details: probe.edit_session_changed.get().then(|| {
                serde_json::json!({
                    "reason": "project_editor_session_changed"
                })
            }),
        })
    }
    pub async fn save_project_draft(
        _: Uuid,
        _: i64,
        _: ProjectForm,
    ) -> Result<DraftSaved, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.writes.set(probe.writes.get() + 1);
        panic!("loading a saved draft must not write it");
    }
    pub async fn finalize_project_draft(
        _: Uuid,
        _: i64,
        _: ProjectForm,
    ) -> Result<Uuid, ServerFnError> {
        panic!("unexpected create");
    }
    pub async fn discard_project_draft(_: Uuid, _: i64) -> Result<(), ServerFnError> {
        panic!("unexpected discard");
    }
    pub async fn create_project_client(
        _: String,
        _: String,
        _: String,
    ) -> Result<CreationClient, ServerFnError> {
        panic!("unexpected client creation");
    }
}
