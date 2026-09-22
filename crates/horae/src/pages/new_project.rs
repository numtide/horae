//! Shared project fields with separate draft-creation and explicit-edit lifecycles.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::icons::NavIcon;
use crate::components::modal::Modal;
use crate::models::project_creation::{
    CreationOptions, CreationSearch, EditableProject, ProjectDraft, ProjectEditRequest,
    ProjectFormField, TaskSource,
};
use crate::route::Route;
use crate::server_fns;

#[path = "new_project/basics.rs"]
mod basics;
#[path = "new_project/billing.rs"]
mod billing;
#[path = "new_project/date_field.rs"]
mod date_field;
#[path = "new_project/draft.rs"]
mod draft;
#[path = "new_project/invoice_defaults.rs"]
mod invoice_defaults;
#[path = "new_project/tasks.rs"]
mod tasks;
#[path = "new_project/team.rs"]
mod team;
use basics::Basics;
use billing::Billing;
use draft::DraftState;
use invoice_defaults::InvoiceDefaults;
use tasks::Tasks;
use team::{Team, Visibility};

#[component]
pub fn NewProject() -> Element {
    let mut initial = use_resource(|| async {
        let mut options = server_fns::project_creation_options(CreationSearch::default()).await?;
        let draft = server_fns::load_project_draft().await?;
        if let Some(id) = draft.as_ref().and_then(|draft| draft.form.client_id)
            && !options.clients.iter().any(|client| client.id == id)
            && let Some(client) = server_fns::project_creation_client(id).await?
        {
            options.clients.push(client);
        }
        if let Some(draft) = &draft {
            let task_ids: Vec<_> = draft
                .form
                .tasks
                .iter()
                .filter_map(|task| match &task.source {
                    TaskSource::Existing { task_id }
                        if !options.tasks.iter().any(|task| task.id == *task_id) =>
                    {
                        Some(*task_id)
                    }
                    _ => None,
                })
                .collect();
            let user_ids: Vec<_> = draft
                .form
                .team
                .iter()
                .map(|member| member.user_id)
                .filter(|id| !options.people.iter().any(|person| person.id == *id))
                .collect();
            if !task_ids.is_empty() || !user_ids.is_empty() {
                let selected = server_fns::project_creation_selection(task_ids, user_ids).await?;
                options.tasks.extend(selected.tasks);
                options.people.extend(selected.people);
            }
        }
        Ok::<_, ServerFnError>((options, draft))
    });
    if initial.state()() != UseResourceState::Ready {
        return rsx! { p { role: "status", "Loading project settings…" } };
    }
    match &*initial.read() {
        Some(Ok((options, draft))) => rsx! {
            ProjectEditor { options: options.clone(), draft: draft.clone(), on_reload: move |_| initial.restart() }
        },
        Some(Err(error)) => rsx! {
            h1 { class: "text-4xl font-semibold text-strong", "New project" }
            div { class: "alert alert-danger", role: "alert", "Could not load project settings: {error}" }
            button { class: "btn btn-secondary", onclick: move |_| initial.restart(), "Retry" }
            Link { to: Route::ProjectList {}, class: "btn btn-ghost", "Back to Projects" }
        },
        None => rsx! { p { role: "status", "Loading project settings…" } },
    }
}

