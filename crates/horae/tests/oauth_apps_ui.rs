#![cfg(feature = "server")]

//! Render the production apps pages of the administration with controlled
//! server responses: the list with Horae's own app first, registering a
//! device app and a server app (whose secret shows once), the built-in app's
//! limits, and revoking an authorization through its confirmation. No
//! database or HTTP server is needed.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Utc;
use dioxus::core::{AttributeValue, DynamicNode, ElementId, TemplateAttribute, TemplateNode};
use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use dioxus_html::{SerializedFormData, SerializedHtmlEventConverter, SerializedMouseData};
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/back_link.rs"]
pub mod back_link;
#[path = "../src/components/badge.rs"]
pub mod badge;
#[allow(dead_code)]
#[path = "../src/components/button.rs"]
pub mod button;
#[allow(dead_code)]
#[path = "../src/components/card.rs"]
pub mod card;
#[path = "../src/components/confirm_dialog.rs"]
pub mod confirm_dialog;
#[allow(dead_code)]
#[path = "../src/components/controls.rs"]
pub mod controls;
#[allow(dead_code)]
#[path = "../src/components/form.rs"]
pub mod form;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;
#[allow(dead_code)]
#[path = "../src/components/icons.rs"]
pub mod icons;
#[allow(dead_code)]
#[path = "../src/components/menu.rs"]
pub mod menu;
#[allow(dead_code)]
#[path = "../src/components/modal.rs"]
pub mod modal;
#[allow(dead_code)]
#[path = "../src/components/page_header.rs"]
pub mod page_header;
#[path = "../src/components/revealed_secret.rs"]
pub mod revealed_secret;
#[allow(dead_code)]
#[path = "../src/components/screen_state.rs"]
pub mod screen_state;
#[allow(dead_code)]
#[path = "../src/components/table.rs"]
pub mod table;
mod components {
    pub use super::{
        back_link, badge, card, confirm_dialog, controls, form, icons, menu, modal, page_header,
        revealed_secret, screen_state, table,
    };
}
mod models {
    pub use horae_types::oauth_apps;
}
#[path = "../src/pages/admin/apps.rs"]
mod apps;

use models::oauth_apps::{AppInput, AppKind, AppRow, CreatedApp, GrantRow};

const SECRET: &str = "horae_ocs_abcdefghTHE-REST-OF-THE-SECRET";

#[derive(Clone)]
struct Probe {
    path: String,
    apps: Rc<RefCell<Vec<AppRow>>>,
    grants: Rc<RefCell<Vec<GrantRow>>>,
    can_connect: bool,
    creates: Rc<RefCell<Vec<AppInput>>>,
    revokes: Rc<RefCell<Vec<String>>>,
    /// The next create's refusal, if any.
    refusal: Rc<RefCell<Option<ServerFnError>>>,
}

impl Probe {
    fn at(path: &str) -> Self {
        Self {
            path: path.into(),
            apps: Rc::new(RefCell::new(vec![android()])),
            grants: Rc::default(),
            can_connect: true,
            creates: Rc::default(),
            revokes: Rc::default(),
            refusal: Rc::default(),
        }
    }
}

fn android() -> AppRow {
    AppRow {
        id: Uuid::now_v7(),
        client_id: "horae-android".into(),
        name: "Horae for Android".into(),
        kind: AppKind::Public,
        scopes: vec![
            "time:read".into(),
            "time:write".into(),
            "catalog:read".into(),
        ],
        redirect_uris: vec!["dev.horae.android:/oauth/callback".into()],
        built_in: true,
        enabled: false,
        secret_prefix: None,
        active_grants: 0,
        created_at: Utc::now(),
        created_by_name: None,
    }
}

fn server_app() -> AppRow {
    AppRow {
        id: Uuid::now_v7(),
        client_id: "horae_app_7Q2fK9xV3mWbQ2".into(),
        name: "Invoice sync".into(),
        kind: AppKind::Confidential,
        scopes: vec!["time:read".into(), "catalog:read".into()],
        redirect_uris: vec!["https://sync.example.com/callback".into()],
        built_in: false,
        enabled: true,
        secret_prefix: Some("abcdefgh".into()),
        active_grants: 1,
        created_at: Utc::now(),
        created_by_name: Some("Ada Admin".into()),
    }
}

