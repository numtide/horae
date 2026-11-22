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
use dioxus::router::Navigator;
use dioxus::router::components::HistoryProvider;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use horae_core::permissions::catalog::Permission;
use tokio::sync::oneshot;

#[path = "../src/components/admin_shell.rs"]
mod admin_shell;
#[allow(dead_code)]
#[path = "../src/i18n.rs"]
mod i18n;
#[path = "../src/components/icons.rs"]
pub mod icons;
#[path = "../src/components/page_header.rs"]
pub mod page_header;
#[allow(dead_code)]
#[path = "../src/components/screen_state.rs"]
pub mod screen_state;
#[path = "../src/components/session.rs"]
pub mod session;
pub use horae_types::{own_permissions as own_permissions_model, user as user_model};
mod components {
    pub use super::{icons, page_header, screen_state, session};
}
mod models {
    pub use super::{own_permissions_model as own_permissions, user_model as user};
}

type AuthResponse = Result<Access, ServerFnError>;

/// The stored permissions a controlled `get_my_permissions` answers with.
struct Access {
    administrator: bool,
    grants: Vec<Permission>,
}

impl Access {
    fn new(administrator: bool, grants: &[Permission]) -> Self {
        Self {
            administrator,
            grants: grants.to_vec(),
        }
    }
}

#[derive(Clone, Default)]
struct Probe {
    responses: Rc<RefCell<VecDeque<oneshot::Receiver<AuthResponse>>>>,
    org_requests: Rc<Cell<usize>>,
    panel_mounts: Rc<Cell<usize>>,
    audit: bool,
    importers: bool,
    api_tokens: bool,
    own_error: bool,
    future_catalog: bool,
    canonical_requests: Rc<Cell<usize>>,
    me_requests: Rc<Cell<usize>>,
    /// `get_me` fails with this status: 401 for a session that has ended.
    me_fails: Rc<Cell<Option<u16>>>,
    /// `get_week_start` fails on the server.
    week_start_error: bool,
    /// Answer `get_my_permissions` at once instead of waiting for a response.
    canonical: Option<(bool, Vec<Permission>)>,
    /// The People panel's router and scope, and the shared session it sees.
    panel: Rc<Cell<Option<(Navigator, ScopeId, session::Session)>>>,
}

impl Probe {
    fn request(&self) -> oneshot::Sender<AuthResponse> {
        let (send, receive) = oneshot::channel();
        self.responses.borrow_mut().push_back(receive);
        send
    }
}

/// The app's own error boundary (`App`) around a stand-in for `AppLayout`.
fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! {
        ErrorBoundary { handle_error: session::page_failed, Layout {} }
    }
}

#[component]
fn Layout() -> Element {
    let probe = use_context::<Probe>();
    let path = if probe.importers {
        "/admin/importers"
    } else if probe.audit {
        "/admin/audit"
    } else if probe.api_tokens {
        "/admin/api-tokens"
    } else {
        "/admin/workspace"
    };
    // AppLayout provides the session above every admin route.
    session::use_session_provider()?;
    rsx! {
        session::SessionRefreshFailed {}
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
        probe.panel_mounts.get(),
        0,
        "admin panel must not mount before authorization"
    );
}

/// Until the session arrives the shell suspends: the server render waits for
/// it, so no screen of its own says "Loading".
#[tokio::test]
async fn a_pending_session_suspends_the_shell() {
    let probe = Probe::default();
    let _response = probe.request();
    let dom = start(&probe);
    assert_panel_hidden(&dom, &probe);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains("Loading"), "{html}");
}

/// What the server does: build, wait for every suspended read, render once.
#[tokio::test]
async fn the_server_render_has_the_workspace_and_its_panel() {
    let probe = Probe {
        canonical: Some((true, vec![Permission::CompanyWrite])),
        ..Probe::default()
    };
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    dom.wait_for_suspense().await;
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Test workspace"), "{html}");
    assert!(html.contains("Administrator"), "{html}");
    assert!(html.contains("Administrative panel"), "{html}");
    assert!(html.contains("/admin/audit"), "{html}");
    assert!(!html.contains("Loading"), "{html}");
}

fn failed_page(probe: &Probe) -> String {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    assert_panel_hidden(&dom, probe);
    dioxus::ssr::render(&dom)
}

