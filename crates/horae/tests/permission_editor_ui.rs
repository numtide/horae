#![cfg(feature = "server")]
// The directory and the Add person dialog are mounted on their own and again
// inside the whole People page, which declares them as its submodules.
#![allow(clippy::duplicate_mod)]

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

#[allow(dead_code)]
#[path = "../src/components/avatar.rs"]
pub mod avatar;
#[path = "../src/components/badge.rs"]
pub mod badge;
#[allow(dead_code)]
#[path = "../src/components/button.rs"]
pub mod button;
#[path = "../src/components/confirm_dialog.rs"]
pub mod confirm_dialog;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/date_picker.rs"]
pub mod date_picker;
#[path = "../src/components/filter_bar.rs"]
pub mod filter_bar;
#[path = "../src/components/form.rs"]
pub mod form;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/menu.rs"]
pub mod menu;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[path = "../src/components/page_header.rs"]
pub mod page_header;
#[path = "../src/components/period_nav.rs"]
pub mod period_nav;
#[path = "../src/components/permission_description.rs"]
pub mod permission_description;
#[allow(dead_code)]
#[path = "../src/components/screen_state.rs"]
pub mod screen_state;
#[path = "../src/components/table.rs"]
pub mod table;
#[path = "../src/components/toast.rs"]
pub mod toast;
#[path = "../src/components/today.rs"]
pub mod today;
mod components {
    pub use super::{
        avatar, badge, button, confirm_dialog, controls, filter_bar, form, icons, menu, modal,
        page_header, period_nav, permission_description, screen_state, session, table, toast,
        today,
    };
}
pub use horae_types::{
    people as people_model, permission_editor as permission_editor_model, user as user_model,
};
mod models {
    pub use super::people_model as people;
    pub use super::permission_editor_model as permission_editor;
    pub use super::user_model as user;
}
#[path = "../src/pages/people/add_person.rs"]
mod add_person;
#[path = "../src/pages/people/directory.rs"]
mod people;
// The whole People page, for its access gate. It includes the directory again
// as its own submodule.
#[path = "../src/pages/people.rs"]
mod people_page;
mod admin {
    pub(crate) use super::{permission_editor, rates};
    use dioxus::prelude::*;

    /// Stands in for the recovery dialog, which has its own tests.
    #[component]
    pub fn PermissionRecovery(on_saved: EventHandler<bool>) -> Element {
        let _ = on_saved;
        rsx! { div { id: "permission-recovery" } }
    }
}
mod pages {
    pub mod timesheet {
        //! The Timesheet's route values, as the People page names them.
        use std::fmt;
        use std::str::FromStr;

        #[derive(Clone, Copy, PartialEq, Debug, Default)]
        pub enum ViewMode {
            #[default]
            Week,
        }
        #[derive(Clone, Copy, PartialEq, Debug)]
        pub struct Anchor(pub chrono::NaiveDate);
        #[derive(Clone, Copy, PartialEq, Debug, Default)]
        pub enum CalSpan {
            #[default]
            Week,
        }

        impl fmt::Display for ViewMode {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("week")
            }
        }
        impl FromStr for ViewMode {
            type Err = String;
            fn from_str(_: &str) -> Result<Self, String> {
                Ok(Self::Week)
            }
        }
        impl fmt::Display for Anchor {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl FromStr for Anchor {
            type Err = String;
            fn from_str(text: &str) -> Result<Self, String> {
                text.parse().map(Self).map_err(|e| format!("{e}"))
            }
        }
        impl fmt::Display for CalSpan {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("week")
            }
        }
        impl FromStr for CalSpan {
            type Err = String;
            fn from_str(_: &str) -> Result<Self, String> {
                Ok(Self::Week)
            }
        }
    }
}
mod route {
    use super::pages::timesheet::{Anchor, CalSpan, ViewMode};
    use super::people_page::People;
    use dioxus::prelude::*;

    #[derive(Routable, Clone, PartialEq)]
    #[rustfmt::skip]
    pub enum Route {
        #[route("/people")]
        People {},
        #[route("/timesheet/:view/:date/:span/:user")]
        Timesheet { view: ViewMode, date: Anchor, span: CalSpan, user: String },
        #[route("/admin/audit")]
        PermissionAudit {},
    }

    #[component]
    fn Timesheet(view: ViewMode, date: Anchor, span: CalSpan, user: String) -> Element {
        let _ = (view, date, span, user);
        rsx! {}
    }

    #[component]
    fn PermissionAudit() -> Element {
        rsx! {}
    }
}
pub mod session {
    //! The shared session, answering with the probe's permissions.
    use dioxus::prelude::*;
    use horae_types::own_permissions::OwnPermissions;

    #[derive(Clone, Copy)]
    pub struct Session {
        pub permissions: Resource<Result<OwnPermissions, ServerFnError>>,
    }

    impl Session {
        pub fn refresh(mut self) {
            self.permissions.restart();
        }
    }

    pub fn use_session() -> Session {
        use_context::<Session>()
    }