fn grant(app: &AppRow) -> GrantRow {
    GrantRow {
        id: Uuid::now_v7(),
        app_name: app.name.clone(),
        client_id: app.client_id.clone(),
        person_id: Uuid::now_v7(),
        person_name: "Jane Cooper".into(),
        person_email: "jane@example.com".into(),
        scopes: app.scopes.clone(),
        approved_at: Utc::now(),
        last_used_at: None,
        expires_at: Utc::now(),
    }
}

fn app(probe: Probe) -> Element {
    let path = probe.path.clone();
    use_context_provider(|| probe);
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

fn mount(probe: &Probe) -> VirtualDom {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

fn settle(dom: &mut VirtualDom) {
    for _ in 0..30 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("the page did not settle");
}

type Hit<'a> = &'a dyn Fn(&VNode) -> Option<usize>;

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

/// The element whose `id` is `id`, written statically or computed.
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

fn fire(dom: &mut VirtualDom, id: &str, kind: &str, data: Box<dyn std::any::Any>) {
    let target = find(dom, dom.base_scope().root_node(), &|vnode| by_id(vnode, id))
        .unwrap_or_else(|| panic!("{id} rendered"));
    let event = Event::new(
        Rc::new(PlatformEventData::new(data)) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event(kind, event, target);
    settle(dom);
}

fn click(dom: &mut VirtualDom, id: &str) {
    fire(dom, id, "click", Box::<SerializedMouseData>::default());
}

fn type_into(dom: &mut VirtualDom, id: &str, text: &str) {
    let data = Box::new(SerializedFormData::new(text.into(), vec![]));
    fire(dom, id, "input", data);
}

fn submit(dom: &mut VirtualDom, id: &str) {
    let data = Box::new(SerializedFormData::new(String::new(), vec![]));
    fire(dom, id, "submit", data);
}

fn html(dom: &VirtualDom) -> String {
    dioxus::ssr::render(dom)
}

#[tokio::test]
async fn the_list_puts_horaes_own_app_first_with_access_in_words() {
    let probe = Probe::at("/admin/apps");
    let server = server_app();
    probe.apps.borrow_mut().push(server.clone());
    let html = html(&mount(&probe));
    let android = html.find("Horae for Android").expect("listed");
    assert!(android < html.find("Invoice sync").unwrap(), "{html}");
    assert!(html.contains("Horae’s own app · horae-android"), "{html}");
    assert!(html.contains("Time, projects"), "{html}");
    assert!(html.contains("Time (read), projects"), "{html}");
    assert!(html.contains("Runs on a server"), "{html}");
    assert!(!html.contains("time:read"), "no scope codes: {html}");
    assert!(!html.contains("apps-not-ready"), "{html}");
}

#[tokio::test]
async fn without_a_public_address_a_notice_says_apps_cannot_connect() {
    let probe = Probe {
        can_connect: false,
        ..Probe::at("/admin/apps")
    };
    let html = html(&mount(&probe));
    assert!(html.contains("Apps can’t connect yet"), "{html}");
    assert!(!html.contains("HORAE_PUBLIC_URL"), "{html}");
    assert!(html.contains("apps-register"), "registering still works");
}

#[tokio::test]
async fn the_built_in_app_offers_only_enable_or_disable() {
    let probe = Probe::at("/admin/apps");
    let android = probe.apps.borrow()[0].clone();
    let server = server_app();
    probe.apps.borrow_mut().push(server.clone());
    let html = html(&mount(&probe));
    let id = android.id;
    assert!(html.contains(&format!("app-toggle-{id}")), "{html}");
    assert!(!html.contains(&format!("app-edit-{id}")), "{html}");
    assert!(!html.contains(&format!("app-delete-{id}")), "{html}");
    assert!(
        !html.contains(&format!("app-replace-secret-{id}")),
        "{html}"
    );
    let id = server.id;
    for item in ["edit", "delete", "replace-secret", "toggle"] {
        assert!(html.contains(&format!("app-{item}-{id}")), "{item}: {html}");
    }
}

#[tokio::test]
async fn a_device_app_is_registered_without_a_secret_and_its_page_opens() {
    let probe = Probe::at("/admin/apps");
    let mut dom = mount(&probe);
    click(&mut dom, "apps-register");
    type_into(&mut dom, "app-name", "Shortcuts timer");
    type_into(&mut dom, "app-redirects", "com.example.timer:/callback\n");
    submit(&mut dom, "app-editor-form");

    let creates = probe.creates.borrow().clone();
    let [created] = &creates[..] else {
        panic!("one create: {creates:?}");
    };
    assert_eq!(created.name, "Shortcuts timer");
    assert_eq!(created.kind, AppKind::Public);
    assert_eq!(created.redirect_uris, ["com.example.timer:/callback"]);
    assert_eq!(
        created.scopes,
        ["time:read", "time:write", "catalog:read"],
        "a new app starts with time and projects"
    );
    let html = html(&dom);
    assert!(!html.contains(SECRET), "{html}");
    assert!(html.contains("Back to Apps"), "its page opened: {html}");
    assert!(html.contains("com.example.timer:/callback"), "{html}");
}

#[tokio::test]
async fn a_server_apps_secret_shows_once() {
    let probe = Probe::at("/admin/apps");
    let mut dom = mount(&probe);
    click(&mut dom, "apps-register");
    type_into(&mut dom, "app-name", "Invoice sync");
    click(&mut dom, "app-kind-confidential");
    type_into(
        &mut dom,
        "app-redirects",
        "https://sync.example.com/callback",
    );
    submit(&mut dom, "app-editor-form");

    assert_eq!(probe.creates.borrow()[0].kind, AppKind::Confidential);
    let shown = html(&dom);
    assert!(shown.contains(SECRET), "{shown}");
    assert!(shown.contains("Invoice sync is registered"), "{shown}");

    click(&mut dom, "app-secret-done");
    let html = html(&dom);
    assert!(!html.contains(SECRET), "the secret is gone: {html}");
    assert!(html.contains("Back to Apps"), "its page opened: {html}");
}

#[tokio::test]
async fn a_refused_address_is_shown_under_its_field() {
    let probe = Probe::at("/admin/apps");
    *probe.refusal.borrow_mut() = Some(ServerFnError::ServerError {
        message: "A redirect URI cannot have a fragment".into(),
        code: 400,
        details: Some(serde_json::json!({ "field": "redirect_uris" })),
    });
    let mut dom = mount(&probe);
    click(&mut dom, "apps-register");
    type_into(&mut dom, "app-name", "Sync");
    type_into(&mut dom, "app-redirects", "https://sync.example.com/#x");
    submit(&mut dom, "app-editor-form");
    let html = html(&dom);
    let error = html
        .find("id=\"app-redirects-error\"")
        .expect("under the field");
    assert!(
        html[error..].contains("A redirect URI cannot have a fragment"),
        "{html}"
    );
    assert!(html.contains("https://sync.example.com/#x"), "kept: {html}");
}

#[tokio::test]
async fn the_built_in_apps_page_has_no_edit_secret_or_delete() {
    let probe = Probe::at("/admin/apps");
    let id = probe.apps.borrow()[0].id;
    let probe = Probe {
        path: format!("/admin/apps/{id}"),
        ..probe
    };
    let html = html(&mount(&probe));
    assert!(html.contains("dev.horae.android:/oauth/callback"), "{html}");
    assert!(
        html.contains("Add, change and delete your time entries and timers"),
        "access in sentences: {html}"
    );
    assert!(html.contains(&format!("app-toggle-{id}")), "{html}");
    for item in ["edit", "delete", "replace-secret"] {
        assert!(
            !html.contains(&format!("app-{item}-{id}")),
            "{item}: {html}"
        );
    }
}

#[tokio::test]
async fn revoking_an_authorization_asks_first() {
    let server = server_app();
    let probe = Probe::at(&format!("/admin/apps/{}", server.id));
    let person = grant(&server);
    let id = person.id;
    probe.apps.borrow_mut().push(server);
    probe.grants.borrow_mut().push(person);
    let mut dom = mount(&probe);
    let page = html(&dom);
    assert!(page.contains("Jane Cooper"), "{page}");
    assert!(page.contains("jane@example.com"), "{page}");
    assert!(page.contains("Ada Admin"), "{page}");

    click(&mut dom, &format!("grant-revoke-{id}"));
    assert!(html(&dom).contains("Revoke Jane Cooper’s access?"));
    click(&mut dom, "grant-revoke-dialog-cancel");
    assert!(probe.revokes.borrow().is_empty(), "cancel revokes nothing");

    click(&mut dom, &format!("grant-revoke-{id}"));
    let confirm = html(&dom);
    assert!(
        confirm.contains(
            r#"id="grant-revoke-dialog-confirm" type="button" class="btn btn-danger-solid""#
        ),
        "{confirm}"
    );
    click(&mut dom, "grant-revoke-dialog-confirm");
    assert_eq!(*probe.revokes.borrow(), [id.to_string()]);
    let html = html(&dom);
    assert!(!html.contains("Jane Cooper"), "{html}");
    assert!(html.contains("Nobody has allowed this app yet"), "{html}");
}

mod route {
    use super::apps::{AdminApp, AdminApps};
    use dioxus::prelude::*;
    use uuid::Uuid;

    #[derive(Clone, PartialEq, Routable)]
    pub enum Route {
        #[route("/admin/apps")]
        AdminApps {},
        #[route("/admin/apps/:id")]
        AdminApp { id: Uuid },
    }
}

mod server_fns {
    use super::*;

    fn probe() -> Probe {
        use_context::<Probe>()
    }

    fn not_found() -> ServerFnError {
        ServerFnError::ServerError {
            message: "App not found".into(),
            code: 404,
            details: None,
        }
    }

    pub async fn list_oauth_apps() -> Result<Vec<AppRow>, ServerFnError> {
        Ok(probe().apps.borrow().clone())
    }

    pub async fn oauth_apps_can_connect() -> Result<bool, ServerFnError> {
        Ok(probe().can_connect)
    }

    pub async fn get_oauth_app(id: String) -> Result<AppRow, ServerFnError> {
        let apps = probe().apps.borrow().clone();
        apps.into_iter()
            .find(|app| app.id.to_string() == id)
            .ok_or_else(not_found)
    }

    pub async fn create_oauth_app(input: AppInput) -> Result<CreatedApp, ServerFnError> {
        let probe = probe();
        probe.creates.borrow_mut().push(input.clone());
        if let Some(refusal) = probe.refusal.borrow_mut().take() {
            return Err(refusal);
        }
        let confidential = input.kind == AppKind::Confidential;
        let app = AppRow {
            id: Uuid::now_v7(),
            client_id: "horae_app_new".into(),
            name: input.name,
            kind: input.kind,
            scopes: input.scopes,
            redirect_uris: input.redirect_uris,
            built_in: false,
            enabled: true,
            secret_prefix: confidential.then(|| "abcdefgh".into()),
            active_grants: 0,
            created_at: Utc::now(),
            created_by_name: Some("Ada Admin".into()),
        };
        probe.apps.borrow_mut().push(app.clone());
        Ok(CreatedApp {
            app,
            secret: confidential.then(|| SECRET.into()),
        })
    }

    pub async fn update_oauth_app(id: String, changes: AppInput) -> Result<AppRow, ServerFnError> {
        let probe = probe();
        let mut apps = probe.apps.borrow_mut();
        let app = apps
            .iter_mut()
            .find(|app| app.id.to_string() == id)
            .ok_or_else(not_found)?;
        app.name = changes.name;
        app.redirect_uris = changes.redirect_uris;
        app.scopes = changes.scopes;
        Ok(app.clone())
    }

    pub async fn set_oauth_app_enabled(id: String, enabled: bool) -> Result<AppRow, ServerFnError> {
        let probe = probe();
        let mut apps = probe.apps.borrow_mut();
        let app = apps
            .iter_mut()
            .find(|app| app.id.to_string() == id)
            .ok_or_else(not_found)?;
        app.enabled = enabled;
        Ok(app.clone())
    }

    pub async fn replace_oauth_app_secret(id: String) -> Result<CreatedApp, ServerFnError> {
        let app = get_oauth_app(id).await?;
        Ok(CreatedApp {
            app,
            secret: Some(SECRET.into()),
        })
    }

    pub async fn delete_oauth_app(id: String) -> Result<(), ServerFnError> {
        probe()
            .apps
            .borrow_mut()
            .retain(|app| app.id.to_string() != id);
        Ok(())
    }

    pub async fn list_oauth_app_grants(_id: String) -> Result<Vec<GrantRow>, ServerFnError> {
        Ok(probe().grants.borrow().clone())
    }

    pub async fn revoke_oauth_grant(id: String) -> Result<(), ServerFnError> {
        let probe = probe();
        probe.revokes.borrow_mut().push(id.clone());
        probe
            .grants
            .borrow_mut()
            .retain(|grant| grant.id.to_string() != id);
        Ok(())
    }
}
