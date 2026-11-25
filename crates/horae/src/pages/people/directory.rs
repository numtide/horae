use chrono::{Duration, NaiveDate, Weekday};
use dioxus::prelude::*;
use horae_core::duration::format_hhmm;
use uuid::Uuid;

use super::add_person::AddPerson;
use super::permission_editor::PermissionEditorDialog;
use super::rates::{PersonRatesDialog, RatesTarget};
use crate::components::avatar::{Avatar, initials};
use crate::components::badge::Badge;
use crate::components::button::{Button, ButtonVariant};
use crate::components::filter_bar::{FilterBar, SearchField};
use crate::components::menu::{Menu, MenuDivider, MenuItem, RowActions};
use crate::components::page_header::PageHeader;
use crate::components::period_nav::PeriodNav;
use crate::components::screen_state::{EmptyState, LoadError, Skeleton};
use crate::components::table::DataTable;
use crate::components::toast::{Toast, ToastContainer};
use crate::i18n::{Tr, use_tr};
use crate::models::people::{
    PeopleActivity, PeoplePage, PersonProfile, PersonSummary, WeekMinutes,
};
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns;
use horae_core::permissions::catalog::BuiltInProfile;

/// A confirmation toast; `undo` names the archive or restore to reverse.
#[derive(Clone, PartialEq)]
struct Notice {
    message: String,
    undo: Option<(Uuid, bool)>,
}

