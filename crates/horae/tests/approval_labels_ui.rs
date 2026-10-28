#![cfg(feature = "server")]

//! Render the production approval page with controlled record responses.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use chrono::{NaiveDate, Weekday};
use dioxus::core::Mutation;
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus::router::Navigator;
use dioxus::router::components::HistoryProvider;
use futures_util::FutureExt;
use horae_core::types::OrgRole;
use horae_core::week::week_bounds;
use uuid::Uuid;

#[path = "../src/models/approval.rs"]
pub mod approval_model;
#[path = "../src/components/avatar.rs"]
pub mod avatar;
#[path = "../src/components/badge.rs"]
pub mod badge;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/table.rs"]
pub mod table;
#[path = "../src/models/user.rs"]
pub mod user_model;
mod components {
    pub use super::{avatar, badge, controls, table};
}
#[path = "../src/pages/approvals.rs"]
mod approvals;

/// The one route the page links to, so it can render inside a real router.
mod route {
    use super::approvals::Approvals;
    use dioxus::prelude::*;

    #[derive(Routable, Clone, PartialEq, Debug)]
    pub enum Route {
        #[route("/approvals?:week")]
        Approvals { week: String },
    }
}

#[derive(Clone, Default)]
struct Probe {
    rows: Vec<approval_model::ApprovalSummary>,
    manager: bool,
    pending: bool,
    error: Option<String>,
    /// The `?week=` the page is opened with.
    week: String,
    sunday: bool,
    week_start_error: Option<String>,
    /// Weeks whose list request never completes.
    pending_weeks: Vec<String>,
    /// Approve actions never complete.
    action_pending: bool,
    /// Overrides the count returned by bulk approval.
    approved: Option<usize>,
    directory_reads: Rc<Cell<usize>>,
    requests: Rc<RefCell<Vec<String>>>,
    actions: Rc<RefCell<Vec<Vec<String>>>>,
    /// The page's router and scope, captured on its first list request.
    navigator: Rc<Cell<Option<(Navigator, ScopeId)>>>,
}

fn is_manager(me: &Resource<Result<user_model::CurrentUser, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(user)) if user.is_manager_or_above())
}

fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(value)) => render(value),
        Some(Err(error)) => rsx! { div { class: "alert alert-danger", "{error}" } },
        None => rsx! { div { class: "text-muted text-sm", "Loading…" } },
    }
}

fn app(probe: Probe) -> Element {
    let path = format!("/approvals?week={}", probe.week);
    use_context_provider(|| probe);
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
            Router::<route::Route> {}
        }
    }
}

fn settle(dom: &mut VirtualDom) -> Vec<Mutation> {
    let mut edits = Vec::new();
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return edits;
        }
        edits.extend(dom.render_immediate_to_vec().edits);
    }
    panic!("approval resources did not settle");
}

fn render(probe: &Probe) -> (VirtualDom, Vec<Mutation>) {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let mut edits = dom.rebuild_to_vec().edits;
    edits.extend(settle(&mut dom));
    (dom, edits)
}

fn html(dom: &VirtualDom) -> String {
    dioxus::ssr::render(dom)
}

/// Navigate the rendered page to another `?week=`, through its own router.
/// The push runs in the page's scope because the router reads its history
/// from the surrounding context.
fn go_to(dom: &mut VirtualDom, probe: &Probe, week: &str) {
    let (navigator, scope) = probe
        .navigator
        .get()
        .expect("the page has requested a list");
    let week = week.to_string();
    dom.in_scope(scope, || {
        navigator.push(route::Route::Approvals { week });
    });
    settle(dom);
}

fn click(dom: &mut VirtualDom, listener: dioxus::core::ElementId) {
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<
            dioxus_html::SerializedMouseData,
        >::default())) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event("click", event, listener);
    settle(dom);
}

fn click_listeners(edits: Vec<Mutation>) -> Vec<dioxus::core::ElementId> {
    edits
        .into_iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "click" => Some(id),
            _ => None,
        })
        .collect()
}

