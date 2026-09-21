use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::project::{BudgetMode, RateMode};
use horae_core::types::ProjectType;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::Input;
use crate::components::modal::Modal;
use crate::models::project_creation::{
    CreationOptions, CreationSearch, ProjectForm, ProjectTaskInput, TaskAccess, TaskSource,
};
use crate::server_fns;

#[component]
pub(super) fn Tasks(
    mut form: Signal<ProjectForm>,
    mut options: Signal<CreationOptions>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut editing_access = use_signal(|| None::<Uuid>);
    let mut results = use_resource(move || {
        let query = query();
        async move {
            let mut search = CreationSearch::default();
            search.tasks.query = query;
            server_fns::project_creation_options(search).await
        }
    });
    let pending = results.state()() != UseResourceState::Ready;
    let search_ready = !pending && matches!(&*results.read(), Some(Ok(_)));
    let choices = results
        .read()
        .as_ref()
        .filter(|_| !pending)
        .and_then(|result| result.as_ref().ok())
        .map(|result| result.tasks.clone())
        .unwrap_or_default();
    let add = use_callback(move |source: TaskSource| {
        let mut draft = form.write();
        if draft.tasks.len() >= 500 {
            error.set(Some("A project can have at most 500 tasks.".into()));
            return;
        }
        if draft.tasks.iter().any(|task| task.source == source) {
            error.set(Some("This task is already selected.".into()));
            return;
        }
        let billable = match &source {
            TaskSource::Existing { task_id } => options
                .peek()
                .tasks
                .iter()
                .find(|task| task.id == *task_id)
                .is_some_and(|task| task.billable),
            TaskSource::New { .. } => true,
        };
        draft.tasks.push(ProjectTaskInput {
            id: Uuid::now_v7(),
            source,
            billable,
            rate: String::new(),
            budget: String::new(),
            access: TaskAccess::Everyone,
        });
        error.set(None);
        query.set(String::new());
    });
    let add_named = use_callback(move |_| {
        if results.state()() != UseResourceState::Ready || !matches!(&*results.read(), Some(Ok(_)))
        {
            return;
        }
        let name = query.read().trim().to_string();
        if name.is_empty() || name.chars().count() > 200 {
            error.set(Some("Enter a task name from 1 to 200 characters.".into()));
            return;
        }
        let existing = results
            .read()
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|result| {
                result
                    .tasks
                    .iter()
                    .find(|task| task.name.to_lowercase() == name.to_lowercase())
            })
            .cloned();
        if let Some(task) = existing {
            let id = task.id;
            if !options.peek().tasks.iter().any(|item| item.id == id) {
                options.write().tasks.push(task);
            }
            add.call(TaskSource::Existing { task_id: id });
        } else {
            add.call(TaskSource::New { name });
        }
    });
    let billable_project = form.read().project_type != ProjectType::NonBillable;
    rsx! {
        section { class: "bg-secondary border rounded-xl mt-2", aria_labelledby: "np-tasks-heading",
            div { class: "flex flex-wrap items-baseline gap-3 py-4 px-5 border-b",
                h2 { id: "np-tasks-heading", class: "text-xl font-semibold m-0", "Tasks" }
                span { class: "text-xs text-subtle", "{form.read().tasks.len()} tasks" }
                if billable_project {
                    div { class: "flex items-center gap-2 ml-auto",
                        span { class: "text-xs text-faint", "Billable" }
                        button { r#type: "button", class: "btn btn-ghost btn-sm", onclick: move |_| { for task in &mut form.write().tasks { task.billable = true; } }, "All" }
                        button { r#type: "button", class: "btn btn-ghost btn-sm", onclick: move |_| { for task in &mut form.write().tasks { task.billable = false; } }, "None" }
                    }
                }
            }
            p { class: "text-xs text-subtle px-5 py-3 m-0 border-b", "Everyone on the project can track to unrestricted tasks. Open a task's access settings to limit who can." }
            for task in form.read().tasks.clone() {
                TaskRow { key: "{task.id}", form, options, id: task.id, on_access: move |id| editing_access.set(Some(id)) }
            }
            if form.read().tasks.is_empty() { p { class: "text-sm text-subtle px-5", "No tasks selected yet." } }
            div { class: "px-5 py-3",
                if form.read().project_type == ProjectType::TimeAndMaterials && form.read().rate_mode == RateMode::Task {
                    p { class: "form-hint mt-0", "Blank rates inherit the catalog rate, then the client's default. Rates are not converted between currencies." }
                }
                if let Some(message) = error() { p { class: "text-sm text-danger", role: "alert", "{message}" } }
                div { class: "flex flex-wrap items-center gap-3", aria_busy: pending,
                    div { class: "input-group flex-1 basis-assignment-picker min-w-0 h-10 px-3 gap-2",
                        span { class: "flex items-center text-faint", aria_hidden: "true", "+" }
                        input { class: "input-group-field p-0", id: "np-task-search", aria_label: "Find or create a task", value: query(), placeholder: "Add a task and press Enter", oninput: move |event| query.set(event.value()), onkeydown: move |event| { if event.key() == Key::Enter { event.prevent_default(); add_named.call(()); } } }
                    }
                    button { r#type: "button", class: "btn btn-secondary", disabled: !search_ready || query.read().trim().is_empty(), onclick: move |_| {
                        add_named.call(());
                        document::eval("document.getElementById('np-task-search')?.focus()");
                    }, "Add task" }
                    for task in choices {
                        button { key: "{task.id}", r#type: "button", class: "btn btn-secondary btn-sm rounded-full",
                            disabled: form.read().tasks.iter().any(|item| matches!(item.source, TaskSource::Existing { task_id } if task_id == task.id)),
                            onclick: move |_| {
                                if results.state()() != UseResourceState::Ready { return; }
                                let id = task.id;
                                if !options.peek().tasks.iter().any(|item| item.id == id) { options.write().tasks.push(task.clone()); }
                                add.call(TaskSource::Existing { task_id: id });
                            },
                            "{task.name}"
                        }
                    }
                }
                p { class: "form-hint", "New catalog tasks are created only when you save this project." }
                if pending {
                    p { class: "text-sm text-subtle m-0", role: "status", "Searching tasks…" }
                } else {
                    match &*results.read() {
                        Some(Err(problem)) => rsx! {
                            p { class: "text-sm text-danger", role: "alert", "Could not search tasks: {problem}" }
                            button { r#type: "button", class: "btn btn-secondary btn-sm", onclick: move |_| results.restart(), "Retry task search" }
                        },
                        Some(Ok(result)) if result.tasks.is_empty() && !query.read().trim().is_empty() => rsx! { p { class: "text-sm text-subtle m-0", role: "status", "No matching tasks. Add this name as a new task." } },
                        Some(Ok(result)) if result.more_tasks => rsx! { p { class: "form-hint", "Showing 50 matches. Refine the name to find another task." } },
                        _ => rsx! {},
                    }
                }
            }
        }
        TaskAccessDialog { form, options, task_id: editing_access(), on_dismiss: move |_| editing_access.set(None) }
    }
}

#[component]
fn TaskRow(
    mut form: Signal<ProjectForm>,
    options: Signal<CreationOptions>,
    id: Uuid,
    on_access: EventHandler<Uuid>,
) -> Element {
    let Some(task) = form.read().tasks.iter().find(|task| task.id == id).cloned() else {
        return rsx! {};
    };
    let catalog = match &task.source {
        TaskSource::Existing { task_id } => options
            .read()
            .tasks
            .iter()
            .find(|item| item.id == *task_id)
            .cloned(),
        _ => None,
    };
    let name = match &task.source {
        TaskSource::New { name } => name.clone(),
        TaskSource::Existing { task_id } => catalog
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_else(|| format!("Unavailable task ({task_id})")),
    };
    let access_label = match &task.access {
        TaskAccess::Everyone => "Everyone".to_string(),
        TaskAccess::Restricted { user_ids } => format!("Restricted ({})", user_ids.len()),
    };
    let currency = form
        .read()
        .currency
        .clone()
        .or_else(|| {
            options
                .read()
                .clients
                .iter()
                .find(|client| Some(client.id) == form.read().client_id)
                .map(|client| client.currency.clone())
        })
        .unwrap_or_else(|| "project currency".into());
    let billable_project = form.read().project_type != ProjectType::NonBillable;
    rsx! {
        div { class: "np-assignment-row grid items-center gap-4 px-5 py-3 border-b border-light",
            Checkbox { checked: task.billable && billable_project, compact: true, disabled: !billable_project, label: "{name} is billable", onclick: move |_| { if let Some(task) = form.write().tasks.iter_mut().find(|task| task.id == id) { task.billable = !task.billable; } } }
            span { class: "text-sm text-strong truncate", title: "{name}", "{name}" }
            div { class: "np-row-controls flex flex-wrap items-center gap-4 min-w-0",
                if form.read().project_type == ProjectType::TimeAndMaterials && form.read().rate_mode == RateMode::Task {
                    label { class: "flex items-center gap-2 text-xs text-subtle", r#for: "np-task-rate-{id}",
                        "rate"
                        Input { class: "w-30 max-w-full font-mono text-right", id: "np-task-rate-{id}", label: "Hourly rate for {name} ({currency})", value: task.rate,
                            placeholder: if options.read().organization_currency == currency { catalog.as_ref().and_then(|task| task.default_rate_cents).map(format_cents_plain).unwrap_or_else(|| "Inherit".into()) } else { "Inherit".into() },
                            oninput: move |event: FormEvent| { if let Some(task) = form.write().tasks.iter_mut().find(|task| task.id == id) { task.rate = event.value(); } }
                        }
                        "{currency}/h"
                    }
                }
                if matches!(form.read().budget_mode, BudgetMode::HoursPerTask | BudgetMode::FeesPerTask) {
                    label { class: "flex items-center gap-2 text-xs text-subtle", r#for: "np-task-budget-{id}",
                        "budget"
                        Input { class: "w-30 max-w-full font-mono text-right", id: "np-task-budget-{id}", label: if form.read().budget_mode == BudgetMode::FeesPerTask { format!("Budget for {name} ({currency})") } else { format!("Budget hours for {name}") }, value: task.budget, oninput: move |event: FormEvent| { if let Some(task) = form.write().tasks.iter_mut().find(|task| task.id == id) { task.budget = event.value(); } } }
                        if form.read().budget_mode == BudgetMode::FeesPerTask { "{currency}" } else { "h" }
                    }
                }
                button { r#type: "button", class: "btn btn-ghost btn-sm", aria_label: "Access for {name}: {access_label}", onclick: move |_| on_access.call(id), "{access_label}" }
            }
            button { r#type: "button", class: "np-row-remove btn btn-ghost p-0 size-8", aria_label: "Remove task {name}", onclick: move |_| form.write().tasks.retain(|task| task.id != id), "×" }
        }
    }
}

#[component]
fn TaskAccessDialog(
    mut form: Signal<ProjectForm>,
    options: Signal<CreationOptions>,
    task_id: Option<Uuid>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let mut access = use_signal(|| TaskAccess::Everyone);
    use_effect(use_reactive!(|task_id| {
        if let Some(task) = form
            .peek()
            .tasks
            .iter()
            .find(|task| Some(task.id) == task_id)
        {
            access.set(task.access.clone());
        }
    }));
    rsx! {
        Modal { id: "np-task-access", labelledby: "np-task-access-title", open: task_id.is_some(), on_dismiss,
            div { class: "modal-body",
                h2 { id: "np-task-access-title", class: "modal-title", "Who can track to this task?" }
                fieldset { class: "border-0 p-0 m-0 flex flex-col gap-3", aria_label: "Task access",
                    label { class: "flex items-center gap-3",
                        input { r#type: "radio", name: "np-task-access-mode", checked: matches!(*access.read(), TaskAccess::Everyone), onchange: move |_| access.set(TaskAccess::Everyone) }
                        "Everyone on the project"
                    }
                    label { class: "flex items-center gap-3",
                        input { r#type: "radio", name: "np-task-access-mode", checked: matches!(*access.read(), TaskAccess::Restricted { .. }), onchange: move |_| access.set(TaskAccess::Restricted { user_ids: Vec::new() }) }
                        "Only selected people"
                    }
                }
                if let TaskAccess::Restricted { user_ids } = access() {
                    p { class: "text-sm text-subtle", "No selected people means nobody can track to this task." }
                    for member in form.read().team.clone() {
                        Checkbox { key: "{member.user_id}", checked: user_ids.contains(&member.user_id), label: options.read().people.iter().find(|person| person.id == member.user_id).map(|person| person.name.clone()).unwrap_or_else(|| format!("Unavailable teammate ({})", member.user_id)),
                            onclick: move |_| {
                                if let TaskAccess::Restricted { user_ids } = &mut *access.write() {
                                    if user_ids.contains(&member.user_id) { user_ids.retain(|id| *id != member.user_id); }
                                    else { user_ids.push(member.user_id); }
                                }
                            }
                        }
                    }
                    if form.read().team.is_empty() { p { class: "form-hint", "Add people to the project team to grant access." } }
                }
                div { class: "modal-actions",
                    button { r#type: "button", class: "btn btn-primary", onclick: move |_| {
                        if let Some(task) = form.write().tasks.iter_mut().find(|task| Some(task.id) == task_id) { task.access = access(); }
                        on_dismiss.call(());
                    }, "Apply access" }
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_dismiss.call(()), "Cancel" }
                }
            }
        }
    }
}