/// The People list for one week. The page around it decides who may open it
/// and passes what the viewer may do.
#[component]
pub(super) fn PeopleDirectory(
    can_edit_permissions: bool,
    can_manage_people: bool,
    first_day: Weekday,
    today: NaiveDate,
    on_saved: EventHandler<bool>,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<Uuid>>,
) -> Element {
    let tr = use_tr();
    let this_week = horae_core::week::week_start(today, first_day).unwrap_or(today);
    let mut week = use_signal(|| this_week);
    let status = use_signal(|| PeopleActivity::Active);
    let profile = use_signal(|| None::<PersonProfile>);
    let query = use_signal(String::new);
    let mut adding = use_signal(|| false);
    let mut notice = use_signal(|| None::<Notice>);
    let mut row_error = use_signal(|| None::<String>);
    let mut changing = use_signal(|| false);
    let mut rates_target = use_signal(|| None::<RatesTarget>);
    let mut person = use_signal(|| None::<(Uuid, PermissionRequester)>);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    // Every person at once; status, profile and search narrow it here.
    let mut page = use_resource(move || async move {
        let week = week();
        let loaded = server_fns::list_people(week).await?;
        if requester
            .peek()
            .is_some_and(|expected| expected != loaded.requester)
        {
            return Err(ServerFnError::new(tr.people_session_changed()));
        }
        requester.set(Some(loaded.requester));
        Ok::<_, ServerFnError>((week, loaded))
    });
    let retry = use_callback(move |()| page.restart());
    let step = use_callback(move |forward: bool| {
        let days = if forward { 7 } else { -7 };
        week.set(week() + Duration::days(days));
    });
    let pick = use_callback(move |picked: NaiveDate| {
        week.set(horae_core::week::week_start(picked, first_day).unwrap_or(picked));
    });
    let open_add = use_callback(move |_: MouseEvent| adding.set(true));
    let close_add = use_callback(move |()| adding.set(false));
    let on_added = use_callback(move |_name: String| {
        adding.set(false);
        notice.set(Some(Notice {
            message: tr.people_added(),
            undo: None,
        }));
        page.restart();
    });
    let on_rates_saved = use_callback(move |message: String| {
        notice.set(Some(Notice {
            message,
            undo: None,
        }));
    });
    let on_permissions_saved = use_callback(move |changed: bool| {
        let message = if changed {
            tr.people_permissions_saved()
        } else {
            tr.people_permissions_unchanged()
        };
        notice.set(Some(Notice {
            message,
            undo: None,
        }));
        on_saved.call(changed);
        page.restart();
    });
    // Archiving keeps the person's time; only their sign-in stops. It applies at
    // once, and the toast offers to undo it.
    let set_active = use_callback(move |(id, active, undoable): (Uuid, bool, bool)| {
        if *changing.peek() {
            return;
        }
        changing.set(true);
        spawn(async move {
            match server_fns::set_user_active(id.to_string(), active).await {
                Ok(_) => {
                    row_error.set(None);
                    let message = if active {
                        tr.people_restored()
                    } else {
                        tr.people_archived_toast()
                    };
                    let undo = undoable.then_some((id, !active));
                    notice.set(Some(Notice { message, undo }));
                }
                Err(failure) => row_error.set(Some(failure.to_string())),
            }
            changing.set(false);
            page.restart();
        });
    });
    let change_active =
        use_callback(move |(id, active): (Uuid, bool)| set_active.call((id, active, true)));
    let open_rates = use_callback(move |(id, name): (Uuid, String)| {
        if let Some(requester) = *requester.peek() {
            rates_target.set(Some(RatesTarget {
                id,
                name,
                requester,
            }));
        }
    });
    let open_permissions = use_callback(move |id: Uuid| {
        if let Some(requester) = *requester.peek() {
            person.set(Some((id, requester)));
        }
    });

    let go_this_week = use_callback(move |_: MouseEvent| week.set(this_week));
    let dismiss_notice = use_callback(move |_: MouseEvent| notice.set(None));
    let undo = use_callback(move |_: MouseEvent| {
        if let Some((id, active)) = notice.peek().as_ref().and_then(|notice| notice.undo) {
            set_active.call((id, active, false));
        }
    });

    let reloading = page.state()() != UseResourceState::Ready;
    let shown_week = week();
    let response = page.read();
    let content = match &*response {
        None => rsx! { Skeleton { columns: 5, label: tr.people_loading() } },
        Some(Err(error)) => rsx! {
            LoadError { error: error.clone(), what: tr.people_load_subject(), retry, pending: reloading }
        },
        Some(Ok((loaded_week, loaded))) => {
            // A new week keeps the old rows on screen, dimmed, until its hours arrive.
            let stale = reloading || *loaded_week != shown_week;
            // The summary counts the people the status filter shows.
            let shown = status();
            let totals = loaded
                .people
                .iter()
                .filter(|person| shows(shown, person))
                .filter_map(|person| person.week)
                .fold(WeekMinutes::default(), |sum, week| WeekMinutes {
                    total: sum.total + week.total,
                    billable: sum.billable + week.billable,
                });
            rsx! {
                Summary { week: totals }
                Listing {
                    page: loaded.clone(), week: *loaded_week, stale, status, profile, query,
                    can_edit_permissions, busy: changing(),
                    on_view_timesheet, on_view_history,
                    on_rates: open_rates, on_permissions: open_permissions, on_set_active: change_active,
                }
            }
        }
    };

    rsx! {
        PageHeader { title: tr.people_title(),
            actions: rsx! {
                if can_manage_people {
                    Button { id: "people-add", label: tr.people_add(), onclick: open_add }
                }
            },
        }
        div { class: "flex flex-wrap items-center gap-4 mb-5",
            PeriodNav { id: "people-week", week_start: shown_week, day: shown_week, today, on_step: step, on_pick: pick }
            if shown_week != this_week {
                Button { variant: ButtonVariant::Ghost, small: true, label: tr.people_go_this_week(), onclick: go_this_week }
            }
        }
        if let Some(message) = row_error() {
            p { class: "alert alert-danger", role: "alert", "{message}" }
        }
        {content}
        if can_manage_people {
            AddPerson { open: adding(), choose_profile: can_edit_permissions, on_added, on_dismiss: close_add }
        }
        PermissionEditorDialog { person, on_saved: on_permissions_saved }
        PersonRatesDialog { target: rates_target, on_saved: on_rates_saved }
        ToastContainer {
            if let Some(Notice { message, undo: reversible }) = notice() {
                Toast { message, variant: "success", icon: "✓", dismissible: true,
                    ondismiss: dismiss_notice,
                    action_label: if reversible.is_some() { tr.people_undo() } else { String::new() },
                    action_busy: changing(),
                    onaction: undo,
                }
            }
        }
    }
}

