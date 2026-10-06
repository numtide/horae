use super::*;
use crate::components::{
    controls::Checkbox,
    form::{FormGroup, Input},
    modal::Modal,
};
use crate::models::task::{TaskCatalogEntry, TaskRateEdit};
use horae_core::money::parse_cents;

#[derive(Clone, PartialEq)]
pub(super) struct Selection {
    pub task: TaskCatalogEntry,
    pub requester: PermissionRequester,
    pub rate_currency: Option<String>,
}

#[component]
pub(super) fn TaskEditor(
    mut selected: Signal<Option<Selection>>,
    on_saved: EventHandler<String>,
    on_denied: EventHandler<()>,
) -> Element {
    let busy = use_signal(|| false);
    rsx! {
        Modal { id: "task-editor", labelledby: "task-editor-title", open: selected.read().is_some(), busy: busy(),
            focus_fallback: Some("tasks-refresh"),
            on_dismiss: move |_| { if !busy() { selected.set(None); } },
            if let Some(selection) = selected() {
                TaskForm { key: "{selection.task.id}", selection, busy,
                    on_cancel: move |_| selected.set(None), on_saved, on_denied }
            }
        }
    }
}

#[component]
fn TaskForm(
    selection: Selection,
    mut busy: Signal<bool>,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<String>,
    on_denied: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| selection.task.name.clone());
    let mut billable = use_signal(|| selection.task.billable_default);
    let mut rate_action = use_signal(|| "preserve".to_string());
    let mut amount = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut confirm_activity = use_signal(|| false);
    let task_id = selection.task.id;
    let requester = selection.requester;
    let active = selection.task.active;
    let currency = selection.rate_currency.clone();
    rsx! {
        h2 { id: "task-editor-title", class: "modal-title", "Edit task" }
        form { id: "task-edit-form", onsubmit: move |event| {
            event.prevent_default();
            if busy() { return; }
            if name.read().trim().is_empty() { error.set(Some("Enter a task name.".into())); return; }
            let rate = match rate_action.read().as_str() {
                "clear" if currency.is_some() => TaskRateEdit::Clear {},
                "set" if currency.is_some() => {
                    let Ok(cents) = parse_cents(&amount.read()) else { error.set(Some("Enter a rate with at most two decimal places.".into())); return; };
                    if cents < 0 { error.set(Some("The hourly rate cannot be negative.".into())); return; }
                    TaskRateEdit::Set { amount_cents: cents, currency: currency.clone().unwrap_or_default() }
                }
                _ => TaskRateEdit::Preserve {},
            };
            busy.set(true); error.set(None);
            let edited_name = name(); let edited_billable = billable();
            spawn(async move {
                let result = server_fns::update_task(task_id.to_string(), edited_name, edited_billable, rate, requester).await;
                finish(result, busy, error, on_saved, on_denied, "Task updated.");
            });
        },
            div { class: "modal-body",
                if let Some(message) = error() { p { id: "task-edit-error", role: "alert", class: "alert alert-danger", "{message}" } }
                FormGroup { id: "task-name", label: "Task name",
                    Input { id: "task-name", value: name(), disabled: busy(), oninput: move |event: FormEvent| name.set(event.value()) }
                }
                Checkbox { id: "task-billable", label: "Billable by default", checked: billable(), disabled: busy(), onclick: move |_| billable.set(!billable()) }
                p { class: "form-hint", "Changes to defaults apply when adding the task to projects; existing project settings stay unchanged." }
                if let Some(currency) = &selection.rate_currency {
                    p { class: "text-sm text-secondary mt-3", "Current default: {rate_label(selection.task.default_rate_cents, selection.task.default_rate_currency.as_deref())}" }
                    FormGroup { id: "task-rate-action", label: "Default hourly rate",
                        select { id: "task-rate-action", class: "form-select", disabled: busy(), value: rate_action(),
                            onchange: move |event| rate_action.set(event.value()),
                            option { value: "preserve", "Keep current rate" }
                            option { value: "set", "Set a new rate" }
                            option { value: "clear", "Clear default rate" }
                        }
                    }
                    if rate_action() == "set" {
                        FormGroup { id: "task-rate", label: "Hourly rate ({currency})",
                            Input { id: "task-rate", value: amount(), disabled: busy(), oninput: move |event: FormEvent| amount.set(event.value()) }
                        }
                    }
                }
            }
            div { class: "px-6 pb-6 flex flex-wrap justify-end gap-3",
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| on_cancel.call(()), "Cancel" }
                button { id: "task-save", r#type: "submit", class: "btn btn-primary", disabled: busy(), if busy() { "Saving…" } else { "Save task" } }
            }
        }
        div { class: "px-6 pb-6",
            if confirm_activity() {
                p { class: "text-sm text-secondary mb-3",
                    if active { "Archive this task in every project? Existing time is kept. Stop running timers using this task first." }
                    else { "Restore this task to the catalog? To use it again in an existing project, restore it separately in that project's editor." }
                }
                div { class: "flex flex-wrap gap-3",
                    button { id: "task-activity-confirm", r#type: "button", class: "btn btn-secondary", disabled: busy(),
                        onclick: move |_| {
                            if busy() { return; }
                            busy.set(true); error.set(None);
                            spawn(async move {
                                let result = server_fns::set_task_active(task_id.to_string(), !active, requester).await;
                                finish(result, busy, error, on_saved, on_denied, if active { "Task archived." } else { "Task restored to the catalog. Restore it in each project where needed." });
                            });
                        }, if active { "Confirm archive" } else { "Confirm restore" }
                    }
                    button { r#type: "button", class: "btn btn-ghost", disabled: busy(), onclick: move |_| confirm_activity.set(false), "Keep current status" }
                }
            } else {
                button { id: "task-activity", r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| confirm_activity.set(true),
                    if active { "Archive task…" } else { "Restore task…" }
                }
            }
        }
    }
}

fn finish(
    result: Result<crate::models::Task, ServerFnError>,
    mut busy: Signal<bool>,
    mut error: Signal<Option<String>>,
    on_saved: EventHandler<String>,
    on_denied: EventHandler<()>,
    message: &str,
) {
    busy.set(false);
    match result {
        Ok(_) => on_saved.call(message.into()),
        Err(ServerFnError::ServerError { code: UNAUTHORIZED | FORBIDDEN, .. }) => on_denied.call(()),
        Err(ServerFnError::ServerError { code: 400 | 409, message, .. }) => error.set(Some(message)),
        Err(_) => error.set(Some("Could not save the task. Your inputs are kept; retry or refresh tasks to check its current state.".into())),
    }
}
