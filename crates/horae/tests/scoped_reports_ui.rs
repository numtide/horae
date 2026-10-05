#![cfg(feature = "server")]

//! Exercise the production Reports route with controlled server responses.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::core::Mutation;
use dioxus::prelude::*;
use futures_util::FutureExt;
use models::time_report::{TimeReportAccess, TimeReportPolicy};
use tokio::sync::oneshot;

// These production modules also define types for unrelated routes.
#[allow(dead_code)]
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
#[allow(dead_code)]
#[path = "../src/models/time_report.rs"]
pub mod time_report;
#[allow(dead_code)]
#[path = "../src/models/user.rs"]
pub mod user;
mod models {
    pub use super::{permission_editor, time_report, user};
    // Only the legacy endpoints are stubbed; canonical wire types are production imports.
    pub struct CatalogItem {
        pub id: uuid::Uuid,
        pub tag_id: uuid::Uuid,
        pub name: String,
    }
    pub type Client = CatalogItem;
    pub type Project = CatalogItem;
    pub type ProjectTagLink = CatalogItem;
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct ReportRow {
        pub group_id: uuid::Uuid,
        pub label: String,
        pub total_minutes: i64,
        pub rounded_minutes: i64,
        pub billable_minutes: i64,
        pub billable_cents: i64,
        pub cost_cents: Option<i64>,
        pub currency: String,
        pub cost_currency: String,
    }
    pub struct DetailedReportRow {
        pub spent_date: chrono::NaiveDate,
        pub project_name: String,
        pub task_name: String,
        pub user_name: String,
        pub minutes: i32,
        pub rounded_minutes: Option<i32>,
        pub billable: bool,
        pub notes: Option<String>,
    }
}
#[allow(dead_code)]
#[path = "../src/components/badge.rs"]
pub mod badge;
#[allow(dead_code)]
#[path = "../src/components/form.rs"]
pub mod form;
#[path = "../src/components/table.rs"]
pub mod table;
mod components {
    pub use super::{badge, form, table};
}
#[path = "../src/pages/reports.rs"]
mod reports;

type AccessResponse = Result<TimeReportAccess, ServerFnError>;
type IdentityResponse = Result<models::user::CurrentUser, ServerFnError>;
type ReportResponse = Result<models::time_report::TimeReportPage, ServerFnError>;
type GroupResponse = Result<models::time_report::TimeReportGroupPage, ServerFnError>;

#[derive(Clone, Default)]
struct Probe {
    access: Rc<RefCell<VecDeque<oneshot::Receiver<AccessResponse>>>>,
    identity: Rc<RefCell<VecDeque<oneshot::Receiver<IdentityResponse>>>>,
    reports: Rc<RefCell<VecDeque<oneshot::Receiver<ReportResponse>>>>,
    groups: Rc<RefCell<VecDeque<oneshot::Receiver<GroupResponse>>>>,
    group_queries: Rc<RefCell<Vec<time_report::TimeReportGroupQuery>>>,
    access_reads: Rc<Cell<usize>>,
    identity_reads: Rc<Cell<usize>>,
    legacy_reads: Rc<RefCell<Vec<&'static str>>>,
    queries: Rc<RefCell<Vec<models::time_report::TimeReportQuery>>>,
    access_bindings: Rc<RefCell<Vec<Option<permission_editor::PermissionRequester>>>>,
}

fn queue<T>(receivers: &RefCell<VecDeque<oneshot::Receiver<T>>>) -> oneshot::Sender<T> {
    let (send, receive) = oneshot::channel();
    receivers.borrow_mut().push_back(receive);
    send
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { reports::Reports {} }
}

fn settle(dom: &mut VirtualDom) -> Vec<Mutation> {
    let mut edits = Vec::new();
    for _ in 0..30 {
        if dom.wait_for_work().now_or_never().is_none() {
            return edits;
        }
        edits.extend(dom.render_immediate_to_vec().edits);
    }
    panic!("Reports resources did not settle");
}

