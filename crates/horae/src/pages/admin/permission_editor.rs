use dioxus::prelude::*;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::FormGroup;
use crate::components::modal::Modal;
use crate::components::permission_description::permission_description;
use crate::models::permission_editor::{ProfileAction, ProfilePreview, ProfileSource};
use crate::server_fns;

#[path = "permission_editor/draft.rs"]
mod draft;
use draft::DraftState;

const BAD_REQUEST: u16 = 400;
const UNAUTHORIZED: u16 = 401;
const FORBIDDEN: u16 = 403;
const NOT_FOUND: u16 = 404;
const CONFLICT: u16 = 409;
const PROFILE_FIELD: &str = "person-permissions-profile";
const REVIEW_BUTTON: &str = "permission-review";
const SAVE_BUTTON: &str = "permission-save";

#[component]
pub(super) fn PermissionEditorDialog(
    mut person: Signal<Option<Uuid>>,
    on_saved: EventHandler<bool>,
) -> Element {
    let mut locked = use_signal(|| false);
    let mut editor = use_resource(move || async move {
        match person() {
            Some(id) => server_fns::load_permission_editor(id).await.map(Some),
            None => Ok(None),
        }
    });
    let ready = editor.state()() == UseResourceState::Ready;
    rsx! {
        Modal {
            id: "person-permissions-dialog", labelledby: "person-permissions-title",
            open: person().is_some(), busy: locked(), large: true,
            on_dismiss: move |_| { if !locked() { person.set(None); } },
            div { class: "px-6 pt-6",
                h2 { id: "person-permissions-title", class: "text-2xl font-semibold", "Edit permissions" }
            }
            div { class: "modal-body wrap-anywhere",
                if !ready {
                    p { role: "status", class: "text-sm text-secondary", "Loading permissions…" }
                } else {
                    {match &*editor.read() {
                        Some(Ok(Some(value))) if Some(value.user_id) == person() => {
                            match DraftState::new(value.clone()) {
                                Ok(initial) => rsx! {
                                    PermissionForm {
                                        key: "{value.user_id}-{value.access_revision}-{value.permissions.revision}",
                                        initial, locked,
                                        on_reload: move |_| editor.restart(),
                                        on_saved: move |changed| {
                                            locked.set(false);
                                            person.set(None);
                                            on_saved.call(changed);
                                        },
                                        on_cancel: move |_| { if !locked() { person.set(None); } },
                                    }
                                },
                                Err(message) => rsx! {
                                    p { class: "text-danger text-sm", role: "alert", "{message}" }
                                    button { r#type: "button", class: "btn btn-secondary mt-4",
                                        onclick: move |_| editor.restart(), "Reload permissions"
                                    }
                                },
                            }
                        },
                        Some(Err(error)) => rsx! {
                            p { role: "alert", class: "text-danger text-sm", "{load_error(error)}" }
                            button { r#type: "button", class: "btn btn-secondary mt-4",
                                onclick: move |_| editor.restart(), "Reload permissions"
                            }
                        },
                        _ => rsx! {},
                    }}
                }
            }
            if !locked() {
                div { class: "px-6 pb-6",
                    button { r#type: "button", class: "btn btn-secondary",
                        onclick: move |_| person.set(None), "Close"
                    }
                }
            }
        }
    }
}

