//! The audit log: every permission change, newest first, grouped by day in
//! the viewer's time zone, each told as a sentence (`DESIGN.md`, *Audit log*).
//! Only those who may read it get here; the server refuses everyone else.

use chrono::{Days, NaiveDate};
use dioxus::prelude::*;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use uuid::Uuid;

use crate::components::access_table::{AccessTable, OTHER, changed_rows};
use crate::components::badge::Badge;
use crate::components::filter_bar::FilterBar;
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::permission_description::permission_description;
use crate::components::screen_state::{
    EmptyState, ErrorState, LOAD_FAILED, NoAccess, Skeleton, is_refusal, load_pinned,
};
use crate::i18n::{Tr, split_tags, use_tr};
use crate::models::permission_audit::*;
use crate::models::permission_editor::PermissionRequester;
use crate::route::Route;
use crate::server_fns;

const UNAUTHORIZED: u16 = 401;

/// The rows on screen and the filter they answer. A reload keeps them until
/// the new answer replaces them.
#[derive(Clone, PartialEq)]
struct Shown {
    filter: AuditFilter,
    page: AuditPage,
}

/// Whether the rows on screen are `filter`'s and end where `cursor` starts.
fn continues(shown: Option<&Shown>, filter: AuditFilter, cursor: AuditCursor) -> bool {
    shown.is_some_and(|shown| shown.filter == filter && shown.page.next_after == Some(cursor))
}

/// Where a read of `filter` starts: after the "Load more" cursor only while it
/// still continues the rows on screen. A cursor left from an earlier visit to
/// the filter would show its later pages as if they were the newest.
fn continuation(
    filter: AuditFilter,
    more: Option<(AuditFilter, AuditCursor)>,
    shown: Option<&Shown>,
) -> Option<AuditCursor> {
    more.filter(|&(more_filter, cursor)| more_filter == filter && continues(shown, filter, cursor))
        .map(|(_, cursor)| cursor)
}

/// Adds a loaded page to what is shown. The page read `after` a cursor
/// appends when it continues the rows on screen; anything else replaces
/// them, and so does a page for another account (`changed`).
fn merge(
    shown: &mut Option<Shown>,
    filter: AuditFilter,
    after: Option<AuditCursor>,
    changed: bool,
    page: AuditPage,
) {
    let appends = !changed && after.is_some_and(|cursor| continues(shown.as_ref(), filter, cursor));
    match shown {
        // Only the first page carries the people; the shown ones stay.
        Some(current) if appends => {
            current.page.entries.extend(page.entries);
            current.page.next_after = page.next_after;
            current.page.projects.extend(page.projects);
            current.page.profiles.extend(page.profiles);
            current.page.today = page.today;
        }
        _ => *shown = Some(Shown { filter, page }),
    }
}

/// What the history resource answers: the filter and whether it was a "Load
/// more", with the outcome. The loaded rows themselves go to `Shown`.
type Answer = (AuditFilter, bool, Result<(), ServerFnError>);

/// The failure on screen: only one for the filter now in the address, so a
/// new filter shows its skeleton instead of the last filter's error.
fn current_failure(filter: AuditFilter, answer: Option<&Answer>) -> Option<(bool, &ServerFnError)> {
    answer
        .filter(|(answered, _, _)| *answered == filter)
        .and_then(|(_, appending, result)| result.as_ref().err().map(|error| (*appending, error)))
}

#[component]
pub fn PermissionAudit(filter: AuditFilter) -> Element {
    let tr = use_tr();
    // "Load more" asks for the page after this cursor, for the filter it was
    // set under; `continuation` decides whether it still applies.
    let mut more = use_signal(|| None::<(AuditFilter, AuditCursor)>);
    let mut shown = use_signal(|| None::<Shown>);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut history = use_resource(use_reactive!(|filter| async move {
        let after = continuation(filter, more(), shown.peek().as_ref());
        let pinned = *requester.peek();
        let (result, changed) = load_pinned(
            pinned,
            |pin| server_fns::list_permission_audit(filter, after, pin),
            |page: &AuditPage| page.requester,
        )
        .await;
        let page = match result {
            Ok(page) => page,
            Err(error) => return (filter, after.is_some(), Err(error)),
        };
        if *requester.peek() != Some(page.requester) {
            requester.set(Some(page.requester));
        }
        merge(&mut shown.write(), filter, after, changed, page);
        (filter, after.is_some(), Ok(()))
    }));
    let retry = use_callback(move |_| {
        if *history.state().peek() == UseResourceState::Ready {
            history.restart();
        }
    });
    let load_more = use_callback(move |_: MouseEvent| {
        let next = shown
            .peek()
            .as_ref()
            .and_then(|shown| Some((shown.filter, shown.page.next_after?)));
        if *history.state().peek() == UseResourceState::Ready && next.is_some() {
            more.set(next);
        }
    });
    let pending = history.state()() != UseResourceState::Ready;
    let answer = history.read();
    let shown = shown.read();
    audit_log(
        tr,
        filter,
        shown.as_ref(),
        current_failure(filter, answer.as_ref()),
        pending,
        load_more,
        retry,
    )
}