/// The week's hours of the people the status filter shows whose time the
/// viewer may read; the profile filter and the search leave it alone.
#[component]
fn Summary(week: WeekMinutes) -> Element {
    let tr = use_tr();
    let share = percent(week.billable, week.total);
    let figures = [
        (tr.people_total_hours(), week.total),
        (tr.people_billable(), week.billable),
        (tr.people_non_billable(), week.total - week.billable),
    ];
    rsx! {
        section { class: "ppl-summary card rounded-xl mb-5", aria_label: tr.people_summary_label(),
            for (label, minutes) in figures {
                div { key: "{label}", class: "flex flex-col",
                    span { class: "text-sm text-secondary", "{label}" }
                    span { class: "font-display text-3xl font-semibold text-strong", "{format_hhmm(minutes)}" }
                }
            }
            div { class: "flex flex-col justify-center gap-2",
                if let Some(share) = share {
                    p { class: "flex justify-between text-sm text-secondary m-0",
                        span { {tr.people_billable_share(share)} }
                        span { {tr.people_non_billable_share(100 - share)} }
                    }
                }
                HoursBar { week, scale: week.total }
            }
        }
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn Listing(
    page: PeoplePage,
    week: NaiveDate,
    stale: bool,
    status: Signal<PeopleActivity>,
    profile: Signal<Option<PersonProfile>>,
    query: Signal<String>,
    can_edit_permissions: bool,
    busy: bool,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<Uuid>>,
    on_rates: Callback<(Uuid, String)>,
    on_permissions: Callback<Uuid>,
    on_set_active: Callback<(Uuid, bool)>,
) -> Element {
    let tr = use_tr();
    let (mut status, mut profile, mut query) = (status, profile, query);
    let clear_filters = use_callback(move |_: MouseEvent| {
        profile.set(None);
        status.set(PeopleActivity::Active);
    });
    let clear_search = use_callback(move |_: MouseEvent| query.set(String::new()));
    let people = &page.people;
    let count = |wanted: PeopleActivity| people.iter().filter(|p| shows(wanted, p)).count();
    let (active, archived, all) = (
        count(PeopleActivity::Active),
        count(PeopleActivity::Archived),
        people.len(),
    );
    let mut profiles: Vec<(String, PersonProfile)> = Vec::new();
    for present in people.iter().filter_map(|p| p.profile.as_ref()) {
        if !profiles.iter().any(|(_, seen)| seen == present) {
            profiles.push((profile_label(tr, present), present.clone()));
        }
    }
    profiles.sort_by(|a, b| a.0.cmp(&b.0));
    let typed = query();
    let needle = typed.trim().to_lowercase();
    let wanted = profile();
    let visible: Vec<&PersonSummary> = people
        .iter()
        .filter(|p| shows(status(), p))
        .filter(|p| wanted.is_none() || p.profile == wanted)
        .filter(|p| {
            needle.is_empty()
                || p.name.to_lowercase().contains(&needle)
                || p.email.to_lowercase().contains(&needle)
        })
        .collect();
    let scale = visible
        .iter()
        .filter_map(|p| p.week)
        .map(|w| w.total)
        .max()
        .unwrap_or(0);
    let status_label = match status() {
        PeopleActivity::Active => tr.people_status_active(active),
        PeopleActivity::Archived => tr.people_status_archived(archived),
        PeopleActivity::All => tr.people_status_all(all),
    };
    let profile_shown = wanted
        .as_ref()
        .map_or_else(|| tr.people_any_profile(), |p| profile_label(tr, p));
    let search_hint = match status() {
        PeopleActivity::Active => tr.people_search_hint_active(),
        PeopleActivity::Archived => tr.people_search_hint_archived(),
        PeopleActivity::All => tr.people_search_hint_all(),
    };
    let filters_title = if status() == PeopleActivity::Archived && wanted.is_none() {
        tr.people_none_archived()
    } else {
        tr.people_none_match_filters()
    };
    let actions = people
        .iter()
        .any(|p| has_menu(p, can_edit_permissions, on_view_history.is_some()));

    rsx! {
        FilterBar {
            filters: rsx! {
                Menu { id: "people-profile-menu", label: profile_shown, trigger_class: "text-sm px-4",
                    ProfileOption { value: None, label: tr.people_any_profile(), profile }
                    for (label, value) in profiles {
                        ProfileOption { key: "{label}", value: Some(value), label, profile }
                    }
                }
                Menu { id: "people-status-menu", label: status_label, trigger_class: "text-sm px-4",
                    StatusOption { id: "people-status-active", value: PeopleActivity::Active, label: tr.people_status_active(active), status }
                    StatusOption { id: "people-status-archived", value: PeopleActivity::Archived, label: tr.people_status_archived(archived), status }
                    StatusOption { id: "people-status-all", value: PeopleActivity::All, label: tr.people_status_all(all), status }
                }
            },
            search: rsx! { SearchField { id: "people-search", label: tr.people_search_label(), query } },
        }
        if visible.is_empty() {
            if needle.is_empty() {
                EmptyState { icon: "team", title: filters_title, body: tr.people_none_match_filters_hint(),
                    actions: rsx! {
                        Button { variant: ButtonVariant::Secondary, label: tr.people_clear_filters(), onclick: clear_filters }
                    },
                }
            } else {
                EmptyState { icon: "team", title: tr.people_none_match_search(typed.trim()), body: search_hint,
                    actions: rsx! {
                        Button { variant: ButtonVariant::Secondary, label: tr.people_clear_search(), onclick: clear_search }
                    },
                }
            }
        } else {
            div { class: "ppl-content", aria_busy: stale,
                DataTable { label: tr.people_title(), striped: true,
                    table { class: "name-table",
                        thead { tr {
                            th { scope: "col", {tr.people_column_person()} }
                            th { scope: "col", {tr.people_column_hours()} }
                            th { scope: "col", class: "text-right", {tr.people_billable()} }
                            th { scope: "col", class: "text-right", {tr.people_column_billable_share()} }
                            // Named without text: an absolutely placed `.sr-only` would
                            // escape the table's scroll container on narrow screens.
                            if actions { th { scope: "col", aria_label: tr.people_actions() } }
                        } }
                        tbody {
                            for entry in visible {
                                PersonRow { key: "{entry.id}", person: entry.clone(), week, scale, actions,
                                    can_edit_permissions, busy,
                                    on_view_timesheet, on_view_history, on_rates, on_permissions, on_set_active,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The name a profile shows under. A custom profile's name is data.
fn profile_label(tr: Tr, profile: &PersonProfile) -> String {
    match profile {
        PersonProfile::BuiltIn(BuiltInProfile::Member) => tr.people_profile_member(),
        PersonProfile::BuiltIn(BuiltInProfile::ProjectManager) => {
            tr.people_profile_project_manager()
        }
        PersonProfile::BuiltIn(BuiltInProfile::PeopleAdmin) => tr.people_profile_people_admin(),
        PersonProfile::BuiltIn(BuiltInProfile::Accounting) => tr.people_profile_accounting(),
        PersonProfile::BuiltIn(BuiltInProfile::ExecutiveManager) => {
            tr.people_profile_executive_manager()
        }
        PersonProfile::BuiltIn(BuiltInProfile::Administrator) => tr.people_profile_administrator(),
        PersonProfile::Custom(name) => name.clone(),
        PersonProfile::Individual => tr.people_profile_individual(),
    }
}

/// Whether the row has any action for this viewer.
fn has_menu(person: &PersonSummary, can_edit_permissions: bool, has_history: bool) -> bool {
    opens_timesheet(person)
        || can_edit_permissions
        || person.can_read_rates
        || has_history
        || person.can_change_active
}

/// The Timesheet opens only for an active person whose time the viewer reads.
fn opens_timesheet(person: &PersonSummary) -> bool {
    person.active && person.week.is_some()
}

/// One profile in the profile filter; `None` is "Any profile".
#[component]
fn ProfileOption(
    value: Option<PersonProfile>,
    label: String,
    profile: Signal<Option<PersonProfile>>,
) -> Element {
    let mut profile = profile;
    let selected = profile() == value;
    let choose = use_callback(move |_: MouseEvent| profile.set(value.clone()));
    rsx! { MenuItem { selected, onclick: choose, "{label}" } }
}

/// One status in the status filter.
#[component]
fn StatusOption(
    id: &'static str,
    value: PeopleActivity,
    label: String,
    status: Signal<PeopleActivity>,
) -> Element {
    let mut status = status;
    let choose = use_callback(move |_: MouseEvent| status.set(value));
    rsx! { MenuItem { id, selected: status() == value, onclick: choose, "{label}" } }
}

fn shows(status: PeopleActivity, person: &PersonSummary) -> bool {
    match status {
        PeopleActivity::Active => person.active,
        PeopleActivity::Archived => !person.active,
        PeopleActivity::All => true,
    }
}

/// `part` as a whole percentage of `whole`, `None` for an empty week.
fn percent(part: i64, whole: i64) -> Option<i64> {
    (whole > 0).then(|| (part * 100 + whole / 2) / whole)
}

#[component]
#[allow(clippy::too_many_arguments)]
fn PersonRow(
    person: PersonSummary,
    week: NaiveDate,
    scale: i64,
    actions: bool,
    can_edit_permissions: bool,
    busy: bool,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<Uuid>>,
    on_rates: Callback<(Uuid, String)>,
    on_permissions: Callback<Uuid>,
    on_set_active: Callback<(Uuid, bool)>,
) -> Element {
    let tr = use_tr();
    let id = person.id;
    let timesheet = opens_timesheet(&person);
    let above_divider =
        timesheet || can_edit_permissions || person.can_read_rates || on_view_history.is_some();
    let menu = has_menu(&person, can_edit_permissions, on_view_history.is_some());
    let (name, active) = (person.name.clone(), person.active);
    let view_timesheet = use_callback(move |_: MouseEvent| on_view_timesheet.call((id, week)));
    let edit_permissions = use_callback(move |_: MouseEvent| on_permissions.call(id));
    let edit_rates = use_callback(move |_: MouseEvent| on_rates.call((id, name.clone())));
    let view_history = use_callback(move |_: MouseEvent| {
        if let Some(history) = on_view_history {
            history.call(id);
        }
    });
    let toggle_active = use_callback(move |_: MouseEvent| on_set_active.call((id, !active)));
    rsx! {
        tr {
            td {
                div { class: "flex items-center gap-3",
                    Avatar { initials: initials(&person.name), size: "sm" }
                    div { class: "min-w-0",
                        div { class: "flex flex-wrap items-center gap-2",
                            span { class: "font-semibold", "{person.name}" }
                            if let Some(profile) = &person.profile {
                                Badge { variant: "neutral", {profile_label(tr, profile)} }
                            }
                            if !person.active {
                                Badge { variant: "warning", {tr.people_archived()} }
                            }
                        }
                        div { class: "text-xs text-secondary truncate", "{person.email}" }
                    }
                }
            }
            match person.week {
                Some(hours) => rsx! {
                    td {
                        div { class: "flex items-center gap-3",
                            span { class: "text-mono", "{format_hhmm(hours.total)}" }
                            div { class: "ppl-bar", HoursBar { week: hours, scale } }
                        }
                    }
                    td { class: "text-right text-mono", "{format_hhmm(hours.billable)}" }
                    td { class: "text-right text-mono",
                        match percent(hours.billable, hours.total) {
                            Some(share) => rsx! { {tr.people_share(share)} },
                            None => rsx! { "—" },
                        }
                    }
                },
                // The viewer may not read this person's time.
                None => rsx! {
                    td { span { class: "text-muted", aria_label: tr.people_hours_unavailable_label(), "—" } }
                    td { class: "text-right", span { class: "text-muted", aria_label: tr.people_billable_unavailable_label(), "—" } }
                    td { class: "text-right", span { class: "text-muted", aria_label: tr.people_share_unavailable_label(), "—" } }
                },
            }
            if actions {
                td { class: "text-right",
                    if menu {
                        RowActions { id: "person-actions-{id}", name: person.name.clone(),
                            if timesheet {
                                MenuItem { id: "person-timesheet-{id}", onclick: view_timesheet, {tr.people_view_timesheet()} }
                            }
                            if can_edit_permissions {
                                MenuItem { id: "person-permissions-{id}", onclick: edit_permissions, {tr.people_permissions()} }
                            }
                            if person.can_read_rates {
                                MenuItem { id: "person-rates-{id}", onclick: edit_rates, {tr.people_rates()} }
                            }
                            if on_view_history.is_some() {
                                MenuItem { id: "person-history-{id}", onclick: view_history, {tr.people_permission_history()} }
                            }
                            if person.can_change_active {
                                if above_divider { MenuDivider {} }
                                MenuItem { id: "person-active-{id}", disabled: busy, onclick: toggle_active,
                                    if person.active { {tr.people_archive()} } else { {tr.people_restore()} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Billable then non-billable time, as a share of `scale` minutes. SVG
/// attributes size the parts, so the bar needs no inline style.
#[component]
fn HoursBar(week: WeekMinutes, scale: i64) -> Element {
    // Thousandths of the scale keep the parts in whole numbers.
    let width = |minutes: i64| if scale > 0 { minutes * 1000 / scale } else { 0 };
    let billable = width(week.billable);
    let other = width(week.total - week.billable);
    rsx! {
        svg { class: "hours-bar", view_box: "0 0 1000 6", preserve_aspect_ratio: "none", "aria-hidden": "true",
            rect { class: "hours-bar-track", width: "1000", height: "6" }
            rect { class: "hours-bar-billable", width: "{billable}", height: "6" }
            rect { class: "hours-bar-other", x: "{billable}", width: "{other}", height: "6" }
        }
    }
}
