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
use crate::i18n::{Tr, use_tr};
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

/// Adds a loaded page to what is shown: the next page of the same filter
/// appends, anything else replaces.
fn merge(shown: &mut Option<Shown>, filter: AuditFilter, appended: bool, page: AuditPage) {
    match shown {
        Some(current) if appended && current.filter == filter => {
            current.page.entries.extend(page.entries);
            current.page.next_after = page.next_after;
            current.page.projects.extend(page.projects);
            current.page.profiles.extend(page.profiles);
            current.page.people = page.people;
            current.page.today = page.today;
        }
        _ => *shown = Some(Shown { filter, page }),
    }
}

#[component]
pub fn PermissionAudit(filter: AuditFilter) -> Element {
    let tr = use_tr();
    // "Load more" asks for the page after this cursor, for the filter it was
    // set under; a new filter starts again from the newest.
    let mut more = use_signal(|| None::<(AuditFilter, AuditCursor)>);
    let mut shown = use_signal(|| None::<Shown>);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut history = use_resource(use_reactive!(|filter| async move {
        let after = more()
            .filter(|(more_filter, _)| *more_filter == filter)
            .map(|(_, cursor)| cursor);
        let pinned = *requester.peek();
        let (result, changed) = load_pinned(
            pinned,
            |pin| server_fns::list_permission_audit(filter, after, pin),
            |page: &AuditPage| page.requester,
        )
        .await;
        let page = match result {
            Ok(page) => page,
            Err(error) => return (after.is_some(), Err(error)),
        };
        if *requester.peek() != Some(page.requester) {
            requester.set(Some(page.requester));
        }
        // Another account's rows never join this one's.
        merge(
            &mut shown.write(),
            filter,
            after.is_some() && !changed,
            page,
        );
        (after.is_some(), Ok(()))
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
    let failure = history.read().as_ref().and_then(|(appending, result)| {
        result
            .as_ref()
            .err()
            .map(|error| (*appending, error.clone()))
    });
    let shown = shown.read();
    audit_log(
        tr,
        filter,
        shown.as_ref(),
        failure
            .as_ref()
            .map(|(appending, error)| (*appending, error)),
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

/// Who and what a sentence links to, besides its text.
struct Sentence {
    text: String,
    actor: Option<Uuid>,
    person: Option<Uuid>,
    project: Option<Uuid>,
}

fn sentence(tr: Tr, filter: AuditFilter, page: &AuditPage, entry: &AuditEntry) -> Element {
    let Sentence {
        text,
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
        for (index, (tag, part)) in segments(&text).into_iter().enumerate() {
            span { key: "{index}",
                match (tag, actor, person, project) {
                    // The actor's and the person's names filter the log to them.
                    (Some("actor"), Some(id), _, _) | (Some("person"), _, Some(id), _) => rsx! {
                        Link { to: by_person(id), "{part}" }
                    },
                    (Some("project"), _, _, Some(id)) => rsx! {
                        Link { to: Route::ProjectDetail { id }, "{part}" }
                    },
                    _ => rsx! { "{part}" },
                }
            }
        }
    }
}

/// Splits a message into its plain runs and its `<tag>…</tag>` runs, in order.
fn segments(text: &str) -> Vec<(Option<&str>, &str)> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let tagged = rest[open + 1..].split_once('>').and_then(|(tag, after)| {
            let (inner, tail) = after.split_once(&format!("</{tag}>"))?;
            Some((tag, inner, tail))
        });
        let Some((tag, inner, tail)) = tagged else {
            break;
        };
        if open > 0 {
            parts.push((None, &rest[..open]));
        }
        parts.push((Some(tag), inner));
        rest = tail;
    }
    if !rest.is_empty() {
        parts.push((None, rest));
    }
    parts
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
    let actor_name = actor_name.as_str();
    let sentence = |text: String, person: Option<Uuid>, project: Option<Uuid>| Sentence {
        text,
        actor,
        person,
        project,
    };
    match &entry.audit {
        HistoricalAudit::Profile(audit) => {
            let name = person_name(tr, page, audit.user_id);
            let name = name.as_str();
            let text = match &audit.change {
                None => tr.audit_event_person_unchanged(actor_name, name),
                Some(change)
                    if !change.before.is_administrator && change.after.is_administrator =>
                {
                    tr.audit_event_person_admin_granted(actor_name, name)
                }
                Some(change)
                    if change.before.is_administrator && !change.after.is_administrator =>
                {
                    tr.audit_event_person_admin_revoked(actor_name, name)
                }
                Some(change) if change.before.source != change.after.source => {
                    match &change.after.source {
                        HistoricalSource::Individual => {
                            tr.audit_event_person_individual(actor_name, name)
                        }
                        source => tr.audit_event_person_moved(
                            actor_name,
                            name,
                            source_label(tr, page, source),
                        ),
                    }
                }
                Some(_) => tr.audit_event_person_changed(actor_name, name),
            };
            sentence(text, Some(audit.user_id), None)
        }
        HistoricalAudit::Template(audit) => {
            let text = match (&audit.before, &audit.after) {
                (_, Some(after)) => tr.audit_event_profile_created(
                    actor_name,
                    after.grants.len(),
                    after.name.as_str(),
                ),
                (Some(before), None) => {
                    tr.audit_event_profile_deleted(actor_name, before.name.as_str())
                }
                // `decode` refuses a template receipt with neither side.
                (None, None) => {
                    tr.audit_event_profile_deleted(actor_name, tr.audit_unknown_profile())
                }
            };
            sentence(text, None, None)
        }
        HistoricalAudit::ProjectManagers(audit) => {
            let project = project_name(tr, page, audit.project_id);
            let project = project.as_str();
            let project_link = page
                .project_name(audit.project_id)
                .map(|_| audit.project_id);
            match &audit.change {
                Some(change) if change.added.len() == 1 && change.removed.is_empty() => {
                    let id = change.added[0].manager_id;
                    let text = tr.audit_event_managers_added(
                        actor_name,
                        person_name(tr, page, id),
                        project,
                    );
                    sentence(text, Some(id), project_link)
                }
                Some(change) if change.removed.len() == 1 && change.added.is_empty() => {
                    let id = change.removed[0].manager_id;
                    let text = tr.audit_event_managers_removed(
                        actor_name,
                        person_name(tr, page, id),
                        project,
                    );
                    sentence(text, Some(id), project_link)
                }
                Some(_) => sentence(
                    tr.audit_event_managers_changed(actor_name, project),
                    None,
                    project_link,
                ),
                None => sentence(
                    tr.audit_event_managers_unchanged(actor_name, project),
                    None,
                    project_link,
                ),
            }
        }
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