#[component]
pub fn EditProject(id: Uuid) -> Element {
    let mut initial = use_resource(use_reactive!(|id| async move {
        let mut options = server_fns::project_creation_options(CreationSearch::default()).await?;
        let project = server_fns::load_project_editor(id).await?;
        options
            .clients
            .retain(|client| client.id != project.client.id);
        options.clients.push(project.client.clone());
        for task in &project.selection.tasks {
            options.tasks.retain(|existing| existing.id != task.id);
            options.tasks.push(task.clone());
        }
        for person in &project.selection.people {
            options.people.retain(|existing| existing.id != person.id);
            options.people.push(person.clone());
        }
        Ok::<_, ServerFnError>((options, project))
    }));
    if initial.state()() != UseResourceState::Ready {
        return rsx! { p { role: "status", "Loading project…" } };
    }
    match &*initial.read() {
        Some(Ok((options, project))) if project.id == id => rsx! {
            ProjectEditor {
                key: "{project.id}-{project.revision}",
                options: options.clone(), draft: None, existing: Some(project.clone()),
                on_reload: move |_| initial.restart(),
            }
        },
        Some(Err(error)) => rsx! {
            h1 { class: "text-4xl font-semibold text-strong", "Edit project" }
            div { class: "alert alert-danger", role: "alert", "Could not load project: {error}" }
            button { class: "btn btn-secondary", onclick: move |_| initial.restart(), "Retry" }
            Link { to: Route::ProjectList {}, class: "btn btn-ghost", "Back to Projects" }
        },
        Some(Ok(_)) | None => rsx! { p { role: "status", "Loading project…" } },
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Create,
    Save,
    Leave,
    Discard,
}

#[component]
fn ProjectEditor(
    options: CreationOptions,
    draft: Option<ProjectDraft>,
    #[props(default)] existing: Option<EditableProject>,
    on_reload: EventHandler<()>,
) -> Element {
    let existing = use_signal(|| existing);
    let editing = existing.read().is_some();
    let mut state = use_signal(|| DraftState::new(draft));
    let form = use_signal(|| {
        existing.peek().as_ref().map_or_else(
            || state.peek().saved.clone(),
            |project| project.form.clone(),
        )
    });
    let options = use_signal(|| options);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut invalid_field = use_signal(|| None::<ProjectFormField>);
    let mut intent = use_signal(|| None::<Intent>);
    let mut discard_open = use_signal(|| false);
    let mut reload_open = use_signal(|| false);
    let mut pending_edit = use_signal(|| None::<ProjectEditRequest>);
    let catalog_busy = use_signal(|| false);
    let navigator = use_navigator();
    let request_intent = use_callback(move |action| {
        if editing && action == Intent::Leave {
            if existing
                .peek()
                .as_ref()
                .is_some_and(|project| project.form != *form.peek())
            {
                discard_open.set(true);
            } else {
                navigator.push(Route::ProjectList {});
            }
            return;
        }
        invalid_field.set(None);
        error.set(None);
        intent.set(Some(action));
    });

    use_effect(move || {
        if editing {
            return;
        }
        let current = form();
        let dirty = state.read().is_dirty(&current);
        let requested = intent();
        if busy() || error().is_some() || (!dirty && requested.is_none()) {
            return;
        }
        busy.set(true);
        spawn(async move {
            // Only one writer runs at a time. Typing during a request schedules
            // another snapshot after its acknowledgement, never a parallel save.
            if requested.is_none() {
                let mut observed = form.peek().clone();
                loop {
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::TimeoutFuture::new(600).await;
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
                    let current = form.peek().clone();
                    if current == observed || intent.peek().is_some() {
                        break;
                    }
                    observed = current;
                }
            }
            let current = form.peek().clone();
            let action = *intent.peek();
            let result = async {
                if state.peek().is_dirty(&current)
                    || (state.peek().revision == 0 && action == Some(Intent::Create))
                {
                    let id = state.peek().id;
                    let request = state.write().request(&current);
                    let saved = server_fns::save_project_draft(id, request.revision, request.form)
                        .await
                        .map_err(|error| {
                            if is_validation_error(&error) {
                                state.write().reject_validation();
                                intent.set(None);
                            }
                            error.to_string()
                        })?;
                    state.write().acknowledge(saved).map_err(str::to_string)?;
                }
                // A retry may have acknowledged an older snapshot. Save the
                // newer edits before completing the queued navigation or create.
                if state.peek().is_dirty(&form.peek()) {
                    return Ok::<_, String>(());
                }
                let id = state.peek().id;
                let revision = state.peek().revision;
                match action {
                    Some(Intent::Create) => {
                        let id = server_fns::finalize_project_draft(id, revision, current)
                            .await
                            .map_err(|error| {
                                if is_definite_rejection(&error) {
                                    intent.set(None);
                                }
                                invalid_field.set(validation_field(&error));
                                match error {
                                    ServerFnError::ServerError { message, .. } => message,
                                    other => other.to_string(),
                                }
                            })?;
                        intent.set(None);
                        navigator.push(Route::ProjectDetail { id });
                    }
                    Some(Intent::Leave) => {
                        intent.set(None);
                        navigator.push(Route::ProjectList {});
                    }
                    Some(Intent::Discard) => {
                        if revision > 0 {
                            server_fns::discard_project_draft(id, revision)
                                .await
                                .map_err(|error| error.to_string())?;
                        }
                        intent.set(None);
                        navigator.push(Route::ProjectList {});
                    }
                    Some(Intent::Save) | None => {}
                }
                Ok(())
            }
            .await;
            if let Err(message) = result {
                error.set(Some(message));
            }
            busy.set(false);
        });
    });

    use_effect(move || {
        if !editing || intent() != Some(Intent::Save) || busy() || error().is_some() {
            return;
        }
        let Some(project) = existing.peek().clone() else {
            return;
        };
        let request = pending_edit
            .write()
            .get_or_insert_with(|| ProjectEditRequest {
                id: Uuid::now_v7(),
                project_id: project.id,
                expected_revision: project.revision,
                form: form.peek().clone(),
            })
            .clone();
        busy.set(true);
        spawn(async move {
            match server_fns::save_project_editor(request).await {
                Ok(id) => {
                    pending_edit.set(None);
                    intent.set(None);
                    leave_project_editor(navigator, Route::ProjectDetail { id }).await;
                }
                Err(rejection) => {
                    if is_definite_rejection(&rejection) {
                        pending_edit.set(None);
                        intent.set(None);
                    }
                    invalid_field.set(validation_field(&rejection));
                    error.set(Some(match rejection {
                        ServerFnError::ServerError { message, .. } => message,
                        other => other.to_string(),
                    }));
                }
            }
            busy.set(false);
        });
    });

    let dirty = if let Some(project) = existing.read().as_ref() {
        project.form != *form.read()
    } else {
        state.read().is_dirty(&form.read())
    };
    let locked = intent().is_some() || catalog_busy();
    let unresolved_request = if editing {
        pending_edit.read().is_some()
    } else {
        error().is_some() && invalid_field().is_none()
    };
    use_effect(move || {
        if let Some(field) = invalid_field()
            && !busy()
        {
            let id = field_id(field);
            document::eval(&format!(
                "const field = document.getElementById('{id}'); field?.focus(); field?.scrollIntoView({{block: 'center'}});"
            ));
        }
    });
    let status = if error().is_some() {
        "Changes need attention".to_string()
    } else if editing {
        if busy() {
            "Saving changes…"
        } else if dirty {
            "Unsaved changes"
        } else {
            "No unsaved changes"
        }
        .to_owned()
    } else if busy() || dirty {
        "Saving draft…".to_string()
    } else if let Some(at) = state.read().saved_at {
        format!("Draft saved at {} UTC", at.format("%H:%M:%S"))
    } else {
        "No draft saved yet".to_string()
    };
    let can_create = !form.read().name.trim().is_empty()
        && options.read().clients.iter().any(|client| {
            Some(client.id) == form.read().client_id
                && (client.active
                    || existing
                        .read()
                        .as_ref()
                        .is_some_and(|project| project.form.client_id == Some(client.id)))
        });

    rsx! {
        div { class: "np-page flex flex-col h-full",
            "data-project-edit-state": if !editing { "clean" } else if locked || unresolved_request { "pending" } else if dirty { "dirty" } else { "clean" },
            div { class: "np-scroll flex-1 min-h-0 overflow-y-auto",
                div { class: "max-w-project-form px-project-form pt-6 pb-30",
                    button {
                        r#type: "button",
                        class: "btn btn-ghost text-sm text-muted font-normal border-0 px-2.5 py-1.5 -ml-2.5",
                        disabled: locked || unresolved_request,
                        onclick: move |_| request_intent.call(Intent::Leave),
                        span { class: "inline-flex", aria_hidden: "true", NavIcon { name: "arrow-left", class: "size-4" } }
                        "Back to Projects"
                    }
                    header { class: "flex flex-wrap items-end gap-4 mt-5 pb-6 border-b border-light",
                        div {
                            div { class: "text-xs uppercase tracking-eyebrow text-label",
                                "Projects"
                            }
                            h1 { class: "text-4xl font-semibold text-strong tracking-tight mt-2 mb-0",
                                if editing { "Edit project" } else { "New project" }
                            }
                        }
                        p {
                            class: "text-xs text-faint ml-auto mb-0",
                            role: "status",
                            aria_live: "polite",
                            "{status}"
                        }
                    }
                    if let Some(message) = error() {
                        div { class: "alert alert-danger mt-4", role: "alert",
                            p { id: "np-form-error-message", "{message}" }
                            p { class: "text-sm",
                                if editing {
                                    "Your input is still here. Correct the problem and save again. If another session changed this project, reload its current data before continuing."
                                } else if invalid_field().is_some() {
                                    "Your input is still here. Correct the indicated field and save again, or cancel to keep the draft."
                                } else {
                                    "Your input is still here. Retry a failed request, or reload to resolve changes made in another tab."
                                }
                            }
                            div { class: "flex flex-wrap gap-3",
                                button {
                                    class: "btn btn-secondary",
                                    r#type: "button",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        invalid_field.set(None); error.set(None);
                                        if editing { intent.set(Some(Intent::Save)); }
                                    },
                                    "Retry request"
                                }
                                button {
                                    class: "btn btn-ghost",
                                    r#type: "button",
                                    disabled: busy(),
                                    onclick: move |_| {
                                        if editing { reload_open.set(true); } else { on_reload.call(()); }
                                    },
                                    if editing { "Reload project…" } else { "Reload saved draft (lose local edits)" }
                                }
                            }
                        }
                    }
                    fieldset {
                        class: "border-0 p-0 m-0 min-w-0",
                        disabled: locked,
                        aria_label: "Project settings",
                        Basics { form, options, editing, invalid_field: invalid_field(), error_message: error() }
                        Visibility { form, legacy: existing.read().as_ref().is_some_and(|project| !project.configured) }
                        Billing { form, options, invalid_field: invalid_field(), error_message: error() }
                        Tasks { form, options, inactive_ids: existing.read().as_ref().map(|project| project.inactive_task_ids.clone()).unwrap_or_default(), invalid_field: invalid_field(), error_message: error() }
                        Team { form, options, busy: catalog_busy, inactive_ids: existing.read().as_ref().map(|project| project.inactive_user_ids.clone()).unwrap_or_default(), invalid_field: invalid_field(), error_message: error() }
                        if existing.read().as_ref().is_none_or(|project| project.configured) {
                            InvoiceDefaults { form, invalid_field: invalid_field(), error_message: error() }
                        } else {
                            p { class: "form-hint", "This project keeps its existing invoice defaults. Changing to the new billing configuration requires a migration." }
                        }
                    }
                }
            }
            footer { class: "np-footer flex flex-none flex-wrap items-center gap-3 py-4 px-project-form bg-cell-empty border-t border-light",
                button {
                    class: "btn btn-primary",
                    r#type: "button",
                    disabled: !can_create || locked || unresolved_request,
                    onclick: move |_| request_intent.call(if editing { Intent::Save } else { Intent::Create }),
                    if busy() && editing {
                        "Saving changes…"
                    } else if editing {
                        "Save changes"
                    } else if intent() == Some(Intent::Create) {
                        "Saving project…"
                    } else {
                        "Save project"
                    }
                }
                button {
                    class: "btn btn-secondary",
                    r#type: "button",
                    disabled: locked || unresolved_request,
                    onclick: move |_| request_intent.call(Intent::Leave),
                    "Cancel"
                }
                if !editing { button {
                    class: "btn btn-ghost ml-auto",
                    r#type: "button",
                    disabled: busy() || locked || unresolved_request,
                    onclick: move |_| discard_open.set(true),
                    "Discard draft"
                } }
                p { class: "text-xs text-subtle m-0", if editing { "Changes are saved only when you save." } else { "Cancel keeps your draft." } }
            }
        }
        Modal {
            id: "np-discard-dialog",
            labelledby: "np-discard-title",
            open: discard_open(),
            on_dismiss: move |_| discard_open.set(false),
            div { class: "modal-body",
                h2 { id: "np-discard-title", class: "modal-title", if editing { "Discard changes?" } else { "Discard this draft?" } }
                p { if editing { "Your unsaved changes will be lost. The saved project will not be changed." } else { "The draft will be removed. Clients you created will remain available." } }
                div { class: "modal-actions",
                    button {
                        class: "btn btn-danger",
                        onclick: move |_| {
                            discard_open.set(false);
                            if editing {
                                spawn(leave_project_editor(navigator, Route::ProjectList {}));
                            } else { request_intent.call(Intent::Discard); }
                        },
                        if editing { "Discard changes" } else { "Discard draft" }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| discard_open.set(false),
                        "Keep editing"
                    }
                }
            }
        }
        Modal {
            id: "np-reload-dialog", labelledby: "np-reload-title", open: reload_open(),
            on_dismiss: move |_| reload_open.set(false),
            div { class: "modal-body",
                h2 { id: "np-reload-title", class: "modal-title", "Reload project?" }
                p { "Your local edits will be discarded and replaced with the current saved project." }
                div { class: "modal-actions",
                    button { class: "btn btn-danger", onclick: move |_| on_reload.call(()), "Reload project" }
                    button { class: "btn btn-secondary", onclick: move |_| reload_open.set(false), "Keep editing" }
                }
            }
        }
    }
}