fn summary(
    name: &str,
    start: &str,
    end: &str,
    minutes: i64,
    state: &str,
) -> approval_model::ApprovalSummary {
    serde_json::from_value(serde_json::json!({
        "approval": {"id":Uuid::now_v7(),"org_id":Uuid::now_v7(),"user_id":Uuid::now_v7(),
            "period_start":start,"period_end":end,"state":state,
            "submitted_at":"2026-09-27T12:00:00Z","approved_by":null,"approved_at":null},
        "user_name":name,"total_minutes":minutes,"billable_minutes":minutes / 2
    }))
    .unwrap()
}

fn row(name: &str) -> approval_model::ApprovalSummary {
    let mut row = summary(name, "2026-09-21", "2026-09-27", 90, "submitted");
    row.billable_minutes = 60;
    row
}

fn manager(rows: Vec<approval_model::ApprovalSummary>) -> Probe {
    Probe {
        manager: true,
        rows,
        week: "2026-09-21".into(),
        ..Default::default()
    }
}

fn this_week(first_day: Weekday) -> NaiveDate {
    week_bounds(chrono::Utc::now().date_naive(), first_day)
        .unwrap()
        .0
}

#[tokio::test]
async fn approval_names_render_from_rows_without_directory_and_escape_markup() {
    let probe = manager(vec![
        row("Álvaro <script> & 王"),
        row("Álvaro <script> & 王"),
    ]);
    let (dom, _) = render(&probe);
    let html = html(&dom);
    assert_eq!(probe.directory_reads.get(), 0);
    assert_eq!(
        html.matches("Álvaro &#60;script&#62; &#38; 王").count(),
        2,
        "{html}"
    );
    assert!(!html.contains("<script>"));
    assert!(!html.contains(&probe.rows[0].approval.user_id.to_string()));
    assert!(html.contains("1:30"));
    assert!(probe.actions.borrow().is_empty());
}

#[tokio::test]
async fn approval_loading_error_empty_and_denied_states_need_no_directory() {
    for (pending, error, manager, expected) in [
        (true, None, true, "Loading"),
        (
            false,
            Some("Approvals unavailable".to_owned()),
            true,
            "Approvals unavailable",
        ),
        (false, None, true, "No approvals found"),
        (false, None, false, "Manager or admin access is required"),
    ] {
        let probe = Probe {
            pending,
            error,
            manager,
            week: "2026-09-21".into(),
            ..Default::default()
        };
        let (dom, _) = render(&probe);
        assert!(html(&dom).contains(expected));
        assert_eq!(probe.directory_reads.get(), 0);
        assert!(probe.actions.borrow().is_empty());
    }
}

#[tokio::test]
async fn single_approval_action_keeps_the_record_id_not_the_person_label() {
    dioxus::html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = manager(vec![row("Same name")]);
    let (mut dom, edits) = render(&probe);
    let clicks = click_listeners(edits);
    // Three pager links and three status filters, then the row's
    // Reopen/Approve and the bulk Approve.
    assert_eq!(clicks.len(), 9);
    click(&mut dom, clicks[7]);
    assert_eq!(
        *probe.actions.borrow(),
        vec![vec![probe.rows[0].approval.id.to_string()]]
    );
    assert_eq!(probe.directory_reads.get(), 0);
}

// T-U1
#[tokio::test]
async fn each_row_names_its_own_week_including_across_a_year() {
    let probe = manager(vec![
        summary("A", "2026-07-13", "2026-07-19", 60, "submitted"),
        summary("B", "2026-07-20", "2026-07-26", 60, "submitted"),
        summary("C", "2026-12-28", "2027-01-03", 60, "approved"),
    ]);
    let (dom, _) = render(&probe);
    let html = html(&dom);
    for label in [
        "13 Jul – 19 Jul 2026",
        "20 Jul – 26 Jul 2026",
        "28 Dec 2026 – 03 Jan 2027",
    ] {
        assert!(html.contains(label), "{label}: {html}");
    }
}

// T-U2
#[tokio::test]
async fn any_date_in_the_address_selects_its_configured_week() {
    for (sunday, expected) in [(false, "2026-10-26"), (true, "2026-10-25")] {
        let probe = Probe {
            week: "2026-10-29".into(),
            sunday,
            ..manager(vec![])
        };
        render(&probe);
        assert_eq!(*probe.requests.borrow(), vec![expected.to_string()]);
    }
}

