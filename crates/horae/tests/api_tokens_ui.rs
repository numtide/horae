#![cfg(feature = "server")]

//! Render the production "API tokens" settings section with controlled server
//! responses: create, reveal the secret once, and revoke after confirming. No
//! database or HTTP server is needed.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use dioxus::core::{AttributeValue, DynamicNode, ElementId, TemplateAttribute, TemplateNode};
use dioxus::prelude::*;
use dioxus_html::{SerializedFormData, SerializedHtmlEventConverter, SerializedMouseData};
use futures_util::FutureExt;
use uuid::Uuid;

#[path = "../src/components/button.rs"]
pub mod button;
#[path = "../src/components/confirm_dialog.rs"]
pub mod confirm_dialog;
#[allow(dead_code)]
#[path = "../src/components/form.rs"]
pub mod form;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/modal.rs"]
pub mod modal;
#[allow(dead_code)]
#[path = "../src/components/screen_state.rs"]
pub mod screen_state;
mod components {
    pub use super::{confirm_dialog, form, icons, screen_state};
}
mod models {
    pub use horae_types::api_tokens;
}
#[path = "../src/pages/settings/api_tokens.rs"]
mod api_tokens;

use models::api_tokens::{ApiTokenRow, CreatedApiToken};

const SECRET: &str = "horae_pat_abcdefghTHE-REST-OF-THE-SECRET";

/// One create's name and expiry.
type Create = (String, Option<DateTime<Utc>>);

#[derive(Clone, Default)]
struct Probe {
    rows: Rc<RefCell<Vec<ApiTokenRow>>>,
    creates: Rc<RefCell<Vec<Create>>>,
    revokes: Rc<RefCell<Vec<String>>>,
    /// The next create's refusal, if any.
    refusal: Rc<RefCell<Option<ServerFnError>>>,
    /// The Harvest account id, `None` while the Harvest API is off.
    harvest_account: Option<i64>,
}

