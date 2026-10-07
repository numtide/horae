#![cfg(feature = "server")]

//! Render the production access gate and outlet with controlled auth responses
//! and a minimal router. No database or HTTP server is needed for these states.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::core::Mutation;
use dioxus::history::MemoryHistory;
use dioxus::prelude::*;
use dioxus::router::components::HistoryProvider;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::permissions::catalog::Permission;
use horae_core::types::OrgRole;
use tokio::sync::oneshot;

#[path = "../src/components/admin_shell.rs"]
mod admin_shell;

type AuthResponse = Result<User, ServerFnError>;

struct User(OrgRole);

impl User {
    fn is_admin(&self) -> bool {
        self.0 == OrgRole::Admin
    }
}

#[derive(Clone, Default)]
struct Probe {
    responses: Rc<RefCell<VecDeque<oneshot::Receiver<AuthResponse>>>>,
    org_requests: Rc<Cell<usize>>,
    panel_mounts: Rc<Cell<usize>>,
    audit: bool,
    importers: bool,
    own_error: bool,
    future_catalog: bool,
    canonical_requests: Rc<Cell<usize>>,
    canonical: Option<(bool, Vec<Permission>)>,
}

impl Probe {
    fn request(&self) -> oneshot::Sender<AuthResponse> {
        let (send, receive) = oneshot::channel();
        self.responses.borrow_mut().push_back(receive);
        send
    }
}

fn app(probe: Probe) -> Element {
    let path = if probe.importers {
        "/admin/importers"
    } else if probe.audit {
        "/admin/audit"
    } else {
        "/admin/users"
    };
    use_context_provider(|| probe);
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(path)) as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

fn settle(dom: &mut VirtualDom) -> Vec<Mutation> {
    let mut mutations = Vec::new();
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return mutations;
        }
        mutations.extend(dom.render_immediate_to_vec().edits);
    }
    panic!("admin shell did not settle after controlled responses");
}

fn start(probe: &Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

fn assert_panel_hidden(dom: &VirtualDom, probe: &Probe) {
    let html = dioxus::ssr::render(dom);
    assert!(
        !html.contains("adm-nav"),
        "admin navigation must stay hidden: {html}"
    );
    assert!(
        !html.contains("Administrative panel"),
        "admin outlet must stay hidden: {html}"
    );
    assert_eq!(
        probe.org_requests.get(),
        0,
        "workspace must not load before authorization"
    );
    assert_eq!(
        probe.panel_mounts.get(),
        0,
        "admin panel must not mount before authorization"
    );
}

#[tokio::test]
async fn pending_user_does_not_mount_admin_content() {
    let probe = Probe::default();
    let _response = probe.request();
    let dom = start(&probe);
    assert_panel_hidden(&dom, &probe);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Loading"),
        "expected a loading notice: {html}"
    );
}

#[tokio::test]
async fn failed_user_does_not_mount_admin_content() {
    let probe = Probe::default();
    let response = probe.request();
    let mut dom = start(&probe);
    response
        .send(Err(ServerFnError::new("Session unavailable")))
        .unwrap_or_else(|_| panic!("auth request was dropped"));
    settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Could not verify workspace access") && !html.contains("Session unavailable"),
        "expected the auth error: {html}"
    );
}

#[tokio::test]
async fn members_and_managers_never_mount_admin_content() {
    for role in [OrgRole::Member, OrgRole::Manager] {
        let probe = Probe::default();
        let response = probe.request();
        let mut dom = start(&probe);
        response
            .send(Ok(User(role)))
            .unwrap_or_else(|_| panic!("auth request was dropped"));
        settle(&mut dom);
        assert_panel_hidden(&dom, &probe);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Access unavailable"),
            "expected access notice: {html}"
        );
        assert!(html.contains("Back to Timesheet"));
    }
}

#[tokio::test]
async fn resolved_admin_mounts_the_workspace_and_outlet() {
    let probe = Probe::default();
    let response = probe.request();
    let mut dom = start(&probe);
    assert_panel_hidden(&dom, &probe);
    response
        .send(Ok(User(OrgRole::Admin)))
        .unwrap_or_else(|_| panic!("auth request was dropped"));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Test workspace"),
        "expected resolved organization: {html}"
    );
    assert!(
        html.contains("Administrative panel"),
        "expected the routed panel: {html}"
    );
    assert_eq!(probe.org_requests.get(), 1);
    assert_eq!(probe.panel_mounts.get(), 1);
}