fn mount(probe: &Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

fn access(policy: TimeReportPolicy) -> TimeReportAccess {
    TimeReportAccess {
        policy,
        requester: permission_editor::PermissionRequester {
            org_id: uuid::Uuid::now_v7(),
            user_id: uuid::Uuid::now_v7(),
        },
    }
}

#[tokio::test]
async fn unresolved_access_mounts_no_identity_catalog_or_report_resources() {
    let probe = Probe::default();
    let _access = queue(&probe.access);
    let _identity = queue(&probe.identity);
    let dom = mount(&probe);
    assert!(
        probe.legacy_reads.borrow().is_empty(),
        "{:?}",
        probe.legacy_reads
    );
    assert_eq!(probe.identity_reads.get(), 0);
    assert_eq!(probe.access_reads.get(), 1);
    assert!(probe.queries.borrow().is_empty());
    assert!(dioxus::ssr::render(&dom).contains("Loading report access"));
}

#[tokio::test]
async fn denied_or_failed_access_never_falls_back_to_legacy() {
    const FORBIDDEN: u16 = 403;
    const INTERNAL_ERROR: u16 = 500;
    for code in [FORBIDDEN, INTERNAL_ERROR] {
        let probe = Probe::default();
        let send = queue(&probe.access);
        let _identity = queue(&probe.identity);
        let mut dom = mount(&probe);
        send.send(Err(ServerFnError::ServerError {
            code,
            message: "private diagnostic".into(),
            details: None,
        }))
        .unwrap();
        settle(&mut dom);
        assert!(probe.legacy_reads.borrow().is_empty());
        assert_eq!(probe.identity_reads.get(), 0);
        assert!(probe.queries.borrow().is_empty());
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Report access is unavailable"), "{html}");
        assert!(!html.contains("/api/reports/export/"));
        assert!(!html.contains("private diagnostic"));
    }
}

#[tokio::test]
async fn legacy_catalogs_wait_for_explicit_authenticated_legacy_access() {
    let probe = Probe::default();
    let own = queue(&probe.access);
    let mut dom = mount(&probe);
    assert!(probe.legacy_reads.borrow().is_empty());
    own.send(Ok(access(TimeReportPolicy::Legacy))).unwrap();
    settle(&mut dom);
    assert_eq!(probe.legacy_reads.borrow().len(), 6);
    assert!(dioxus::ssr::render(&dom).contains("Export CSV"));
    assert_eq!(probe.identity_reads.get(), 0);
    assert!(probe.queries.borrow().is_empty());
}

fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(value)) => render(value),
        Some(Err(error)) => rsx! { p { "{error}" } },
        None => rsx! { p { "Loading…" } },
    }
}