fn row(name: &str) -> ApiTokenRow {
    ApiTokenRow {
        id: Uuid::now_v7(),
        user_id: Uuid::nil(),
        owner_name: "Mia Member".into(),
        name: name.into(),
        display_prefix: "abcdefgh".into(),
        created_at: Utc::now(),
        expires_at: None,
        last_used_at: None,
        revoked_at: None,
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { api_tokens::ApiTokensSection {} }
}

fn mount(probe: &Probe) -> VirtualDom {
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
    panic!("the section did not settle");
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

fn html(dom: &VirtualDom) -> String {
    dioxus::ssr::render(dom)
}

#[tokio::test]
async fn the_harvest_account_id_shows_only_while_the_harvest_api_is_on() {
    let off = html(&mount(&Probe::default()));
    assert!(!off.contains("api-harvest-account"), "{off}");

    let on = html(&mount(&Probe {
        harvest_account: Some(4217),
        ..Probe::default()
    }));
    assert!(on.contains("api-harvest-account"), "{on}");
    assert!(on.contains("4217"), "{on}");
}

#[tokio::test]
async fn an_empty_list_says_so_and_create_waits_for_a_name() {
    let dom = mount(&Probe::default());
    let html = html(&dom);
    assert!(html.contains("No tokens yet"), "{html}");
    assert!(
        html.contains(r#"href="/api/v1/openapi.json""#),
        "the section links the API description: {html}"
    );
    let create = &html[html.find("id=\"api-token-create\"").unwrap()..];
    assert!(
        create[..create.find('>').unwrap()].contains("disabled=true"),
        "{html}"
    );
}

#[tokio::test]
async fn a_created_token_is_revealed_once_and_listed_by_its_prefix() {
    let probe = Probe::default();
    let mut dom = mount(&probe);
    type_into(&mut dom, "api-token-name", "Calendar sync");
    click(&mut dom, "api-token-create");

    let creates = probe.creates.borrow().clone();
    let [(name, Some(expires_at))] = &creates[..] else {
        panic!("one create with the default expiry: {creates:?}");
    };
    assert_eq!(name, "Calendar sync");
    let ninety_days = Utc::now() + Duration::days(90);
    assert!(
        (ninety_days - *expires_at).num_minutes().abs() < 5,
        "{expires_at}"
    );

    let html = html(&dom);
    assert!(html.contains(SECRET), "the secret is shown once: {html}");
    assert!(html.contains("will not show it again"), "{html}");
    assert!(html.contains("horae_pat_abcdefgh…"), "{html}");
    assert!(html.contains("Calendar sync"), "{html}");
    assert!(
        html.contains("value=\"\""),
        "the name field is cleared: {html}"
    );

    click(&mut dom, "api-token-dismiss");
    let html = self::html(&dom);
    assert!(
        !html.contains(SECRET),
        "the secret is gone after Done: {html}"
    );
    assert!(html.contains("Calendar sync"), "{html}");
}

#[tokio::test]
async fn a_refused_name_is_reported_and_kept() {
    let probe = Probe::default();
    *probe.refusal.borrow_mut() = Some(ServerFnError::ServerError {
        message: "Name the token in 1 to 100 characters".into(),
        code: 400,
        details: None,
    });
    let mut dom = mount(&probe);
    type_into(&mut dom, "api-token-name", "x");
    click(&mut dom, "api-token-create");
    let html = html(&dom);
    assert!(
        html.contains("Name the token in 1 to 100 characters"),
        "{html}"
    );
    assert!(html.contains("value=\"x\""), "{html}");
    assert!(!html.contains("api-token-secret"), "{html}");
}

#[tokio::test]
async fn revoking_asks_first_and_then_shows_the_token_revoked() {
    let probe = Probe::default();
    let token = row("Old script");
    let id = token.id;
    probe.rows.borrow_mut().push(token);
    let mut dom = mount(&probe);

    click(&mut dom, &format!("api-token-revoke-{id}"));
    assert!(html(&dom).contains("Revoke “Old script”?"));
    click(&mut dom, "api-token-revoke-dialog-cancel");
    assert!(probe.revokes.borrow().is_empty(), "cancel revokes nothing");

    click(&mut dom, &format!("api-token-revoke-{id}"));
    let confirm = html(&dom);
    assert!(
        confirm.contains(
            r#"id="api-token-revoke-dialog-confirm" type="button" class="btn btn-danger-solid""#
        ),
        "an irreversible revoke confirms with the solid danger button: {confirm}"
    );
    click(&mut dom, "api-token-revoke-dialog-confirm");
    assert_eq!(*probe.revokes.borrow(), [id.to_string()]);
    let html = html(&dom);
    assert!(html.contains("Revoked"), "{html}");
    assert!(!html.contains(&format!("api-token-revoke-{id}")), "{html}");
}

mod server_fns {
    use super::*;

    pub async fn harvest_account_id() -> Result<Option<i64>, ServerFnError> {
        Ok(use_context::<Probe>().harvest_account)
    }

    pub async fn list_api_tokens() -> Result<Vec<ApiTokenRow>, ServerFnError> {
        Ok(use_context::<Probe>().rows.borrow().clone())
    }

    pub async fn create_api_token(
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<CreatedApiToken, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.creates.borrow_mut().push((name.clone(), expires_at));
        if let Some(refusal) = probe.refusal.borrow_mut().take() {
            return Err(refusal);
        }
        let row = ApiTokenRow {
            expires_at,
            ..row(name.trim())
        };
        probe.rows.borrow_mut().insert(0, row.clone());
        Ok(CreatedApiToken {
            token: SECRET.into(),
            row,
        })
    }

    pub async fn revoke_api_token(id: String) -> Result<ApiTokenRow, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.revokes.borrow_mut().push(id.clone());
        let mut rows = probe.rows.borrow_mut();
        let row = rows
            .iter_mut()
            .find(|row| row.id.to_string() == id)
            .expect("a listed token");
        row.revoked_at = Some(Utc::now());
        Ok(row.clone())
    }
}