/// The whole screen for one state of the log. `failure` says whether the
/// failed read was a "Load more"; `pending` is true while a read runs.
fn audit_log(
    tr: Tr,
    filter: AuditFilter,
    shown: Option<&Shown>,
    failure: Option<(bool, &ServerFnError)>,
    pending: bool,
    load_more: Callback<MouseEvent>,
    retry: Callback<()>,
) -> Element {
    // Without access the screen keeps only its header and the state.
    if failure.is_some_and(|(_, error)| is_refusal(error)) {
        return rsx! {
            PageHeader { title: tr.audit_title(), title_id: "audit-title" }
            NoAccess {}
        };
    }
    let page = shown.map(|shown| &shown.page);
    let error_state = failure.map(|(_, error)| {
        if matches!(
            error,
            ServerFnError::ServerError {
                code: UNAUTHORIZED,
                ..
            }
        ) {
            rsx! { ErrorState { title: tr.audit_failed(), body: tr.common_session_ended() } }
        } else {
            rsx! {
                ErrorState { title: tr.audit_failed(), body: LOAD_FAILED, onretry: retry,
                    retry_disabled: pending, focus_on_recovery: "audit-title" }
            }
        }
    });
    // A failed "Load more" keeps the rows above its error; any other failure
    // replaces them, since they may answer another filter.
    let rows = match (shown, failure) {
        (Some(shown), None | Some((true, _))) => Some(shown),
        _ => None,
    };
    let stale = pending || shown.is_some_and(|shown| shown.filter != filter);
    rsx! {
        section { aria_labelledby: "audit-title", class: "min-w-0",
            PageHeader { title: tr.audit_title(), title_id: "audit-title" }
            p { class: "text-sm text-secondary mb-5",
                {tr.audit_intro()}
                if let Some(page) = page {
                    {" "}
                    {tr.audit_time_zone(page.time_zone.as_str())}
                }
            }
            {filters(tr, filter, page)}
            match rows {
                Some(shown) if shown.page.entries.is_empty() => rsx! {
                    div { class: "audit-content", aria_busy: stale,
                        {empty(tr, shown.filter)}
                    }
                },
                Some(shown) => rsx! {
                    div { class: "audit-content", aria_busy: stale,
                        div { class: "audit-log", role: "region", aria_label: tr.audit_list_label(),
                            for (day, entries) in days(&shown.page.entries) {
                                section { key: "{day}",
                                    h2 { class: "audit-day", {day_label(tr, day, shown.page.today)} }
                                    for entry in entries {
                                        details { key: "{entry.id}", id: "audit-entry-{entry.id}", class: "audit-row",
                                            {summary(tr, shown.filter, &shown.page, entry)}
                                            div { class: "audit-detail", {detail(tr, &shown.page, entry)} }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if failure.is_none() && shown.page.next_after.is_some() {
                        div { class: "flex justify-center mt-4",
                            button { id: "audit-load-more", r#type: "button", class: "btn btn-secondary",
                                disabled: pending, onclick: load_more, {tr.audit_load_more()}
                            }
                        }
                    }
                },
                None if error_state.is_none() => rsx! {
                    Skeleton { rows: 6, columns: 4, grouped: true, label: tr.audit_loading() }
                },
                None => rsx! {},
            }
            if let Some(error_state) = error_state {
                div { class: if rows.is_some() { "mt-4" } else { "" }, {error_state} }
            }
        }
    }
}

/// The filter bar. The person filter waits for the names it offers.
fn filters(tr: Tr, filter: AuditFilter, page: Option<&AuditPage>) -> Element {
    let kind_label = |kind: Option<AuditKind>| match kind {
        None => tr.audit_kind_all(),
        Some(AuditKind::Person) => tr.audit_filter_kind_person(),
        Some(AuditKind::Profile) => tr.audit_filter_kind_profile(),
        Some(AuditKind::Managers) => tr.audit_filter_kind_managers(),
    };
    let period_label = |period: Option<AuditPeriod>| match period {
        None => tr.audit_period_all(),
        Some(period) => tr.audit_period_days(period.days()),
    };
    let person_label = match filter.person {
        None => tr.audit_filter_anyone(),
        Some(id) => page
            .and_then(|page| page.person_name(id))
            .map_or_else(|| tr.audit_filter_someone(), str::to_owned),
    };
    let people = page.map(|page| page.people.clone()).unwrap_or_default();
    rsx! {
        FilterBar {
            filters: rsx! {
                Menu { id: "audit-kind-menu", label: kind_label(filter.kind), trigger_class: "text-sm px-4",
                    for kind in std::iter::once(None).chain(AuditKind::ALL.map(Some)) {
                        FilterOption { key: "{kind:?}",
                            id: format!("audit-kind-{}", kind.map_or("all", AuditKind::as_str)),
                            label: kind_label(kind), selected: filter.kind == kind,
                            to: AuditFilter { kind, ..filter },
                        }
                    }
                }
                Menu { id: "audit-person-menu", label: person_label, trigger_class: "text-sm px-4",
                    disabled: page.is_none(),
                    FilterOption { id: "audit-person-anyone", label: tr.audit_filter_anyone(),
                        selected: filter.person.is_none(), to: AuditFilter { person: None, ..filter },
                    }
                    for person in people {
                        FilterOption { key: "{person.id}", id: format!("audit-person-{}", person.id),
                            label: person.name, selected: filter.person == Some(person.id),
                            to: AuditFilter { person: Some(person.id), ..filter },
                        }
                    }
                }
                Menu { id: "audit-period-menu", label: period_label(filter.period), trigger_class: "text-sm px-4",
                    for period in std::iter::once(None).chain(AuditPeriod::ALL.map(Some)) {
                        FilterOption { key: "{period:?}",
                            id: format!("audit-period-{}", period.map_or("all", AuditPeriod::as_str)),
                            label: period_label(period), selected: filter.period == period,
                            to: AuditFilter { period, ..filter },
                        }
                    }
                }
            },
        }
    }
}

/// One choice of a filter menu: it opens the log with `to` in the address.
#[component]
fn FilterOption(id: String, label: String, selected: bool, to: AuditFilter) -> Element {
    let navigator = use_navigator();
    let open = use_callback(move |_: MouseEvent| {
        navigator.push(Route::PermissionAudit { filter: to });
    });
    rsx! {
        MenuItem { id, selected, onclick: open, "{label}" }
    }
}

fn empty(tr: Tr, filter: AuditFilter) -> Element {
    if filter.is_empty() {
        rsx! {
            EmptyState { icon: "history", title: tr.audit_empty_title(), body: tr.audit_empty_body(),
                actions: rsx! {
                    Link { class: "btn btn-secondary", to: Route::People {}, {tr.audit_empty_action()} }
                },
            }
        }
    } else {
        rsx! {
            EmptyState { icon: "history", title: tr.audit_filtered_empty_title(), body: tr.audit_filtered_empty_body(),
                actions: rsx! {
                    Link { id: "audit-clear-filters", class: "btn btn-secondary",
                        to: Route::PermissionAudit { filter: AuditFilter::default() },
                        {tr.audit_clear_filters()}
                    }
                },
            }
        }
    }
}

/// The entries in runs of one local day, newest first as they arrive.
fn days(entries: &[AuditEntry]) -> Vec<(NaiveDate, &[AuditEntry])> {
    entries
        .chunk_by(|a, b| a.local.date() == b.local.date())
        .map(|run| (run[0].local.date(), run))
        .collect()
}

fn day_label(tr: Tr, day: NaiveDate, today: NaiveDate) -> String {
    if day == today {
        tr.audit_today()
    } else if today.checked_sub_days(Days::new(1)) == Some(day) {
        tr.audit_yesterday()
    } else {
        day.format("%d %b %Y").to_string()
    }
}

/// The line a row shows folded: time, kind, sentence and the "+2 −1" summary.
fn summary(tr: Tr, filter: AuditFilter, page: &AuditPage, entry: &AuditEntry) -> Element {
    let (variant, kind) = match entry.kind() {
        AuditKind::Person => ("success", tr.audit_kind_person()),
        AuditKind::Profile => ("warning", tr.audit_kind_profile()),
        AuditKind::Managers => ("neutral", tr.audit_kind_managers()),
    };
    let (added, removed) = counts(&entry.audit);
    rsx! {
        summary { class: "audit-row-summary",
            time { class: "text-mono text-sm text-secondary", datetime: entry.created_at.to_rfc3339(),
                {entry.local.format("%H:%M").to_string()}
            }
            span { Badge { variant, "{kind}" } }
            span { class: "audit-sentence", {sentence(tr, filter, page, entry)} }
            span { class: "text-mono text-xs whitespace-nowrap",
                if added + removed > 0 {
                    span { class: "sr-only", {tr.audit_summary_label(added, removed)} }
                }
                span { aria_hidden: "true",
                    if added > 0 { span { class: "text-success", {tr.audit_added(added)} } }
                    if added > 0 && removed > 0 { {" "} }
                    if removed > 0 { span { class: "text-danger", {tr.audit_removed(removed)} } }
                }
            }
            span { class: "audit-chevron text-secondary", aria_hidden: "true", NavIcon { name: "chevron-down" } }
        }
    }
}

/// Grants (or managers) a change added and removed.
fn counts(audit: &HistoricalAudit) -> (usize, usize) {
    let missing = |from: &[Permission], to: &[Permission]| {
        from.iter().filter(|grant| !to.contains(grant)).count()
    };
    match audit {
        HistoricalAudit::Profile(ProfileAudit {
            change: Some(change),
            ..
        }) => (
            missing(&change.after.grants, &change.before.grants),
            missing(&change.before.grants, &change.after.grants),
        ),
        HistoricalAudit::Template(audit) => (
            audit.after.as_ref().map_or(0, |after| after.grants.len()),
            audit
                .before
                .as_ref()
                .map_or(0, |before| before.grants.len()),
        ),
        HistoricalAudit::ProjectManagers(ProjectAudit {
            change: Some(change),
            ..
        }) => (change.added.len(), change.removed.len()),
        _ => (0, 0),
    }
}

// Stand-ins for the names a sentence holds, from Unicode's private use area.
// The message is formatted with them and split into its tagged runs before
// the names go in, so a name such as "Leads <b>EU</b>" is never read as markup.
const ACTOR: &str = "\u{E000}";
const PERSON: &str = "\u{E001}";
const PROJECT: &str = "\u{E002}";
const PROFILE: &str = "\u{E003}";

/// A sentence with stand-ins, the names they hold, and what its tagged parts
/// link to.
struct Sentence {
    text: String,
    names: Vec<(&'static str, String)>,
    actor: Option<Uuid>,
    person: Option<Uuid>,
    project: Option<Uuid>,
}

/// `part` with each stand-in replaced by its name.
fn fill(part: &str, names: &[(&'static str, String)]) -> String {
    names
        .iter()
        .fold(part.to_owned(), |text, (stand_in, name)| {
            text.replace(stand_in, name)
        })
}

fn sentence(tr: Tr, filter: AuditFilter, page: &AuditPage, entry: &AuditEntry) -> Element {
    let Sentence {
        text,
        names,
        actor,
        person,
        project,
    } = describe(tr, page, entry);
    let by_person = |id: Uuid| Route::PermissionAudit {
        filter: AuditFilter {
            person: Some(id),
            ..filter
        },
    };
    rsx! {
        for (index, (tag, part)) in split_tags(&text).into_iter().enumerate() {
            span { key: "{index}",
                match (tag, actor, person, project) {
                    // The actor's and the person's names filter the log to them.
                    (Some("actor"), Some(id), _, _) | (Some("person"), _, Some(id), _) => rsx! {
                        Link { to: by_person(id), {fill(part, &names)} }
                    },
                    (Some("project"), _, _, Some(id)) => rsx! {
                        Link { to: Route::ProjectDetail { id }, {fill(part, &names)} }
                    },
                    _ => rsx! { {fill(part, &names)} },
                }
            }
        }
    }
}

fn person_name(tr: Tr, page: &AuditPage, id: Uuid) -> String {
    page.person_name(id)
        .map_or_else(|| tr.audit_unknown_person(), str::to_owned)
}

fn project_name(tr: Tr, page: &AuditPage, id: Uuid) -> String {
    page.project_name(id)
        .map_or_else(|| tr.audit_unknown_project(), str::to_owned)
}

fn built_in_label(tr: Tr, profile: BuiltInProfile) -> String {
    match profile {
        BuiltInProfile::Member => tr.people_profile_member(),
        BuiltInProfile::ProjectManager => tr.people_profile_project_manager(),
        BuiltInProfile::PeopleAdmin => tr.people_profile_people_admin(),
        BuiltInProfile::Accounting => tr.people_profile_accounting(),
        BuiltInProfile::ExecutiveManager => tr.people_profile_executive_manager(),
        BuiltInProfile::Administrator => tr.people_profile_administrator(),
    }
}

fn source_label(tr: Tr, page: &AuditPage, source: &HistoricalSource) -> String {
    match source {
        HistoricalSource::BuiltIn(profile) => built_in_label(tr, *profile),
        HistoricalSource::Template { id, .. } => page
            .profile_name(*id)
            .map_or_else(|| tr.audit_unknown_profile(), str::to_owned),
        HistoricalSource::Individual => tr.people_profile_individual(),
    }
}

fn describe(tr: Tr, page: &AuditPage, entry: &AuditEntry) -> Sentence {
    let (actor_name, actor) = match &entry.actor {
        AuditPrincipal::User { user_id } => (person_name(tr, page, *user_id), Some(*user_id)),
        AuditPrincipal::Operator { command, .. } => (tr.audit_operator(command.as_str()), None),
    };
    let mut names = vec![(ACTOR, actor_name)];
    let (mut person, mut project) = (None, None);
    let text = match &entry.audit {
        HistoricalAudit::Profile(audit) => {
            names.push((PERSON, person_name(tr, page, audit.user_id)));
            person = Some(audit.user_id);
            match &audit.change {
                None => tr.audit_event_person_unchanged(ACTOR, PERSON),
                Some(change)
                    if !change.before.is_administrator && change.after.is_administrator =>
                {
                    tr.audit_event_person_admin_granted(ACTOR, PERSON)
                }
                Some(change)
                    if change.before.is_administrator && !change.after.is_administrator =>
                {
                    tr.audit_event_person_admin_revoked(ACTOR, PERSON)
                }
                Some(change) if change.before.source != change.after.source => {
                    match &change.after.source {
                        HistoricalSource::Individual => {
                            tr.audit_event_person_individual(ACTOR, PERSON)
                        }
                        source => {
                            names.push((PROFILE, source_label(tr, page, source)));
                            tr.audit_event_person_moved(ACTOR, PERSON, PROFILE)
                        }
                    }
                }
                Some(_) => tr.audit_event_person_changed(ACTOR, PERSON),
            }
        }
        HistoricalAudit::Template(audit) => match (&audit.before, &audit.after) {
            (_, Some(after)) => {
                names.push((PROFILE, after.name.clone()));
                tr.audit_event_profile_created(ACTOR, after.grants.len(), PROFILE)
            }
            (before, None) => {
                // `decode` refuses a template receipt with neither side.
                let name = before
                    .as_ref()
                    .map_or_else(|| tr.audit_unknown_profile(), |before| before.name.clone());
                names.push((PROFILE, name));
                tr.audit_event_profile_deleted(ACTOR, PROFILE)
            }
        },
        HistoricalAudit::ProjectManagers(audit) => {
            names.push((PROJECT, project_name(tr, page, audit.project_id)));
            project = page
                .project_name(audit.project_id)
                .map(|_| audit.project_id);
            // The one manager a change added (or removed) and nothing else.
            let only = |one: &[Designation], none: &[Designation]| {
                (one.len() == 1 && none.is_empty()).then(|| one[0].manager_id)
            };
            match &audit.change {
                Some(change) => {
                    if let Some(id) = only(&change.added, &change.removed) {
                        names.push((PERSON, person_name(tr, page, id)));
                        person = Some(id);
                        tr.audit_event_managers_added(ACTOR, PERSON, PROJECT)
                    } else if let Some(id) = only(&change.removed, &change.added) {
                        names.push((PERSON, person_name(tr, page, id)));
                        person = Some(id);
                        tr.audit_event_managers_removed(ACTOR, PERSON, PROJECT)
                    } else {
                        tr.audit_event_managers_changed(ACTOR, PROJECT)
                    }
                }
                None => tr.audit_event_managers_unchanged(ACTOR, PROJECT),
            }
        }
    };
    Sentence {
        text,
        names,
        actor,
        person,
        project,
    }
}

/// The expanded part of a row: what changed, limited to the areas that moved,
/// then the receipt.
fn detail(tr: Tr, page: &AuditPage, entry: &AuditEntry) -> Element {
    let mut lines: Vec<String> = Vec::new();
    let (before, after): (&[Permission], &[Permission]) = match &entry.audit {
        HistoricalAudit::Profile(ProfileAudit {
            change: Some(change),
            ..
        }) => {
            if change.before.source != change.after.source {
                lines.push(tr.audit_detail_profile(
                    source_label(tr, page, &change.after.source),
                    source_label(tr, page, &change.before.source),
                ));
            }
            for link in &change.removed_projects {
                lines.push(tr.audit_detail_no_longer_manages(project_name(
                    tr,
                    page,
                    link.subject_id,
                )));
            }
            for link in &change.removed_people {
                lines.push(tr.audit_detail_no_longer_manages(person_name(
                    tr,
                    page,
                    link.subject_id,
                )));
            }
            (&change.before.grants, &change.after.grants)
        }
        HistoricalAudit::Template(audit) => {
            for person in &audit.detached_people {
                lines.push(tr.audit_detail_detached(person_name(tr, page, person.user_id)));
            }
            (
                audit
                    .before
                    .as_ref()
                    .map_or(&[][..], |before| &before.grants),
                audit.after.as_ref().map_or(&[][..], |after| &after.grants),
            )
        }
        HistoricalAudit::ProjectManagers(ProjectAudit {
            change: Some(change),
            ..
        }) => {
            for designation in &change.added {
                lines.push(tr.audit_detail_manager_added(person_name(
                    tr,
                    page,
                    designation.manager_id,
                )));
            }
            for designation in &change.removed {
                lines.push(tr.audit_detail_manager_removed(person_name(
                    tr,
                    page,
                    designation.manager_id,
                )));
            }
            (&[], &[])
        }
        _ => (&[], &[]),
    };
    let rows = changed_rows(before, after);
    // Grants outside the table are listed by their description.
    for grant in OTHER {
        match (before.contains(&grant), after.contains(&grant)) {
            (false, true) => lines.push(tr.audit_detail_added(permission_description(grant))),
            (true, false) => lines.push(tr.audit_detail_removed(permission_description(grant))),
            _ => {}
        }
    }
    let table = !rows.is_empty();
    rsx! {
        if table {
            AccessTable { label: tr.audit_detail_label(), rows }
        }
        if !lines.is_empty() {
            ul { class: "audit-lines text-sm",
                for (index, line) in lines.into_iter().enumerate() {
                    li { key: "{index}", "{line}" }
                }
            }
        }
        div { class: "flex flex-wrap justify-between gap-3 text-xs text-muted",
            span { if table { {tr.audit_detail_areas()} } }
            span { class: "flex items-center gap-2",
                span { class: "text-mono", {tr.audit_receipt(entry.id.to_string())} }
                CopyReceipt { id: entry.id }
            }
        }
    }
}

/// Copies a receipt id, for quoting it to support or in a ticket.
#[component]
fn CopyReceipt(id: Uuid) -> Element {
    let tr = use_tr();
    let mut copied = use_signal(|| false);
    rsx! {
        button { r#type: "button", class: "btn btn-ghost btn-sm",
            onclick: move |_| {
                document::eval(&format!("navigator.clipboard.writeText({:?});", id.to_string()));
                copied.set(true);
            },
            if copied() { {tr.audit_copied()} } else { {tr.audit_copy()} }
        }
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "permission_audit/tests.rs"]
mod tests;
