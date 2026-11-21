use super::*;
use chrono::NaiveDateTime;
use dioxus::history::{History, MemoryHistory};
use dioxus::router::components::HistoryProvider;
use std::rc::Rc;

const ADMIN: Uuid = Uuid::from_u128(1);
const MARIA: Uuid = Uuid::from_u128(2);
const JAMIE: Uuid = Uuid::from_u128(3);
const PROJECT: Uuid = Uuid::from_u128(4);
const TEMPLATE: Uuid = Uuid::from_u128(5);

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()
}

fn page(entries: Vec<AuditEntry>, more: bool) -> AuditPage {
    let next_after = more.then(|| AuditCursor {
        created_at: entries.last().unwrap().created_at,
        id: entries.last().unwrap().id,
    });
    AuditPage {
        requester: PermissionRequester {
            org_id: Uuid::from_u128(9),
            user_id: ADMIN,
        },
        entries,
        next_after,
        time_zone: "Europe/Madrid".into(),
        today: today(),
        people: [
            (ADMIN, "Admin User"),
            (MARIA, "Maria Ruiz"),
            (JAMIE, "Jamie Chen"),
        ]
        .map(|(id, name)| AuditName {
            id,
            name: name.into(),
        })
        .into(),
        projects: vec![AuditName {
            id: PROJECT,
            name: "Website Redesign".into(),
        }],
        profiles: vec![AuditName {
            id: TEMPLATE,
            name: "Team lead".into(),
        }],
    }
}

fn entry(local: &str, audit: HistoricalAudit) -> AuditEntry {
    let local: NaiveDateTime = local.parse().unwrap();
    AuditEntry {
        id: Uuid::now_v7(),
        actor: AuditPrincipal::User { user_id: ADMIN },
        created_at: local.and_utc() - chrono::Duration::hours(1),
        local,
        audit,
    }
}

fn snapshot(grants: &[Permission], source: HistoricalSource, admin: bool) -> PersonSnapshot {
    PersonSnapshot {
        grants: grants.to_vec(),
        is_administrator: admin,
        source,
        revision: 1,
    }
}

fn person(before: PersonSnapshot, after: PersonSnapshot) -> HistoricalAudit {
    HistoricalAudit::Profile(ProfileAudit {
        user_id: MARIA,
        previous_access_revision: 1,
        access_revision: 2,
        change: Some(ProfileChange {
            catalog_version: 1,
            before,
            after,
            removed_projects: vec![],
            removed_people: vec![],
        }),
    })
}

fn time_change() -> HistoricalAudit {
    use Permission::*;
    person(
        snapshot(
            &[TimeReadAll, TimeWriteAll, ClientReadAll],
            HistoricalSource::Individual,
            false,
        ),
        snapshot(
            &[
                TimeReadAll,
                TimeWriteManaged,
                TimeApproveManaged,
                ClientReadAll,
            ],
            HistoricalSource::Individual,
            false,
        ),
    )
}

fn managers(added: &[Uuid], removed: &[Uuid]) -> HistoricalAudit {
    let designations = |ids: &[Uuid]| {
        ids.iter()
            .map(|&manager_id| Designation {
                id: Uuid::now_v7(),
                manager_id,
                revision: 1,
            })
            .collect()
    };
    HistoricalAudit::ProjectManagers(ProjectAudit {
        project_id: PROJECT,
        previous_access_revision: 1,
        access_revision: 2,
        change: Some(ProjectChange {
            added: designations(added),
            removed: designations(removed),
        }),
    })
}

fn template(created: bool) -> HistoricalAudit {
    let snapshot = TemplateSnapshot {
        id: TEMPLATE,
        name: "Team lead".into(),
        catalog_version: 1,
        grants: vec![Permission::TimeReadOwn, Permission::TimeWriteOwn],
        revision: 1,
    };
    let (before, after) = if created {
        (None, Some(snapshot))
    } else {
        (Some(snapshot), None)
    };
    HistoricalAudit::Template(TemplateAudit {
        previous_access_revision: 1,
        access_revision: 2,
        before,
        after,
        detached_people: vec![],
    })
}

