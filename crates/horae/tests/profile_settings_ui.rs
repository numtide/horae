#![cfg(feature = "server")]

//! Render the production Profile card and workspace time zone field with
//! controlled server responses. No database or HTTP server is needed.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::core::{AttributeValue, DynamicNode, ElementId, TemplateAttribute, TemplateNode};
use dioxus::prelude::*;
use dioxus_html::{SerializedFormData, SerializedHtmlEventConverter, SerializedMouseData};
use futures_util::FutureExt;
use horae_core::types::OrgRole;

#[allow(dead_code)]
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/select_field.rs"]
pub mod select_field;
#[allow(dead_code)]
#[path = "../src/components/session.rs"]
pub mod session;
mod components {
    pub use super::{form, select_field, session};
}
#[allow(dead_code)]
#[path = "../src/models/own_permissions.rs"]
pub mod own_permissions_model;
#[allow(dead_code)]
#[path = "../src/models/user.rs"]
pub mod user_model;
mod models {
    pub use super::{own_permissions_model as own_permissions, user_model as user};
}
#[path = "../src/pages/settings/profile.rs"]
mod profile;
#[path = "../src/pages/admin/workspace.rs"]
mod workspace;

use own_permissions_model::OwnPermissions;
use user_model::{CurrentUser, ProfileOptions};

/// Each profile save's name and personal zone, in order.
type ProfileSaves = Vec<(String, Option<String>)>;

#[derive(Clone)]
struct Probe {
    me: Rc<RefCell<CurrentUser>>,
    me_reads: Rc<Cell<usize>>,
    /// `None` is policy zero (no detailed permissions).
    permissions: Option<OwnPermissions>,
    sign_in: Option<String>,
    profile_saves: Rc<RefCell<ProfileSaves>>,
    workspace_saves: Rc<RefCell<Vec<String>>>,
    /// The next save's refusal, if any.
    refusal: Rc<RefCell<Option<ServerFnError>>>,
}

impl Probe {
    fn new() -> Self {
        Self {
            me: Rc::new(RefCell::new(CurrentUser {
                id: uuid::Uuid::nil(),
                org_id: uuid::Uuid::nil(),
                email: "mia@example.test".into(),
                name: "Mia Member".into(),
                org_role: OrgRole::Member,
                timezone: None,
                workspace_timezone: "Europe/Madrid".into(),
                utc_offset_seconds: 3600,
            })),
            me_reads: Rc::default(),
            permissions: None,
            sign_in: Some("sso.example.test".into()),
            profile_saves: Rc::default(),
            workspace_saves: Rc::default(),
            refusal: Rc::default(),
        }
    }

    fn refuse_next(&self, code: u16, message: &str) {
        *self.refusal.borrow_mut() = Some(ServerFnError::ServerError {
            message: message.into(),
            code,
            details: None,
        });
    }
}

fn profile_app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    session::use_session_provider();
    rsx! { profile::ProfileSection {} }
}

fn workspace_app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    session::use_session_provider();
    rsx! { workspace::WorkspaceSettings {} }
}

fn mount(app: fn(Probe) -> Element, probe: &Probe) -> VirtualDom {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

fn settle(dom: &mut VirtualDom) {
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("settings did not settle");
}

/// Which of a template's dynamic attributes marks the element to act on.
type Hit<'a> = &'a dyn Fn(&VNode) -> Option<usize>;

/// The first mounted element, depth first, that `hit` picks out.
fn find(dom: &VirtualDom, vnode: &VNode, hit: Hit) -> Option<ElementId> {
    if let Some(index) = hit(vnode) {
        return vnode.mounted_dynamic_attribute(index, dom);
    }
    vnode
        .dynamic_nodes
        .iter()
        .enumerate()
        .find_map(|(index, node)| match node {
            DynamicNode::Component(component) => component
                .mounted_scope(index, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .and_then(|node| find(dom, node, hit)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|node| find(dom, node, hit)),
            _ => None,
        })
}

/// The element whose `id` is `id`, written statically or computed (an element
/// with only a static id must also carry some dynamic attribute).
fn by_id(vnode: &VNode, id: &str) -> Option<usize> {
    let template = &vnode.template;
    template.attr_paths.iter().enumerate().position(|(index, path)| {
        let dynamic = vnode.dynamic_attrs[index]
            .iter()
            .any(|a| a.name == "id" && matches!(&a.value, AttributeValue::Text(v) if v == id));
        let mut node = &template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                return dynamic;
            };
            node = &children[usize::from(*child)];
        }
        dynamic
            || matches!(node, TemplateNode::Element { attrs, .. }
                if attrs.iter().any(|a| matches!(a, TemplateAttribute::Static { name: "id", value, .. } if *value == id)))
    })
}