#[tokio::test]
async fn retry_waits_for_fresh_authorization_then_recovers() {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let probe = Probe::default();
    let response = probe.request();
    let mut dom = start(&probe);
    response
        .send(Err(ServerFnError::new("Session unavailable")))
        .unwrap_or_else(|_| panic!("auth request was dropped"));
    let mutations = settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
    assert!(dioxus::ssr::render(&dom).contains("Retry"));
    let retry = mutations
        .into_iter()
        .find_map(|mutation| match mutation {
            Mutation::NewEventListener { name, id } if name == "click" => Some(id),
            _ => None,
        })
        .expect("the error state must expose a retry button");

    let response = probe.request();
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    );
    dom.runtime().handle_event("click", event, retry);
    settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Loading"),
        "retry must render a pending state: {html}"
    );
    assert!(
        !html.contains("Session unavailable"),
        "retry must not retain a stale error: {html}"
    );

    response
        .send(Ok(User(OrgRole::Admin)))
        .unwrap_or_else(|_| panic!("retry request was dropped"));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Test workspace"));
    assert!(html.contains("Administrative panel"));
    assert_eq!(probe.org_requests.get(), 1);
    assert_eq!(probe.panel_mounts.get(), 1);
}

mod server_fns {
    use super::*;

    pub struct OwnPermissions {
        pub catalog_version: u32,
        pub is_administrator: bool,
        pub grants: Vec<Permission>,
    }

    pub async fn get_my_permissions() -> Result<Option<OwnPermissions>, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe
            .canonical_requests
            .set(probe.canonical_requests.get() + 1);
        if probe.own_error {
            return Err(ServerFnError::new("private projection error"));
        }
        if let Some((is_administrator, grants)) = probe.canonical.clone() {
            return Ok(Some(OwnPermissions {
                catalog_version: horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION
                    + u32::from(probe.future_catalog),
                is_administrator,
                grants,
            }));
        }
        if !probe.audit {
            return Ok(None);
        }
        let reply = probe
            .responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected canonical read");
        reply
            .await
            .expect("resolve canonical authorization")
            .map(|user| {
                Some(OwnPermissions {
                    catalog_version: horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION,
                    is_administrator: user.is_admin(),
                    grants: vec![],
                })
            })
    }

    pub async fn get_me() -> AuthResponse {
        if consume_context::<Probe>().audit {
            return Ok(User(OrgRole::Member));
        }
        let response = consume_context::<Probe>()
            .responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected auth request");
        response.await.expect("test must resolve the auth request")
    }

    pub async fn get_org_name() -> Result<String, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.org_requests.set(probe.org_requests.get() + 1);
        Ok("Test workspace".into())
    }
}

// Only route argument shapes are needed by the shell; timesheet behavior is
// outside this access-gate test.
mod pages {
    pub mod admin {
        use dioxus::prelude::*;
        #[component]
        pub fn PermissionRecovery(on_saved: EventHandler<bool>) -> Element {
            rsx! { "Recovery available" }
        }
    }
    pub mod timesheet {
        pub type Anchor = String;
        pub type CalSpan = String;

        #[derive(Clone, PartialEq, Default)]
        pub enum ViewMode {
            #[default]
            Week,
        }

        impl std::fmt::Display for ViewMode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("week")
            }
        }

        impl std::str::FromStr for ViewMode {
            type Err = std::convert::Infallible;
            fn from_str(_: &str) -> Result<Self, Self::Err> {
                Ok(Self::Week)
            }
        }
    }
}

mod route {
    use super::*;
    use admin_shell::AdminShell;
    use pages::timesheet::{Anchor, CalSpan, ViewMode};

    #[derive(Clone, PartialEq, Routable)]
    pub enum Route {
        #[route("/timesheet?:view&:date&:span")]
        Timesheet {
            view: ViewMode,
            date: Anchor,
            span: CalSpan,
        },
        #[layout(AdminShell)]
        #[route("/admin/users")]
        AdminUsers {},
        #[route("/admin/importers")]
        HarvestImport {},
        #[route("/admin/audit")]
        PermissionAudit {},
    }

    pub fn route_is_active(to: &Route) -> bool {
        std::mem::discriminant(&use_route::<Route>()) == std::mem::discriminant(to)
    }

    #[component]
    fn Timesheet(view: ViewMode, date: Anchor, span: CalSpan) -> Element {
        rsx! { "Timesheet" }
    }

