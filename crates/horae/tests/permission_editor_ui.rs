#![cfg(feature = "server")]

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use dioxus::core::{AttributeValue, ElementId, Mutation};
use dioxus::prelude::*;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/permission_description.rs"]
pub mod permission_description;
mod components {
    pub use super::{controls, form, modal, permission_description};
}
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor_model;
mod models {
    pub use super::permission_editor_model as permission_editor;
}
#[path = "../src/pages/admin/permission_editor.rs"]
mod permission_editor;
use permission_editor_model::*;

type PreviewReply = oneshot::Receiver<Result<ProfilePreview, ServerFnError>>;
type SaveReply = oneshot::Receiver<Result<ProfileOutcome, ServerFnError>>;
type TemplateReply = oneshot::Receiver<Result<TemplateOutcome, ServerFnError>>;
type DeletionReply = oneshot::Receiver<Result<TemplateDeletionPreview, ServerFnError>>;
type LoadReply = oneshot::Receiver<Result<PermissionEditor, ServerFnError>>;
type IdentityReply = oneshot::Receiver<Result<PermissionRequester, ServerFnError>>;

#[derive(Clone)]
struct Probe {
    open_person: bool,
    editor: PermissionEditor,
    previews: Rc<RefCell<Vec<ProfileDraft>>>,
    preview_replies: Rc<RefCell<VecDeque<PreviewReply>>>,
    saves: Rc<RefCell<Vec<ProfileCommand>>>,
    save_replies: Rc<RefCell<VecDeque<SaveReply>>>,
    saved: Rc<RefCell<Vec<bool>>>,
    templates: Rc<RefCell<Vec<TemplateCommand>>>,
    template_replies: Rc<RefCell<VecDeque<TemplateReply>>>,
    deletions: Rc<RefCell<Vec<Uuid>>>,
    deletion_replies: Rc<RefCell<VecDeque<DeletionReply>>>,
    loads: Rc<RefCell<usize>>,
    load_replies: Rc<RefCell<VecDeque<LoadReply>>>,
    confirmations: Rc<RefCell<VecDeque<Result<bool, document::EvalError>>>>,
    storage: Rc<RefCell<HashMap<String, String>>>,
    storage_failure: Rc<RefCell<Option<String>>>,
    identity_replies: Rc<RefCell<VecDeque<IdentityReply>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            open_person: true,
            editor: PermissionEditor {
                requester: PermissionRequester {
                    org_id: Uuid::now_v7(),
                    user_id: Uuid::now_v7(),
                },
                user_id: Uuid::now_v7(),
                name: "Example person".into(),
                active: true,
                access_revision: 7,
                permissions: PermissionSnapshot {
                    grants: BuiltInProfile::ProjectManager.selection().iter().collect(),
                    is_administrator: false,
                    source: ProfileSource::BuiltIn(BuiltInProfile::ProjectManager),
                    revision: 3,
                },
                templates: vec![],
            },
            previews: Default::default(),
            preview_replies: Default::default(),
            saves: Default::default(),
            save_replies: Default::default(),
            saved: Default::default(),
            templates: Default::default(),
            template_replies: Default::default(),
            deletions: Default::default(),
            deletion_replies: Default::default(),
            loads: Default::default(),
            load_replies: Default::default(),
            confirmations: Default::default(),
            storage: Default::default(),
            storage_failure: Default::default(),
            identity_replies: Default::default(),
        }
    }

    fn preview_reply(&self) -> oneshot::Sender<Result<ProfilePreview, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.preview_replies.borrow_mut().push_back(receive);
        send
    }

    fn load_reply(&self) -> oneshot::Sender<Result<PermissionEditor, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.load_replies.borrow_mut().push_back(receive);
        send
    }

    fn save_reply(&self) -> oneshot::Sender<Result<ProfileOutcome, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.save_replies.borrow_mut().push_back(receive);
        send
    }

    fn effects(&self) -> ProfilePreview {
        ProfilePreview {
            user_id: self.editor.user_id,
            access_revision: self.editor.access_revision,
            before: self.editor.permissions.clone(),
            after: self.editor.permissions.clone(),
            changed: false,
            remove_projects: vec![],
            remove_people: vec![],
        }
    }

    fn template_reply(&self) -> oneshot::Sender<Result<TemplateOutcome, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.template_replies.borrow_mut().push_back(receive);
        send
    }

    fn deletion_reply(&self) -> oneshot::Sender<Result<TemplateDeletionPreview, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.deletion_replies.borrow_mut().push_back(receive);
        send
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe.clone());
    use_context_provider(|| Rc::new(probe.clone()) as Rc<dyn document::Document>);
    let person = use_signal(|| probe.open_person.then_some(probe.editor.user_id));
    rsx! { permission_editor::PermissionEditorDialog {
        person, on_saved: move |changed| probe.saved.borrow_mut().push(changed),
    } }
}