    pub fn use_week_start() -> Resource<Result<chrono::Weekday, ServerFnError>> {
        use_resource(|| async { Ok(chrono::Weekday::Mon) })
    }
}
#[path = "../src/pages/admin/permission_editor.rs"]
mod permission_editor;
#[path = "../src/pages/admin/rates.rs"]
mod rates;
use permission_editor_model::*;

type PreviewReply = oneshot::Receiver<Result<ProfilePreview, ServerFnError>>;
type SaveReply = oneshot::Receiver<Result<ProfileOutcome, ServerFnError>>;
type TemplateReply = oneshot::Receiver<Result<TemplateOutcome, ServerFnError>>;
type DeletionReply = oneshot::Receiver<Result<TemplateDeletionPreview, ServerFnError>>;
type LoadReply = oneshot::Receiver<Result<PermissionEditor, ServerFnError>>;
type IdentityReply = oneshot::Receiver<Result<PermissionRequester, ServerFnError>>;
type SubjectsReply = oneshot::Receiver<Result<PermissionSubjectPage, ServerFnError>>;
type PeopleReply = oneshot::Receiver<Result<people_model::PeoplePage, ServerFnError>>;
type PeopleQuery = chrono::NaiveDate;

/// A Wednesday; its week, starting on Monday, begins on the 7th.
const TODAY: chrono::NaiveDate = chrono::NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
const WEEK: chrono::NaiveDate = chrono::NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();

#[derive(Clone)]
struct Probe {
    /// Mount the whole People page, with these as the viewer's permissions.
    page_access: Option<Result<horae_types::own_permissions::OwnPermissions, ServerFnError>>,
    directory: Option<bool>,
    manage_people: bool,
    view_rates: bool,
    view_history: bool,
    people_replies: Rc<RefCell<VecDeque<PeopleReply>>>,
    people_queries: Rc<RefCell<Vec<PeopleQuery>>>,
    archives: Rc<RefCell<Vec<(Uuid, bool)>>>,
    timesheets: Rc<RefCell<Vec<(Uuid, chrono::NaiveDate)>>>,
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
    storage: Rc<RefCell<HashMap<String, String>>>,
    storage_failure: Rc<RefCell<Option<String>>>,
    identity_replies: Rc<RefCell<VecDeque<IdentityReply>>>,
    identity_reads: Rc<RefCell<usize>>,
    subject_replies: Rc<RefCell<VecDeque<SubjectsReply>>>,
    subject_cursors: Rc<RefCell<Vec<Option<Uuid>>>>,
    loaded_ids: Rc<RefCell<Vec<Uuid>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            page_access: None,
            directory: None,
            manage_people: false,
            view_rates: false,
            view_history: false,
            people_replies: Default::default(),
            people_queries: Default::default(),
            archives: Default::default(),
            timesheets: Default::default(),
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
            storage: Default::default(),
            storage_failure: Default::default(),
            identity_replies: Default::default(),
            identity_reads: Default::default(),
            subject_replies: Default::default(),
            subject_cursors: Default::default(),
            loaded_ids: Default::default(),
        }
    }

    fn preview_reply(&self) -> oneshot::Sender<Result<ProfilePreview, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.preview_replies.borrow_mut().push_back(receive);
        send
    }

    fn subjects_reply(&self) -> oneshot::Sender<Result<PermissionSubjectPage, ServerFnError>> {
        let (send, receive) = oneshot::channel();
        self.subject_replies.borrow_mut().push_back(receive);
        send
    }

    fn subjects_page(
        &self,
        id: Uuid,
        name: &str,
        next_after: Option<Uuid>,
    ) -> PermissionSubjectPage {
        PermissionSubjectPage {
            requester: self.editor.requester,
            subjects: vec![PermissionSubject {
                id,
                name: name.into(),
                active: false,
            }],
            next_after,
        }
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
    let person = use_signal(|| {
        probe
            .open_person
            .then_some((probe.editor.user_id, probe.editor.requester))
    });
    let timesheets = probe.timesheets.clone();
    let view_timesheet = use_callback(move |view: (Uuid, chrono::NaiveDate)| {
        timesheets.borrow_mut().push(view);
    });
    let view_history = use_callback(|()| {});
    if probe.page_access.is_some() {
        return rsx! { PeoplePageHost {} };
    }
    if let Some(can_edit_permissions) = probe.directory {
        return rsx! { people::PeopleDirectory {
            can_edit_permissions, can_manage_people: probe.manage_people,
            first_day: chrono::Weekday::Mon, today: TODAY,
            on_saved: move |changed| probe.saved.borrow_mut().push(changed),
            on_view_timesheet: view_timesheet,
            on_view_history: probe.view_history.then_some(view_history),
        } };
    }
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
        // Discarding is confirmed in a ConfirmDialog, never the browser's confirm.
        assert!(!script.contains("confirm("), "native dialog: {script}");
        let reply = Err(document::EvalError::Unsupported);
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

/// How a test answers the discard confirmation when it keeps the changes.
#[derive(Clone, Copy)]
enum Keep {
    Button,
    Escape,
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
        let id = self.target(name);
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

    /// Answer the discard confirmation by keeping the changes.
    fn keep_changes(&mut self, keep: Keep) {
        match keep {
            Keep::Button => self.click("permission-discard-dialog-cancel"),
            Keep::Escape => self.dispatch(
                "permission-discard-dialog",
                "cancel",
                Box::new(SerializedCancelData {}),
            ),
        }
        assert!(!self.html().contains("Discard unsaved permission changes?"));
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
        let id = self.target(name);
        let data = SerializedFormData::new(value.into(), vec![]);
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::new(data))) as Rc<dyn Any>,
            true,
        );
        self.dom.runtime().handle_event(event_name, event, id);
        self.settle();
    }

    fn target(&self, name: &str) -> ElementId {
        self.targets
            .get(name)
            .copied()
            .or_else(|| static_target(&self.dom, self.dom.base_scope().root_node(), name))
            .unwrap_or_else(|| panic!("missing {name}: {}", self.html()))
    }
}