    #[component]
    fn AdminUsers() -> Element {
        use_hook(|| {
            let probe = consume_context::<Probe>();
            probe.panel_mounts.set(probe.panel_mounts.get() + 1);
        });
        rsx! { "Administrative panel" }
    }

    #[component]
    fn HarvestImport() -> Element {
        rsx! { "Importer panel" }
    }

    #[component]
    fn PermissionAudit() -> Element {
        rsx! { "Permission audit panel" }
    }
}

#[tokio::test]
async fn audit_route_uses_canonical_authority_without_replacing_legacy_page_checks() {
    for allowed in [true, false] {
        let probe = Probe {
            audit: true,
            ..Probe::default()
        };
        let response = probe.request();
        let mut dom = start(&probe);
        assert_panel_hidden(&dom, &probe);
        response
            .send(Ok(User(if allowed {
                OrgRole::Admin
            } else {
                OrgRole::Member
            })))
            .unwrap_or_else(|_| panic!("audit request dropped"));
        settle(&mut dom);
        assert_eq!(probe.canonical_requests.get(), 1);
        assert_eq!(
            dioxus::ssr::render(&dom).contains("Permission audit panel"),
            allowed
        );
    }
    let probe = Probe::default();
    let response = probe.request();
    let mut dom = start(&probe);
    response
        .send(Ok(User(OrgRole::Member)))
        .unwrap_or_else(|_| panic!("legacy request dropped"));
    settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
    assert_eq!(probe.canonical_requests.get(), 1);
}

#[tokio::test]
async fn audit_gate_hides_internal_authentication_failures() {
    let probe = Probe {
        audit: true,
        ..Probe::default()
    };
    let response = probe.request();
    let mut dom = start(&probe);
    response
        .send(Err(ServerFnError::new("private SQL diagnostic")))
        .unwrap_or_else(|_| panic!("audit request dropped"));
    settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains("private SQL diagnostic"));
    assert!(html.contains("Could not verify workspace access"));
}

#[tokio::test]
async fn canonical_people_does_not_require_a_legacy_admin_role() {
    for (administrator, grant) in [
        (true, Permission::PeopleReadAll),
        (false, Permission::PeopleReadManaged),
    ] {
        let probe = Probe {
            canonical: Some((administrator, vec![grant])),
            ..Probe::default()
        };
        let reply = probe.request();
        let mut dom = start(&probe);
        reply
            .send(Ok(User(OrgRole::Member)))
            .unwrap_or_else(|_| panic!("identity request dropped"));
        settle(&mut dom);
        assert!(dioxus::ssr::render(&dom).contains("Administrative panel"));
        assert_eq!(probe.panel_mounts.get(), 1);
    }
}

#[tokio::test]
async fn legacy_admin_cannot_bypass_canonical_people_scope() {
    let probe = Probe {
        canonical: Some((false, vec![])),
        ..Probe::default()
    };
    let reply = probe.request();
    let mut dom = start(&probe);
    reply
        .send(Ok(User(OrgRole::Admin)))
        .unwrap_or_else(|_| panic!("identity request dropped"));
    settle(&mut dom);
    assert_panel_hidden(&dom, &probe);
}

#[tokio::test]
async fn importers_preserves_legacy_authority_when_canonical_projection_fails() {
    for allowed in [false, true] {
        let probe = Probe {
            importers: true,
            own_error: true,
            ..Probe::default()
        };
        let reply = probe.request();
        let mut dom = start(&probe);
        reply
            .send(Ok(User(if allowed {
                OrgRole::Admin
            } else {
                OrgRole::Member
            })))
            .unwrap_or_else(|_| panic!("identity read dropped"));
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert_eq!(html.contains("Importer panel"), allowed);
        assert!(!html.contains("/admin/users"));
        assert!(!html.contains("/admin/audit"));
        assert!(!html.contains("private projection error"));
    }
}

#[tokio::test]
async fn failed_or_future_people_projection_keeps_only_recovery() {
    for future_catalog in [false, true] {
        let probe = Probe {
            own_error: !future_catalog,
            future_catalog,
            canonical: Some((true, vec![Permission::PeopleReadAll])),
            ..Probe::default()
        };
        let reply = probe.request();
        let mut dom = start(&probe);
        reply
            .send(Ok(User(OrgRole::Admin)))
            .unwrap_or_else(|_| panic!("identity read dropped"));
        settle(&mut dom);
        assert_panel_hidden(&dom, &probe);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Could not verify workspace access"));
        assert!(html.contains("Recovery available"));
        assert!(!html.contains("private projection error"));
    }
}