impl document::Document for Probe {
    fn eval(&self, script: String) -> document::Eval {
        if script == include_str!("../assets/js/permission-recovery-storage.js") {
            return document::Eval::new(dioxus::core::current_owner().insert(Box::new(
                StorageReply {
                    probe: self.clone(),
                    reply: RefCell::new(None),
                },
            )));
        }
        struct Reply(Option<Result<bool, document::EvalError>>);
        impl document::Evaluator for Reply {
            fn poll_join(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<serde_json::Value, document::EvalError>> {
                std::task::Poll::Ready(self.0.take().unwrap().map(serde_json::Value::Bool))
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
        let reply = if script.contains("window.confirm(") {
            assert_eq!(
                script,
                "return window.confirm('Discard unsaved permission changes?');"
            );
            self.confirmations
                .borrow_mut()
                .pop_front()
                .expect("unexpected discard confirmation")
        } else {
            Err(document::EvalError::Unsupported)
        };
        document::Eval::new(dioxus::core::current_owner().insert(Box::new(Reply(Some(reply)))))
    }
}

// The JS protocol is tested separately; this provider controls bridge failures
// and verifies production handlers do not send before storage acknowledgement.
struct StorageReply {
    probe: Probe,
    reply: RefCell<Option<Result<serde_json::Value, document::EvalError>>>,
}
impl document::Evaluator for StorageReply {
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
        std::task::Poll::Ready(
            self.reply
                .borrow_mut()
                .take()
                .expect("missing storage operation"),
        )
    }
    fn send(&self, message: serde_json::Value) -> Result<(), document::EvalError> {
        let (key, operation, value): (String, String, Option<String>) =
            serde_json::from_value(message).unwrap();
        let mut storage = self.probe.storage.borrow_mut();
        let failure = self.probe.storage_failure.borrow().clone();
        let result: Result<Option<String>, String> = if failure.as_deref() == Some(&operation) {
            Err("Browser session storage is unavailable. Keep this tab and retry.".into())
        } else if operation == "load" {
            Ok(storage.get(&key).cloned())
        } else if storage
            .get(&key)
            .is_some_and(|current| Some(current) != value.as_ref())
        {
            Err("Another permission request is unresolved. Reload to recover it.".into())
        } else {
            if operation == "store" {
                storage.insert(key, value.unwrap());
            } else {
                assert_eq!(operation, "clear");
                storage.remove(&key);
            }
            Ok(None)
        };
        let reply = if failure.as_deref() == Some("ack-store") && operation == "store" {
            Err(document::EvalError::Unsupported)
        } else {
            Ok(serde_json::to_value(result).unwrap())
        };
        *self.reply.borrow_mut() = Some(reply);
        Ok(())
    }
}

struct Ui {
    dom: VirtualDom,
    targets: HashMap<String, ElementId>,
}

impl Ui {
    fn new(probe: Probe) -> Self {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut ui = Self {
            dom: VirtualDom::new_with_props(app, probe),
            targets: HashMap::new(),
        };
        let mutations = ui.dom.rebuild_to_vec().edits;
        ui.record(mutations);
        ui.settle();
        ui
    }

    fn record(&mut self, mutations: Vec<Mutation>) {
        for mutation in mutations {
            if let Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(name),
                id,
                ..
            } = mutation
            {
                self.targets.insert(name, id);
            }
        }
    }

    fn settle(&mut self) {
        for _ in 0..30 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            let mutations = self.dom.render_immediate_to_vec().edits;
            self.record(mutations);
        }
        panic!("permission editor did not settle");
    }

    fn click(&mut self, name: &str) {
        self.dispatch(name, "click", Box::<SerializedMouseData>::default());
    }

    fn dispatch(&mut self, name: &str, kind: &str, data: Box<dyn Any>) {
        let id = *self
            .targets
            .get(name)
            .unwrap_or_else(|| panic!("missing {name}: {}", self.html()));
        let event = Event::new(Rc::new(PlatformEventData::new(data)) as Rc<dyn Any>, true);
        self.dom.runtime().handle_event(kind, event, id);
        self.settle();
    }

    fn dismiss(&mut self, path: &str) {
        match path {
            "escape" => self.dispatch(
                "person-permissions-dialog",
                "cancel",
                Box::new(SerializedCancelData {}),
            ),
            "backdrop" => {
                let mut data = serde_json::to_value(SerializedMouseData::default()).unwrap();
                data.as_object_mut().unwrap().extend(
                    serde_json::json!({
                        "pointer_id": 1, "width": 1, "height": 1, "pressure": 0,
                        "tangential_pressure": 0, "tilt_x": 0, "tilt_y": 0,
                        "twist": 0, "pointer_type": "mouse", "is_primary": true,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                );
                let data: SerializedPointerData = serde_json::from_value(data).unwrap();
                self.dispatch("person-permissions-dialog", "pointerdown", Box::new(data));
                self.click("person-permissions-dialog");
            }
            _ => self.click(path),
        }
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }

    fn navigation_state(&self, state: &str) {
        let html = self.html();
        assert!(html.contains("data-editor-kind=\"permissions\""), "{html}");
        assert!(
            html.contains(&format!("data-editor-state=\"{state}\"")),
            "{html}"
        );
    }

    fn select_profile(&mut self, value: &str) {
        self.form_event("person-permissions-profile", "change", value);
    }

    fn form_event(&mut self, name: &str, event_name: &str, value: &str) {
        let id = self.targets[name];
        let data = SerializedFormData::new(value.into(), vec![]);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event(event_name, event, id);
        self.settle();
    }
}

#[tokio::test]
async fn loading_and_reloading_protect_navigation_until_success_or_error() {
    let probe = Probe::new();
    let reply = probe.load_reply();
    let mut ui = Ui::new(probe.clone());
    ui.navigation_state("pending");
    assert!(ui.html().contains("aria-busy=true"));
    ui.dismiss("escape");
    ui.dismiss("backdrop");
    ui.navigation_state("pending");
    reply
        .send(Err(ServerFnError::new("private transport")))
        .unwrap();
    ui.settle();
    ui.navigation_state("clean");
    let reply = probe.load_reply();
    ui.click("permission-editor-reload");
    ui.navigation_state("pending");
    reply.send(Ok(probe.editor.clone())).unwrap();
    ui.settle();
    ui.navigation_state("clean");
    assert!(ui.html().contains("Example person"));
}

#[tokio::test]
async fn dirty_close_requires_confirmation_and_failure_preserves_the_draft() {
    for path in [
        "permission-editor-close",
        "permission-editor-cancel",
        "escape",
        "backdrop",
    ] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        ui.select_profile("Member");
        for reply in [Ok(false), Err(document::EvalError::Unsupported)] {
            probe.confirmations.borrow_mut().push_back(reply);
            ui.dismiss(path);
            ui.navigation_state("dirty");
            assert!(ui.html().contains("Example person"));
        }
        probe.confirmations.borrow_mut().push_back(Ok(true));
        ui.dismiss(path);
        ui.navigation_state("clean");
        assert!(!ui.html().contains("Example person"));
        assert!(probe.confirmations.borrow().is_empty());
        assert!(probe.saves.borrow().is_empty());
        assert!(probe.templates.borrow().is_empty());
    }
}

#[tokio::test]
async fn clean_dismissal_needs_no_confirmation() {
    for path in [
        "permission-editor-close",
        "permission-editor-cancel",
        "escape",
        "backdrop",
    ] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        ui.dismiss(path);
        ui.navigation_state("clean");
        assert!(!ui.html().contains("Example person"));
        assert!(probe.saves.borrow().is_empty());
    }
}