#[component]
fn PermissionForm(
    initial: DraftState,
    mut locked: Signal<bool>,
    on_reload: EventHandler<()>,
    on_saved: EventHandler<bool>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut state = use_signal(|| initial);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut reload_required = use_signal(|| false);
    let mut unavailable = use_signal(|| false);
    let snapshot = state.read();
    let frozen = busy() || snapshot.request.is_some() || reload_required() || unavailable();
    let administrator = snapshot.administrator();
    let selection_key = match snapshot.action {
        ProfileAction::Edit => "current".to_owned(),
        ProfileAction::BuiltIn { profile } => profile_label(profile).to_owned(),
        ProfileAction::Template { id, .. } => id.to_string(),
    };
    rsx! {
        if let Some(message) = error() {
            p { role: "alert", class: "text-danger text-sm mb-4", "{message}" }
        }
        if !unavailable() {
            h3 { class: "text-xl mb-2", "{snapshot.editor.name}" }
            p { class: "text-sm text-secondary mb-4", "Saved configuration: {source_label(&snapshot)}" }
            p { class: "text-sm mb-4",
                if snapshot.editor.permissions.is_administrator { "Current Administrator access: Yes" }
                else { "Current Administrator access: No" }
            }
            if !snapshot.editor.active {
                p { class: "text-sm text-secondary mb-4", "This person is inactive. Saving permissions will not activate their account." }
            }
            FormGroup { label: "Profile", id: "person-permissions-profile",
                select { id: PROFILE_FIELD, class: "form-select",
                    value: selection_key, disabled: frozen,
                    onchange: move |event| {
                        if busy() || state.peek().request.is_some() || reload_required() || unavailable() { return; }
                        let value = event.value();
                        let result = if value == "current" {
                            let restored = DraftState::new(state.peek().editor.clone());
                            restored.map(|restored| state.set(restored))
                        } else if let Some(profile) = BuiltInProfile::ALL.iter().find(|p| profile_label(**p) == value).copied() {
                            state.write().choose_builtin(profile)
                        } else if let Ok(id) = value.parse::<Uuid>() {
                            state.write().choose_template(id)
                        } else {
                            Err("This profile is unavailable. Reload to try again.".into())
                        };
                        error.set(result.err());
                    },
                    option { value: "current", "Current configuration" }
                    for profile in BuiltInProfile::ALL.iter().copied() {
                        option { value: profile_label(profile), "{profile_label(profile)}" }
                    }
                    for template in &snapshot.editor.templates {
                        option { value: "{template.id}", "{template.name} (custom profile)" }
                    }
                }
            }
            if !matches!(snapshot.action, ProfileAction::Edit) {
                button { r#type: "button", class: "btn btn-secondary btn-sm mb-4", disabled: frozen,
                    onclick: move |_| {
                        if busy() || reload_required() || unavailable() { return; }
                        let action = state.peek().action.clone();
                        let result = match action {
                            ProfileAction::BuiltIn { profile } => state.write().choose_builtin(profile),
                            ProfileAction::Template { id, .. } => state.write().choose_template(id),
                            ProfileAction::Edit => Ok(()),
                        };
                        error.set(result.err());
                    }, "Reset selected profile"
                }
            }
            if administrator {
                p { class: "banner banner-warning mb-4",
                    "Administrator access is unrestricted. Choose a different profile to limit access."
                }
            } else {
                p { class: "text-sm text-secondary mb-3",
                    "Required permissions are included automatically. Removing one also removes permissions that depend on it. Own time and expense permissions are always included."
                }
                div { class: "flex flex-col gap-3 mb-5",
                    for permission in Permission::ALL.iter().copied() {
                        Checkbox {
                            key: "{permission:?}", checked: snapshot.selection.contains(permission),
                            id: format!("permission-{permission:?}"),
                            label: permission_description(permission),
                            disabled: frozen || is_baseline(permission),
                            onclick: move |_| {
                                if !busy() && !reload_required() && !unavailable() {
                                    error.set(state.write().toggle(permission).err());
                                }
                            },
                        }
                    }
                }
            }
            p { class: "text-sm text-secondary mb-4",
                "Permissions do not enable unavailable features or remove approval and invoice locks. Changes apply only after confirmation."
            }
            if let Some(preview) = &snapshot.preview {
                {preview_content(preview)}
                if !preview.remove_projects.is_empty() {
                    Checkbox { id: "permission-keep-projects", checked: false, disabled: frozen,
                        label: "Keep project access by adding managed-project reading and editing",
                        onclick: move |_| {
                            if !busy() && !reload_required() && !unavailable() {
                                error.set(state.write().keep_project_access().err());
                            }
                        },
                    }
                    p { class: "text-sm text-secondary my-3", "Choosing this changes the draft. Review it again before saving; person-management removals are evaluated separately." }
                }
                if !preview.remove_projects.is_empty() || !preview.remove_people.is_empty() {
                    Checkbox { id: "permission-confirm-removals", checked: snapshot.confirmed, disabled: frozen,
                        label: "I confirm all management removals listed above",
                        onclick: move |_| {
                            if !busy() && state.peek().request.is_none() && !reload_required() && !unavailable() {
                                let confirmed = state.peek().confirmed;
                                state.write().confirmed = !confirmed;
                            }
                        },
                    }
                }
            }
        }
        div { class: "flex flex-wrap items-center gap-3 mt-5",
            if reload_required() || unavailable() {
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(),
                    onclick: move |_| { if !busy() { on_reload.call(()); } }, "Reload permissions"
                }
            } else if snapshot.preview.is_some() {
                button { id: SAVE_BUTTON, r#type: "button", class: "btn btn-primary",
                    disabled: busy() || (snapshot.request.is_none() && !snapshot.confirmed && snapshot.preview.as_ref().is_some_and(|p| !p.remove_projects.is_empty() || !p.remove_people.is_empty())),
                    onclick: move |_| {
                        if busy() || reload_required() || unavailable() { return; }
                        let Some(command) = state.write().begin_save() else { return; };
                        busy.set(true);
                        locked.set(true);
                        error.set(None);
                        spawn(async move {
                            match server_fns::save_person_permissions(command).await {
                                Ok(outcome) => { locked.set(false); on_saved.call(outcome.changed); },
                                Err(problem) => {
                                    if definite_rejection(&problem) {
                                        state.write().request = None;
                                        state.write().invalidate_preview();
                                        reload_required.set(true);
                                        unavailable.set(access_denied(&problem));
                                        locked.set(false);
                                        error.set(Some(rejection_message(&problem)));
                                    } else {
                                        error.set(Some("The save could not be confirmed. Retry the same save to check its result before making more changes.".into()));
                                    }
                                    busy.set(false);
                                },
                            }
                        });
                    },
                    if busy() { "Saving…" } else if snapshot.request.is_some() { "Retry same save" } else { "Confirm permissions" }
                }
                if snapshot.request.is_none() {
                    button { r#type: "button", class: "btn btn-secondary", disabled: busy(),
                        onclick: move |_| { if !busy() { state.write().invalidate_preview(); } }, "Back to editing"
                    }
                }
            } else {
                button { id: REVIEW_BUTTON, r#type: "button", class: "btn btn-primary", disabled: frozen,
                    onclick: move |_| {
                        if busy() || state.peek().request.is_some() || reload_required() || unavailable() { return; }
                        let proposal = state.peek().draft();
                        busy.set(true);
                        locked.set(true);
                        error.set(None);
                        spawn(async move {
                            match server_fns::preview_person_permissions(proposal).await {
                                Ok(preview) => {
                                    if let Err(message) = state.write().reviewed(preview) {
                                        error.set(Some(message));
                                        reload_required.set(true);
                                    }
                                },
                                Err(problem) => {
                                    unavailable.set(access_denied(&problem));
                                    reload_required.set(definite_rejection(&problem));
                                    error.set(Some(rejection_message(&problem)));
                                },
                            }
                            busy.set(false);
                            locked.set(false);
                        });
                    },
                    if busy() { "Reviewing…" } else { "Review changes" }
                }
            }
            button { r#type: "button", class: "btn btn-secondary", disabled: locked(),
                onclick: move |_| { if !locked() { on_cancel.call(()); } }, "Cancel"
            }
        }
    }
}

