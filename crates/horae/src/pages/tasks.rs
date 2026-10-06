use dioxus::prelude::*;
use horae_core::money::format_cents;

use crate::models::permission_editor::PermissionRequester;
use crate::models::task::{TaskActivity, TaskCursor};
use crate::server_fns;

#[path = "tasks/editor.rs"]
mod editor;

const FORBIDDEN: u16 = 403;
const UNAUTHORIZED: u16 = 401;

#[component]
pub fn TaskCatalog() -> Element {
    let mut activity = use_signal(|| TaskActivity::Active);
    let mut cursors = use_signal(|| vec![None::<TaskCursor>]);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut selected = use_signal(|| None);
    let mut notice = use_signal(|| None::<String>);
    let mut catalog = use_resource(move || {
        let filter = activity();
        let cursor = cursors.read().last().cloned().flatten();
        let expected = *requester.peek();
        async move {
            let result = server_fns::load_task_catalog(filter, cursor.clone(), expected)
                .await
                .and_then(|page| {
                    if expected.is_some_and(|identity| identity != page.requester) {
                        return Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            message: "The active account changed".into(),
                            details: None,
                        });
                    }
                    if expected.is_none() {
                        requester.set(Some(page.requester));
                    }
                    Ok(page)
                });
            ((filter, cursor), result)
        }
    });
    let key = (activity(), cursors.read().last().cloned().flatten());
    let response = catalog.read();
    let current = response
        .as_ref()
        .filter(|(requested, _)| *requested == key)
        .filter(|_| catalog.state()() == UseResourceState::Ready)
        .map(|(_, result)| result);
    let pending = current.is_none();
    let next = current
        .and_then(|result| result.as_ref().ok())
        .and_then(|page| page.next_after.clone());

    rsx! {
        section { aria_labelledby: "tasks-title", class: "min-w-0",
            div { class: "flex flex-wrap items-center justify-between gap-3 mb-4",
                h1 { id: "tasks-title", class: "text-2xl font-semibold", "Tasks" }
                div { class: "flex flex-wrap gap-3",
                    if let Some(Ok(page)) = current && page.can_edit {
                        button { id: "tasks-new", r#type: "button", class: "btn btn-primary btn-sm",
                            onclick: {
                                let selection = editor::Selection { task: None, requester: page.requester,
                                    rate_currency: page.rate_currency.clone().filter(|_| page.can_edit_rates) };
                                move |_| { notice.set(None); selected.set(Some(selection.clone())); }
                            }, "New task"
                        }
                    }
                    button { id: "tasks-refresh", r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending,
                        onclick: move |_| { selected.set(None); notice.set(None); catalog.restart(); }, "Refresh tasks"
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-3 mb-4",
                label { r#for: "tasks-activity", class: "form-label", "Show" }
                select { id: "tasks-activity", class: "form-select", disabled: pending,
                    value: match activity() { TaskActivity::Active => "active", TaskActivity::Archived => "archived", TaskActivity::All => "all" },
                    onchange: move |event| {
                        let filter = match event.value().as_str() {
                            "active" => TaskActivity::Active, "archived" => TaskActivity::Archived,
                            "all" => TaskActivity::All, _ => return,
                        };
                        selected.set(None); notice.set(None); cursors.set(vec![None]); activity.set(filter);
                    },
                    option { value: "active", "Active tasks" }
                    option { value: "archived", "Archived tasks" }
                    option { value: "all", "All tasks" }
                }
            }
            if let Some(message) = notice() { p { class: "text-sm text-secondary mb-3", role: "status", "{message}" } }
            if pending {
                p { role: "status", class: "text-secondary", "Loading tasks…" }
            } else if let Some(Err(error)) = current {
                p { role: "alert", class: "text-danger", "{error_message(error)}" }
            } else if let Some(Ok(page)) = current {
                if !page.can_edit { p { class: "text-sm text-secondary mb-3", "You have read-only access to tasks." } }
                if page.tasks.is_empty() {
                    p { role: "status", class: "text-secondary", "No tasks match this filter. Try another filter." }
                } else {
                    div { class: "table-container rounded-xl bg-secondary",
                        table { aria_label: "Task catalog",
                            thead { tr {
                                th { scope: "col", "Task" }
                                th { scope: "col", "Billable by default" }
                                if page.can_read_rates { th { scope: "col", "Default hourly rate" } }
                                th { scope: "col", "Status" }
                                if page.can_edit { th { scope: "col", "Actions" } }
                            } }
                            tbody { for task in &page.tasks {
                                tr { key: "{task.id}",
                                    td { class: "wrap-anywhere", "{task.name}" }
                                    td { if task.billable_default { "Yes" } else { "No" } }
                                    if page.can_read_rates { td { class: "font-mono text-sm whitespace-nowrap", {rate_label(task.default_rate_cents, task.default_rate_currency.as_deref())} } }
                                    td { if task.active { "Active" } else { "Archived" } }
                                    if page.can_edit { td {
                                        button { id: "tasks-edit-{task.id}", r#type: "button", class: "btn btn-secondary btn-sm",
                                            aria_label: "Edit {task.name}",
                                            onclick: {
                                                let selection = editor::Selection { task: Some(task.clone()), requester: page.requester,
                                                    rate_currency: page.rate_currency.clone().filter(|_| page.can_edit_rates) };
                                                move |_| { notice.set(None); selected.set(Some(selection.clone())); }
                                            }, "Edit"
                                        }
                                    } }
                                }
                            } }
                        }
                    }
                }
            }
            nav { class: "flex flex-wrap gap-3 mt-4", aria_label: "Task pages",
                button { id: "tasks-previous", r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending || cursors.read().len() <= 1,
                    onclick: move |_| { selected.set(None); if cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
                }
                button { id: "tasks-next", r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending || next.is_none(),
                    onclick: move |_| { selected.set(None); if let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
                }
            }
            editor::TaskEditor { selected,
                on_saved: move |message| {
                    let created = selected.peek().as_ref().is_some_and(|selection| selection.task.is_none());
                    notice.set(Some(message)); selected.set(None);
                    if created { activity.set(TaskActivity::Active); cursors.set(vec![None]); }
                    catalog.restart();
                },
                on_denied: move |_| { notice.set(Some("Your session or task permissions changed. The editor was closed; check your access before retrying.".into())); selected.set(None); catalog.restart(); }
            }
        }
    }
}

fn rate_label(cents: Option<i64>, currency: Option<&str>) -> String {
    match cents {
        Some(cents) => format_cents(cents, currency.unwrap_or("Unknown currency")),
        None => "Not set".into(),
    }
}

fn error_message(error: &ServerFnError) -> &'static str {
    match error {
        ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => "Sign in again to view tasks.",
        ServerFnError::ServerError {
            code: FORBIDDEN, ..
        } => "Task access is unavailable. If you changed accounts, reopen this page.",
        _ => "Could not load tasks. Refresh tasks to try again.",
    }
}