// Static IDs live in templates rather than SetAttribute mutations. Resolve the
// real mounted event target without forcing production IDs to become dynamic.
fn static_target(dom: &VirtualDom, vnode: &VNode, name: &str) -> Option<ElementId> {
    use dioxus::core::{DynamicNode, TemplateAttribute, TemplateNode};
    for (index, path) in vnode.template.attr_paths.iter().enumerate() {
        let mut node = &vnode.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = &children[usize::from(*child)];
        }
        if let TemplateNode::Element { attrs, .. } = node
            && attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name: "id", value, .. } if *value == name)) {
            return vnode.mounted_dynamic_attribute(index, dom);
        }
    }
    for (index, node) in vnode.dynamic_nodes.iter().enumerate() {
        let found = match node {
            DynamicNode::Component(component) => component
                .mounted_scope(index, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .and_then(|node| static_target(dom, node, name)),
            DynamicNode::Fragment(nodes) => {
                nodes.iter().find_map(|node| static_target(dom, node, name))
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

#[tokio::test]
async fn subject_picker_empty_and_failed_pages_offer_read_only_recovery() {
    let probe = Probe::new();
    let reply = probe.subjects_reply();
    let mut ui = Ui::new(probe.clone());
    reply
        .send(Err(ServerFnError::new("private database diagnostic")))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Could not load people"));
    assert!(!ui.html().contains("private database diagnostic"));
    let reply = probe.subjects_reply();
    ui.click("permission-subject-retry");
    reply
        .send(Ok(PermissionSubjectPage {
            requester: probe.editor.requester,
            subjects: vec![],
            next_after: None,
        }))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("No people on this page."));
    assert!(probe.saves.borrow().is_empty());
    assert_eq!(*probe.subject_cursors.borrow(), vec![None, None]);
}

#[tokio::test]
async fn subject_picker_pages_hide_stale_choices_and_keep_duplicate_names_by_id() {
    let probe = Probe::new();
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let reply = probe.subjects_reply();
    let mut ui = Ui::new(probe.clone());
    assert!(ui.html().contains("Loading people"));
    reply
        .send(Ok(probe.subjects_page(first, "Same name", Some(first))))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains("Same name (inactive)"));
    let reply = probe.subjects_reply();
    ui.click("permission-subject-next");
    assert!(!ui.html().contains(&format!("permission-subject-{first}")));
    reply
        .send(Ok(probe.subjects_page(second, "Same name", None)))
        .unwrap();
    ui.settle();
    assert!(ui.html().contains(&format!("permission-subject-{second}")));
    assert!(!ui.html().contains("id=\"permission-subject-next\""));
    let reply = probe.subjects_reply();
    ui.click("permission-subject-previous");
    reply
        .send(Ok(probe.subjects_page(first, "Same name", Some(first))))
        .unwrap();
    ui.settle();
    assert_eq!(
        *probe.subject_cursors.borrow(),
        vec![None, Some(first), None]
    );
    assert!(probe.saves.borrow().is_empty());
}

#[tokio::test]
async fn subject_picker_preserves_dirty_draft_until_confirmed_and_reloads_selected_person() {
    let probe = Probe::new();
    let other = Uuid::now_v7();
    probe
        .subjects_reply()
        .send(Ok(probe.subjects_page(other, "Other person", None)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    ui.select_profile("Member");
    for keep in [Keep::Button, Keep::Escape] {
        ui.click(&format!("permission-subject-{other}"));
        assert!(ui.html().contains("Discard unsaved permission changes?"));
        ui.keep_changes(keep);
        ui.navigation_state("dirty");
        assert_eq!(*probe.loaded_ids.borrow(), vec![probe.editor.user_id]);
    }
    let reply = probe.load_reply();
    ui.click(&format!("permission-subject-{other}"));
    ui.click("permission-discard-dialog-confirm");
    ui.navigation_state("pending");
    let mut selected = probe.editor.clone();
    selected.user_id = other;
    selected.name = "Other person".into();
    reply.send(Ok(selected)).unwrap();
    ui.settle();
    ui.navigation_state("clean");
    assert_eq!(
        *probe.loaded_ids.borrow(),
        vec![probe.editor.user_id, other]
    );
    assert!(ui.html().contains("Other person"));
    assert!(probe.saves.borrow().is_empty());
}

#[tokio::test]
async fn subject_picker_rejects_a_changed_requester_on_next_page_and_selected_load() {
    for changed_on_page in [true, false] {
        let probe = Probe::new();
        let other = Uuid::now_v7();
        let mut foreign = probe.editor.requester;
        foreign.user_id = Uuid::now_v7();
        probe
            .subjects_reply()
            .send(Ok(probe.subjects_page(other, "Local choice", Some(other))))
            .unwrap();
        let mut ui = Ui::new(probe.clone());
        if changed_on_page {
            let reply = probe.subjects_reply();
            ui.click("permission-subject-next");
            let mut page = probe.subjects_page(Uuid::now_v7(), "Must not appear", None);
            page.requester = foreign;
            reply.send(Ok(page)).unwrap();
        } else {
            let reply = probe.load_reply();
            ui.click(&format!("permission-subject-{other}"));
            let mut selected = probe.editor.clone();
            selected.requester = foreign;
            selected.user_id = other;
            selected.name = "Must not appear".into();
            reply.send(Ok(selected)).unwrap();
        }
        ui.settle();
        assert!(!ui.html().contains("Must not appear"));
        assert!(probe.saves.borrow().is_empty());
    }
}

#[tokio::test]
async fn reload_cannot_adopt_another_requester_after_a_rejected_subject_load() {
    for change_org in [false, true] {
        let probe = Probe::new();
        let other = Uuid::now_v7();
        probe
            .subjects_reply()
            .send(Ok(probe.subjects_page(other, "Local choice", None)))
            .unwrap();
        let mut ui = Ui::new(probe.clone());
        let mut selected = probe.editor.clone();
        selected.user_id = other;
        selected.name = "Selected local person".into();
        let mut foreign = selected.clone();
        foreign.name = "Must not appear".into();
        if change_org {
            foreign.requester.org_id = Uuid::now_v7();
        } else {
            foreign.requester.user_id = Uuid::now_v7();
        }

        let reply = probe.load_reply();
        ui.click(&format!("permission-subject-{other}"));
        reply.send(Ok(foreign.clone())).unwrap();
        ui.settle();
        assert!(ui.html().contains("Permission editing is unavailable"));

        let reply = probe.load_reply();
        ui.click("permission-editor-reload");
        ui.navigation_state("pending");
        reply.send(Ok(foreign)).unwrap();
        ui.settle();
        assert!(!ui.html().contains("Must not appear"), "{}", ui.html());
        assert!(ui.html().contains("Permission editing is unavailable"));

        let reply = probe.load_reply();
        ui.click("permission-editor-reload");
        reply.send(Ok(selected)).unwrap();
        ui.settle();
        assert!(ui.html().contains("Selected local person"));
        assert!(probe.saves.borrow().is_empty());
        assert!(probe.templates.borrow().is_empty());
        assert!(probe.storage.borrow().is_empty());
    }
}

#[tokio::test]
async fn initial_editor_selection_pins_requester_across_reload() {
    for change_org in [false, true] {
        let probe = Probe::new();
        let mut invalid = probe.editor.clone();
        invalid.permissions.grants.clear();
        probe.load_reply().send(Ok(invalid)).unwrap();
        let mut ui = Ui::new(probe.clone());
        assert!(!ui.html().contains("Review changes"));
        let mut foreign = probe.editor.clone();
        foreign.name = "Must not appear".into();
        if change_org {
            foreign.requester.org_id = Uuid::now_v7();
        } else {
            foreign.requester.user_id = Uuid::now_v7();
        }
        let reply = probe.load_reply();
        ui.click("permission-editor-reload");
        reply.send(Ok(foreign)).unwrap();
        ui.settle();
        assert!(!ui.html().contains("Must not appear"), "{}", ui.html());
        assert!(ui.html().contains("Permission editing is unavailable"));
        let reply = probe.load_reply();
        ui.click("permission-editor-reload");
        reply.send(Ok(probe.editor.clone())).unwrap();
        ui.settle();
        assert!(ui.html().contains("Example person"));
        assert!(probe.saves.borrow().is_empty());
    }
}

#[tokio::test]
async fn first_editor_response_cannot_replace_the_directory_requester() {
    for change_org in [false, true] {
        let probe = Probe::new();
        let mut foreign = probe.editor.clone();
        foreign.name = "Foreign response".into();
        if change_org {
            foreign.requester.org_id = Uuid::now_v7();
        } else {
            foreign.requester.user_id = Uuid::now_v7();
        }
        probe.load_reply().send(Ok(foreign)).unwrap();
        let ui = Ui::new(probe.clone());
        assert!(ui.html().contains("Permission editing is unavailable"));
        assert!(!ui.html().contains("Foreign response"));
        assert!(!ui.html().contains("Review changes"));
        assert!(probe.saves.borrow().is_empty());
    }
}

fn directory_reply(
    probe: &Probe,
) -> oneshot::Sender<Result<people_model::PeoplePage, ServerFnError>> {
    let (send, receive) = oneshot::channel();
    probe.people_replies.borrow_mut().push_back(receive);
    send
}

fn directory_page(probe: &Probe) -> people_model::PeoplePage {
    people_model::PeoplePage {
        requester: probe.editor.requester,
        people: vec![people_model::PersonSummary {
            id: probe.editor.user_id,
            name: "Directory person".into(),
            email: "person@example.test".into(),
            active: true,
            profile: Some(people_model::PersonProfile::BuiltIn(
                horae_core::permissions::catalog::BuiltInProfile::ProjectManager,
            )),
            week: Some(people_model::WeekMinutes {
                total: 120,
                billable: 90,
            }),
            can_change_active: probe.manage_people,
            can_read_rates: probe.view_rates,
        }],
    }
}

/// The People page under a router at `/people`, with the probe's permissions
/// as the session's.
#[component]
fn PeoplePageHost() -> Element {
    let probe = use_context::<Probe>();
    let permissions = use_resource(move || {
        let answer = probe.page_access.clone().expect("page access");
        async move { answer }
    });
    use_context_provider(|| session::Session { permissions });
    rsx! {
        dioxus::router::components::HistoryProvider {
            history: move |_| {
                std::rc::Rc::new(dioxus::history::MemoryHistory::with_initial_path("/people"))
                    as std::rc::Rc<dyn dioxus::history::History>
            },
            Router::<route::Route> {}
        }
    }
}

fn own_permissions(
    administrator: bool,
    grants: &[Permission],
) -> horae_types::own_permissions::OwnPermissions {
    horae_types::own_permissions::OwnPermissions {
        catalog_version: horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION,
        grants: grants.to_vec(),
        is_administrator: administrator,
        profile: None,
        access_revision: 1,
        person_revision: 1,
        managed_person_ids: vec![],
        managed_project_ids: vec![],
    }
}

/// Without a People grant the page shows the access notice and the recovery
/// dialog, and never mounts the directory or asks for it.
#[tokio::test]
async fn people_without_a_people_grant_never_mount_the_directory() {
    for access in [
        own_permissions(false, &[]),
        own_permissions(false, &[Permission::TaskReadAll]),
        // The Administrator identity is not a People grant.
        own_permissions(true, &[]),
    ] {
        let mut probe = Probe::new();
        probe.open_person = false;
        probe.page_access = Some(Ok(access));
        let ui = Ui::new(probe.clone());
        let html = ui.html();
        assert!(html.contains("Access unavailable"), "{html}");
        assert!(html.contains("Ask an Administrator"), "{html}");
        assert!(html.contains("id=\"permission-recovery\""), "{html}");
        assert!(!html.contains("people-week"), "{html}");
        assert!(probe.people_queries.borrow().is_empty());
    }
}

/// Either People read grant opens the directory.
#[tokio::test]
async fn people_opens_with_either_people_read_grant() {
    for (administrator, grant) in [
        (true, Permission::PeopleReadAll),
        (false, Permission::PeopleReadManaged),
    ] {
        let mut probe = Probe::new();
        probe.open_person = false;
        probe.page_access = Some(Ok(own_permissions(administrator, &[grant])));
        let _pending = directory_reply(&probe);
        let ui = Ui::new(probe.clone());
        let html = ui.html();
        assert!(html.contains("id=\"people-week"), "{html}");
        assert!(!html.contains("permission-recovery"), "{html}");
        assert_eq!(probe.people_queries.borrow().len(), 1);
    }
}

/// A failed permission read, or permissions from another catalog version,
/// keep only the notice and recovery.
#[tokio::test]
async fn failed_or_future_people_projection_keeps_only_recovery() {
    let mut future = own_permissions(true, &[Permission::PeopleReadAll]);
    future.catalog_version += 1;
    for (access, notice) in [
        (
            Err(ServerFnError::new("private projection error")),
            "Could not verify your access",
        ),
        (Ok(future), "Access unavailable"),
    ] {
        let mut probe = Probe::new();
        probe.open_person = false;
        probe.page_access = Some(access);
        let ui = Ui::new(probe.clone());
        let html = ui.html();
        assert!(html.contains(notice), "{html}");
        assert!(html.contains("id=\"permission-recovery\""), "{html}");
        assert!(!html.contains("private projection error"), "{html}");
        assert!(!html.contains("people-week"), "{html}");
        assert!(probe.people_queries.borrow().is_empty());
    }
}

/// The ids of the row's menu items, in order.
fn row_actions(html: &str, person: Uuid) -> Vec<&'static str> {
    ["timesheet", "permissions", "rates", "history", "active"]
        .into_iter()
        .filter(|item| html.contains(&format!("id=\"person-{item}-{person}\"")))
        .collect()
}

#[tokio::test]
async fn directory_filters_loaded_people_without_asking_again() {
    let mut probe = Probe::new();
    probe.directory = Some(false);
    let first = directory_reply(&probe);
    let mut ui = Ui::new(probe.clone());
    assert!(ui.html().contains("Loading people"));
    let mut page = directory_page(&probe);
    let mut archived = page.people[0].clone();
    archived.id = Uuid::now_v7();
    archived.name = "Archived person".into();
    archived.active = false;
    archived.profile = Some(people_model::PersonProfile::Individual);
    archived.week = Some(people_model::WeekMinutes {
        total: 600,
        billable: 0,
    });
    page.people.push(archived);
    first.send(Ok(page)).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Directory person"));
    assert!(!html.contains("Archived person"));
    assert!(html.contains("Active (1)"));
    assert!(html.contains("2:00"), "the week's hours: {html}");
    assert!(html.contains("75%"), "the billable share: {html}");
    // The summary counts only the people the status filter shows.
    assert!(
        !html.contains("12:00"),
        "an archived person's hours: {html}"
    );
    // Profiles are labelled from the catalog, in the badge and the filter.
    assert!(html.contains("Project Manager"), "{html}");
    assert!(html.contains("Individual"), "the profile filter: {html}");

    ui.click("people-status-archived");
    assert!(ui.html().contains("Archived person"));
    assert!(!ui.html().contains("Directory person"));
    ui.form_event("people-search", "input", "nobody");
    assert!(ui.html().contains("No one matches “nobody”"));
    ui.click("people-status-all");
    ui.form_event("people-search", "input", "directory");
    assert!(ui.html().contains("Directory person"));
    assert!(!ui.html().contains("Archived person"));

    assert_eq!(*probe.people_queries.borrow(), vec![WEEK]);
    assert_eq!(*probe.loads.borrow(), 0);
}

#[tokio::test]
async fn changing_the_week_keeps_the_rows_dimmed_until_its_hours_arrive() {
    let mut probe = Probe::new();
    probe.directory = Some(false);
    directory_reply(&probe)
        .send(Ok(directory_page(&probe)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    assert!(ui.html().contains("Directory person"));
    assert!(!ui.html().contains("aria-busy=true"));

    let next = directory_reply(&probe);
    ui.click("people-week-next");
    let html = ui.html();
    assert!(html.contains("Directory person"), "{html}");
    assert!(
        html.contains("class=\"ppl-content\" aria-busy=true"),
        "{html}"
    );
    assert!(html.contains("14 Sep – 20 Sep 2026"), "{html}");
    let mut later = directory_page(&probe);
    later.people[0].week = Some(people_model::WeekMinutes {
        total: 0,
        billable: 0,
    });
    next.send(Ok(later)).unwrap();
    ui.settle();
    assert!(!ui.html().contains("aria-busy=true"));

    // A failed week shows the error, not the previous week's rows.
    let failed = directory_reply(&probe);
    ui.click("people-week-prev");
    failed
        .send(Err(ServerFnError::new("Server unavailable")))
        .unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Could not load people"), "{html}");
    assert!(html.contains("Try again"), "{html}");
    assert!(!html.contains("Directory person"));

    let next_week = WEEK + chrono::Duration::days(7);
    assert_eq!(*probe.people_queries.borrow(), vec![WEEK, next_week, WEEK]);
}

/// The row's menu offers only what the viewer may do on that row.
#[tokio::test]
async fn the_row_menu_follows_the_viewers_permissions() {
    // (administrator, rates, history, manage, hours, active, expected)
    for (administrator, rates, history, manage, hours, active, expected) in [
        (false, false, false, false, true, true, vec!["timesheet"]),
        (false, false, false, false, false, true, vec![]),
        (
            false,
            true,
            false,
            false,
            true,
            true,
            vec!["timesheet", "rates"],
        ),
        (false, false, false, true, false, true, vec!["active"]),
        // The Timesheet refuses an archived person, so the row does not offer it.
        (false, false, false, false, true, false, vec![]),
        (false, false, false, true, true, false, vec!["active"]),
        (
            true,
            true,
            true,
            true,
            true,
            true,
            vec!["timesheet", "permissions", "rates", "history", "active"],
        ),
    ] {
        let mut probe = Probe::new();
        probe.directory = Some(administrator);
        probe.view_rates = rates;
        probe.view_history = history;
        probe.manage_people = manage;
        let mut page = directory_page(&probe);
        if !hours {
            page.people[0].week = None;
        }
        page.people[0].active = active;
        directory_reply(&probe).send(Ok(page)).unwrap();
        let mut ui = Ui::new(probe.clone());
        if !active {
            ui.click("people-status-archived");
        }
        let html = ui.html();
        assert_eq!(row_actions(&html, probe.editor.user_id), expected, "{html}");
        let menu = format!("id=\"person-actions-{}\"", probe.editor.user_id);
        assert_eq!(html.contains(&menu), !expected.is_empty());
        if !expected.is_empty() {
            // Each row's trigger is named after its person.
            assert!(
                html.contains("aria-label=\"Actions for Directory person\""),
                "{html}"
            );
        }
        if !hours {
            assert!(html.contains("Hours not available"));
        }
    }
}

#[tokio::test]
async fn view_timesheet_opens_the_persons_shown_week() {
    let mut probe = Probe::new();
    probe.directory = Some(false);
    directory_reply(&probe)
        .send(Ok(directory_page(&probe)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    ui.click(&format!("person-timesheet-{}", probe.editor.user_id));
    assert_eq!(
        *probe.timesheets.borrow(),
        vec![(probe.editor.user_id, WEEK)]
    );
}

#[tokio::test]
async fn archiving_applies_at_once_and_offers_undo() {
    let mut probe = Probe::new();
    probe.directory = Some(false);
    probe.manage_people = true;
    directory_reply(&probe)
        .send(Ok(directory_page(&probe)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    let reload = directory_reply(&probe);
    ui.click(&format!("person-active-{}", probe.editor.user_id));
    assert_eq!(
        *probe.archives.borrow(),
        vec![(probe.editor.user_id, false)]
    );
    let mut archived = directory_page(&probe);
    archived.people[0].active = false;
    reload.send(Ok(archived)).unwrap();
    ui.settle();
    let html = ui.html();
    assert!(html.contains("Person archived"));
    assert!(html.contains("Undo"));
    assert!(!html.contains("ConfirmDialog") && !html.contains("Archive Directory person?"));
}

/// Adding people shows only with permission to manage people; the profile
/// choice shows only to administrators, and never offers Administrator.
#[tokio::test]
async fn add_person_is_in_the_header_by_grant() {
    for (administrator, manage_people) in [(false, false), (false, true), (true, true)] {
        let mut probe = Probe::new();
        probe.directory = Some(administrator);
        probe.manage_people = manage_people;
        directory_reply(&probe)
            .send(Ok(directory_page(&probe)))
            .unwrap();
        let mut ui = Ui::new(probe.clone());
        let html = ui.html();
        assert_eq!(html.contains("id=\"people-add\""), manage_people);
        if manage_people {
            ui.click("people-add");
            let html = ui.html();
            assert!(html.contains("people-add-email"));
            assert_eq!(html.contains("people-add-profile-member"), administrator);
            assert!(!html.contains("people-add-profile-administrator"));
        }
    }
}

#[tokio::test]
async fn directory_keeps_requester_on_week_changes_and_initial_editor_load() {
    for change_org in [false, true] {
        for action in ["editor", "week"] {
            let mut probe = Probe::new();
            probe.directory = Some(true);
            directory_reply(&probe)
                .send(Ok(directory_page(&probe)))
                .unwrap();
            let mut ui = Ui::new(probe.clone());
            let mut foreign = probe.editor.requester;
            if change_org {
                foreign.org_id = Uuid::now_v7();
            } else {
                foreign.user_id = Uuid::now_v7();
            }
            if action == "editor" {
                let reply = probe.load_reply();
                ui.click(&format!("person-permissions-{}", probe.editor.user_id));
                let mut loaded = probe.editor.clone();
                loaded.requester = foreign;
                loaded.name = "Foreign response".into();
                reply.send(Ok(loaded)).unwrap();
            } else {
                let reply = directory_reply(&probe);
                ui.click("people-week-next");
                let mut loaded = directory_page(&probe);
                loaded.requester = foreign;
                loaded.people[0].name = "Foreign response".into();
                reply.send(Ok(loaded)).unwrap();
            }
            ui.settle();
            assert!(!ui.html().contains("Foreign response"));
            assert!(!ui.html().contains("Review changes"));
            assert!(probe.saves.borrow().is_empty());
        }
    }
}

#[tokio::test]
async fn subject_picker_cannot_switch_during_preview_or_uncertain_save() {
    let probe = Probe::new();
    let other = Uuid::now_v7();
    probe
        .subjects_reply()
        .send(Ok(probe.subjects_page(other, "Other person", None)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    let preview = probe.preview_reply();
    ui.click("permission-review");
    ui.click(&format!("permission-subject-{other}"));
    assert_eq!(*probe.loads.borrow(), 1);
    preview.send(Ok(probe.effects())).unwrap();
    ui.settle();
    let save = probe.save_reply();
    ui.click("permission-save");
    save.send(Err(ServerFnError::new("connection lost")))
        .unwrap();
    ui.settle();
    ui.click(&format!("permission-subject-{other}"));
    assert_eq!(*probe.loads.borrow(), 1);
    assert_eq!(probe.saves.borrow().len(), 1);
    ui.navigation_state("pending");
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
    // The open form's footer has one Cancel; it dismisses as Escape and the backdrop do.
    for path in ["permission-editor-cancel", "escape", "backdrop"] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        ui.select_profile("Member");
        for keep in [Keep::Button, Keep::Escape] {
            ui.dismiss(path);
            assert!(ui.html().contains("Discard unsaved permission changes?"));
            ui.keep_changes(keep);
            ui.navigation_state("dirty");
            assert!(ui.html().contains("Example person"));
        }
        ui.dismiss(path);
        ui.click("permission-discard-dialog-confirm");
        ui.navigation_state("clean");
        assert!(!ui.html().contains("Example person"));
        assert!(probe.saves.borrow().is_empty());
        assert!(probe.templates.borrow().is_empty());
    }
}

#[tokio::test]
async fn clean_dismissal_needs_no_confirmation() {
    for path in ["permission-editor-cancel", "escape", "backdrop"] {
        let probe = Probe::new();
        let mut ui = Ui::new(probe.clone());
        ui.dismiss(path);
        ui.navigation_state("clean");
        assert!(!ui.html().contains("Example person"));
        assert!(probe.saves.borrow().is_empty());
    }
}

#[tokio::test]
async fn closing_a_person_does_not_recheck_the_session() {
    let probe = Probe::new();
    let mut ui = Ui::new(probe.clone());
    assert!(ui.html().contains("Review changes"));
    ui.dismiss("permission-editor-cancel");
    ui.navigation_state("clean");
    assert_eq!(*probe.identity_reads.borrow(), 1);
}

#[tokio::test]
async fn opening_a_person_rechecks_the_session() {
    let mut probe = Probe::new();
    probe.directory = Some(true);
    directory_reply(&probe)
        .send(Ok(directory_page(&probe)))
        .unwrap();
    let mut ui = Ui::new(probe.clone());
    assert_eq!(*probe.identity_reads.borrow(), 1);
    for reads in [2, 3] {
        ui.click(&format!("person-permissions-{}", probe.editor.user_id));
        assert!(ui.html().contains("Review changes"));
        assert_eq!(*probe.identity_reads.borrow(), reads);
        ui.dismiss("permission-editor-cancel");
        assert!(!ui.html().contains("Review changes"));
        assert_eq!(*probe.identity_reads.borrow(), reads);
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
    ui.click(DELETE_CANCEL);
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
    let reply = probe.template_reply();
    ui.click(DELETE_CONFIRM);
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
    assert!(!ui.html().contains(">Delete profile<"));
    assert!(probe.saved.borrow().is_empty());
}

/// The buttons of the profile deletion's ConfirmDialog.
const DELETE_CONFIRM: &str = "permission-template-delete-dialog-confirm";
const DELETE_CANCEL: &str = "permission-template-delete-dialog-cancel";

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
        assert!(!html.contains(">Delete profile<"), "{html}");
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
    assert!(probe.templates.borrow().is_empty());
    let reply = probe.template_reply();
    ui.click(DELETE_CONFIRM);
    reply
        .send(Err(ServerFnError::new("response lost")))
        .unwrap();
    ui.settle();
    // An unconfirmed request cannot be abandoned, only retried exactly.
    ui.click(DELETE_CANCEL);
    assert!(ui.html().contains("Retry same save"), "{}", ui.html());
    let reply = probe.template_reply();
    ui.click(DELETE_CONFIRM);
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
    ui.click(DELETE_CANCEL);
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
    assert!(ui.html().contains("Try again"));
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

    pub async fn load_person_rates(
        _person_id: Uuid,
        _expected_requester: PermissionRequester,
    ) -> Result<models::user::PersonRates, ServerFnError> {
        unreachable!("the directory tests never open the rates dialog")
    }

    pub async fn update_person_rates(
        _person_id: Uuid,
        _expected_requester: PermissionRequester,
        _currency: String,
        _billable: horae_core::permissions::rates::RateEdit,
        _cost: horae_core::permissions::rates::RateEdit,
    ) -> Result<models::user::PersonRates, ServerFnError> {
        unreachable!("the directory tests never open the rates dialog")
    }

    pub async fn create_user(
        _email: String,
        _name: String,
        _profile: String,
    ) -> Result<models::user::UserListItem, ServerFnError> {
        unreachable!("the directory tests never add people")
    }

    pub async fn set_user_active(
        user_id: String,
        active: bool,
    ) -> Result<models::user::UserListItem, ServerFnError> {
        let probe = use_context::<Probe>();
        let id = user_id.parse().unwrap();
        probe.archives.borrow_mut().push((id, active));
        Ok(models::user::UserListItem {
            id,
            email: "person@example.test".into(),
            name: "Directory person".into(),
            active,
        })
    }

    pub async fn list_people(
        week: chrono::NaiveDate,
    ) -> Result<people_model::PeoplePage, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.people_queries.borrow_mut().push(week);
        let reply = probe
            .people_replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected directory request");
        reply.await.expect("directory response dropped")
    }

    pub struct CurrentUser {
        pub id: Uuid,
        pub org_id: Uuid,
    }
    pub async fn get_me() -> Result<CurrentUser, ServerFnError> {
        let probe = use_context::<Probe>();
        *probe.identity_reads.borrow_mut() += 1;
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
        probe.loaded_ids.borrow_mut().push(id);
        let reply = probe.load_replies.borrow_mut().pop_front();
        match reply {
            Some(reply) => reply.await.expect("load reply dropped"),
            None => {
                assert_eq!(id, probe.editor.user_id);
                Ok(probe.editor)
            }
        }
    }

    pub async fn list_permission_subjects(
        after: Option<Uuid>,
    ) -> Result<PermissionSubjectPage, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.subject_cursors.borrow_mut().push(after);
        let reply = probe.subject_replies.borrow_mut().pop_front();
        match reply {
            Some(reply) => reply.await.expect("subjects reply dropped"),
            None => Ok(probe.subjects_page(probe.editor.user_id, &probe.editor.name, None)),
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
