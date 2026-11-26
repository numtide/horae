use dioxus::prelude::*;
use horae_core::money::format_cents;

use crate::i18n::Tr;

use crate::components::badge::Badge;
use crate::components::filter_bar::FilterBar;
use crate::components::menu::{Menu, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{
    EmptyState, ErrorState, LOAD_FAILED, Loading, NoAccess, is_refusal, load_pinned,
};
use crate::models::permission_editor::PermissionRequester;
use crate::models::task::{TaskActivity, TaskCatalogPage, TaskCursor};
use crate::server_fns;

#[path = "tasks/activity.rs"]
mod activity;
#[path = "tasks/editor.rs"]
mod editor;

/// (common, heading id, heading, explanation), in the server's listing order.
const TASK_GROUPS: [(bool, &str, &str, &str); 2] = [
    (
        true,
        "tasks-common",
        "Common tasks",
        "Active common tasks are added to every new project.",
    ),
    (
        false,
        "tasks-other",
        "Other tasks",
        "Added to a project from its task list.",
    ),
];

/// One status in the Tasks filter menu.
#[component]
fn ActivityOption(
    choice: TaskActivity,
    current: TaskActivity,
    on_pick: Callback<TaskActivity>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let pick = use_callback(move |_: MouseEvent| on_pick.call(choice));
    rsx! {
        MenuItem { id: "tasks-activity-{activity_value(choice)}", selected: current == choice, onclick: pick,
            {activity_label(tr, choice)}
        }
    }
}

fn activity_value(activity: TaskActivity) -> &'static str {
    match activity {
        TaskActivity::Active => "active",
        TaskActivity::Archived => "archived",
        TaskActivity::All => "all",
    }
}

fn activity_label(tr: Tr, activity: TaskActivity) -> String {
    match activity {
        TaskActivity::Active => tr.tasks_filter_active(),
        TaskActivity::Archived => tr.tasks_filter_archived(),
        TaskActivity::All => tr.tasks_filter_all(),
    }
}

const FORBIDDEN: u16 = 403;
const UNAUTHORIZED: u16 = 401;
/// A rejected input or a conflicting change: the server's message says what to fix.
const BAD_REQUEST: u16 = 400;
const CONFLICT: u16 = 409;

