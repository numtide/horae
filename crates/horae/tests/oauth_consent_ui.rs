#![cfg(feature = "server")]

//! Render the production OAuth approval page with controlled server answers:
//! the details, Allow and Deny each calling their server function once, the
//! expired state, and the sign-out that keeps the request. No database or
//! HTTP server is needed.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::core::{AttributeValue, DynamicNode, ElementId, TemplateAttribute, TemplateNode};
use dioxus::prelude::*;
use dioxus_html::{SerializedHtmlEventConverter, SerializedMouseData};
use futures_util::FutureExt;

#[path = "../src/components/badge.rs"]
pub mod badge;
#[path = "../src/components/button.rs"]
pub mod button;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/logo.rs"]
pub mod logo;
#[allow(dead_code)]
#[path = "../src/components/screen_state.rs"]
pub mod screen_state;
mod components {
    pub use super::{badge, button, icons, logo, screen_state};
}
mod models {
    pub use horae_types::oauth;
}
#[path = "../src/pages/oauth_consent.rs"]
mod oauth_consent;

use models::oauth::ConsentDetails;

const REQUEST: &str = "req-123";

#[derive(Clone, Default)]
struct Probe {
    /// `None` answers the details as expired.
    details: Option<ConsentDetails>,
    approvals: Rc<RefCell<Vec<String>>>,
    denials: Rc<RefCell<Vec<String>>>,
}

fn details(built_in: bool) -> ConsentDetails {
    ConsentDetails {
        request_id: REQUEST.into(),
        app_name: "Horae for Android".into(),
        built_in,
        scopes: vec!["time:read".into(), "catalog:read".into()],
        return_to_display: "dev.horae.android".into(),
        person_name: "Ada Admin".into(),
        organization_name: "Test Org".into(),
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { oauth_consent::OAuthConsent { request: REQUEST } }
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

fn click(dom: &mut VirtualDom, id: &str) {
    let target = find(dom, dom.base_scope().root_node(), &|vnode| by_id(vnode, id))
        .unwrap_or_else(|| panic!("{id} rendered"));
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
            as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event("click", event, target);
    settle(dom);
}

fn html(dom: &VirtualDom) -> String {
    dioxus::ssr::render(dom)
}

/// The opening tag of the element with `id`.
fn tag<'a>(html: &'a str, id: &str) -> &'a str {
    let at = html
        .find(&format!("id=\"{id}\""))
        .unwrap_or_else(|| panic!("{id}: {html}"));
    let start = html[..at].rfind('<').unwrap();
    &html[start..at + html[at..].find('>').unwrap()]
}

#[tokio::test]
async fn the_page_names_the_app_its_access_and_where_it_returns() {
    let html = html(&mount(&Probe {
        details: Some(details(true)),
        ..Probe::default()
    }));
    assert!(
        html.contains("Horae for Android wants to use your Horae account"),
        "{html}"
    );
    assert!(html.contains("Horae’s own app"), "{html}");
    assert!(html.contains("See your time entries and timers"), "{html}");
    assert!(
        html.contains("See the clients, projects and tasks you work on"),
        "{html}"
    );
    assert!(!html.contains("Add, change and delete"), "{html}");
    assert!(html.contains("dev.horae.android"), "{html}");
    assert!(html.contains("Signed in as Ada Admin · Test Org"), "{html}");
    assert!(
        !tag(&html, "consent-allow").contains("disabled=true"),
        "{html}"
    );

    let registered = self::html(&mount(&Probe {
        details: Some(details(false)),
        ..Probe::default()
    }));
    assert!(
        registered.contains("Registered by your workspace"),
        "{registered}"
    );
}

#[tokio::test]
async fn allow_calls_approve_once_and_disables_both_buttons() {
    let probe = Probe {
        details: Some(details(false)),
        ..Probe::default()
    };
    let mut dom = mount(&probe);
    click(&mut dom, "consent-allow");
    click(&mut dom, "consent-allow");
    click(&mut dom, "consent-deny");
    assert_eq!(*probe.approvals.borrow(), [REQUEST]);
    assert!(probe.denials.borrow().is_empty());
    let html = html(&dom);
    assert!(
        tag(&html, "consent-allow").contains("disabled=true"),
        "{html}"
    );
    assert!(
        tag(&html, "consent-deny").contains("disabled=true"),
        "{html}"
    );
}

#[tokio::test]
async fn deny_calls_deny_once() {
    let probe = Probe {
        details: Some(details(false)),
        ..Probe::default()
    };
    let mut dom = mount(&probe);
    click(&mut dom, "consent-deny");
    click(&mut dom, "consent-deny");
    assert_eq!(*probe.denials.borrow(), [REQUEST]);
    assert!(probe.approvals.borrow().is_empty());
}

#[tokio::test]
async fn an_expired_request_offers_no_action() {
    let html = html(&mount(&Probe::default()));
    assert!(html.contains("This request has expired"), "{html}");
    assert!(html.contains("Nothing was shared"), "{html}");
    assert!(!html.contains("consent-allow"), "{html}");
    assert!(!html.contains("consent-deny"), "{html}");
    // Who is signed in comes with the request, so an expired one names no
    // one, but signing out is still offered.
    assert!(!html.contains("Signed in as"), "{html}");
    assert!(html.contains("Not you? Sign out"), "{html}");
    assert!(
        html.contains(r#"name="return_to" value="/oauth/consent?request=req-123""#),
        "{html}"
    );
}

#[tokio::test]
async fn signing_out_comes_back_to_the_same_request() {
    let html = html(&mount(&Probe {
        details: Some(details(false)),
        ..Probe::default()
    }));
    assert!(html.contains(r#"action="/auth/logout""#), "{html}");
    assert!(
        html.contains(r#"name="return_to" value="/oauth/consent?request=req-123""#),
        "{html}"
    );
    assert!(html.contains("Not you? Sign out"), "{html}");
}

mod server_fns {
    use super::*;

    fn not_found() -> ServerFnError {
        ServerFnError::ServerError {
            message: "This request has expired".into(),
            code: 404,
            details: None,
        }
    }

    pub async fn oauth_consent_details(request: String) -> Result<ConsentDetails, ServerFnError> {
        assert_eq!(request, REQUEST);
        use_context::<Probe>().details.ok_or_else(not_found)
    }

    pub async fn approve_oauth_request(request: String) -> Result<String, ServerFnError> {
        use_context::<Probe>().approvals.borrow_mut().push(request);
        Ok("dev.horae.android:/oauth/callback?code=horae_oac_x".into())
    }

    pub async fn deny_oauth_request(request: String) -> Result<String, ServerFnError> {
        use_context::<Probe>().denials.borrow_mut().push(request);
        Ok("dev.horae.android:/oauth/callback?error=access_denied".into())
    }
}
