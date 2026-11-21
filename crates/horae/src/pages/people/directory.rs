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
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::period_nav::PeriodNav;
use crate::components::screen_state::{EmptyState, LoadError, Skeleton};
use crate::components::table::DataTable;
use crate::components::toast::{Toast, ToastContainer};
use crate::models::people::{PeopleActivity, PeoplePage, PersonSummary, WeekMinutes};
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns;

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
    can_view_rates: bool,
    first_day: Weekday,
    today: NaiveDate,
    on_saved: EventHandler<bool>,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<()>>,
) -> Element {
    let this_week = horae_core::week::week_start(today, first_day).unwrap_or(today);
    let mut week = use_signal(|| this_week);
    let status = use_signal(|| PeopleActivity::Active);
    let profile = use_signal(|| None::<String>);
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
        let loaded = server_fns::list_people(PeopleActivity::All, week).await?;
        if requester.peek().is_some_and(|expected| expected != loaded.requester) {
            return Err(ServerFnError::new(
                "Your session changed. Reload this page before selecting a person.",
            ));
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
        notice.set(Some(Notice { message: "Person added".into(), undo: None }));
        page.restart();
    });
    let on_rates_saved = use_callback(move |message: String| {
        notice.set(Some(Notice { message, undo: None }));
    });
    let on_permissions_saved = use_callback(move |changed: bool| {
        let message = if changed { "Permissions saved" } else { "Permissions are unchanged" };
        notice.set(Some(Notice { message: message.into(), undo: None }));
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
                    let message = if active { "Person restored" } else { "Person archived" };
                    let undo = undoable.then_some((id, !active));
                    notice.set(Some(Notice { message: message.into(), undo }));
                }
                Err(failure) => row_error.set(Some(failure.to_string())),
            }
            changing.set(false);
            page.restart();
        });
    });
    let change_active = use_callback(move |(id, active): (Uuid, bool)| set_active.call((id, active, true)));
    let open_rates = use_callback(move |(id, name): (Uuid, String)| {
        if let Some(requester) = *requester.peek() {
            rates_target.set(Some(RatesTarget { id, name, requester }));
        }
    });
    let open_permissions = use_callback(move |id: Uuid| {
        if let Some(requester) = *requester.peek() {
            person.set(Some((id, requester)));
        }
    });

    let reloading = page.state()() != UseResourceState::Ready;
    let shown_week = week();
    let response = page.read();
    let content = match &*response {
        None => rsx! { Skeleton { columns: 5, label: "Loading people…" } },
        Some(Err(error)) => rsx! {
            LoadError { error: error.clone(), what: "people", retry, pending: reloading }
        },
        Some(Ok((loaded_week, loaded))) => {
            // A new week keeps the old rows on screen, dimmed, until its hours arrive.
            let stale = reloading || *loaded_week != shown_week;
            rsx! {
                Summary { people: loaded.people.clone() }
                Listing {
                    page: loaded.clone(), week: *loaded_week, stale, status, profile, query,
                    can_view_rates, can_edit_permissions, busy: changing(),
                    on_view_timesheet, on_view_history,
                    on_rates: open_rates, on_permissions: open_permissions, on_set_active: change_active,
                }
            }
        }
    };

    rsx! {
        PageHeader { title: "People",
            actions: rsx! {
                if can_manage_people {
                    Button { id: "people-add", label: "Add person", onclick: open_add }
                }
            },
        }
        div { class: "flex flex-wrap items-center gap-4 mb-5",
            PeriodNav { id: "people-week", week_start: shown_week, day: shown_week, today, on_step: step, on_pick: pick }
            if shown_week != this_week {
                Button { variant: ButtonVariant::Ghost, small: true, label: "This week",
                    onclick: move |_| week.set(this_week),
                }
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
            if let Some(Notice { message, undo }) = notice() {
                Toast { message, variant: "success", icon: "✓", dismissible: true,
                    ondismiss: move |_| notice.set(None),
                    action_label: if undo.is_some() { "Undo" } else { "" },
                    action_busy: changing(),
                    onaction: move |_| {
                        if let Some((id, active)) = undo {
                            set_active.call((id, active, false));
                        }
                    },
                }
            }
        }
    }
}