#[component]
pub fn TaskCatalog() -> Element {
    let tr = crate::i18n::use_tr();
    let mut activity = use_signal(|| TaskActivity::Active);
    let mut cursors = use_signal(|| vec![None::<TaskCursor>]);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut selected = use_signal(|| None);
    let mut notice = use_signal(|| None::<String>);
    let mut catalog = use_resource(move || {
        let filter = activity();
        let cursor = cursors.read().last().cloned().flatten();
        let pinned = *requester.peek();
        async move {
            let (result, changed) = load_pinned(
                pinned,
                |pin| server_fns::load_task_catalog(filter, cursor.clone(), pin),
                |page: &TaskCatalogPage| page.requester,
            )
            .await;
            if changed {
                // The editor and its notice belong to the old account.
                selected.set(None);
                notice.set(None);
            }
            if let Ok(page) = &result
                && *requester.peek() != Some(page.requester)
            {
                requester.set(Some(page.requester));
            }
            ((filter, cursor), result)
        }
    });
    let retry = use_callback(move |_| {
        selected.set(None);
        notice.set(None);
        catalog.restart();
    });
    let pick_activity = use_callback(move |choice: TaskActivity| {
        if *activity.peek() == choice {
            return;
        }
        selected.set(None);
        notice.set(None);
        cursors.set(vec![None]);
        activity.set(choice);
    });
    let key = (activity(), cursors.read().last().cloned().flatten());
    let response = catalog.read();
    let current = response
        .as_ref()
        .filter(|(requested, _)| *requested == key)
        .filter(|_| catalog.state()() == UseResourceState::Ready)
        .map(|(_, result)| result);
    let pending = current.is_none();
    // The latest answer for this filter, even while a retry is pending: a
    // failure stays on screen, disabled, instead of becoming a loading line.
    let failed = response
        .as_ref()
        .filter(|(requested, _)| *requested == key)
        .and_then(|(_, result)| result.as_ref().err());
    let next = current
        .and_then(|result| result.as_ref().ok())
        .and_then(|page| page.next_after.clone());
    let mut changing = use_signal(|| None::<activity::ActivityTarget>);
    // The page the menus act on: who asks, and in which rate currency.
    let context = current.and_then(|result| result.as_ref().ok()).map(|page| {
        (
            page.requester,
            page.rate_currency.clone().filter(|_| page.can_edit_rates),
        )
    });
    let requester_now = context.as_ref().map(|(requester, _)| *requester);
    let on_edit = use_callback(move |task: crate::models::task::TaskCatalogEntry| {
        if let Some((requester, rate_currency)) = context.clone() {
            notice.set(None);
            selected.set(Some(editor::Selection {
                task: Some(task),
                requester,
                rate_currency,
            }));
        }
    });
    let activity_changed = use_callback(move |message: String| {
        notice.set(Some(message));
        catalog.restart();
    });
    let activity_denied = use_callback(move |()| {
        selected.set(None);
        notice.set(Some(tr.tasks_access_changed()));
        catalog.restart();
    });
    let on_activity = use_callback(move |task: crate::models::task::TaskCatalogEntry| {
        if let Some(requester) = requester_now {
            notice.set(None);
            changing.set(Some(activity::ActivityTarget { task, requester }));
        }
    });
    // Without access the screen keeps only its header and the state.
    let refused = failed.is_some_and(is_refusal);

    rsx! {
        section { aria_labelledby: "tasks-title", class: "min-w-0",
            PageHeader { title: "Tasks", title_id: "tasks-title",
                actions: rsx! { if !refused {
                    if let Some(Ok(page)) = current && page.can_edit {
                        button { id: "tasks-new", r#type: "button", class: "btn btn-primary",
                            onclick: {
                                let selection = editor::Selection { task: None, requester: page.requester,
                                    rate_currency: page.rate_currency.clone().filter(|_| page.can_edit_rates) };
                                move |_| { notice.set(None); selected.set(Some(selection.clone())); }
                            }, "New task"
                        }
                    }
                    button { id: "tasks-refresh", r#type: "button", class: "btn btn-secondary", disabled: pending,
                        onclick: move |_| { selected.set(None); notice.set(None); catalog.restart(); }, "Refresh tasks"
                    }
                } },
            }
            if !refused {
                FilterBar {
                    filters: rsx! {
                        Menu { id: "tasks-activity", label: activity_label(tr, activity()), trigger_class: "text-sm px-4", disabled: pending,
                            for choice in [TaskActivity::Active, TaskActivity::Archived, TaskActivity::All] {
                                ActivityOption { key: "{choice:?}", choice, current: activity(), on_pick: pick_activity }
                            }
                        }
                    },
                }
            }
            if let Some(message) = notice() { p { class: "text-sm text-secondary mb-3", role: "status", "{message}" } }
            if let Some(error) = failed {
                if refused {
                    NoAccess {}
                } else if is_unauthorized(error) {
                    ErrorState { title: tr.tasks_load_failed(), body: tr.common_session_ended() }
                } else {
                    ErrorState {
                        title: tr.tasks_load_failed(),
                        body: LOAD_FAILED,
                        onretry: retry,
                        retry_disabled: pending,
                        focus_on_recovery: "tasks-title",
                    }
                }
            } else if pending {
                Loading { label: "Loading tasks…" }
            } else if let Some(Ok(page)) = current {
                if !page.can_edit { p { class: "text-sm text-secondary mb-3", "You have read-only access to tasks." } }
                if page.tasks.is_empty() {
                    EmptyState { icon: "tasks", title: "No tasks match this filter", body: "Try another filter." }
                } else {
                    // The server lists common tasks first, so a page holds the end of
                    // one group and the start of the other at most.
                    for (common, group_id, title, hint) in TASK_GROUPS {
                        if page.tasks.iter().any(|task| task.is_default == common) {
                    section { aria_labelledby: group_id, class: "mb-6",
                    h2 { id: group_id, class: "card-title mb-1", {title} }
                    p { class: "text-sm text-secondary mb-3", {hint} }
                    div { class: "table-container rounded-xl bg-secondary",
                        table { class: "name-table", aria_label: title,
                            thead { tr {
                                th { scope: "col", "Task" }
                                th { scope: "col", "Billable by default" }
                                if page.can_read_rates { th { scope: "col", "Default hourly rate" } }
                                th { scope: "col", "Status" }
                                if page.can_edit { th { scope: "col", class: "text-right", "Actions" } }
                            } }
                            tbody { for task in page.tasks.iter().filter(|task| task.is_default == common) {
                                tr { key: "{task.id}",
                                    td { "{task.name}" }
                                    td { if task.billable_default { "Yes" } else { "No" } }
                                    if page.can_read_rates { td { class: "text-mono text-sm whitespace-nowrap", {rate_label(tr, task.default_rate_cents, task.default_rate_currency.as_deref())} } }
                                    td {
                                        if task.active {
                                            Badge { variant: "success", {tr.tasks_status_active()} }
                                        } else {
                                            Badge { variant: "neutral", {tr.tasks_status_archived()} }
                                        }
                                    }
                                    if page.can_edit { td { class: "text-right",
                                        activity::TaskActions { task: task.clone(), on_edit, on_activity }
                                    } }
                                }
                            } }
                        }
                    }
                    }
                        }
                    }
                }
            }
            if !refused { nav { class: "flex flex-wrap gap-3 mt-4", aria_label: "Task pages",
                button { id: "tasks-previous", r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending || cursors.read().len() <= 1,
                    onclick: move |_| { selected.set(None); if cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
                }
                button { id: "tasks-next", r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending || next.is_none(),
                    onclick: move |_| { selected.set(None); if let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
                }
            } }
            editor::TaskEditor { selected,
                on_saved: move |message| {
                    let created = selected.peek().as_ref().is_some_and(|selection| selection.task.is_none());
                    notice.set(Some(message)); selected.set(None);
                    if created { activity.set(TaskActivity::Active); cursors.set(vec![None]); }
                    catalog.restart();
                },
                on_denied: move |_| { notice.set(Some("Your session or task permissions changed. The editor was closed; check your access before retrying.".into())); selected.set(None); catalog.restart(); }
            }
            activity::ActivityDialog { target: changing, on_changed: activity_changed, on_denied: activity_denied }
        }
    }
}

fn rate_label(tr: Tr, cents: Option<i64>, currency: Option<&str>) -> String {
    match (cents, currency) {
        (Some(cents), Some(currency)) => tr.tasks_rate_per_hour(format_cents(cents, currency)),
        // Harvest task rates arrive without a currency, and none is invented.
        // An empty code leaves the grouped amount after a leading space.
        (Some(cents), None) => {
            tr.tasks_rate_currency_not_set(format_cents(cents, "").trim_start().to_owned())
        }
        (None, _) => tr.tasks_rate_none(),
    }
}

fn is_unauthorized(error: &ServerFnError) -> bool {
    matches!(
        error,
        ServerFnError::ServerError {
            code: UNAUTHORIZED,
            ..
        }
    )
}