fn target(dom: &VirtualDom, vnode: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::{AttributeValue, DynamicNode, TemplateAttribute, TemplateNode};
    for (index, path) in vnode.template.attr_paths.iter().enumerate() {
        let mut node = &vnode.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = &children[usize::from(*child)];
        }
        let static_id = matches!(node, TemplateNode::Element { attrs, .. }
            if attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name: "id", value, .. } if *value == name)));
        let dynamic_id = vnode.dynamic_attrs[index].iter().any(|attr| {
            attr.name == "id" && matches!(&attr.value, AttributeValue::Text(value) if value == name)
        });
        if static_id || dynamic_id {
            return vnode.mounted_dynamic_attribute(index, dom);
        }
    }
    for (index, node) in vnode.dynamic_nodes.iter().enumerate() {
        let found = match node {
            DynamicNode::Component(component) => component
                .mounted_scope(index, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .and_then(|node| target(dom, node, name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|node| target(dom, node, name)),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

fn event(dom: &mut VirtualDom, name: &str, kind: &str, data: Box<dyn std::any::Any>) {
    set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let id = target(dom, dom.base_scope().root_node(), name).expect("missing event target");
    dom.runtime().handle_event(
        kind,
        Event::new(
            Rc::new(PlatformEventData::new(data)) as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
    settle(dom);
}

fn click(dom: &mut VirtualDom, name: &str) {
    event(dom, name, "click", Box::<SerializedMouseData>::default());
}

fn input(dom: &mut VirtualDom, name: &str, value: &str) {
    event(
        dom,
        name,
        "input",
        Box::new(SerializedFormData::new(value.into(), vec![])),
    );
}

fn report(allowed: TimeReportAccess, name: &str) -> time_report::TimeReportPage {
    let entry = time_report::TimeReportEntry {
        id: uuid::Uuid::now_v7(),
        spent_date: chrono::Utc::now().date_naive(),
        project_name: name.into(),
        task_name: "Task".into(),
        user_name: "Person".into(),
        minutes: 60,
        rounded_minutes: 60,
        billable: true,
        notes: Some("<private & note>".into()),
    };
    time_report::TimeReportPage {
        requester: allowed.requester,
        next_after: Some(time_report::TimeReportCursor {
            spent_date: entry.spent_date,
            project_name: entry.project_name.clone(),
            task_name: entry.task_name.clone(),
            id: entry.id,
        }),
        entries: vec![entry],
        totals: time_report::TimeReportTotals {
            entry_count: 503,
            total_minutes: 30180,
            rounded_minutes: 30180,
            billable_minutes: 30180,
        },
    }
}

fn assert_no_results(dom: &VirtualDom) {
    let html = dioxus::ssr::render(dom);
    for hidden in [
        "Old project",
        "Full-period totals",
        "/api/reports/export/",
        "/api/reports/time/grouped/xlsx",
        "<tbody",
    ] {
        assert!(!html.contains(hidden), "stale {hidden}: {html}");
    }
}

fn grouped(allowed: TimeReportAccess, name: &str) -> time_report::TimeReportGroupPage {
    let totals = report(allowed, "unused").totals;
    let group = time_report::TimeReportGroup {
        id: uuid::Uuid::now_v7(),
        name: name.into(),
        totals: time_report::TimeReportTotals {
            entry_count: 7,
            total_minutes: 419,
            rounded_minutes: 420,
            billable_minutes: 360,
        },
    };
    time_report::TimeReportGroupPage {
        requester: allowed.requester,
        next_after: Some(time_report::TimeReportGroupCursor {
            group_by: time_report::TimeReportGrouping::Client,
            name: group.name.clone(),
            id: group.id,
        }),
        groups: vec![group],
        totals,
    }
}

#[tokio::test]
async fn grouped_tabs_reauthorize_and_hour_drilldown_binds_detail_and_download_filters() {
    use time_report::TimeReportGrouping as Grouping;
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    first.send(Ok(report(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let clients = queue(&probe.groups);
    click(&mut dom, "report-view-time");
    assert_no_results(&dom);
    assert_eq!(probe.group_queries.borrow()[0].group_by, Grouping::Client);
    assert_eq!(
        probe.group_queries.borrow()[0].expected_requester,
        Some(allowed.requester)
    );
    clients.send(Ok(grouped(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("503.00") && html.contains("7.00") && html.contains("6.00"));
    assert!(html.contains("/api/reports/time/grouped/xlsx?group_by=client"));
    assert!(html.contains(&format!("expected_user_id={}", allowed.requester.user_id)));
    assert!(!html.contains("after="));
    assert!(
        !html.contains("/api/reports/export/"),
        "grouped view must not mislabel detailed downloads"
    );
    let projects = queue(&probe.groups);
    click(&mut dom, "report-group-project");
    assert_no_results(&dom);
    let mut page = grouped(allowed, "<Project & name>");
    page.next_after = None;
    let project_id = page.groups[0].id;
    projects.send(Ok(page)).unwrap();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("&#60;Project &#38; name&#62;"));
    let detail = queue(&probe.reports);
    click(&mut dom, &format!("report-hours-{project_id}"));
    assert_no_results(&dom);
    let query = probe.queries.borrow().last().unwrap().clone();
    assert_eq!(query.project_ids, vec![project_id]);
    assert!(query.client_ids.is_empty() && query.user_ids.is_empty() && query.task_ids.is_empty());
    assert!(query.after.is_none());
    detail
        .send(Ok(report(allowed, "Selected project")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains(&format!("project_ids={project_id}"))
            && html.contains("expected_policy=scoped")
    );
    assert!(probe.legacy_reads.borrow().is_empty());
    assert_eq!(probe.identity_reads.get(), 0);
}

#[tokio::test]
async fn grouped_navigation_hides_stale_rows_and_rejects_changed_identity() {
    use time_report::TimeReportGrouping as Grouping;
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    first.send(Ok(report(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let clients = queue(&probe.groups);
    click(&mut dom, "report-view-time");
    clients.send(Ok(grouped(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let pending = queue(&probe.groups);
    click(&mut dom, "report-group-next");
    assert_no_results(&dom);
    assert!(probe.group_queries.borrow().last().unwrap().after.is_some());
    let tasks = queue(&probe.groups);
    click(&mut dom, "report-group-task");
    assert_no_results(&dom);
    let query = probe.group_queries.borrow().last().unwrap().clone();
    assert_eq!(query.group_by, Grouping::Task);
    assert!(query.after.is_none());
    let _ = pending.send(Ok(grouped(allowed, "Old project")));
    settle(&mut dom);
    assert_no_results(&dom);
    tasks
        .send(Ok(grouped(access(TimeReportPolicy::Scoped), "Old project")))
        .unwrap();
    settle(&mut dom);
    assert_no_results(&dom);
    assert!(dioxus::ssr::render(&dom).contains("Your session changed"));
    input(&mut dom, "report-from", "");
    assert_eq!(probe.group_queries.borrow().len(), 3);
    assert_no_results(&dom);
    assert!(probe.legacy_reads.borrow().is_empty());
}

#[tokio::test]
async fn grouped_period_changes_preserve_dimension_reset_cursor_and_replace_ready_data() {
    use time_report::TimeReportGrouping as Grouping;
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    first.send(Ok(report(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let client = queue(&probe.groups);
    click(&mut dom, "report-view-time");
    client.send(Ok(grouped(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let person = queue(&probe.groups);
    click(&mut dom, "report-group-person");
    let mut person_page = grouped(allowed, "Old project");
    person_page.next_after.as_mut().unwrap().group_by = Grouping::Person;
    person.send(Ok(person_page.clone())).unwrap();
    settle(&mut dom);
    let second = queue(&probe.groups);
    click(&mut dom, "report-group-next");
    second.send(Ok(person_page)).unwrap();
    settle(&mut dom);
    assert!(probe.group_queries.borrow().last().unwrap().after.is_some());
    let changed = queue(&probe.groups);
    input(&mut dom, "report-to", "2099-12-31");
    assert_no_results(&dom);
    let query = probe.group_queries.borrow().last().unwrap().clone();
    assert_eq!(query.date_to.to_string(), "2099-12-31");
    assert_eq!(query.group_by, Grouping::Person);
    assert!(query.after.is_none());
    let mut changed_page = grouped(allowed, "New period");
    changed_page.next_after = None;
    changed.send(Ok(changed_page)).unwrap();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("New period"));
    assert!(!dioxus::ssr::render(&dom).contains("Old project"));
}

#[tokio::test]
async fn canonical_pages_use_current_identity_and_full_period_totals_without_legacy_reads() {
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    assert_no_results(&dom);
    assert_eq!(
        probe.queries.borrow()[0].expected_requester,
        Some(allowed.requester)
    );
    let page = report(allowed, "Old project");
    let cursor = page.next_after.clone();
    first.send(Ok(page)).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("503.00"),
        "must use full-period totals: {html}"
    );
    assert!(html.contains("&#60;private &#38; note&#62;"), "{html}");
    assert!(html.contains(&format!("expected_user_id={}", allowed.requester.user_id)));
    assert!(!html.contains("after="));
    assert!(html.contains("expected_policy=scoped"));
    assert!(probe.legacy_reads.borrow().is_empty());
    assert_eq!(probe.identity_reads.get(), 0);

    let second = queue(&probe.reports);
    click(&mut dom, "report-next");
    assert_no_results(&dom);
    assert_eq!(probe.queries.borrow()[1].after, cursor);
    let mut last = report(allowed, "Last project");
    last.next_after = None;
    second.send(Ok(last)).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(!html.contains("Old project"));
    assert!(html.contains("Last project") && html.contains("503.00"));

    let third = queue(&probe.reports);
    click(&mut dom, "report-previous");
    assert_no_results(&dom);
    assert!(probe.queries.borrow()[2].after.is_none());
    third
        .send(Err(ServerFnError::new("private backend details")))
        .unwrap();
    settle(&mut dom);
    assert_no_results(&dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Could not load the report"));
    assert!(!html.contains("private backend details"));

    let retry = queue(&probe.reports);
    click(&mut dom, "report-refresh");
    let mut empty = report(allowed, "unused");
    empty.entries.clear();
    empty.next_after = None;
    retry.send(Ok(empty)).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("No entries on this page") && html.contains("503.00"));
    assert!(html.contains("Export CSV"));
}

#[tokio::test]
async fn date_changes_reset_paging_hide_stale_results_and_do_not_fetch_invalid_ranges() {
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    first.send(Ok(report(allowed, "Old project"))).unwrap();
    settle(&mut dom);
    let pending = queue(&probe.reports);
    click(&mut dom, "report-next");
    input(&mut dom, "report-from", "");
    assert_no_results(&dom);
    assert_eq!(probe.queries.borrow().len(), 2);
    assert!(dioxus::ssr::render(&dom).contains("Enter a valid start date"));
    // The cancelled page cannot restore data after the selection has changed.
    let _ = pending.send(Ok(report(allowed, "Old project")));
    settle(&mut dom);
    assert_no_results(&dom);

    input(&mut dom, "report-from", "2099-12-31");
    assert_eq!(probe.queries.borrow().len(), 2);
    assert!(dioxus::ssr::render(&dom).contains("end date must be on or after"));
    let corrected = queue(&probe.reports);
    input(&mut dom, "report-to", "2099-12-31");
    assert_no_results(&dom);
    let query = probe.queries.borrow().last().unwrap().clone();
    assert_eq!(query.date_from, query.date_to);
    assert!(query.after.is_none());
    assert_eq!(query.expected_requester, Some(allowed.requester));
    corrected
        .send(Ok(report(allowed, "Corrected project")))
        .unwrap();
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("from=2099-12-31&#38;to=2099-12-31"));
}

#[tokio::test]
async fn refresh_keeps_requester_and_policy_across_errors_and_child_remounts() {
    for change_policy in [false, true] {
        let probe = Probe::default();
        let own = queue(&probe.access);
        let first = queue(&probe.reports);
        let allowed = access(TimeReportPolicy::Scoped);
        let mut dom = mount(&probe);
        own.send(Ok(allowed)).unwrap();
        settle(&mut dom);
        first.send(Ok(report(allowed, "Old project"))).unwrap();
        settle(&mut dom);
        let revoked = queue(&probe.access);
        click(&mut dom, "reports-retry-access");
        assert_no_results(&dom);
        assert_eq!(probe.access_bindings.borrow()[1], Some(allowed.requester));
        revoked
            .send(Err(ServerFnError::new("private revocation")))
            .unwrap();
        settle(&mut dom);
        assert_no_results(&dom);
        let changed = queue(&probe.access);
        click(&mut dom, "reports-retry-access");
        let other = if change_policy {
            TimeReportAccess {
                policy: TimeReportPolicy::Legacy,
                ..allowed
            }
        } else {
            access(TimeReportPolicy::Scoped)
        };
        changed.send(Ok(other)).unwrap();
        settle(&mut dom);
        assert_no_results(&dom);
        assert!(dioxus::ssr::render(&dom).contains("session or report policy changed"));
        assert!(probe.legacy_reads.borrow().is_empty());
        assert_eq!(probe.queries.borrow().len(), 1);

        let restored = queue(&probe.access);
        let refreshed = queue(&probe.reports);
        click(&mut dom, "reports-retry-access");
        restored.send(Ok(allowed)).unwrap();
        settle(&mut dom);
        assert_eq!(
            probe.queries.borrow()[1].expected_requester,
            Some(allowed.requester)
        );
        refreshed
            .send(Ok(report(allowed, "Restored project")))
            .unwrap();
        settle(&mut dom);
        assert!(dioxus::ssr::render(&dom).contains("Restored project"));
    }
}

#[tokio::test]
async fn a_mismatched_page_identity_never_displays_results_or_downloads() {
    let probe = Probe::default();
    let own = queue(&probe.access);
    let first = queue(&probe.reports);
    let allowed = access(TimeReportPolicy::Scoped);
    let mut dom = mount(&probe);
    own.send(Ok(allowed)).unwrap();
    settle(&mut dom);
    first
        .send(Ok(report(access(TimeReportPolicy::Scoped), "Old project")))
        .unwrap();
    settle(&mut dom);
    assert_no_results(&dom);
    assert!(dioxus::ssr::render(&dom).contains("Your session changed"));
}

#[allow(dead_code)]
mod server_fns {
    use super::*;

    pub async fn list_visible_time_report_groups(
        query: time_report::TimeReportGroupQuery,
    ) -> GroupResponse {
        let probe = use_context::<Probe>();
        probe.group_queries.borrow_mut().push(query);
        let response = probe
            .groups
            .borrow_mut()
            .pop_front()
            .expect("unexpected grouped report read");
        response.await.expect("group sender dropped")
    }

    pub async fn get_time_report_access(
        expected: Option<permission_editor::PermissionRequester>,
    ) -> AccessResponse {
        let probe = use_context::<Probe>();
        probe.access_bindings.borrow_mut().push(expected);
        probe.access_reads.set(probe.access_reads.get() + 1);
        let response = probe
            .access
            .borrow_mut()
            .pop_front()
            .expect("unexpected access read");
        response.await.expect("access sender dropped")
    }

    pub async fn get_me() -> IdentityResponse {
        let probe = use_context::<Probe>();
        probe.identity_reads.set(probe.identity_reads.get() + 1);
        let response = probe
            .identity
            .borrow_mut()
            .pop_front()
            .expect("unexpected identity read");
        response.await.expect("identity sender dropped")
    }

    pub async fn list_visible_time_report_entries(
        query: models::time_report::TimeReportQuery,
    ) -> ReportResponse {
        let probe = use_context::<Probe>();
        probe.queries.borrow_mut().push(query);
        let response = probe
            .reports
            .borrow_mut()
            .pop_front()
            .expect("unexpected report read");
        response.await.expect("report sender dropped")
    }

    fn legacy<T>(name: &'static str) -> Result<Vec<T>, ServerFnError> {
        use_context::<Probe>().legacy_reads.borrow_mut().push(name);
        Ok(vec![])
    }
    pub async fn list_clients(_: bool) -> Result<Vec<models::Client>, ServerFnError> {
        legacy("clients")
    }
    pub async fn list_users(_: bool) -> Result<Vec<models::user::UserListItem>, ServerFnError> {
        legacy("users")
    }
    pub async fn list_project_tags() -> Result<Vec<models::ProjectTagLink>, ServerFnError> {
        legacy("tags")
    }
    pub async fn list_projects(
        _: Option<String>,
        _: bool,
    ) -> Result<Vec<models::Project>, ServerFnError> {
        legacy("projects")
    }
    pub async fn report_time(
        _: String,
        _: String,
        _: String,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
    ) -> Result<Vec<models::ReportRow>, ServerFnError> {
        legacy("summary")
    }
    pub async fn report_detailed(
        _: String,
        _: String,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
    ) -> Result<Vec<models::DetailedReportRow>, ServerFnError> {
        legacy("detailed")
    }
}