// T-U2
#[tokio::test]
async fn a_missing_or_unparseable_week_selects_the_current_week() {
    for week in ["", "garbage"] {
        let before = this_week(Weekday::Mon);
        let probe = Probe {
            week: week.into(),
            ..manager(vec![])
        };
        let (dom, _) = render(&probe);
        let after = this_week(Weekday::Mon);
        assert_eq!(before, after, "the test straddled a UTC date change");
        assert_eq!(
            *probe.requests.borrow(),
            vec![before.to_string()],
            "{week:?}"
        );
        assert!(
            html(&dom).contains("<span class=\"cur\">This week</span>"),
            "{week:?}"
        );
    }
}

// T-U2b
#[tokio::test]
async fn an_unrepresentable_week_shows_an_error_and_requests_nothing() {
    let probe = Probe {
        week: NaiveDate::MAX.to_string(),
        ..manager(vec![])
    };
    let (dom, _) = render(&probe);
    let html = html(&dom);
    assert!(
        html.contains("This week is outside the supported date range."),
        "{html}"
    );
    assert!(probe.requests.borrow().is_empty());
    // Neither direction is offered; only the way back to the current week.
    let this_week = format!("href=\"/approvals?week={}\"", this_week(Weekday::Mon));
    assert_eq!(html.matches("href=\"/approvals?week=").count(), 1, "{html}");
    assert!(html.contains(&this_week), "{html}");
}

// T-U3
#[tokio::test]
async fn the_pager_links_to_the_adjacent_and_current_weeks() {
    let probe = Probe {
        week: "2026-10-19".into(),
        ..manager(vec![])
    };
    let (dom, _) = render(&probe);
    let html = html(&dom);
    for target in [
        "/approvals?week=2026-10-12".to_string(),
        "/approvals?week=2026-10-26".to_string(),
        format!("/approvals?week={}", this_week(Weekday::Mon)),
    ] {
        assert!(
            html.contains(&format!("href=\"{target}\"")),
            "{target}: {html}"
        );
    }
}

// T-U4
#[tokio::test]
async fn totals_are_the_exact_sum_of_the_shown_rows() {
    let probe = manager(vec![
        summary("A", "2026-09-21", "2026-09-27", 450, "submitted"),
        summary("B", "2026-09-21", "2026-09-27", 135, "approved"),
    ]);
    let (dom, _) = render(&probe);
    assert!(html(&dom).contains("9:45"));
}

// T-U5
#[tokio::test]
async fn approve_visible_submits_exactly_the_shown_pending_approvals() {
    dioxus::html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = manager(vec![
        summary("A", "2026-09-21", "2026-09-27", 60, "submitted"),
        summary("B", "2026-09-21", "2026-09-27", 60, "approved"),
        summary("C", "2026-09-21", "2026-09-27", 60, "submitted"),
    ]);
    let (mut dom, edits) = render(&probe);
    let bulk = *click_listeners(edits).last().unwrap();
    click(&mut dom, bulk);
    let expected = vec![
        probe.rows[0].approval.id.to_string(),
        probe.rows[2].approval.id.to_string(),
    ];
    assert_eq!(*probe.actions.borrow(), vec![expected]);
}

// T-U6
#[tokio::test]
async fn a_previous_weeks_rows_never_show_under_another_week() {
    let probe = Probe {
        pending_weeks: vec!["2026-09-28".into()],
        ..manager(vec![row("Previous week person")])
    };
    let (mut dom, _) = render(&probe);
    assert!(html(&dom).contains("Previous week person"));
    go_to(&mut dom, &probe, "2026-09-28");
    let html = html(&dom);
    assert!(!html.contains("Previous week person"), "{html}");
    assert!(html.contains("Loading"), "{html}");
}