async fn leave_project_editor(navigator: dioxus::router::Navigator, destination: Route) {
    // Saving or explicitly confirming discard has already resolved the edits.
    let _ = document::eval(
        "document.querySelector('[data-project-edit-state]')?.setAttribute('data-project-edit-leaving', '');",
    )
    .await;
    navigator.push(destination);
}

fn is_validation_error(error: &ServerFnError) -> bool {
    const BAD_REQUEST: u16 = 400;
    matches!(
        error,
        ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        }
    )
}

fn validation_field(error: &ServerFnError) -> Option<ProjectFormField> {
    const BAD_REQUEST: u16 = 400;
    const NOT_FOUND: u16 = 404;
    const CONFLICT: u16 = 409;
    let ServerFnError::ServerError {
        code,
        details: Some(details),
        ..
    } = error
    else {
        return None;
    };
    let field = serde_json::from_value(details.get("field")?.clone()).ok()?;
    // Missing selections and archived-name conflicts are correctable. Draft
    // revision conflicts and uncertain responses must retain their retry path.
    match (*code, field) {
        (BAD_REQUEST, _)
        | (
            NOT_FOUND,
            ProjectFormField::Client | ProjectFormField::Task(_) | ProjectFormField::Person(_),
        )
        | (CONFLICT, ProjectFormField::TaskName(_)) => Some(field),
        _ => None,
    }
}