#[tokio::test]
async fn an_ended_session_asks_to_sign_in_again_not_a_blank_page() {
    let html = failed_page(&Probe {
        me_fails: Rc::new(Cell::new(Some(401))),
        canonical: Some((true, vec![Permission::CompanyWrite])),
        ..Probe::default()
    });
    assert!(html.contains("Session ended"), "{html}");
    assert!(html.contains("Sign in again"), "{html}");
    assert!(html.contains(r#"action="/auth/logout""#), "{html}");
    assert!(!html.contains("private identity diagnostic"), "{html}");
}

/// A loaded shell whose session is then re-read with `get_me` failing with `code`.
fn reread_failing_with(code: u16) -> (Probe, VirtualDom) {
    let probe = Probe {
        canonical: Some((true, vec![Permission::CompanyWrite])),
        ..Probe::default()
    };
    let mut dom = start(&probe);
    assert!(dioxus::ssr::render(&dom).contains("Administrative panel"));
    let (_, scope, session) = probe.panel.get().expect("the Settings panel mounted");
    probe.me_fails.set(Some(code));
    dom.in_scope(scope, || session.refresh());
    settle(&mut dom);
    (probe, dom)
}

/// A re-read that fails for another reason keeps the last session, the rail
/// and the open screen, and says so with a way to try again.
#[tokio::test]
async fn a_failed_reread_keeps_the_session_and_the_screen() {
    let (probe, mut dom) = reread_failing_with(500);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Administrative panel"), "{html}");
    assert!(html.contains("Test workspace"), "{html}");
    assert!(html.contains("Could not update your session"), "{html}");
    assert!(html.contains("Try again"), "{html}");
    assert!(!html.contains("Could not load this page"), "{html}");
    assert!(!html.contains("private identity diagnostic"), "{html}");
    assert_eq!(probe.panel_mounts.get(), 1, "the screen stayed mounted");

    // The next re-read that succeeds clears the notice.
    let (_, scope, session) = probe.panel.get().expect("the Settings panel mounted");
    probe.me_fails.set(None);
    dom.in_scope(scope, || session.refresh());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains("Could not update your session"), "{html}");
    assert!(html.contains("Administrative panel"), "{html}");
    assert_eq!(probe.panel_mounts.get(), 1);
}

/// An ended session during a visit replaces the page: nothing on it works any more.
#[tokio::test]
async fn an_ended_session_on_a_reread_asks_to_sign_in_again() {
    let (_, dom) = reread_failing_with(401);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Session ended"), "{html}");
    assert!(html.contains(r#"action="/auth/logout""#), "{html}");
    assert!(!html.contains("Administrative panel"), "{html}");
    assert!(!html.contains("Could not update your session"), "{html}");
}

#[tokio::test]
async fn a_failed_session_read_shows_the_page_error_not_a_blank_page() {
    let html = failed_page(&Probe {
        week_start_error: true,
        canonical: Some((true, vec![Permission::CompanyWrite])),
        ..Probe::default()
    });
    assert!(html.contains("Could not load this page"), "{html}");
    assert!(html.contains("Try again"), "{html}");
    assert!(!html.contains("week start unavailable"), "{html}");
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
async fn people_without_the_company_grant_never_mount_settings() {
    for access in [
        Access::new(false, &[]),
        Access::new(false, &[Permission::TaskReadAll]),
        // People is a main screen now: its grants open no admin section.
        Access::new(false, &[Permission::PeopleReadAll]),
        Access::new(false, &[Permission::PeopleReadManaged]),
        // The Administrator identity is not the company grant.
        Access::new(true, &[]),
    ] {
        let probe = Probe::default();
        let response = probe.request();
        let mut dom = start(&probe);
        response
            .send(Ok(access))
            .unwrap_or_else(|_| panic!("auth request was dropped"));
        settle(&mut dom);
        assert_panel_hidden(&dom, &probe);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Access unavailable"),
            "expected access notice: {html}"
        );
        // A no-permission state names who can grant access instead of a way out.
        assert!(html.contains("Ask an administrator"), "{html}");
        assert!(!html.contains("Back to Timesheet"), "{html}");
    }
}

#[tokio::test]
async fn resolved_company_writer_mounts_the_workspace_and_outlet() {
    let probe = Probe::default();
    let response = probe.request();
    let mut dom = start(&probe);
    assert_panel_hidden(&dom, &probe);
    response
        .send(Ok(Access::new(false, &[Permission::CompanyWrite])))
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
    assert!(dioxus::ssr::render(&dom).contains("Try again"));
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
    // The error stays, with its "Try again" disabled, until the new answer.
    assert!(html.contains("Could not verify workspace access"), "{html}");
    // Only the error state's own "Try again" has this class.
    assert!(
        html.contains(r#"class="btn btn-secondary is-disabled""#),
        "{html}"
    );

    response
        .send(Ok(Access::new(false, &[Permission::CompanyWrite])))
        .unwrap_or_else(|_| panic!("retry request was dropped"));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Test workspace"));
    assert!(html.contains("Administrative panel"));
    // The retry re-reads the whole session, the workspace name included.
    assert_eq!(probe.org_requests.get(), 2);
    assert_eq!(probe.panel_mounts.get(), 1);
}

mod server_fns {
    use super::*;
    use own_permissions_model::OwnPermissions;
    use user_model::CurrentUser;

    fn own(
        catalog_version: u32,
        is_administrator: bool,
        grants: Vec<Permission>,
    ) -> OwnPermissions {
        OwnPermissions {
            catalog_version,
            grants,
            is_administrator,
            profile: None,
            access_revision: 1,
            person_revision: 1,
            managed_person_ids: Vec::new(),
            managed_project_ids: Vec::new(),
        }
    }

    pub async fn get_week_start() -> Result<chrono::Weekday, ServerFnError> {
        if consume_context::<Probe>().week_start_error {
            return Err(ServerFnError::new("week start unavailable"));
        }
        Ok(chrono::Weekday::Mon)
    }

    pub async fn get_my_permissions() -> Result<OwnPermissions, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe
            .canonical_requests
            .set(probe.canonical_requests.get() + 1);
        if probe.own_error {
            return Err(ServerFnError::new("private projection error"));
        }
        let version = horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION
            + u32::from(probe.future_catalog);
        if let Some((is_administrator, grants)) = probe.canonical.clone() {
            return Ok(own(version, is_administrator, grants));
        }
        let reply = probe
            .responses
            .borrow_mut()
            .pop_front()
            .expect("unexpected permission read");
        reply
            .await
            .expect("test must resolve the permission read")
            .map(|access| own(version, access.administrator, access.grants))
    }

    /// The shell decides from the permissions alone; the identity answers at once.
    pub async fn get_me() -> Result<CurrentUser, ServerFnError> {
        let probe = consume_context::<Probe>();
        probe.me_requests.set(probe.me_requests.get() + 1);
        if let Some(code) = probe.me_fails.get() {
            return Err(ServerFnError::ServerError {
                message: "private identity diagnostic".into(),
                code,
                details: None,
            });
        }
        Ok(CurrentUser {
            id: uuid::Uuid::nil(),
            org_id: uuid::Uuid::nil(),
            email: "admin@example.test".into(),
            name: "Admin".into(),
            timezone: None,
            workspace_timezone: "UTC".into(),
            utc_offset_seconds: 0,
        })
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
        #[route("/timesheet?:view&:date&:span&:user")]
        Timesheet {
            view: ViewMode,
            date: Anchor,
            span: CalSpan,
            user: String,
        },
        #[layout(AdminShell)]
        #[route("/admin/workspace")]
        WorkspaceSettings {},
        #[route("/admin/importers")]
        ImporterIndex {},
        #[route("/admin/audit")]
        PermissionAudit {},
        #[route("/admin/api-tokens")]
        AdminApiTokens {},
    }

    pub fn route_is_active(to: &Route) -> bool {
        std::mem::discriminant(&use_route::<Route>()) == std::mem::discriminant(to)
    }

    #[component]
    fn Timesheet(view: ViewMode, date: Anchor, span: CalSpan, user: String) -> Element {
        rsx! { "Timesheet" }
    }

    #[component]
    fn WorkspaceSettings() -> Element {
        use_hook(|| {
            let probe = consume_context::<Probe>();
            probe.panel_mounts.set(probe.panel_mounts.get() + 1);
            probe.panel.set(Some((
                navigator(),
                dioxus::core::current_scope_id(),
                consume_context::<super::session::Session>(),
            )));
        });
        rsx! { "Administrative panel" }
    }

    #[component]
    fn ImporterIndex() -> Element {
        use_hook(|| {
            consume_context::<Probe>().panel.set(Some((
                navigator(),
                dioxus::core::current_scope_id(),
                consume_context::<super::session::Session>(),
            )));
        });
        rsx! { "Importer panel" }
    }

    #[component]
    fn PermissionAudit() -> Element {
        rsx! { "Permission audit panel" }
    }

    #[component]
    fn AdminApiTokens() -> Element {
        rsx! { "API tokens panel" }
    }
}

#[tokio::test]
async fn audit_route_follows_the_administrator_identity() {
    for allowed in [true, false] {
        let probe = Probe {
            audit: true,
            ..Probe::default()
        };
        let response = probe.request();
        let mut dom = start(&probe);
        assert_panel_hidden(&dom, &probe);
        response
            .send(Ok(Access::new(allowed, &[])))
            .unwrap_or_else(|_| panic!("audit request dropped"));
        settle(&mut dom);
        assert_eq!(probe.canonical_requests.get(), 1);
        assert_eq!(
            dioxus::ssr::render(&dom).contains("Permission audit panel"),
            allowed
        );
    }
}

#[tokio::test]
async fn api_tokens_route_and_link_follow_the_administrator_identity() {
    for allowed in [true, false] {
        let probe = Probe {
            api_tokens: true,
            ..Probe::default()
        };
        let response = probe.request();
        let mut dom = start(&probe);
        assert_panel_hidden(&dom, &probe);
        // Every grant, so only the administrator identity decides.
        response
            .send(Ok(Access::new(allowed, Permission::ALL)))
            .unwrap_or_else(|_| panic!("API tokens request dropped"));
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert_eq!(html.contains("API tokens panel"), allowed, "{html}");
        assert_eq!(html.contains("/admin/api-tokens"), allowed, "{html}");
    }
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
async fn importers_follow_the_administrator_identity() {
    for allowed in [false, true] {
        let probe = Probe {
            importers: true,
            ..Probe::default()
        };
        let reply = probe.request();
        let mut dom = start(&probe);
        // Every grant short of the identity is not enough.
        let grants = if allowed { &[][..] } else { Permission::ALL };
        reply
            .send(Ok(Access::new(allowed, grants)))
            .unwrap_or_else(|_| panic!("permission read dropped"));
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert_eq!(html.contains("Importer panel"), allowed, "{html}");
    }
}

#[tokio::test]
async fn importers_close_when_the_permissions_cannot_be_read() {
    let probe = Probe {
        importers: true,
        own_error: true,
        ..Probe::default()
    };
    let dom = start(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains("Importer panel"), "{html}");
    assert!(html.contains("Could not verify workspace access"), "{html}");
    assert!(!html.contains("private projection error"));
}

#[tokio::test]
async fn failed_or_future_projection_hides_the_panel() {
    for future_catalog in [false, true] {
        let probe = Probe {
            own_error: !future_catalog,
            future_catalog,
            canonical: Some((true, vec![Permission::CompanyWrite])),
            ..Probe::default()
        };
        let dom = start(&probe);
        assert_panel_hidden(&dom, &probe);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Could not verify workspace access"));
        assert!(!html.contains("private projection error"));
    }
}

fn people_and_tasks_admin() -> (Probe, VirtualDom) {
    let probe = Probe::default();
    let reply = probe.request();
    let mut dom = start(&probe);
    reply
        .send(Ok(Access::new(
            true,
            &[Permission::CompanyWrite, Permission::TaskReadAll],
        )))
        .unwrap_or_else(|_| panic!("permission read dropped"));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Administrative panel"), "{html}");
    // Tasks lives in the main rail, even for someone who may read it.
    assert!(!html.contains("Tasks"), "{html}");
    (probe, dom)
}

#[tokio::test]
async fn switching_sections_keeps_the_workspace_and_reads_the_session_once() {
    let (probe, mut dom) = people_and_tasks_admin();
    for (to, panel) in [
        (route::Route::ImporterIndex {}, "Importer panel"),
        (route::Route::WorkspaceSettings {}, "Administrative panel"),
    ] {
        // Navigate from whichever panel is open; the previous one has unmounted.
        let (navigator, scope, _) = probe.panel.get().expect("a panel is mounted");
        dom.in_scope(scope, || {
            navigator.push(to);
        });
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains(panel), "{panel}: {html}");
        assert!(html.contains("Test workspace"), "{panel}: {html}");
        assert!(!html.contains("Loading"), "{html}");
    }
    assert_eq!(probe.me_requests.get(), 1);
    assert_eq!(probe.canonical_requests.get(), 1);
    // The workspace header stayed mounted across both section changes.
    assert_eq!(probe.org_requests.get(), 1);
}

#[tokio::test]
async fn a_session_refresh_keeps_the_open_panel_until_the_answer_arrives() {
    let (probe, mut dom) = people_and_tasks_admin();
    let (_, scope, session) = probe.panel.get().expect("the Settings panel mounted");
    let reply = probe.request();
    dom.in_scope(scope, || session.refresh());
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Administrative panel"), "{html}");
    assert!(html.contains("Test workspace"), "{html}");
    reply
        .send(Ok(Access::new(
            true,
            &[Permission::CompanyWrite, Permission::TaskReadAll],
        )))
        .unwrap_or_else(|_| panic!("refresh request dropped"));
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Administrative panel"), "{html}");
    assert!(html.contains("Test workspace"), "{html}");
    assert_eq!(probe.me_requests.get(), 2);
    assert_eq!(probe.canonical_requests.get(), 2);
    assert_eq!(probe.panel_mounts.get(), 1);
    assert_eq!(probe.org_requests.get(), 2);
}