fn profile_label(profile: BuiltInProfile) -> &'static str {
    match profile {
        BuiltInProfile::Member => "Member",
        BuiltInProfile::ProjectManager => "Project Manager",
        BuiltInProfile::PeopleAdmin => "People Admin",
        BuiltInProfile::Accounting => "Accounting",
        BuiltInProfile::ExecutiveManager => "Executive Manager",
        BuiltInProfile::Administrator => "Administrator",
    }
}

fn source_label(state: &DraftState) -> String {
    match state.editor.permissions.source {
        ProfileSource::BuiltIn(profile) => profile_label(profile).into(),
        ProfileSource::Template { id, .. } => state
            .editor
            .templates
            .iter()
            .find(|template| template.id == id)
            .map(|template| template.name.clone())
            .unwrap_or_else(|| "Custom configuration".into()),
        ProfileSource::Individual => "Individual configuration".into(),
    }
}

fn is_baseline(permission: Permission) -> bool {
    BuiltInProfile::Member
        .direct_permissions()
        .contains(&permission)
}

fn load_error(error: &ServerFnError) -> &'static str {
    match error {
        ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => "Sign in again to edit permissions.",
        ServerFnError::ServerError {
            code: FORBIDDEN, ..
        } => "Permission editing is unavailable for this account or workspace.",
        ServerFnError::ServerError {
            code: NOT_FOUND, ..
        } => "This person or profile is unavailable.",
        _ => "Could not load permissions. Reload to try again.",
    }
}