#[tokio::test]
async fn creating_a_template_captures_final_grants_and_retries_without_saving_the_person() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.navigation_state("clean");
    ui.select_profile("Member");
    ui.navigation_state("dirty");
    ui.click("permission-ClientReadAll");
    ui.click("permission-template-create");
    assert!(ui.html().contains("Create custom profile"));
    assert!(probe.templates.borrow().is_empty());
    ui.form_event("permission-template-name", "input", "  Equipo  ");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    ui.click("permission-template-save");
    ui.navigation_state("pending");
    assert_eq!(probe.templates.borrow().len(), 1);
    let command = probe.templates.borrow()[0].clone();
    assert_eq!(command.expected_access_revision, 7);
    let TemplateAction::Create { name, grants } = &command.action else {
        panic!("expected create");
    };
    assert_eq!(name, "Equipo");
    assert!(grants.contains(&Permission::ClientReadAll));
    assert!(!grants.contains(&Permission::ProjectReadManaged));
    reply
        .send(Err(ServerFnError::new("private transport")))
        .unwrap();
    ui.settle();
    ui.form_event("permission-template-name", "input", "Changed intent");
    ui.navigation_state("pending");
    ui.click("permission-template-cancel");
    assert!(ui.html().contains("Retry same save"));
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    assert_eq!(probe.templates.borrow()[1], command);
    reply
        .send(Ok(TemplateOutcome {
            template_id: Uuid::now_v7(),
            access_revision: 8,
            detached_people: 0,
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Custom profile created"));
    ui.navigation_state("dirty");
    assert!(
        ui.html()
            .contains("Reload and discard unsaved person changes")
    );
    assert!(!ui.html().contains("private transport"));
    assert!(probe.saves.borrow().is_empty());
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn deleting_a_template_requires_its_current_affected_people_preview() {
    let mut probe = Probe::new();
    let template = TemplateChoice {
        id: Uuid::now_v7(),
        name: "Studio".into(),
        grants: probe.editor.permissions.grants.clone(),
        revision: 2,
    };
    probe.editor.templates.push(template.clone());
    let mut ui = Ui::new(probe.clone());
    let reply = probe.deletion_reply();
    ui.click(&format!("permission-template-delete-{}", template.id));
    assert_eq!(*probe.deletions.borrow(), vec![template.id]);
    assert!(probe.templates.borrow().is_empty());
    ui.click("permission-template-cancel");
    assert!(ui.html().contains("Loading affected people"));
    assert!(ui.html().contains("aria-busy=true"));
    assert!(!ui.html().contains(">Close</button>"));
    reply
        .send(Ok(TemplateDeletionPreview {
            access_revision: 7,
            template: template.clone(),
            people: vec![TemplateAssignee {
                user_id: probe.editor.user_id,
                name: "Affected person".into(),
                permissions: probe.editor.permissions.clone(),
            }],
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Affected person"));
    assert!(ui.html().contains("keep their permissions"));
    ui.click("permission-template-save");
    assert!(probe.templates.borrow().is_empty());
    ui.click("permission-template-confirm");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    assert_eq!(
        probe.templates.borrow()[0].action,
        TemplateAction::Delete {
            id: template.id,
            expected_revision: 2,
        }
    );
    reply
        .send(Err(ServerFnError::ServerError {
            code: 409,
            message: "Permissions changed".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Permissions changed"));
    assert!(
        ui.html()
            .contains("Reload and discard unsaved person changes")
    );
    assert!(!ui.html().contains("permission-template-save"));
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn cancelling_template_creation_keeps_the_unsaved_person_draft() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.select_profile("Member");
    ui.click("permission-ClientReadAll");
    ui.click("permission-template-create");
    ui.click("permission-template-save");
    assert!(
        probe.templates.borrow().is_empty(),
        "blank names cannot submit"
    );
    ui.form_event("permission-template-name", "input", "Unsaved");
    ui.click("permission-template-cancel");
    let reply = probe.preview_reply();
    ui.click("permission-review");
    let draft = probe.previews.borrow()[0].clone();
    assert!(draft.grants.contains(&Permission::ClientReadAll));
    assert!(!draft.grants.contains(&Permission::ProjectReadManaged));
    assert!(probe.templates.borrow().is_empty());
    reply
        .send(Err(ServerFnError::new("test preview stopped")))
        .unwrap();
    ui.settle();
}

#[tokio::test]
async fn template_creation_is_disabled_for_administrator_identity_or_fifty_profiles() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.select_profile("Administrator");
    ui.click("permission-template-create");
    assert!(!ui.html().contains("permission-template-name"));
    assert!(ui.html().contains("Choose a non-administrative profile"));
    let mut full = Probe::new();
    full.editor.templates = (0..50)
        .map(|index| TemplateChoice {
            id: Uuid::now_v7(),
            name: format!("Profile {index}"),
            revision: 0,
            grants: full.editor.permissions.grants.clone(),
        })
        .collect();
    let mut ui = Ui::new(full.clone());
    ui.click("permission-template-create");
    assert!(!ui.html().contains("permission-template-name"));
    assert!(ui.html().contains("limit of 50 custom profiles"));
    assert!(full.templates.borrow().is_empty());
}

#[tokio::test]
async fn template_creation_reports_duplicate_limit_and_validation_errors_without_success() {
    for (code, message) in [
        (409, "A profile with this name already exists"),
        (409, "At most 50 reusable profiles are allowed"),
        (400, "Profile names must have at most 100 characters"),
    ] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        ui.click("permission-template-create");
        ui.form_event("permission-template-name", "input", "Equipo");
        let reply = probe.template_reply();
        ui.click("permission-template-save");
        reply
            .send(Err(ServerFnError::ServerError {
                code,
                message: message.into(),
                details: None,
            }))
            .unwrap();
        ui.settle();
        assert!(ui.html().contains(message), "{}", ui.html());
        assert!(!ui.html().contains("Custom profile created"));
        assert!(!ui.html().contains("permission-template-save"));
        assert!(probe.saves.borrow().is_empty());
    }
}

fn deletion_fixture() -> (Probe, TemplateChoice) {
    let mut probe = Probe::new();
    let template = TemplateChoice {
        id: Uuid::now_v7(),
        name: "Sensitive profile".into(),
        revision: 3,
        grants: probe.editor.permissions.grants.clone(),
    };
    probe.editor.templates.push(template.clone());
    (probe, template)
}

#[tokio::test]
async fn deletion_never_confirms_mismatched_or_denied_previews() {
    for mismatch in 0..4 {
        let (probe, template) = deletion_fixture();
        let mut ui = Ui::new(probe.clone());
        let reply = probe.deletion_reply();
        ui.click(&format!("permission-template-delete-{}", template.id));
        let mut preview = TemplateDeletionPreview {
            access_revision: 7,
            template,
            people: vec![],
        };
        match mismatch {
            0 => preview.access_revision += 1,
            1 => preview.template.revision += 1,
            2 => preview.template.id = Uuid::now_v7(),
            _ => {}
        }
        if mismatch == 3 {
            reply
                .send(Err(ServerFnError::ServerError {
                    code: 403,
                    message: "private authority".into(),
                    details: None,
                }))
                .unwrap();
        } else {
            reply.send(Ok(preview)).unwrap();
        }
        ui.settle();
        let html = ui.html();
        assert!(!html.contains("permission-template-save"), "{html}");
        assert!(!html.contains("Sensitive profile"), "{html}");
        assert!(!html.contains("private authority"), "{html}");
        assert!(probe.templates.borrow().is_empty());
    }
}

#[tokio::test]
async fn empty_profile_deletion_requires_confirmation_and_keeps_exact_retry() {
    let (probe, template) = deletion_fixture();
    let mut ui = Ui::new(probe.clone());
    let reply = probe.deletion_reply();
    ui.click(&format!("permission-template-delete-{}", template.id));
    reply
        .send(Ok(TemplateDeletionPreview {
            access_revision: 7,
            template: template.clone(),
            people: vec![],
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("No people currently use this profile"));
    ui.click("permission-template-save");
    assert!(probe.templates.borrow().is_empty());
    ui.click("permission-template-confirm");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    reply
        .send(Err(ServerFnError::new("response lost")))
        .unwrap();
    ui.settle();
    ui.click("permission-template-confirm");
    ui.click("permission-template-cancel");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    assert_eq!(probe.templates.borrow()[0], probe.templates.borrow()[1]);
    reply
        .send(Ok(TemplateOutcome {
            template_id: template.id,
            access_revision: 8,
            detached_people: 0,
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Custom profile deleted"));
    assert!(!ui.html().contains("style="));
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn template_save_revocation_hides_the_selection_and_does_not_report_success() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.click("permission-template-create");
    ui.form_event("permission-template-name", "input", "Private name");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    reply
        .send(Err(ServerFnError::ServerError {
            code: 403,
            message: "private authority".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Permission editing is unavailable"));
    assert!(!html.contains("Private name"));
    assert!(!html.contains("View managed projects"));
    assert!(!html.contains("private authority"));
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn rejected_template_reload_fetches_and_explicitly_discards_the_person_draft() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    assert_eq!(*probe.loads.borrow(), 1);
    ui.select_profile("Member");
    ui.click("permission-template-create");
    ui.form_event("permission-template-name", "input", "Duplicate");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    reply
        .send(Err(ServerFnError::ServerError {
            code: 409,
            message: "A profile with this name already exists".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    ui.click("permission-template-reload");
    assert_eq!(*probe.loads.borrow(), 2);
    assert!(
        !ui.html().contains("permission-template-name"),
        "{}",
        ui.html()
    );
    assert!(ui.html().contains("Recover permission request"));
    assert!(!ui.html().contains("Review changes"));
    assert_eq!(probe.templates.borrow().len(), 1);
    assert_eq!(probe.storage.borrow().len(), 1);
    ui.settle();
}

#[tokio::test]
async fn cancelling_a_loaded_deletion_preview_preserves_person_edits_without_mutation() {
    let (probe, template) = deletion_fixture();
    let mut ui = Ui::new(probe.clone());
    ui.select_profile("Member");
    let reply = probe.deletion_reply();
    ui.click(&format!("permission-template-delete-{}", template.id));
    reply
        .send(Ok(TemplateDeletionPreview {
            access_revision: 7,
            template,
            people: vec![],
        }))
        .unwrap();
    ui.settle();
    ui.click("permission-template-confirm");
    ui.click("permission-template-cancel");
    assert!(probe.templates.borrow().is_empty());
    let reply = probe.preview_reply();
    ui.click("permission-review");
    assert_eq!(
        probe.previews.borrow()[0].grants,
        BuiltInProfile::Member
            .selection()
            .iter()
            .collect::<Vec<_>>()
    );
    reply
        .send(Err(ServerFnError::new("test preview stopped")))
        .unwrap();
    ui.settle();
}

#[tokio::test]
async fn navigation_tracks_reverted_edits_and_cancelled_template_inputs() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.navigation_state("clean");
    ui.select_profile("Project Manager");
    ui.navigation_state("clean");
    ui.select_profile("Member");
    ui.navigation_state("dirty");
    ui.select_profile("current");
    ui.navigation_state("clean");
    ui.click("permission-template-create");
    ui.navigation_state("clean");
    ui.form_event("permission-template-name", "input", "Unsaved profile");
    ui.navigation_state("dirty");
    ui.click("permission-template-cancel");
    ui.navigation_state("clean");
    assert!(probe.saves.borrow().is_empty());
    assert!(probe.templates.borrow().is_empty());
}

#[tokio::test]
async fn real_controls_review_once_and_retry_the_identical_uncertain_save() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    let reply = probe.preview_reply();
    ui.click("permission-review");
    ui.click("permission-review");
    ui.navigation_state("pending");
    assert_eq!(probe.previews.borrow().len(), 1);
    assert!(probe.saves.borrow().is_empty());
    reply.send(Ok(probe.effects())).unwrap();
    ui.settle();
    ui.navigation_state("clean");
    assert!(ui.html().contains("No changes to the saved configuration"));
    let save_reply = probe.save_reply();
    ui.click("permission-save");
    ui.click("permission-save");
    ui.navigation_state("pending");
    assert_eq!(probe.saves.borrow().len(), 1);
    save_reply
        .send(Err(ServerFnError::new("private transport details")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Retry same save"));
    ui.navigation_state("pending");
    assert!(!ui.html().contains("private transport details"));
    let retry_reply = probe.save_reply();
    ui.click("permission-save");
    assert_eq!(probe.saves.borrow().len(), 2);
    assert_eq!(probe.saves.borrow()[0], probe.saves.borrow()[1]);
    retry_reply
        .send(Ok(ProfileOutcome {
            user_id: probe.editor.user_id,
            access_revision: 7,
            person_revision: 3,
            changed: false,
        }))
        .unwrap();
    ui.settle();
    assert_eq!(*probe.saved.borrow(), vec![false]);
    ui.navigation_state("clean");
}

#[tokio::test]
async fn permission_revocation_hides_the_form_without_reporting_success() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    let reply = probe.preview_reply();
    ui.click("permission-review");
    reply
        .send(Err(ServerFnError::ServerError {
            code: 403,
            message: "private authority details".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Permission editing is unavailable"), "{html}");
    assert!(!html.contains("Example person"), "{html}");
    assert!(!html.contains("View managed projects"), "{html}");
    assert!(!html.contains("private authority"), "{html}");
    assert!(probe.saves.borrow().is_empty());
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn keeping_projects_requires_new_review_and_confirmation_of_remaining_people() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.select_profile("Member");
    let reply = probe.preview_reply();
    ui.click("permission-review");
    let proposed = probe.previews.borrow()[0].clone();
    assert_eq!(
        proposed.action,
        ProfileAction::BuiltIn {
            profile: BuiltInProfile::Member
        }
    );
    assert_eq!(
        proposed.grants,
        BuiltInProfile::Member
            .selection()
            .iter()
            .collect::<Vec<_>>()
    );
    let mut effects = probe.effects();
    effects.after.grants = proposed.grants;
    effects.after.source = ProfileSource::BuiltIn(BuiltInProfile::Member);
    effects.changed = true;
    effects.remove_projects.push(RelationshipRemoval {
        id: Uuid::now_v7(),
        subject_id: Uuid::now_v7(),
        revision: 0,
        name: "Proyecto <Norte> & Sur".into(),
    });
    effects.remove_people.push(RelationshipRemoval {
        id: Uuid::now_v7(),
        subject_id: Uuid::now_v7(),
        revision: 0,
        name: "María <Equipo> & Co.".into(),
    });
    reply.send(Ok(effects.clone())).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(
        html.contains("Proyecto &#60;Norte&#62; &#38; Sur"),
        "{html}"
    );
    assert!(html.contains("María &#60;Equipo&#62; &#38; Co."), "{html}");
    for effect in effects.remove_projects.iter().chain(&effects.remove_people) {
        assert!(!html.contains(&effect.subject_id.to_string()), "{html}");
    }
    ui.click("permission-save");
    assert!(
        probe.saves.borrow().is_empty(),
        "unconfirmed losses must not save"
    );
    ui.click("permission-keep-projects");
    assert!(!ui.html().contains("Confirm permissions"));
    let reply = probe.preview_reply();
    ui.click("permission-review");
    let kept = probe.previews.borrow()[1].clone();
    assert!(kept.grants.contains(&Permission::ProjectReadManaged));
    assert!(kept.grants.contains(&Permission::ProjectWriteManaged));
    assert!(!kept.grants.contains(&Permission::ProjectCreateAll));
    assert!(!kept.grants.contains(&Permission::PeopleReadManaged));
    effects.after.grants = kept.grants;
    effects.remove_projects.clear();
    reply.send(Ok(effects.clone())).unwrap();
    ui.settle();
    ui.click("permission-save");
    assert!(
        probe.saves.borrow().is_empty(),
        "person losses need their own confirmation"
    );
    ui.click("permission-confirm-removals");
    let reply = probe.save_reply();
    ui.click("permission-save");
    assert_eq!(probe.saves.borrow().len(), 1);
    assert!(probe.saves.borrow()[0].remove_projects.is_empty());
    assert_eq!(
        probe.saves.borrow()[0].remove_people,
        vec![effects.remove_people[0].id]
    );
    reply
        .send(Err(ServerFnError::ServerError {
            code: 409,
            message: "Permissions changed".into(),
            details: None,
        }))
        .unwrap();
    ui.settle();
    assert!(
        ui.html()
            .contains("Reload permissions and review your changes again")
    );
    assert!(!ui.html().contains("Confirm permissions"));
    assert!(probe.saved.borrow().is_empty());
}

fn review_person(ui: &mut Ui, probe: &Probe) {
    let reply = probe.preview_reply();
    ui.click("permission-review");
    reply.send(Ok(probe.effects())).unwrap();
    ui.settle();
}

fn person_outcome(probe: &Probe) -> ProfileOutcome {
    ProfileOutcome {
        user_id: probe.editor.user_id,
        access_revision: 7,
        person_revision: 3,
        changed: false,
    }
}

fn interrupted_person(probe: &Probe) {
    let mut ui = Ui::new(probe.clone());
    review_person(&mut ui, probe);
    let reply = probe.save_reply();
    ui.click("permission-save");
    reply
        .send(Err(ServerFnError::new("response lost")))
        .unwrap();
    ui.settle();
    assert_eq!(probe.storage.borrow().len(), 1);
}

#[tokio::test]
async fn remount_recovers_the_same_person_command_without_automatic_submission_or_target_load() {
    let mut probe = Probe::new();
    interrupted_person(&probe);
    let original = probe.saves.borrow()[0].clone();
    probe.open_person = false;
    let mut ui = Ui::new(probe.clone());
    assert!(ui.html().contains("Recover permission request"));
    assert_eq!(probe.saves.borrow().len(), 1);
    assert_eq!(*probe.loads.borrow(), 1);
    ui.navigation_state("pending");
    let reply = probe.save_reply();
    ui.click("permission-recovery-retry");
    ui.click("permission-recovery-retry");
    assert_eq!(
        probe.saves.borrow().as_slice(),
        &[original.clone(), original]
    );
    reply.send(Ok(person_outcome(&probe))).unwrap();
    ui.settle();
    assert!(probe.storage.borrow().is_empty());
    assert_eq!(probe.saved.borrow().as_slice(), &[false]);
    ui.navigation_state("clean");
}

#[tokio::test]
async fn missing_storage_acknowledgement_never_sends_a_command_and_reuses_its_record() {
    for failure in ["store", "ack-store"] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        review_person(&mut ui, &probe);
        *probe.storage_failure.borrow_mut() = Some(failure.into());
        ui.click("permission-save");
        assert!(probe.saves.borrow().is_empty());
        assert!(probe.saved.borrow().is_empty());
        ui.navigation_state("pending");
        let retained = probe.storage.borrow().clone();
        *probe.storage_failure.borrow_mut() = None;
        let reply = probe.save_reply();
        ui.click("permission-save");
        if failure == "ack-store" {
            assert_eq!(*probe.storage.borrow(), retained);
        }
        reply.send(Ok(person_outcome(&probe))).unwrap();
        ui.settle();
        assert_eq!(probe.saves.borrow().len(), 1);
        assert!(probe.storage.borrow().is_empty());
    }
}

#[tokio::test]
async fn acknowledged_person_cleanup_retries_without_resubmitting() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    review_person(&mut ui, &probe);
    *probe.storage_failure.borrow_mut() = Some("clear".into());
    let reply = probe.save_reply();
    ui.click("permission-save");
    reply.send(Ok(person_outcome(&probe))).unwrap();
    ui.settle();
    assert!(ui.html().contains("The server confirmed this request"));
    assert!(ui.html().contains("Finish recovery cleanup"));
    assert_eq!(probe.storage.borrow().len(), 1);
    *probe.storage_failure.borrow_mut() = None;
    ui.click("permission-save");
    assert_eq!(probe.saves.borrow().len(), 1);
    assert!(probe.storage.borrow().is_empty());
    assert_eq!(probe.saved.borrow().as_slice(), &[false]);
}

#[tokio::test]
async fn rejected_recovery_keeps_the_record_until_explicit_checked_discard() {
    for code in [400, 401, 403, 404, 409] {
        let mut probe = Probe::new();
        interrupted_person(&probe);
        let retained = probe.storage.borrow().clone();
        probe.open_person = false;
        let mut ui = Ui::new(probe.clone());
        let reply = probe.save_reply();
        ui.click("permission-recovery-retry");
        reply
            .send(Err(ServerFnError::ServerError {
                code,
                message: "private error".into(),
                details: None,
            }))
            .unwrap();
        ui.settle();
        assert_eq!(*probe.storage.borrow(), retained);
        assert!(
            ui.html()
                .contains("An earlier attempt may still have saved")
        );
        if [401, 403, 404].contains(&code) {
            assert!(!ui.html().contains("private error"));
        }
        ui.click("permission-recovery-discard");
        assert_eq!(*probe.storage.borrow(), retained);
        ui.click("permission-recovery-checked");
        *probe.storage_failure.borrow_mut() = Some("clear".into());
        ui.click("permission-recovery-discard");
        assert_eq!(*probe.storage.borrow(), retained);
        *probe.storage_failure.borrow_mut() = None;
        ui.click("permission-recovery-discard");
        assert!(probe.storage.borrow().is_empty());
        assert!(probe.saved.borrow().is_empty());
        ui.navigation_state("clean");
    }
}

#[tokio::test]
async fn template_recovery_survives_remount_and_cleanup_failure_without_saving_a_person() {
    let mut probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.click("permission-template-create");
    ui.form_event("permission-template-name", "input", "Team");
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    reply
        .send(Err(ServerFnError::new("response lost")))
        .unwrap();
    ui.settle();
    drop(ui);
    probe.open_person = false;
    let mut ui = Ui::new(probe.clone());
    assert_eq!(probe.templates.borrow().len(), 1);
    *probe.storage_failure.borrow_mut() = Some("clear".into());
    let reply = probe.template_reply();
    ui.click("permission-recovery-retry");
    assert_eq!(probe.templates.borrow()[0], probe.templates.borrow()[1]);
    reply
        .send(Ok(TemplateOutcome {
            template_id: Uuid::now_v7(),
            access_revision: 8,
            detached_people: 0,
        }))
        .unwrap();
    ui.settle();
    assert_eq!(probe.storage.borrow().len(), 1);
    *probe.storage_failure.borrow_mut() = None;
    ui.click("permission-recovery-retry");
    assert_eq!(probe.templates.borrow().len(), 2);
    assert!(probe.storage.borrow().is_empty());
    assert!(ui.html().contains("Custom profile request completed"));
    ui.click("permission-recovery-done");
    ui.navigation_state("clean");
    assert!(probe.saves.borrow().is_empty());
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn another_request_cannot_be_overwritten_or_cleared_by_the_live_editor() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    review_person(&mut ui, &probe);
    let key = format!(
        "horae-permission-request:v1:{}:{}",
        probe.editor.requester.org_id, probe.editor.requester.user_id
    );
    probe
        .storage
        .borrow_mut()
        .insert(key, "another unresolved request".into());
    ui.click("permission-save");
    assert!(probe.saves.borrow().is_empty());
    assert!(
        ui.html()
            .contains("Another permission request is unresolved")
    );
    assert_eq!(
        probe.storage.borrow().values().next().unwrap(),
        "another unresolved request"
    );
}

#[tokio::test]
async fn acknowledged_cleanup_does_not_erase_a_replacement_record() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    review_person(&mut ui, &probe);
    let reply = probe.save_reply();
    ui.click("permission-save");
    let key = probe.storage.borrow().keys().next().unwrap().clone();
    probe
        .storage
        .borrow_mut()
        .insert(key, "different request".into());
    reply.send(Ok(person_outcome(&probe))).unwrap();
    ui.settle();
    ui.click("permission-save");
    assert_eq!(
        probe.storage.borrow().values().next().unwrap(),
        "different request"
    );
    assert_eq!(probe.saves.borrow().len(), 1);
    assert!(probe.saved.borrow().is_empty());
    assert!(ui.html().contains("The server confirmed this request"));
}

#[tokio::test]
async fn template_editor_cleanup_failure_never_reissues_the_acknowledged_command() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    ui.click("permission-template-create");
    ui.form_event("permission-template-name", "input", "Team");
    *probe.storage_failure.borrow_mut() = Some("clear".into());
    let reply = probe.template_reply();
    ui.click("permission-template-save");
    reply
        .send(Ok(TemplateOutcome {
            template_id: Uuid::now_v7(),
            access_revision: 8,
            detached_people: 0,
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Finish recovery cleanup"));
    assert_eq!(probe.storage.borrow().len(), 1);
    *probe.storage_failure.borrow_mut() = None;
    ui.click("permission-template-save");
    assert!(ui.html().contains("Custom profile created"));
    assert_eq!(probe.templates.borrow().len(), 1);
    assert!(probe.storage.borrow().is_empty());
}

#[tokio::test]
async fn storage_load_failure_blocks_new_edits_and_retry_restores_them() {
    let probe = Probe::new();
    *probe.storage_failure.borrow_mut() = Some("load".into());
    let mut ui = Ui::new(probe.clone());
    assert!(!ui.html().contains("permission-review"));
    assert!(ui.html().contains("Retry recovery check"));
    *probe.storage_failure.borrow_mut() = None;
    ui.click("permission-editor-reload");
    assert!(ui.html().contains("permission-review"));
    assert!(probe.saves.borrow().is_empty());
}

#[tokio::test]
async fn pending_or_failed_session_check_never_exposes_permission_controls() {
    let probe = Probe::new();
    let (send, receive) = oneshot::channel();
    probe.identity_replies.borrow_mut().push_back(receive);
    let mut ui = Ui::new(probe.clone());
    ui.navigation_state("pending");
    assert!(!ui.html().contains("Review changes"));
    send.send(Err(ServerFnError::new("private session diagnostic")))
        .unwrap();
    ui.settle();
    assert!(!ui.html().contains("private session diagnostic"));
    assert!(!ui.html().contains("Review changes"));
    assert!(ui.html().contains("Cannot check the current session"));
    ui.click("permission-editor-reload");
    assert!(ui.html().contains("Review changes"));
    assert!(probe.saves.borrow().is_empty());
}

#[tokio::test]
async fn uncertain_recovery_keeps_retry_available_without_discard_or_permanent_aria_busy() {
    let mut probe = Probe::new();
    interrupted_person(&probe);
    let retained = probe.storage.borrow().clone();
    probe.open_person = false;
    let mut ui = Ui::new(probe.clone());
    for problem in [
        ServerFnError::new("private diagnostic"),
        ServerFnError::ServerError {
            code: 500,
            message: "private diagnostic".into(),
            details: None,
        },
    ] {
        let reply = probe.save_reply();
        ui.click("permission-recovery-retry");
        assert!(ui.html().contains("aria-busy=true"));
        reply.send(Err(problem)).unwrap();
        ui.settle();
        assert!(ui.html().contains("aria-busy=false"));
        assert!(ui.html().contains("may already have completed"));
        assert!(!ui.html().contains("Discard recovery record"));
        assert!(!ui.html().contains("private diagnostic"));
        ui.dismiss("escape");
        ui.dismiss("backdrop");
        ui.navigation_state("pending");
        assert!(ui.html().contains("Recover permission request"));
        assert_eq!(*probe.storage.borrow(), retained);
    }
    assert!(probe.saved.borrow().is_empty());
}

#[tokio::test]
async fn invalid_or_misbound_storage_is_preserved_without_rendering_its_contents() {
    let original = Probe::new();
    interrupted_person(&original);
    let stored = original.storage.borrow().values().next().unwrap().clone();
    for value in [
        "{broken private data".to_owned(),
        stored.clone(),
        format!("{stored} "),
        "x".repeat(524289),
    ] {
        let probe = Probe::new();
        let key = format!(
            "horae-permission-request:v1:{}:{}",
            probe.editor.requester.org_id, probe.editor.requester.user_id
        );
        probe.storage.borrow_mut().insert(key, value.clone());
        let ui = Ui::new(probe.clone());
        assert!(ui.html().contains("recovery data is invalid"));
        assert!(!ui.html().contains("private data"));
        assert!(!ui.html().contains("Review changes"));
        assert!(probe.saves.borrow().is_empty());
        assert_eq!(probe.storage.borrow().values().next().unwrap(), &value);
    }
}

#[tokio::test]
async fn switching_accounts_or_workspaces_does_not_load_or_erase_the_original_request() {
    let original = Probe::new();
    interrupted_person(&original);
    let retained = original.storage.borrow().clone();
    for change_org in [false, true] {
        let mut probe = original.clone();
        if change_org {
            probe.editor.requester.org_id = Uuid::now_v7();
        } else {
            probe.editor.requester.user_id = Uuid::now_v7();
        }
        let ui = Ui::new(probe.clone());
        assert!(!ui.html().contains("Recover permission request"));
        assert!(ui.html().contains("Review changes"));
        assert_eq!(*probe.storage.borrow(), retained);
        assert_eq!(probe.saves.borrow().len(), 1);
    }
}

mod server_fns {
    use super::*;

    pub struct CurrentUser {
        pub id: Uuid,
        pub org_id: Uuid,
    }
    pub async fn get_me() -> Result<CurrentUser, ServerFnError> {
        let probe = use_context::<Probe>();
        let reply = probe.identity_replies.borrow_mut().pop_front();
        let requester = match reply {
            Some(reply) => reply.await.expect("identity reply dropped")?,
            None => probe.editor.requester,
        };
        Ok(CurrentUser {
            id: requester.user_id,
            org_id: requester.org_id,
        })
    }

    fn assert_stored(probe: &Probe, kind: &str, command: &impl serde::Serialize) {
        let requester = probe.editor.requester;
        let key = format!(
            "horae-permission-request:v1:{}:{}",
            requester.org_id, requester.user_id
        );
        let stored: serde_json::Value = serde_json::from_str(
            probe
                .storage
                .borrow()
                .get(&key)
                .expect("mutation before storage"),
        )
        .unwrap();
        assert_eq!(
            stored["requester"],
            serde_json::to_value(requester).unwrap()
        );
        assert_eq!(stored["command"]["kind"], kind);
        assert_eq!(
            stored["command"]["value"],
            serde_json::to_value(command).unwrap()
        );
    }

    pub async fn load_permission_editor(id: Uuid) -> Result<PermissionEditor, ServerFnError> {
        let probe = use_context::<Probe>();
        *probe.loads.borrow_mut() += 1;
        assert_eq!(id, probe.editor.user_id);
        let reply = probe.load_replies.borrow_mut().pop_front();
        match reply {
            Some(reply) => reply.await.expect("load reply dropped"),
            None => Ok(probe.editor),
        }
    }

    pub async fn preview_person_permissions(
        draft: ProfileDraft,
    ) -> Result<ProfilePreview, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.previews.borrow_mut().push(draft);
        let reply = probe
            .preview_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected preview");
        reply.await.expect("preview reply dropped")
    }

    pub async fn save_person_permissions(
        command: ProfileCommand,
        expected_requester: PermissionRequester,
    ) -> Result<ProfileOutcome, ServerFnError> {
        let probe = use_context::<Probe>();
        assert_eq!(expected_requester, probe.editor.requester);
        assert_stored(&probe, "person", &command);
        probe.saves.borrow_mut().push(command);
        let reply = probe
            .save_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected save");
        reply.await.expect("save reply dropped")
    }

    pub async fn save_permission_template(
        command: TemplateCommand,
        expected_requester: PermissionRequester,
    ) -> Result<TemplateOutcome, ServerFnError> {
        let probe = use_context::<Probe>();
        assert_eq!(expected_requester, probe.editor.requester);
        assert_stored(&probe, "template", &command);
        probe.templates.borrow_mut().push(command);
        let reply = probe
            .template_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected template save");
        reply.await.expect("template reply dropped")
    }

    pub async fn preview_permission_template_deletion(
        id: Uuid,
        expected_access_revision: i64,
        expected_template_revision: i64,
    ) -> Result<TemplateDeletionPreview, ServerFnError> {
        let probe = use_context::<Probe>();
        assert_eq!(expected_access_revision, probe.editor.access_revision);
        assert_eq!(
            expected_template_revision,
            probe
                .editor
                .templates
                .iter()
                .find(|template| template.id == id)
                .unwrap()
                .revision
        );
        probe.deletions.borrow_mut().push(id);
        let reply = probe
            .deletion_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected deletion preview");
        reply.await.expect("deletion reply dropped")
    }
}
