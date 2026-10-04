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

#[derive(Clone)]
struct Probe {
    editor: PermissionEditor,
    previews: Rc<RefCell<Vec<ProfileDraft>>>,
    preview_replies: Rc<RefCell<VecDeque<PreviewReply>>>,
    saves: Rc<RefCell<Vec<ProfileCommand>>>,
    save_replies: Rc<RefCell<VecDeque<SaveReply>>>,
    saved: Rc<RefCell<Vec<bool>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            editor: PermissionEditor {
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
        }
    }

    fn preview_reply(&self) -> oneshot::Sender<Result<ProfilePreview, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.preview_replies.borrow_mut().push_back(receive);
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
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe.clone());
    let person = use_signal(|| Some(probe.editor.user_id));
    rsx! { permission_editor::PermissionEditorDialog {
        person, on_saved: move |changed| probe.saved.borrow_mut().push(changed),
    } }
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
        let id = *self
            .targets
            .get(name)
            .unwrap_or_else(|| panic!("missing {name}: {}", self.html()));
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event("click", event, id);
        self.settle();
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }

    fn select_profile(&mut self, value: &str) {
        let id = self.targets["person-permissions-profile"];
        let data = SerializedFormData::new(value.into(), vec![]);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event("change", event, id);
        self.settle();
    }
}

#[tokio::test]
async fn real_controls_review_once_and_retry_the_identical_uncertain_save() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    let reply = probe.preview_reply();
    ui.click("permission-review");
    ui.click("permission-review");
    assert_eq!(probe.previews.borrow().len(), 1);
    assert!(probe.saves.borrow().is_empty());
    reply.send(Ok(probe.effects())).unwrap();
    ui.settle();
    assert!(ui.html().contains("No changes to the saved configuration"));
    let save_reply = probe.save_reply();
    ui.click("permission-save");
    ui.click("permission-save");
    assert_eq!(probe.saves.borrow().len(), 1);
    save_reply
        .send(Err(ServerFnError::new("private transport details")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Retry same save"));
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
    });
    effects.remove_people.push(RelationshipRemoval {
        id: Uuid::now_v7(),
        subject_id: Uuid::now_v7(),
        revision: 0,
    });
    reply.send(Ok(effects.clone())).unwrap();
    ui.settle();
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

mod server_fns {
    use super::*;

    pub async fn load_permission_editor(id: Uuid) -> Result<PermissionEditor, ServerFnError> {
        let probe = use_context::<Probe>();
        assert_eq!(id, probe.editor.user_id);
        Ok(probe.editor)
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
    ) -> Result<ProfileOutcome, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.saves.borrow_mut().push(command);
        let reply = probe
            .save_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected save");
        reply.await.expect("save reply dropped")
    }
}