fn access_denied(error: &ServerFnError) -> bool {
    matches!(
        error,
        ServerFnError::ServerError {
            code: UNAUTHORIZED | FORBIDDEN | NOT_FOUND,
            ..
        }
    )
}

fn definite_rejection(error: &ServerFnError) -> bool {
    matches!(
        error,
        ServerFnError::ServerError {
            code: BAD_REQUEST | UNAUTHORIZED | FORBIDDEN | NOT_FOUND | CONFLICT,
            ..
        }
    )
}

fn rejection_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError {
            code: BAD_REQUEST | CONFLICT,
            message,
            ..
        } => format!("{message}. Reload permissions and review your changes again."),
        _ => load_error(error).into(),
    }
}

fn preview_content(preview: &ProfilePreview) -> Element {
    let added = preview
        .after
        .grants
        .iter()
        .filter(|grant| !preview.before.grants.contains(grant));
    let removed = preview
        .before
        .grants
        .iter()
        .filter(|grant| !preview.after.grants.contains(grant));
    rsx! {
        section { class: "border rounded-lg p-4 mb-4", aria_labelledby: "permission-review-title",
            h3 { id: "permission-review-title", class: "text-xl mb-3", "Review changes" }
            if !preview.changed { p { class: "text-sm", "No changes to the saved configuration." } }
            if preview.before.source != preview.after.source {
                p { class: "text-sm mb-3", "The saved profile configuration will change." }
            }
            if preview.before.is_administrator != preview.after.is_administrator {
                p { class: "text-sm font-semibold mb-3",
                    if preview.after.is_administrator { "This person will become an Administrator." }
                    else { "This person will no longer be an Administrator." }
                }
            }
            ul { class: "text-sm flex flex-col gap-2 pl-5",
                for permission in added { li { "Allow: {permission_description(*permission)}" } }
                for permission in removed { li { "Remove: {permission_description(*permission)}" } }
            }
            if !preview.remove_projects.is_empty() || !preview.remove_people.is_empty() {
                p { class: "text-sm font-semibold mt-4", "Management responsibilities to remove" }
                p { class: "text-sm text-secondary mt-2", "Membership and existing work will be preserved. Restoring permissions later will not restore these responsibilities." }
                ul { class: "text-sm flex flex-col gap-2 pl-5 mt-3",
                    for item in &preview.remove_projects { li { "Project: {item.subject_id}" } }
                    for item in &preview.remove_people { li { "Person: {item.subject_id}" } }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "server"))]
#[path = "permission_editor/tests.rs"]
mod tests;
