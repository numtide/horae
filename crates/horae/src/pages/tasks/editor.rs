use super::*;
use crate::components::{
    controls::Checkbox,
    form::{FormGroup, Input},
    modal::{Modal, ModalAction, ModalActions, ModalBody, ModalHeader},
    select_field::SelectField,
};
use crate::models::task::{TaskCatalogEntry, TaskRateEdit};
use horae_core::money::{RateTemplate, parse_cents};

#[derive(Clone, PartialEq)]
pub(super) struct Selection {
    pub task: Option<TaskCatalogEntry>,
    pub requester: PermissionRequester,
    pub can_edit_rates: bool,
}

#[component]
pub(super) fn TaskEditor(
    mut selected: Signal<Option<Selection>>,
    on_saved: EventHandler<String>,
    on_denied: EventHandler<()>,
) -> Element {
    let busy = use_signal(|| false);
    let key = selected
        .read()
        .as_ref()
        .and_then(|selection| selection.task.as_ref())
        .map(|task| task.id.to_string())
        .unwrap_or_else(|| "new".into());
    rsx! {
        Modal { id: "task-editor", labelledby: "task-editor-title", open: selected.read().is_some(), busy: busy(),
            focus_fallback: Some("tasks-refresh"),
            on_dismiss: move |_| { if !busy() { selected.set(None); } },
            if let Some(selection) = selected() {
                TaskForm { key: "{key}", selection, busy,
                    on_saved, on_denied }
            }
        }
    }
}

#[component]
fn TaskForm(
    selection: Selection,
    mut busy: Signal<bool>,
    on_saved: EventHandler<String>,
    on_denied: EventHandler<()>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let mut name = use_signal(|| {
        selection
            .task
            .as_ref()
            .map(|task| task.name.clone())
            .unwrap_or_default()
    });
    let mut billable = use_signal(|| {
        selection
            .task
            .as_ref()
            .is_none_or(|task| task.billable_default)
    });
    let mut common = use_signal(|| selection.task.as_ref().is_some_and(|task| task.is_default));
    let toggle_common = use_callback(move |_: MouseEvent| common.set(!common()));
    let mut rate_action = use_signal(|| {
        if selection.task.is_none() {
            "set"
        } else {
            "preserve"
        }
        .to_string()
    });
    let pick_rate_action = use_callback(move |value: String| rate_action.set(value));
    let mut amount = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let task_id = selection.task.as_ref().map(|task| task.id);
    let requester = selection.requester;
    let can_edit_rates = selection.can_edit_rates;
    let primary = ModalAction::submit(
        if busy() { "Saving…" } else { "Save task" },
        "task-edit-form",
    )
    .id("task-save")
    .disabled(busy());
    rsx! {
        ModalActions { primary }
        ModalHeader { id: "task-editor-title", title: if task_id.is_some() { "Edit task" } else { "New task" } }
        ModalBody {
        form { id: "task-edit-form", onsubmit: move |event| {
            event.prevent_default();
            if busy() { return; }
            if name.read().trim().is_empty() { error.set(Some("Enter a task name.".into())); return; }
            let rate = match rate_action.read().as_str() {
                "set" if task_id.is_none() && amount.read().trim().is_empty() => TaskRateEdit::Preserve {},
                "clear" if can_edit_rates => TaskRateEdit::Clear {},
                "set" if can_edit_rates => {
                    let typed = amount.read();
                    match RateTemplate::parse(&typed) {
                        Ok(template) => TaskRateEdit::Set { amount_cents: template.hundredths() },
                        // Zero is no rate.
                        Err(_) if parse_cents(&typed).is_ok_and(|cents| cents == 0) => TaskRateEdit::Clear {},
                        Err(_) => { error.set(Some(tr.tasks_rate_invalid())); return; }
                    }
                }
                _ => TaskRateEdit::Preserve {},
            };
            busy.set(true); error.set(None);
            let edited_name = name(); let edited_billable = billable(); let edited_common = common();
            spawn(async move {
                let (result, message) = if let Some(id) = task_id {
                    (server_fns::update_task(id.to_string(), edited_name, edited_billable, edited_common, rate, requester).await, "Task updated.")
                } else {
                    (server_fns::create_task(edited_name, edited_billable, edited_common, rate, requester).await, "Task created.")
                };
                finish(result, busy, error, on_saved, on_denied, message);
            });
        },
                if let Some(message) = error() { p { id: "task-edit-error", role: "alert", class: "alert alert-danger", "{message}" } }
                FormGroup { id: "task-name", label: "Task name",
                    Input { id: "task-name", value: name(), disabled: busy(), oninput: move |event: FormEvent| name.set(event.value()) }
                }
                div { class: "flex flex-col gap-3",
                    Checkbox { id: "task-billable", label: "Billable by default", checked: billable(), disabled: busy(), onclick: move |_| billable.set(!billable()) }
                    div {
                        Checkbox { id: "task-common", hint_id: "task-common-hint".to_string(), label: "Common task", checked: common(), disabled: busy(), onclick: toggle_common }
                        p { id: "task-common-hint", class: "form-hint", "While active, it is added to every new project." }
                    }
                }
                p { class: "form-hint", "Changes to defaults apply when adding the task to projects; existing project settings stay unchanged." }
                if can_edit_rates {
                    if let Some(task) = &selection.task {
                        p { class: "text-sm text-secondary mt-3", {tr.tasks_rate_current(rate_label(tr, task.default_rate_cents))} }
                        FormGroup { id: "task-rate-action", label: tr.tasks_rate_label(),
                            SelectField { id: "task-rate-action", label: tr.tasks_rate_label(), disabled: busy(), selected: rate_action(),
                                options: vec![
                                    ("preserve".into(), tr.tasks_rate_keep()),
                                    ("set".into(), tr.tasks_rate_set()),
                                    ("clear".into(), tr.tasks_rate_clear()),
                                ],
                                onselect: pick_rate_action,
                            }
                        }
                    }
                    if rate_action() == "set" {
                        // An existing task also has the rate action select, named for the rate.
                        FormGroup { id: "task-rate", label: if task_id.is_some() { tr.tasks_rate_amount_label() } else { tr.tasks_rate_label() },
                            Input { id: "task-rate", value: amount(), disabled: busy(), oninput: move |event: FormEvent| amount.set(event.value()) }
                        }
                        p { class: "form-hint", {tr.tasks_rate_template_hint()} }
                        if task_id.is_none() { p { class: "form-hint", {tr.tasks_rate_optional_hint()} } }
                    }
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
        Err(ServerFnError::ServerError { code: BAD_REQUEST | CONFLICT, message, .. }) => error.set(Some(message)),
        Err(_) => error.set(Some("Could not confirm the save. Your inputs are kept. Refresh tasks to check whether it was saved before retrying.".into())),
    }
}
