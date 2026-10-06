#![cfg(feature = "server")]

//! Exercise the production resource and refresh control with controlled reads.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::core::{ElementId, Mutation};
use dioxus::prelude::*;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use tokio::sync::oneshot;

#[path = "../src/components/permission_description.rs"]
pub mod permission_description;
mod components {
    pub use super::permission_description;
}
#[path = "../src/models/own_permissions.rs"]
pub mod own_permissions_model;
mod models {
    pub use super::own_permissions_model as own_permissions;
}
#[path = "../src/pages/settings/own_permissions.rs"]
mod own_permissions;

type Response = Result<Option<own_permissions_model::OwnPermissions>, ServerFnError>;

#[derive(Clone, Default)]
struct Probe {
    responses: Rc<RefCell<VecDeque<oneshot::Receiver<Response>>>>,
    reads: Rc<Cell<usize>>,
}

impl Probe {
    fn request(&self) -> oneshot::Sender<Response> {
        let (send, receive) = oneshot::channel();
        self.responses.borrow_mut().push_back(receive);
        send
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { own_permissions::OwnPermissionSection {} }
}

fn settle(dom: &mut VirtualDom) {
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return;
        }
        dom.render_immediate_to_vec();
    }
    panic!("own permission resource did not settle");
}

fn click(dom: &mut VirtualDom, id: ElementId) {
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, id);
    settle(dom);
}

#[tokio::test]
async fn refresh_uses_a_fresh_read_hides_stale_grants_and_recovers_from_errors() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let probe = Probe::default();
    let first = probe.request();
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let mutations = dom.rebuild_to_vec();
    settle(&mut dom);
    let refresh = mutations
        .edits
        .into_iter()
        .find_map(|mutation| match mutation {
            Mutation::NewEventListener { name, id } if name == "click" => Some(id),
            _ => None,
        })
        .expect("refresh must be wired");
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading your permissions"), "{html}");
    assert!(html.contains("disabled"), "{html}");
    click(&mut dom, refresh);
    assert_eq!(
        probe.reads.get(),
        1,
        "pending refresh must not start another read"
    );

    first
        .send(Err(ServerFnError::new("private transport details")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Could not load your permissions"), "{html}");
    assert!(!html.contains("private transport details"), "{html}");
    assert!(!html.contains("disabled"), "{html}");

    let second = probe.request();
    click(&mut dom, refresh);
    second
        .send(Ok(Some(own_permissions_model::OwnPermissions {
            catalog_version: PERMISSION_CATALOG_VERSION,
            grants: vec![Permission::TimeReadOwn],
            is_administrator: false,
            access_revision: 2,
            person_revision: 1,
            managed_person_ids: vec![],
            managed_project_ids: vec![],
        })))
        .unwrap();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("View your own time"));
    assert_eq!(probe.reads.get(), 2);

    let third = probe.request();
    click(&mut dom, refresh);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading your permissions"), "{html}");
    assert!(!html.contains("View your own time"), "{html}");
    const FORBIDDEN: u16 = 403;
    third
        .send(Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            message: "private denial context".into(),
            details: None,
        }))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Ask an Administrator"), "{html}");
    assert!(!html.contains("View your own time"), "{html}");
    assert!(!html.contains("private denial context"), "{html}");
    assert_eq!(probe.reads.get(), 3);
}

mod server_fns {
    use super::*;

    pub async fn get_my_permissions() -> Response {
        let probe = use_context::<Probe>();
        probe.reads.set(probe.reads.get() + 1);
        let response = probe
            .responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected read");
        response.await.expect("response sender dropped")
    }
}