fn field_id(field: ProjectFormField) -> String {
    match field {
        ProjectFormField::Client => "np-client",
        ProjectFormField::Name => "np-name",
        ProjectFormField::Code => "np-code",
        ProjectFormField::StartsOn => "np-start",
        ProjectFormField::EndsOn => "np-end",
        ProjectFormField::Currency => "np-currency",
        ProjectFormField::AdminNotes => "np-notes",
        ProjectFormField::ProjectType => "np-project-type",
        ProjectFormField::RateMode => "np-rate-mode",
        ProjectFormField::ProjectRate => "np-project-rate",
        ProjectFormField::BudgetMode => "np-budget-mode",
        ProjectFormField::BudgetValue => "np-budget-value",
        ProjectFormField::BudgetAlert => "np-budget-alert",
        ProjectFormField::BudgetAlertAt => "np-alert-threshold",
        ProjectFormField::FeeAmount => "np-fee-amount",
        ProjectFormField::Milestones => "np-add-milestone",
        ProjectFormField::MilestoneName(id) => return format!("np-milestone-name-{id}"),
        ProjectFormField::MilestoneDate(id) => return format!("np-milestone-date-{id}"),
        ProjectFormField::MilestoneAmount(id) => return format!("np-milestone-amount-{id}"),
        ProjectFormField::TaskName(id) => return format!("np-task-remove-{id}"),
        ProjectFormField::Task(id) => return format!("np-task-remove-{id}"),
        ProjectFormField::Person(id) => return format!("np-person-remove-{id}"),
        ProjectFormField::TaskAccess(id) => return format!("np-task-access-{id}"),
        ProjectFormField::TaskRate(id) => return format!("np-task-rate-{id}"),
        ProjectFormField::TaskBudget(id) => return format!("np-task-budget-{id}"),
        ProjectFormField::PersonRate(id) => return format!("np-person-rate-{id}"),
        ProjectFormField::CostRate(id) => return format!("np-cost-rate-{id}"),
        ProjectFormField::PersonBudget(id) => return format!("np-person-budget-{id}"),
        ProjectFormField::PaymentTerms => "np-terms-days",
        ProjectFormField::PurchaseOrder => "np-po-number",
        ProjectFormField::Tax => "np-tax",
        ProjectFormField::SecondTaxName => "np-second-tax-name",
        ProjectFormField::SecondTax => "np-second-tax",
        ProjectFormField::Discount => "np-discount",
    }
    .to_owned()
}

