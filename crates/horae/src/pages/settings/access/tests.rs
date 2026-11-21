use super::*;
use dioxus::history::{History, MemoryHistory};
use dioxus::router::components::HistoryProvider;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use std::rc::Rc;
use uuid::Uuid;

fn member() -> OwnPermissions {
    OwnPermissions {
        catalog_version: PERMISSION_CATALOG_VERSION,
        grants: vec![
            Permission::TimeReadOwn,
            Permission::TimeWriteOwn,
            Permission::TimeReadManaged,
            Permission::TimeApproveManaged,
            Permission::ClientReadAll,
            Permission::ReportProfitabilityRead,
        ],
        is_administrator: false,
        profile: Some("Manager".into()),
        access_revision: 928314,
        person_revision: 817243,
        managed_person_ids: vec![Uuid::now_v7()],
        managed_project_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
    }
}

type Response = Option<Result<OwnPermissions, ServerFnError>>;

/// A one-page router, so the content's `Link`s have a router without
/// mounting the app shell and its session reads.
#[derive(Routable, Clone, PartialEq)]
enum Host {
    #[route("/")]
    Content {},
}

#[component]
fn Content() -> Element {
    let response = use_context::<Response>();
    let retry = use_callback(|_| {});
    access_page(Tr::default(), &response, retry, false)
}

