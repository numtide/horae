use super::*;
use dioxus::history::{History, MemoryHistory};
use dioxus::router::components::HistoryProvider;
use std::rc::Rc;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
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
    access_content(&response, retry)
}

fn render(response: Response) -> String {
    let mut dom = VirtualDom::new_with_props(
        |response: Response| {
            use_context_provider(|| response);
            let history = use_callback(|_| {
                Rc::new(MemoryHistory::default()) as Rc<dyn History>
            });
            rsx! {
                HistoryProvider { history, Router::<Host> {} }
            }
        },
        response,
    );
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

fn row(grants: &[Permission], area: &str) -> [Cell; 3] {
    access_rows(grants)
        .into_iter()
        .find(|(name, _)| *name == area)
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
        row(&grants, "Time"),
        [Some(Some(Managed)), Some(Some(Own)), Some(Some(Managed))]
    );
    assert_eq!(row(&grants, "Clients"), [Some(Some(All)), Some(None), None]);
    assert_eq!(row(&grants, "Invoices"), [Some(None), Some(None), None]);
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
    assert!(html.contains("You can see and change everything here"), "{html}");
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