/// The week's hours of everyone listed whose time the viewer may read, before
/// any filter.
#[component]
fn Summary(people: Vec<PersonSummary>) -> Element {
    let week = people
        .iter()
        .filter_map(|person| person.week)
        .fold(WeekMinutes::default(), |sum, week| WeekMinutes {
            total: sum.total + week.total,
            billable: sum.billable + week.billable,
        });
    let share = percent(week.billable, week.total);
    rsx! {
        section { class: "ppl-summary card rounded-xl mb-5", aria_label: "Week summary",
            for (label, minutes) in [("Total hours", week.total), ("Billable", week.billable), ("Non-billable", week.total - week.billable)] {
                div { key: "{label}", class: "flex flex-col",
                    span { class: "text-sm text-secondary", "{label}" }
                    span { class: "font-display text-3xl font-semibold text-strong", "{format_hhmm(minutes)}" }
                }
            }
            div { class: "flex flex-col justify-center gap-2",
                if let Some(share) = share {
                    p { class: "flex justify-between text-sm text-secondary m-0",
                        span { "Billable {share}%" }
                        span { "Non-billable {100 - share}%" }
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
    profile: Signal<Option<String>>,
    query: Signal<String>,
    can_view_rates: bool,
    can_edit_permissions: bool,
    busy: bool,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<()>>,
    on_rates: Callback<(Uuid, String)>,
    on_permissions: Callback<Uuid>,
    on_set_active: Callback<(Uuid, bool)>,
) -> Element {
    let (mut status, mut profile, mut query) = (status, profile, query);
    let people = &page.people;
    let count = |wanted: PeopleActivity| people.iter().filter(|p| shows(wanted, p)).count();
    let (active, archived) = (count(PeopleActivity::Active), count(PeopleActivity::Archived));
    let mut profiles: Vec<String> = people.iter().filter_map(|p| p.profile.clone()).collect();
    profiles.sort();
    profiles.dedup();
    let needle = query().trim().to_lowercase();
    let visible: Vec<&PersonSummary> = people
        .iter()
        .filter(|p| shows(status(), p))
        .filter(|p| profile().is_none_or(|wanted| p.profile.as_deref() == Some(wanted.as_str())))
        .filter(|p| {
            needle.is_empty()
                || p.name.to_lowercase().contains(&needle)
                || p.email.to_lowercase().contains(&needle)
        })
        .collect();
    let scale = visible.iter().filter_map(|p| p.week).map(|w| w.total).max().unwrap_or(0);
    let status_label = match status() {
        PeopleActivity::Active => format!("Active ({active})"),
        PeopleActivity::Archived => format!("Archived ({archived})"),
        PeopleActivity::All => format!("All ({})", people.len()),
    };
    let profile_label = profile().unwrap_or_else(|| "Any profile".into());
    let which = match status() {
        PeopleActivity::Active => "active people",
        PeopleActivity::Archived => "archived people",
        PeopleActivity::All => "everyone",
    };
    let actions = can_view_rates
        || can_edit_permissions
        || people.iter().any(|p| p.week.is_some() || p.can_change_active)
        || on_view_history.is_some();

    rsx! {
        FilterBar {
            filters: rsx! {
                Menu { id: "people-profile-menu", label: profile_label, trigger_class: "text-sm px-4",
                    MenuItem { selected: profile().is_none(), onclick: move |_| profile.set(None), "Any profile" }
                    for name in profiles {
                        MenuItem { key: "{name}", selected: profile().as_deref() == Some(name.as_str()),
                            onclick: { let name = name.clone(); move |_| profile.set(Some(name.clone())) },
                            "{name}"
                        }
                    }
                }
                Menu { id: "people-status-menu", label: status_label, trigger_class: "text-sm px-4",
                    MenuItem { id: "people-status-active", selected: status() == PeopleActivity::Active,
                        onclick: move |_| status.set(PeopleActivity::Active), "Active ({active})" }
                    MenuItem { id: "people-status-archived", selected: status() == PeopleActivity::Archived,
                        onclick: move |_| status.set(PeopleActivity::Archived), "Archived ({archived})" }
                    MenuItem { id: "people-status-all", selected: status() == PeopleActivity::All,
                        onclick: move |_| status.set(PeopleActivity::All), "All ({people.len()})" }
                }
            },
            search: rsx! { SearchField { id: "people-search", label: "Search by name or email", query } },
        }
        if visible.is_empty() {
            if needle.is_empty() {
                EmptyState { icon: "team",
                    title: if status() == PeopleActivity::Archived && profile().is_none() { "No archived people" } else { "No people match these filters" },
                    body: "Change the filters to see more people.",
                    actions: rsx! {
                        Button { variant: ButtonVariant::Secondary, label: "Clear filters",
                            onclick: move |_| { profile.set(None); status.set(PeopleActivity::Active); },
                        }
                    },
                }
            } else {
                EmptyState { icon: "team",
                    title: "No one matches “{query().trim()}”",
                    body: "Search looks at names and emails among {which}.",
                    actions: rsx! {
                        Button { variant: ButtonVariant::Secondary, label: "Clear search",
                            onclick: move |_| query.set(String::new()),
                        }
                    },
                }
            }
        } else {
            div { class: "ppl-content", aria_busy: stale,
                DataTable {
                    table { class: "name-table",
                        thead { tr {
                            th { scope: "col", "Person" }
                            th { scope: "col", "Hours" }
                            th { scope: "col", class: "text-right", "Billable" }
                            th { scope: "col", class: "text-right", "Billable %" }
                            if actions { th { scope: "col", span { class: "sr-only", "Actions" } } }
                        } }
                        tbody {
                            for entry in visible {
                                PersonRow { key: "{entry.id}", person: entry.clone(), week, scale, actions,
                                    can_view_rates, can_edit_permissions, busy,
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
    can_view_rates: bool,
    can_edit_permissions: bool,
    busy: bool,
    on_view_timesheet: Callback<(Uuid, NaiveDate)>,
    on_view_history: Option<Callback<()>>,
    on_rates: Callback<(Uuid, String)>,
    on_permissions: Callback<Uuid>,
    on_set_active: Callback<(Uuid, bool)>,
) -> Element {
    let id = person.id;
    let manage = can_view_rates || can_edit_permissions || on_view_history.is_some();
    let has_menu = person.week.is_some() || manage || person.can_change_active;
    rsx! {
        tr {
            td {
                div { class: "flex items-center gap-3",
                    Avatar { initials: initials(&person.name), size: "sm" }
                    div { class: "min-w-0",
                        div { class: "flex flex-wrap items-center gap-2",
                            span { class: "font-semibold", "{person.name}" }
                            if let Some(profile) = &person.profile {
                                Badge { variant: "neutral", "{profile}" }
                            }
                            if !person.active {
                                Badge { variant: "warning", "Archived" }
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
                            Some(share) => rsx! { "{share}%" },
                            None => rsx! { "—" },
                        }
                    }
                },
                // The viewer may not read this person's time.
                None => rsx! {
                    td { span { class: "text-muted", aria_label: "Hours not available", "—" } }
                    td { class: "text-right", span { class: "text-muted", aria_label: "Billable hours not available", "—" } }
                    td { class: "text-right", span { class: "text-muted", aria_label: "Billable share not available", "—" } }
                },
            }
            if actions {
                td { class: "text-right",
                    if has_menu {
                        Menu { id: "person-actions-{id}", label: "Actions", align_right: true,
                            if person.week.is_some() {
                                MenuItem { id: "person-timesheet-{id}", onclick: move |_| on_view_timesheet.call((id, week)), "View timesheet" }
                            }
                            if can_edit_permissions {
                                MenuItem { id: "person-permissions-{id}", onclick: move |_| on_permissions.call(id), "Permissions…" }
                            }
                            if can_view_rates {
                                MenuItem { id: "person-rates-{id}",
                                    onclick: { let name = person.name.clone(); move |_| on_rates.call((id, name.clone())) },
                                    "Rates…"
                                }
                            }
                            if let Some(history) = on_view_history {
                                MenuItem { id: "person-history-{id}", onclick: move |_| history.call(()), "Permission history" }
                            }
                            if person.can_change_active {
                                if person.week.is_some() || manage { MenuDivider {} }
                                MenuItem { id: "person-active-{id}", disabled: busy,
                                    onclick: { let active = !person.active; move |_| on_set_active.call((id, active)) },
                                    if person.active { "Archive" } else { "Restore" }
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