/// What `audit_log` is rendered with.
#[derive(Clone)]
struct View {
    filter: AuditFilter,
    shown: Option<Shown>,
    failure: Option<(bool, ServerFnError)>,
    pending: bool,
}

impl View {
    fn loaded(page: AuditPage) -> Self {
        Self {
            filter: AuditFilter::default(),
            shown: Some(Shown {
                filter: AuditFilter::default(),
                page,
            }),
            failure: None,
            pending: false,
        }
    }
}

/// A one-page router, so the log's `Link`s have a router.
#[derive(Routable, Clone, PartialEq)]
enum Host {
    #[route("/")]
    Content {},
}

#[component]
fn Content() -> Element {
    let view = use_context::<View>();
    let load_more = use_callback(|_: MouseEvent| {});
    let retry = use_callback(|_| {});
    audit_log(
        Tr::default(),
        view.filter,
        view.shown.as_ref(),
        view.failure.as_ref().map(|(more, error)| (*more, error)),
        view.pending,
        load_more,
        retry,
    )
}

fn render(view: View) -> String {
    let mut dom = VirtualDom::new_with_props(
        |view: View| {
            use_context_provider(|| view);
            let history = use_callback(|_| Rc::new(MemoryHistory::default()) as Rc<dyn History>);
            rsx! { HistoryProvider { history, Router::<Host> {} } }
        },
        view,
    );
    dom.rebuild_in_place();
    dioxus::ssr::render(&dom)
}