fn is_definite_rejection(error: &ServerFnError) -> bool {
    matches!(error, ServerFnError::ServerError { code, .. } if (400..500).contains(code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_validation_fields_can_target_a_control() {
        const BAD_REQUEST: u16 = 400;
        const CONFLICT: u16 = 409;
        const NOT_FOUND: u16 = 404;
        const FORBIDDEN: u16 = 403;
        let row = uuid::Uuid::now_v7();
        for (code, details, expected) in [
            (
                BAD_REQUEST,
                Some(serde_json::json!({"field": {"milestone_amount": row}})),
                Some(ProjectFormField::MilestoneAmount(row)),
            ),
            (
                BAD_REQUEST,
                Some(serde_json::json!({"field": {"milestone_amount": "not-a-uuid"}})),
                None,
            ),
            (
                BAD_REQUEST,
                Some(serde_json::json!({"field": "tax"})),
                Some(ProjectFormField::Tax),
            ),
            (
                BAD_REQUEST,
                Some(serde_json::json!({"field": "unknown"})),
                None,
            ),
            (BAD_REQUEST, Some(serde_json::json!({"field": 42})), None),
            (BAD_REQUEST, None, None),
            (CONFLICT, Some(serde_json::json!({"field": "tax"})), None),
            (
                NOT_FOUND,
                Some(serde_json::json!({"field": "client"})),
                Some(ProjectFormField::Client),
            ),
            (
                NOT_FOUND,
                Some(serde_json::json!({"field": {"task": row}})),
                Some(ProjectFormField::Task(row)),
            ),
            (
                NOT_FOUND,
                Some(serde_json::json!({"field": {"person": row}})),
                Some(ProjectFormField::Person(row)),
            ),
            (
                CONFLICT,
                Some(serde_json::json!({"field": {"task_name": row}})),
                Some(ProjectFormField::TaskName(row)),
            ),
            (NOT_FOUND, Some(serde_json::json!({"field": "name"})), None),
            (
                FORBIDDEN,
                Some(serde_json::json!({"field": "client"})),
                None,
            ),
            (CONFLICT, None, None),
        ] {
            let error = ServerFnError::ServerError {
                code,
                details,
                message: "Rejected".into(),
            };
            assert_eq!(validation_field(&error), expected);
        }
        assert_eq!(field_id(ProjectFormField::Tax), "np-tax");
        assert_eq!(
            field_id(ProjectFormField::MilestoneAmount(row)),
            format!("np-milestone-amount-{row}")
        );
    }

    #[test]
    fn a_rejected_creation_can_be_edited_but_an_uncertain_commit_must_be_retried() {
        for code in [400, 403, 404, 409] {
            let error = ServerFnError::ServerError {
                message: "Rejected".into(),
                code,
                details: None,
            };
            assert!(is_definite_rejection(&error));
            assert_eq!(is_validation_error(&error), code == 400);
        }
        let uncertain = ServerFnError::ServerError {
            message: "Unavailable".into(),
            code: 500,
            details: None,
        };
        assert!(!is_definite_rejection(&uncertain));
        assert!(!is_validation_error(&uncertain));
    }
}

#[component]
fn FormRow(
    label: String,
    #[props(default)] id: String,
    #[props(default)] hint: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "np-row grid gap-6 py-5 border-b border-light",
            div { class: "np-label pt-3",
                if id.is_empty() { div { class: "text-sm font-semibold text-strong", "{label}" } }
                else { label { r#for: id, class: "text-sm font-semibold text-strong", "{label}" } }
                if !hint.is_empty() { p { class: "text-xs text-faint mt-1 mb-0", "{hint}" } }
            }
            div { class: "min-w-0", {children} }
        }
    }
}