// T-U7
#[tokio::test]
async fn every_action_is_disabled_while_one_is_in_flight() {
    dioxus::html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe {
        action_pending: true,
        ..manager(vec![row("A"), row("B")])
    };
    let (mut dom, edits) = render(&probe);
    let bulk = *click_listeners(edits).last().unwrap();
    click(&mut dom, bulk);
    let html = html(&dom);
    let buttons: Vec<&str> = html
        .split("<button")
        .skip(1)
        .filter(|tag| tag.trim_start().starts_with("class=\"btn"))
        .map(|tag| tag.split('>').next().unwrap())
        .collect();
    assert_eq!(
        buttons.len(),
        5,
        "two Reopen, two Approve, one bulk: {html}"
    );
    assert!(buttons.iter().all(|tag| tag.contains("disabled")), "{html}");
}

// T-U8
#[tokio::test]
async fn no_list_is_requested_without_manager_access_or_a_week_start() {
    let member = Probe {
        manager: false,
        ..manager(vec![])
    };
    let (dom, _) = render(&member);
    assert!(html(&dom).contains("Manager or admin access is required"));
    assert!(member.requests.borrow().is_empty());

    let unconfigured = Probe {
        week_start_error: Some("Week start unavailable".into()),
        ..manager(vec![])
    };
    let (dom, _) = render(&unconfigured);
    assert!(html(&dom).contains("Week start unavailable"));
    assert!(unconfigured.requests.borrow().is_empty());
}

// T-U9
#[tokio::test]
async fn bulk_approval_reports_the_servers_count_for_its_week_only() {
    dioxus::html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe {
        approved: Some(2),
        ..manager(vec![row("A"), row("B"), row("C")])
    };
    let (mut dom, edits) = render(&probe);
    let bulk = *click_listeners(edits).last().unwrap();
    click(&mut dom, bulk);
    assert!(html(&dom).contains("Approved 2 of 3 timesheets"));
    go_to(&mut dom, &probe, "2026-09-28");
    assert!(!html(&dom).contains("Approved 2 of 3 timesheets"));
}

pub mod server_fns {
    use super::*;

    pub async fn get_me() -> Result<user_model::CurrentUser, ServerFnError> {
        let probe = use_context::<Probe>();
        Ok(user_model::CurrentUser {
            id: Uuid::now_v7(),
            org_id: Uuid::now_v7(),
            name: "Reviewer".into(),
            email: "reviewer@example.test".into(),
            org_role: if probe.manager {
                OrgRole::Manager
            } else {
                OrgRole::Member
            },
        })
    }

    pub async fn get_week_start() -> Result<Weekday, ServerFnError> {
        let probe = use_context::<Probe>();
        match probe.week_start_error {
            Some(error) => Err(ServerFnError::new(error)),
            None if probe.sunday => Ok(Weekday::Sun),
            None => Ok(Weekday::Mon),
        }
    }

    pub async fn list_approvals(
        _: Option<String>,
        week_start: String,
    ) -> Result<Vec<approval_model::ApprovalSummary>, ServerFnError> {
        let probe = use_context::<Probe>();
        probe
            .navigator
            .set(Some((navigator(), dioxus::core::current_scope_id())));
        probe.requests.borrow_mut().push(week_start.clone());
        if probe.pending || probe.pending_weeks.contains(&week_start) {
            std::future::pending::<()>().await;
        }
        if let Some(error) = probe.error {
            return Err(ServerFnError::new(error));
        }
        Ok(probe.rows)
    }

    pub async fn list_users(_: bool) -> Result<Vec<user_model::UserListItem>, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.directory_reads.set(probe.directory_reads.get() + 1);
        Err(ServerFnError::new("Directory must not be needed"))
    }

    pub async fn approve_submission(id: String) -> Result<approval_model::Approval, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.actions.borrow_mut().push(vec![id]);
        if probe.action_pending {
            std::future::pending::<()>().await;
        }
        Ok(probe.rows[0].approval.clone())
    }

    pub async fn approve_submissions(ids: Vec<String>) -> Result<usize, ServerFnError> {
        let probe = use_context::<Probe>();
        let count = probe.approved.unwrap_or(ids.len());
        probe.actions.borrow_mut().push(ids);
        if probe.action_pending {
            std::future::pending::<()>().await;
        }
        Ok(count)
    }

    pub async fn reject_submission(id: String) -> Result<(), ServerFnError> {
        use_context::<Probe>().actions.borrow_mut().push(vec![id]);
        Ok(())
    }
}