fn render(response: Response) -> String {
    let mut dom = VirtualDom::new_with_props(
        |response: Response| {
            use_context_provider(|| response);
            let history = use_callback(|_| Rc::new(MemoryHistory::default()) as Rc<dyn History>);
            rsx! {
                HistoryProvider { history, Router::<Host> {} }
            }
        },
        response,
    );
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

fn row(grants: &[Permission], area: Area) -> [Cell; 3] {
    access_rows(grants)
        .into_iter()
        .find(|(row, _)| *row == area)
        .map(|(_, cells)| cells)
        .unwrap()
}

#[test]
fn every_permission_has_exactly_one_place_on_the_page() {
    for permission in Permission::ALL {
        let in_table = AREAS
            .iter()
            .flat_map(|(_, view, edit, approve)| [*view, *edit, *approve])
            .flatten()
            .filter(|(grant, _)| grant == permission)
            .count();
        let in_other = OTHER.iter().filter(|grant| *grant == permission).count();
        assert_eq!(in_table + in_other, 1, "{permission:?}");
    }
}

#[test]
fn a_cell_shows_the_widest_scope_granted() {
    let grants = member().grants;
    use Scope::*;
    assert_eq!(
        row(&grants, Area::Time),
        [Some(Some(Managed)), Some(Some(Own)), Some(Some(Managed))]
    );
    assert_eq!(
        row(&grants, Area::Clients),
        [Some(Some(All)), Some(None), None]
    );
    assert_eq!(row(&grants, Area::Invoices), [Some(None), Some(None), None]);
}

#[test]
fn a_member_sees_the_table_and_who_changes_it() {
    let html = render(Some(Ok(member())));
    assert!(html.contains("Your access by area"), "{html}");
    assert!(
        html.contains("ask an administrator of this workspace"),
        "{html}"
    );
    assert!(html.contains("View profitability reports"), "{html}");
    assert!(html.contains("People you manage"), "{html}");
    assert!(html.contains("Projects you manage"), "{html}");
    // The audit log is for administrators only.
    assert!(!html.contains("See changes to your access"), "{html}");
}

#[test]
fn an_administrator_sees_one_sentence_and_no_table() {
    let mut own = member();
    own.is_administrator = true;
    let html = render(Some(Ok(own)));
    assert!(!html.contains("<table"), "{html}");
    assert!(
        html.contains("You can see and change everything here"),
        "{html}"
    );
    assert!(html.contains("href=\"/admin/users\""), "{html}");
    assert!(html.contains("href=\"/admin/audit\""), "{html}");
    assert!(html.contains("See changes to your access"), "{html}");
    assert!(html.contains("People you manage"), "{html}");
}

#[test]
fn the_page_has_no_refresh_notice_or_internal_fields() {
    let own = member();
    let ids: Vec<_> = own
        .managed_person_ids
        .iter()
        .chain(&own.managed_project_ids)
        .copied()
        .collect();
    let html = render(Some(Ok(own)));
    for id in ids {
        assert!(!html.contains(&id.to_string()), "{html}");
    }
    for absent in [
        "928314",
        "817243",
        "Refresh",
        "banner-warning",
        "Only an Administrator",
        "<input",
        "<ul",
        "style=",
    ] {
        assert!(!html.contains(absent), "unexpected {absent}: {html}");
    }
}

#[test]
fn failures_name_what_to_do_without_private_details() {
    for (code, expected) in [
        (401, "Sign in again"),
        (403, "Ask an administrator"),
        (500, "Try again"),
    ] {
        let html = render(Some(Err(ServerFnError::ServerError {
            code,
            message: "private diagnostic".into(),
            details: None,
        })));
        assert!(html.contains("Could not load your access"), "{html}");
        assert!(html.contains(expected), "{html}");
        assert!(!html.contains("private diagnostic"), "{html}");
    }
}

#[test]
fn an_unknown_catalog_hides_grants_it_cannot_read() {
    let mut own = member();
    own.catalog_version += 1;
    let html = render(Some(Ok(own)));
    assert!(html.contains("Could not load your access"), "{html}");
    assert!(!html.contains("<table"), "{html}");
}

#[test]
fn only_the_first_load_shows_loading() {
    assert!(render(None).contains("Loading your access"));
}

/// The badge beside the title: `PageHeader` puts it in `page-header-meta`.
fn badge(html: &str) -> Option<&str> {
    let meta = &html[html.find("page-header")?..];
    let start = meta.find("badge-sm\">")? + "badge-sm\">".len();
    Some(&meta[start..start + meta[start..].find('<')?])
}

#[test]
fn the_title_badge_names_the_profile() {
    let mut own = member();
    assert_eq!(badge(&render(Some(Ok(own.clone())))), Some("Manager"));
    own.profile = None;
    assert_eq!(
        badge(&render(Some(Ok(own.clone())))),
        Some("Individual configuration")
    );
    own.is_administrator = true;
    assert_eq!(badge(&render(Some(Ok(own)))), Some("Administrator"));
}

#[test]
fn no_badge_above_an_error_or_an_unknown_catalog() {
    let mut own = member();
    own.is_administrator = true;
    own.catalog_version += 1;
    let html = render(Some(Ok(own)));
    assert!(html.contains("Could not load your access"), "{html}");
    assert_eq!(badge(&html), None, "{html}");
    let html = render(Some(Err(ServerFnError::new("down"))));
    assert_eq!(badge(&html), None, "{html}");
}

#[test]
fn a_column_that_does_not_apply_is_read_as_words() {
    let html = render(Some(Ok(member())));
    assert!(
        html.contains(r#"<span class="text-muted" aria-hidden="true">—</span><span class="sr-only">Does not apply</span>"#),
        "{html}"
    );
    assert!(!html.contains("aria-label=\"Does not apply\""), "{html}");
}

type Reply = Result<OwnPermissions, ServerFnError>;
type Replies =
    Rc<std::cell::RefCell<std::collections::VecDeque<tokio::sync::oneshot::Receiver<Reply>>>>;
type Handle = Rc<std::cell::Cell<Option<(ScopeId, Resource<Reply>)>>>;

#[derive(Routable, Clone, PartialEq)]
enum LiveHost {
    #[route("/")]
    Live {},
}

/// The page over a resource the test answers and restarts, the way the
/// session's permission read behaves.
#[component]
fn Live() -> Element {
    let replies = use_context::<Replies>();
    let handle = use_context::<Handle>();
    let mut access = use_resource(move || {
        let next = replies.borrow_mut().pop_front().expect("a queued reply");
        async move { next.await.unwrap() }
    });
    handle.set(Some((dioxus::core::current_scope_id(), access)));
    let retry = use_callback(move |_| access.restart());
    let pending = access.state()() != UseResourceState::Ready;
    access_page(Tr::default(), &access.read(), retry, pending)
}

struct LiveHarness {
    dom: VirtualDom,
    replies: Replies,
    handle: Handle,
}

impl LiveHarness {
    fn new() -> (Self, tokio::sync::oneshot::Sender<Reply>) {
        let replies = Replies::default();
        let handle = Handle::default();
        let first = Self::queue(&replies);
        let mut dom = VirtualDom::new_with_props(
            |(replies, handle): (Replies, Handle)| {
                use_context_provider(|| replies);
                use_context_provider(|| handle);
                let history =
                    use_callback(|_| Rc::new(MemoryHistory::default()) as Rc<dyn History>);
                rsx! {
                    HistoryProvider { history, Router::<LiveHost> {} }
                }
            },
            (replies.clone(), handle.clone()),
        );
        dom.rebuild_in_place();
        let mut harness = Self {
            dom,
            replies,
            handle,
        };
        harness.settle();
        (harness, first)
    }

    fn queue(replies: &Replies) -> tokio::sync::oneshot::Sender<Reply> {
        let (send, receive) = tokio::sync::oneshot::channel();
        replies.borrow_mut().push_back(receive);
        send
    }

    fn settle(&mut self) {
        use futures_util::FutureExt;
        for _ in 0..20 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            self.dom.render_immediate_to_vec();
        }
        panic!("the page did not settle");
    }

    /// Restarts the read, as `session.refresh()` does, with `reply` queued.
    fn refresh(&mut self) -> tokio::sync::oneshot::Sender<Reply> {
        let reply = Self::queue(&self.replies);
        let (scope, mut access) = self.handle.get().expect("the page mounted");
        self.dom.in_scope(scope, || access.restart());
        self.settle();
        reply
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }
}

#[tokio::test]
async fn a_refresh_keeps_the_access_on_screen_and_a_failed_one_shows_the_error() {
    let (mut page, first) = LiveHarness::new();
    assert!(page.html().contains("Loading your access"));
    first.send(Ok(member())).unwrap();
    page.settle();
    assert!(page.html().contains("Your access by area"));

    let reply = page.refresh();
    let html = page.html();
    assert!(html.contains("Your access by area"), "{html}");
    assert!(!html.contains("Loading your access"), "{html}");

    reply.send(Err(ServerFnError::new("down"))).unwrap();
    page.settle();
    let html = page.html();
    assert!(html.contains("Could not load your access"), "{html}");
    assert!(!html.contains("Your access by area"), "{html}");
    assert!(!html.contains("is-disabled"), "{html}");

    // While its retry runs, "Try again" stays up and disabled.
    let reply = page.refresh();
    assert!(page.html().contains("is-disabled"), "{}", page.html());
    reply.send(Ok(member())).unwrap();
    page.settle();
    assert!(page.html().contains("Your access by area"));
}