/// The page's text without markup, for reading sentences that hold links.
fn text(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn server(code: u16) -> ServerFnError {
    ServerFnError::ServerError {
        code,
        message: "private diagnostic".into(),
        details: None,
    }
}

#[test]
fn each_kind_of_event_reads_as_a_sentence() {
    use Permission::*;
    let at = "2026-11-20T10:00:00";
    let cases = [
        (
            time_change(),
            "Admin User changed the permissions of Maria Ruiz",
        ),
        (
            person(
                snapshot(&[TimeReadOwn], HistoricalSource::Individual, false),
                snapshot(
                    &[TimeReadOwn],
                    HistoricalSource::Template {
                        id: TEMPLATE,
                        applied_revision: 1,
                    },
                    false,
                ),
            ),
            "Admin User moved Maria Ruiz to the Team lead profile",
        ),
        (
            person(
                snapshot(
                    &[TimeReadOwn],
                    HistoricalSource::BuiltIn(BuiltInProfile::Member),
                    false,
                ),
                snapshot(&[TimeReadOwn], HistoricalSource::Individual, false),
            ),
            "Admin User gave Maria Ruiz their own permissions",
        ),
        (
            person(
                snapshot(&[TimeReadOwn], HistoricalSource::Individual, false),
                snapshot(
                    &[TimeReadAll],
                    HistoricalSource::BuiltIn(BuiltInProfile::Administrator),
                    true,
                ),
            ),
            "Admin User made Maria Ruiz an administrator",
        ),
        (
            person(
                snapshot(
                    &[TimeReadAll],
                    HistoricalSource::BuiltIn(BuiltInProfile::Administrator),
                    true,
                ),
                snapshot(&[TimeReadOwn], HistoricalSource::Individual, false),
            ),
            "Admin User removed Maria Ruiz as an administrator",
        ),
        (
            HistoricalAudit::Profile(ProfileAudit {
                user_id: MARIA,
                previous_access_revision: 2,
                access_revision: 2,
                change: None,
            }),
            "Admin User saved the permissions of Maria Ruiz without changes",
        ),
        (
            template(true),
            "Admin User created the profile Team lead with 2 permissions",
        ),
        (template(false), "Admin User deleted the profile Team lead"),
        (
            managers(&[JAMIE], &[]),
            "Admin User made Jamie Chen a manager of Website Redesign",
        ),
        (
            managers(&[], &[JAMIE]),
            "Admin User removed Jamie Chen as a manager of Website Redesign",
        ),
        (
            managers(&[JAMIE], &[MARIA]),
            "Admin User changed the managers of Website Redesign",
        ),
    ];
    for (audit, expected) in cases {
        let html = render(View::loaded(page(vec![entry(at, audit)], false)));
        assert!(
            text(&html).contains(expected),
            "{expected}: {}",
            text(&html)
        );
    }
}

#[test]
fn names_link_to_the_log_filtered_to_them_and_projects_to_their_page() {
    let html = render(View::loaded(page(
        vec![entry("2026-11-20T10:00:00", managers(&[JAMIE], &[]))],
        false,
    )));
    assert!(
        html.contains(&format!("href=\"/admin/audit?person={ADMIN}\"")),
        "{html}"
    );
    assert!(
        html.contains(&format!("href=\"/admin/audit?person={JAMIE}\"")),
        "{html}"
    );
    assert!(
        html.contains(&format!("href=\"/projects/{PROJECT}\"")),
        "{html}"
    );
}

#[test]
fn a_name_link_keeps_the_other_filters() {
    let filter = AuditFilter {
        kind: Some(AuditKind::Managers),
        period: Some(AuditPeriod::Days30),
        person: None,
    };
    let mut view = View::loaded(page(
        vec![entry("2026-11-20T10:00:00", managers(&[JAMIE], &[]))],
        false,
    ));
    view.filter = filter;
    view.shown.as_mut().unwrap().filter = filter;
    let html = render(view);
    assert!(
        html.contains(&format!(
            "href=\"/admin/audit?kind=managers&#38;person={JAMIE}&#38;period=30d\""
        )),
        "{html}"
    );
}

#[test]
fn an_operator_is_named_by_its_command_without_a_link() {
    let mut operated = entry("2026-11-20T10:00:00", template(true));
    operated.actor = AuditPrincipal::Operator {
        invocation_id: "invocation".into(),
        command: "horae migrate".into(),
    };
    let html = render(View::loaded(page(vec![operated], false)));
    assert!(
        text(&html).contains("An operator (horae migrate) created the profile"),
        "{html}"
    );
    assert!(!html.contains("invocation"), "{html}");
}

#[test]
fn events_are_grouped_by_the_viewers_day_with_local_times() {
    let entries = vec![
        entry("2026-11-20T19:42:00", time_change()),
        entry("2026-11-20T18:05:00", managers(&[JAMIE], &[])),
        entry("2026-11-19T11:20:00", template(true)),
        entry("2026-11-18T09:14:00", time_change()),
    ];
    let html = render(View::loaded(page(entries, false)));
    let page_text = text(&html);
    let order: Vec<_> = [
        "Today",
        "19:42",
        "18:05",
        "Yesterday",
        "11:20",
        "18 Nov 2026",
        "09:14",
    ]
    .iter()
    .map(|part| {
        page_text
            .find(part)
            .unwrap_or_else(|| panic!("{part}: {page_text}"))
    })
    .collect();
    assert!(order.is_sorted(), "{order:?}");
    assert_eq!(html.matches("class=\"audit-day\"").count(), 3);
    assert!(page_text.contains("Times in your time zone (Europe/Madrid)."));
    assert!(!page_text.contains("UTC"));
}

#[test]
fn the_detail_shows_only_the_changed_areas_with_the_moved_cells_marked() {
    let row = entry("2026-11-20T10:00:00", time_change());
    let id = row.id;
    let html = render(View::loaded(page(vec![row], false)));
    let page_text = text(&html);
    // Edit fell from All to Managed; Approve rose from No to Managed.
    assert!(page_text.contains("All → Managed"), "{page_text}");
    assert!(page_text.contains("No → Managed"), "{page_text}");
    assert_eq!(html.matches("access-change-down").count(), 1, "{html}");
    assert_eq!(html.matches("access-change-up").count(), 1, "{html}");
    // Time changed; Clients did not.
    assert!(page_text.contains("Time"));
    assert!(!page_text.contains("Clients"), "{page_text}");
    assert!(page_text.contains("Only the areas that changed."));
    assert!(page_text.contains(&format!("Receipt {id}")));
    assert!(page_text.contains("Copy"));
    // The folded row summarises the change.
    assert!(page_text.contains("+2"), "{page_text}");
    assert!(page_text.contains("−1"), "{page_text}");
    assert!(page_text.contains("2 added, 1 removed"), "{page_text}");
}

#[test]
fn the_detail_lists_managers_and_grants_outside_the_table() {
    use Permission::*;
    let audit = person(
        snapshot(&[TimeReadOwn], HistoricalSource::Individual, false),
        snapshot(
            &[TimeReadOwn, ReportProfitabilityRead],
            HistoricalSource::Individual,
            false,
        ),
    );
    let html = render(View::loaded(page(
        vec![
            entry("2026-11-20T10:00:00", audit),
            entry("2026-11-20T09:00:00", managers(&[JAMIE], &[MARIA])),
        ],
        false,
    )));
    let page_text = text(&html);
    assert!(
        page_text.contains("Now: View profitability reports"),
        "{page_text}"
    );
    assert!(
        page_text.contains("Jamie Chen became a manager"),
        "{page_text}"
    );
    assert!(
        page_text.contains("Maria Ruiz is no longer a manager"),
        "{page_text}"
    );
    // Neither change moved a cell of the table.
    assert!(!html.contains("<table"), "{html}");
}

#[test]
fn load_more_shows_only_when_there_is_more() {
    let rows = || vec![entry("2026-11-20T10:00:00", time_change())];
    assert!(render(View::loaded(page(rows(), true))).contains("id=\"audit-load-more\""));
    let html = render(View::loaded(page(rows(), false)));
    assert!(!html.contains("audit-load-more"), "{html}");
    for gone in ["Refresh", "Newest events", "Older events"] {
        assert!(!html.contains(gone), "{gone}");
    }
}

#[test]
fn an_empty_log_says_what_to_do_next() {
    let html = render(View::loaded(page(vec![], false)));
    let page_text = text(&html);
    assert!(
        page_text.contains("No permission changes yet"),
        "{page_text}"
    );
    assert!(html.contains("href=\"/people\""), "{html}");
    assert!(!page_text.contains("Clear filters"));
}

#[test]
fn a_filter_that_matches_nothing_offers_to_clear_it() {
    let filter = AuditFilter::person(MARIA);
    let mut view = View::loaded(page(vec![], false));
    view.filter = filter;
    view.shown.as_mut().unwrap().filter = filter;
    let html = render(view);
    let page_text = text(&html);
    assert!(
        page_text.contains("No changes match these filters"),
        "{page_text}"
    );
    assert!(page_text.contains("Clear filters"));
    assert!(html.contains("id=\"audit-clear-filters\""), "{html}");
    // The router writes an empty query as a bare `?`.
    assert!(html.contains("href=\"/admin/audit?\""), "{html}");
    // The person filter names whom it shows.
    assert!(page_text.contains("Maria Ruiz"), "{page_text}");
}

#[test]
fn the_first_load_shows_grouped_skeleton_rows_under_live_filters() {
    let html = render(View {
        filter: AuditFilter::default(),
        shown: None,
        failure: None,
        pending: true,
    });
    assert!(html.contains("Loading permission changes"), "{html}");
    assert!(html.contains("skeleton-group"), "{html}");
    assert!(html.contains("id=\"audit-kind-menu-trigger\""), "{html}");
    // Without the names, the person filter waits.
    let person = html.find("id=\"audit-person-menu-trigger\"").unwrap();
    assert!(
        html[person..]
            .split('>')
            .next()
            .unwrap()
            .contains("disabled"),
        "{html}"
    );
    assert!(!html.contains("Times in your time zone"));
    assert!(!html.contains("audit-load-more"));
}

#[test]
fn a_reload_keeps_the_rows_dimmed() {
    let row = entry("2026-11-20T10:00:00", time_change());
    let id = row.id.to_string();
    let mut view = View::loaded(page(vec![row], true));
    view.filter = AuditFilter {
        kind: Some(AuditKind::Person),
        ..AuditFilter::default()
    };
    view.pending = true;
    let html = render(view);
    assert!(html.contains(&id), "{html}");
    assert!(
        html.contains("class=\"audit-content\" aria-busy=true"),
        "{html}"
    );
    assert!(!html.contains("skeleton"), "{html}");
}

#[test]
fn a_refusal_leaves_only_the_header_and_the_state() {
    let html = render(View {
        filter: AuditFilter::default(),
        shown: None,
        failure: Some((false, server(403))),
        pending: false,
    });
    assert!(html.contains("Access unavailable"), "{html}");
    assert!(!html.contains("audit-kind-menu"), "{html}");
    assert!(!html.contains("private diagnostic"), "{html}");
}

#[test]
fn failures_name_what_to_do_without_private_details() {
    for (code, expected) in [(401, "Sign in again"), (500, "Try again")] {
        let html = render(View {
            filter: AuditFilter::default(),
            shown: None,
            failure: Some((false, server(code))),
            pending: false,
        });
        assert!(html.contains(expected), "{html}");
        assert!(!html.contains("private diagnostic"), "{html}");
        assert!(!html.contains("skeleton"), "{html}");
    }
}

#[test]
fn a_failed_load_more_keeps_the_rows_above_its_error() {
    let row = entry("2026-11-20T10:00:00", time_change());
    let id = row.id.to_string();
    let mut view = View::loaded(page(vec![row], true));
    view.failure = Some((true, server(500)));
    let html = render(view);
    assert!(html.contains(&id), "{html}");
    assert!(html.contains("Try again"), "{html}");
    assert!(!html.contains("audit-load-more"), "{html}");
}

#[test]
fn the_next_page_appends_and_a_new_filter_replaces() {
    let first = entry("2026-11-20T10:00:00", time_change());
    let second = entry("2026-11-19T10:00:00", time_change());
    let all = AuditFilter::default();
    let mut shown = None;
    let newest = page(vec![first.clone()], true);
    let cursor = newest.next_after;
    merge(&mut shown, all, None, false, newest);
    // Later pages carry no people; the first page's stay.
    let mut older = page(vec![second.clone()], false);
    older.people.clear();
    merge(&mut shown, all, cursor, false, older);
    let current = shown.as_ref().unwrap();
    assert_eq!(current.page.entries, vec![first.clone(), second]);
    assert_eq!(current.page.next_after, None);
    assert_eq!(current.page.people.len(), 3);
    let filter = AuditFilter::person(MARIA);
    merge(
        &mut shown,
        filter,
        cursor,
        false,
        page(vec![first.clone()], false),
    );
    let current = shown.unwrap();
    assert_eq!(current.filter, filter);
    assert_eq!(current.page.entries, vec![first]);
}

#[test]
fn another_accounts_page_replaces_even_after_a_cursor() {
    let first = entry("2026-11-20T10:00:00", time_change());
    let all = AuditFilter::default();
    let mut shown = None;
    let newest = page(vec![first], true);
    let cursor = newest.next_after;
    merge(&mut shown, all, None, false, newest);
    let theirs = entry("2026-11-19T10:00:00", template(true));
    merge(
        &mut shown,
        all,
        cursor,
        true,
        page(vec![theirs.clone()], false),
    );
    assert_eq!(shown.unwrap().page.entries, vec![theirs]);
}

/// "Load more" under one filter, then another filter, then back: the old
/// cursor must not show the first filter's later pages as its newest.
#[test]
fn a_cursor_from_an_earlier_visit_to_a_filter_is_not_reused() {
    let persons = AuditFilter {
        kind: Some(AuditKind::Person),
        ..AuditFilter::default()
    };
    let managers = AuditFilter {
        kind: Some(AuditKind::Managers),
        ..AuditFilter::default()
    };
    let mut shown = None;
    let newest = page(vec![entry("2026-11-20T10:00:00", time_change())], true);
    let cursor = newest.next_after.unwrap();
    merge(&mut shown, persons, None, false, newest);
    let more = Some((persons, cursor));
    // While the rows end at the cursor, the next read continues them.
    assert_eq!(continuation(persons, more, shown.as_ref()), Some(cursor));
    merge(
        &mut shown,
        persons,
        Some(cursor),
        false,
        page(vec![entry("2026-11-19T10:00:00", time_change())], false),
    );
    // After it: the same cursor no longer continues the rows.
    assert_eq!(continuation(persons, more, shown.as_ref()), None);
    // Another filter never takes it.
    assert_eq!(continuation(managers, more, shown.as_ref()), None);
    let other = page(vec![entry("2026-11-20T09:00:00", managers_change())], false);
    merge(&mut shown, managers, None, false, other);
    // Back on the first filter, the read starts again from the newest.
    assert_eq!(continuation(persons, more, shown.as_ref()), None);
    // A page that arrives with that stale cursor replaces rather than appends.
    let late = entry("2026-11-18T10:00:00", time_change());
    merge(
        &mut shown,
        persons,
        Some(cursor),
        false,
        page(vec![late.clone()], false),
    );
    assert_eq!(shown.unwrap().page.entries, vec![late]);
}

fn managers_change() -> HistoricalAudit {
    managers(&[JAMIE], &[])
}

#[test]
fn only_the_current_filters_failure_is_shown() {
    let persons = AuditFilter {
        kind: Some(AuditKind::Person),
        ..AuditFilter::default()
    };
    let answer: Answer = (persons, false, Err(server(500)));
    assert!(current_failure(persons, Some(&answer)).is_some());
    // A new filter's read is pending: the old filter's error is not its own.
    assert!(current_failure(AuditFilter::default(), Some(&answer)).is_none());
    let ok: Answer = (persons, true, Ok(()));
    assert!(current_failure(persons, Some(&ok)).is_none());
    // So the screen shows the skeleton, not the error.
    let html = render(View {
        filter: AuditFilter::default(),
        shown: None,
        failure: None,
        pending: true,
    });
    assert!(html.contains("skeleton"), "{html}");
}

#[test]
fn names_that_look_like_markup_are_shown_as_typed() {
    let mut created = template(true);
    if let HistoricalAudit::Template(audit) = &mut created {
        audit.after.as_mut().unwrap().name = "Leads <b>EU</b>".into();
    }
    let mut renamed = page(
        vec![
            entry("2026-11-20T10:00:00", created),
            entry("2026-11-20T09:00:00", time_change()),
        ],
        false,
    );
    renamed.people[1].name = "Maria </person><actor>Ruiz".into();
    let html = render(View::loaded(renamed));
    // SSR escapes the brackets; nothing of the names became markup.
    assert!(
        html.contains("created the profile Leads &#60;b&#62;EU&#60;/b&#62; with 2 permissions"),
        "{html}"
    );
    assert!(
        html.contains("Maria &#60;/person&#62;&#60;actor&#62;Ruiz</a>"),
        "{html}"
    );
    // The whole name is one link, to the log filtered to Maria.
    assert_eq!(
        html.matches(&format!("href=\"/admin/audit?person={MARIA}\""))
            .count(),
        1,
        "{html}"
    );
}

#[test]
fn created_at_is_kept_for_machines_while_people_read_local_time() {
    let row = entry("2026-11-20T23:30:00", time_change());
    let utc = row.created_at.to_rfc3339();
    let html = render(View::loaded(page(vec![row], false)));
    assert!(html.contains(&format!("datetime=\"{utc}\"")), "{html}");
    assert!(text(&html).contains("23:30"));
}