/// A `SelectField` option's button (its template root), found by the label's title.
fn option_button(vnode: &VNode, label: &str) -> Option<usize> {
    let labelled = vnode
        .dynamic_attrs
        .iter()
        .flat_map(|attrs| attrs.iter())
        .any(|a| a.name == "title" && matches!(&a.value, AttributeValue::Text(v) if v == label));
    labelled
        .then(|| {
            vnode
                .template
                .attr_paths
                .iter()
                .position(|path| path.len() == 1)
        })
        .flatten()
}

fn fire(dom: &mut VirtualDom, hit: Hit, kind: &str, data: Box<dyn std::any::Any>) {
    let id = find(dom, dom.base_scope().root_node(), hit).expect("event target rendered");
    let event = Event::new(
        Rc::new(PlatformEventData::new(data)) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event(kind, event, id);
    settle(dom);
}

fn click(dom: &mut VirtualDom, id: &str) {
    let mouse = Box::<SerializedMouseData>::default();
    fire(dom, &|vnode| by_id(vnode, id), "click", mouse);
}

fn type_into(dom: &mut VirtualDom, id: &str, text: &str) {
    let data = Box::new(SerializedFormData::new(text.into(), vec![]));
    fire(dom, &|vnode| by_id(vnode, id), "input", data);
}

/// Pick a `SelectField` option by its label.
fn pick(dom: &mut VirtualDom, label: &str) {
    let mouse = Box::<SerializedMouseData>::default();
    fire(dom, &|vnode| option_button(vnode, label), "click", mouse);
}

fn html(dom: &VirtualDom) -> String {
    dioxus::ssr::render(dom)
}

/// Whether the rendered `<button id=…>` is disabled.
fn disabled(html: &str, id: &str) -> bool {
    let start = html.find(&format!("id=\"{id}\"")).expect("button rendered");
    html[start..start + html[start..].find('>').unwrap()].contains("disabled=true")
}

/// Whether an input shows `value` and cannot be edited.
fn read_only(html: &str, value: &str) -> bool {
    html.contains(&format!("value=\"{value}\" placeholder=\"\" readonly=true"))
}

#[tokio::test]
async fn profile_shows_identity_read_only_and_the_role_at_policy_zero() {
    let probe = Probe::new();
    let dom = mount(profile_app, &probe);
    let html = html(&dom);
    assert!(
        html.contains("value=\"Mia Member\" placeholder=\"\"/>"),
        "{html}"
    );
    for value in ["mia@example.test", "sso.example.test", "Member"] {
        assert!(read_only(&html, value), "{value} must be read-only: {html}");
    }
    assert!(
        html.contains("Only an Administrator can change your role."),
        "{html}"
    );
    assert!(html.contains("Workspace default (Europe/Madrid)"), "{html}");
    assert!(disabled(&html, "profile-save"), "{html}");
}

#[tokio::test]
async fn detailed_permissions_take_over_from_the_role_row() {
    let mut probe = Probe::new();
    probe.permissions = Some(OwnPermissions {
        catalog_version: 1,
        grants: Vec::new(),
        is_administrator: false,
        access_revision: 1,
        person_revision: 1,
        managed_person_ids: Vec::new(),
        managed_project_ids: Vec::new(),
    });
    probe.sign_in = None;
    let html = html(&mount(profile_app, &probe));
    assert!(!html.contains("profile-role"), "{html}");
    assert!(read_only(&html, "Development sign-in"), "{html}");
}

#[tokio::test]
async fn saving_sends_the_name_and_zone_and_rereads_the_session() {
    let probe = Probe::new();
    let mut dom = mount(profile_app, &probe);
    type_into(&mut dom, "profile-name", "Ana Ruiz");
    pick(&mut dom, "Asia/Tokyo");
    assert!(!disabled(&html(&dom), "profile-save"));
    click(&mut dom, "profile-save");
    assert_eq!(
        *probe.profile_saves.borrow(),
        [("Ana Ruiz".to_owned(), Some("Asia/Tokyo".to_owned()))]
    );
    assert_eq!(probe.me_reads.get(), 2, "the session must be re-read");
    let html = html(&dom);
    assert!(html.contains("Profile saved."), "{html}");
    assert!(disabled(&html, "profile-save"), "{html}");
}

#[tokio::test]
async fn the_workspace_default_is_saved_as_no_zone_of_ones_own() {
    let probe = Probe::new();
    probe.me.borrow_mut().timezone = Some("Asia/Tokyo".into());
    let mut dom = mount(profile_app, &probe);
    pick(&mut dom, "Workspace default (Europe/Madrid)");
    click(&mut dom, "profile-save");
    assert_eq!(
        *probe.profile_saves.borrow(),
        [("Mia Member".to_owned(), None)]
    );
}

#[tokio::test]
async fn a_refused_save_keeps_what_was_typed_and_says_why() {
    let probe = Probe::new();
    probe.refuse_next(400, "Enter a name of 1 to 200 characters");
    let mut dom = mount(profile_app, &probe);
    type_into(&mut dom, "profile-name", "   ");
    click(&mut dom, "profile-save");
    let html = html(&dom);
    assert!(
        html.contains("Enter a name of 1 to 200 characters"),
        "{html}"
    );
    assert!(html.contains("value=\"   \""), "{html}");
    assert_eq!(probe.me_reads.get(), 1, "nothing changed, so no re-read");
}

#[tokio::test]
async fn cancel_restores_the_saved_values() {
    let probe = Probe::new();
    let mut dom = mount(profile_app, &probe);
    type_into(&mut dom, "profile-name", "Someone Else");
    click(&mut dom, "profile-cancel");
    let html = html(&dom);
    assert!(html.contains("value=\"Mia Member\""), "{html}");
    assert!(probe.profile_saves.borrow().is_empty());
}

#[tokio::test]
async fn an_administrator_saves_the_workspace_zone() {
    let probe = Probe::new();
    probe.me.borrow_mut().org_role = OrgRole::Admin;
    let mut dom = mount(workspace_app, &probe);
    assert!(disabled(&html(&dom), "workspace-timezone-save"));
    pick(&mut dom, "Asia/Tokyo");
    click(&mut dom, "workspace-timezone-save");
    assert_eq!(*probe.workspace_saves.borrow(), ["Asia/Tokyo"]);
    assert_eq!(probe.me_reads.get(), 2, "the session must be re-read");
    let html = html(&dom);
    assert!(
        html.contains("<span class=\"truncate\">Asia/Tokyo</span>"),
        "{html}"
    );
    assert!(disabled(&html, "workspace-timezone-save"), "{html}");
}

#[tokio::test]
async fn a_refused_workspace_change_is_reported() {
    let probe = Probe::new();
    probe.refuse_next(403, "Admin access required");
    let mut dom = mount(workspace_app, &probe);
    pick(&mut dom, "Asia/Tokyo");
    click(&mut dom, "workspace-timezone-save");
    let html = html(&dom);
    assert!(html.contains("Admin access required"), "{html}");
    assert_eq!(probe.me_reads.get(), 1);
}

mod server_fns {
    use super::*;

    fn refusal(probe: &Probe) -> Result<(), ServerFnError> {
        probe.refusal.borrow_mut().take().map_or(Ok(()), Err)
    }

    pub async fn get_me() -> Result<CurrentUser, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.me_reads.set(probe.me_reads.get() + 1);
        Ok(probe.me.borrow().clone())
    }

    pub async fn get_my_permissions() -> Result<Option<OwnPermissions>, ServerFnError> {
        Ok(use_context::<Probe>().permissions)
    }

    pub async fn get_week_start() -> Result<chrono::Weekday, ServerFnError> {
        panic!("settings do not read the week start")
    }

    pub async fn get_profile_options() -> Result<ProfileOptions, ServerFnError> {
        Ok(ProfileOptions {
            sign_in: use_context::<Probe>().sign_in,
            zones: ["Asia/Tokyo", "Europe/Madrid", "UTC"]
                .map(String::from)
                .to_vec(),
        })
    }

    pub async fn update_own_profile(
        name: String,
        timezone: Option<String>,
    ) -> Result<(), ServerFnError> {
        let probe = use_context::<Probe>();
        probe
            .profile_saves
            .borrow_mut()
            .push((name.clone(), timezone.clone()));
        refusal(&probe)?;
        let mut me = probe.me.borrow_mut();
        me.name = name;
        me.timezone = timezone;
        Ok(())
    }

    pub async fn set_workspace_timezone(timezone: String) -> Result<(), ServerFnError> {
        let probe = use_context::<Probe>();
        probe.workspace_saves.borrow_mut().push(timezone.clone());
        refusal(&probe)?;
        probe.me.borrow_mut().workspace_timezone = timezone;
        Ok(())
    }
}
