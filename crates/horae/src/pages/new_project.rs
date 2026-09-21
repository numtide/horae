//! The creation screen keeps incomplete input separate from operational projects.

use dioxus::prelude::*;

use crate::components::modal::Modal;
use crate::models::project_creation::{CreationOptions, CreationSearch, ProjectDraft, TaskSource};
use crate::route::Route;
use crate::server_fns;

#[path = "new_project/basics.rs"]
mod basics;
#[path = "new_project/billing.rs"]
mod billing;
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Intent {
    Create,
    Leave,
    Discard,
}

#[component]
fn ProjectEditor(
    options: CreationOptions,
    draft: Option<ProjectDraft>,
    on_reload: EventHandler<()>,
) -> Element {
    let mut state = use_signal(|| DraftState::new(draft));
    let form = use_signal(|| state.peek().saved.clone());
    let options = use_signal(|| options);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut intent = use_signal(|| None::<Intent>);
    let mut discard_open = use_signal(|| false);
    let catalog_busy = use_signal(|| false);
    let navigator = use_navigator();

    use_effect(move || {
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
                                error.to_string()
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
                    None => {}
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

    let dirty = state.read().is_dirty(&form.read());
    let locked = intent().is_some() || catalog_busy();
    let status = if error().is_some() {
        "Changes need attention".to_string()
    } else if busy() || dirty {
        "Saving draft…".to_string()
    } else if let Some(at) = state.read().saved_at {
        format!("Draft saved at {} UTC", at.format("%H:%M:%S"))
    } else {
        "No draft saved yet".to_string()
    };
    let can_create = !form.read().name.trim().is_empty()
        && options
            .read()
            .clients
            .iter()
            .any(|client| Some(client.id) == form.read().client_id && client.active);

    rsx! {
        div { class: "np-page",
            button { r#type: "button", class: "btn btn-ghost text-sm", disabled: locked || error().is_some(), onclick: move |_| intent.set(Some(Intent::Leave)), "← Back to Projects" }
            header { class: "flex flex-wrap items-end gap-4 mt-5 pb-6 border-b",
                div {
                    div { class: "text-xs uppercase tracking-wider text-label", "Projects" }
                    h1 { class: "text-4xl font-semibold text-strong tracking-tight mt-2 mb-0", "New project" }
                }
                p { class: "text-xs text-subtle ml-auto mb-0", role: "status", aria_live: "polite", "{status}" }
            }
            if let Some(message) = error() {
                div { class: "alert alert-danger mt-4", role: "alert",
                    p { "{message}" }
                    p { class: "text-sm", "Your input is still here. Retry a failed request, or reload to resolve changes made in another tab." }
                    div { class: "flex flex-wrap gap-3",
                        button { class: "btn btn-secondary", r#type: "button", disabled: busy(), onclick: move |_| error.set(None), "Retry request" }
                        button { class: "btn btn-ghost", r#type: "button", disabled: busy(), onclick: move |_| on_reload.call(()), "Reload saved draft (lose local edits)" }
                    }
                }
            }
            fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: locked, aria_label: "Project settings",
                Basics { form, options }
                Visibility { form }
                Billing { form, options }
                Tasks { form, options }
                Team { form, options, busy: catalog_busy }
                InvoiceDefaults { form }
            }
            footer { class: "np-footer flex flex-wrap items-center gap-3 py-4 bg-base border-t",
                button { class: "btn btn-primary", r#type: "button", disabled: !can_create || locked || error().is_some(), onclick: move |_| intent.set(Some(Intent::Create)),
                    if intent() == Some(Intent::Create) { "Saving project…" } else { "Save project" }
                }
                button { class: "btn btn-secondary", r#type: "button", disabled: locked || error().is_some(), onclick: move |_| intent.set(Some(Intent::Leave)), "Cancel" }
                button { class: "btn btn-ghost ml-auto", r#type: "button", disabled: busy() || locked || error().is_some(), onclick: move |_| discard_open.set(true), "Discard draft" }
                p { class: "text-xs text-subtle m-0", "Cancel keeps your draft." }
            }
        }
        Modal { id: "np-discard-dialog", labelledby: "np-discard-title", open: discard_open(), on_dismiss: move |_| discard_open.set(false),
            div { class: "modal-body",
                h2 { id: "np-discard-title", class: "modal-title", "Discard this draft?" }
                p { "The draft will be removed. Clients you created will remain available." }
                div { class: "modal-actions",
                    button { class: "btn btn-danger", onclick: move |_| { discard_open.set(false); intent.set(Some(Intent::Discard)); }, "Discard draft" }
                    button { class: "btn btn-secondary", onclick: move |_| discard_open.set(false), "Keep editing" }
                }
            }
        }
    }
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

fn is_definite_rejection(error: &ServerFnError) -> bool {
    matches!(error, ServerFnError::ServerError { code, .. } if (400..500).contains(code))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        div { class: "np-row grid gap-6 py-5 border-b",
            div { class: "np-label pt-3",
                if id.is_empty() { div { class: "text-sm font-semibold text-strong", "{label}" } }
                else { label { r#for: id, class: "text-sm font-semibold text-strong", "{label}" } }
                if !hint.is_empty() { p { class: "text-xs text-faint mt-1 mb-0", "{hint}" } }
            }
            div { class: "min-w-0", {children} }
        }
    }
}
