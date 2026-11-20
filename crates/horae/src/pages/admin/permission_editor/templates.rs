use dioxus::prelude::*;
use horae_core::permissions::catalog::Permission;
use uuid::Uuid;

use crate::components::button::ButtonVariant;
use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::form::{FormGroup, Input};
use crate::components::modal::{ModalAction, ModalActions, ModalCancel};
use crate::components::permission_description::permission_description;
use crate::components::screen_state::Loading;
use crate::models::permission_editor::{
    PermissionRequester, TemplateAction, TemplateChoice, TemplateCommand,
};
use crate::server_fns;

use super::recovery_storage::{
    AcknowledgedPermission, PendingCommand, PendingPermission, RecoveryError,
};
use super::{access_denied, definite_rejection, load_error, rejection_message};

const SAVE_BUTTON: &str = "permission-template-save";
const CANCEL_BUTTON: &str = "permission-template-cancel";
const RELOAD_BUTTON: &str = "permission-template-reload";

#[derive(Clone, PartialEq)]
pub(super) enum TemplateIntent {
    Create(Vec<Permission>),
    Delete(TemplateChoice),
}

#[component]
pub(super) fn TemplateEditor(
    intent: TemplateIntent,
    access_revision: i64,
    requester: PermissionRequester,
    mut locked: Signal<bool>,
    mut dirty: Signal<bool>,
    on_cancel: EventHandler<()>,
    on_reload: EventHandler<()>,
) -> Element {
    let initial = intent.clone();
    let preview = use_resource(move || {
        let initial = initial.clone();
        async move {
            match initial {
                TemplateIntent::Delete(template) => {
                    server_fns::preview_permission_template_deletion(
                        template.id,
                        access_revision,
                        template.revision,
                    )
                    .await
                    .map(Some)
                }
                TemplateIntent::Create(_) => Ok(None),
            }
        }
    });
    let mut name = use_signal(String::new);
    let mut pending = use_signal(|| None::<TemplateCommand>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut rejected = use_signal(|| false);
    let mut denied = use_signal(|| false);
    let mut saved = use_signal(|| false);
    let acknowledged = use_signal(|| None::<AcknowledgedPermission>);
    use_effect(move || dirty.set(!saved() && !name.read().is_empty()));
    let create = matches!(intent, TemplateIntent::Create(_));
    let preview_ready = preview.state()() == UseResourceState::Ready;
    let preview_pending = !create && !preview_ready;
    use_effect(move || {
        locked.set(
            (!create && preview.state()() != UseResourceState::Ready)
                || busy()
                || pending.read().is_some(),
        );
    });
    let deletion = preview.read();
    let matching_preview = match (&intent, &*deletion) {
        (TemplateIntent::Create(_), _) => true,
        (TemplateIntent::Delete(template), Some(Ok(Some(value)))) => {
            preview_ready && value.access_revision == access_revision && value.template == *template
        }
        _ => false,
    };
    let preview_failed =
        matches!(intent, TemplateIntent::Delete(_)) && preview_ready && !matching_preview;
    let freeze = preview_pending
        || busy()
        || pending.read().is_some()
        || rejected()
        || saved()
        || preview_failed;
    // A deletion is confirmed by its dialog once the preview matches.
    let ready_to_save = matching_preview
        && match &intent {
            TemplateIntent::Create(_) => !name.read().trim().is_empty(),
            TemplateIntent::Delete(_) => true,
        };
    let command_intent = intent.clone();
    let reload = use_callback(move |()| {
        if !busy() && pending.peek().is_none() {
            on_reload.call(());
        }
    });
    let save = use_callback(move |()| {
        if busy() || rejected() || saved() || preview_failed {
            return;
        }
        let command = if let Some(command) = pending.peek().as_ref() {
            command.clone()
        } else {
            if !ready_to_save {
                return;
            }
            let action = match &command_intent {
                TemplateIntent::Create(grants) => TemplateAction::Create {
                    name: name.peek().trim().to_owned(),
                    grants: grants.clone(),
                },
                TemplateIntent::Delete(template) => TemplateAction::Delete {
                    id: template.id,
                    expected_revision: template.revision,
                },
            };
            TemplateCommand {
                request_id: Uuid::now_v7(),
                expected_access_revision: access_revision,
                action,
            }
        };
        pending.set(Some(command.clone()));
        busy.set(true);
        locked.set(true);
        error.set(None);
        spawn(async move {
            let request = PendingPermission {
                requester,
                command: PendingCommand::Template(command),
            };
            match request.attempt(acknowledged).await {
                Ok(_) => {
                    pending.set(None);
                    saved.set(true);
                    locked.set(false);
                }
                Err(RecoveryError::Storage(message) | RecoveryError::Cleanup(message)) => {
                    error.set(Some(message))
                }
                Err(RecoveryError::Server(problem)) => {
                    if definite_rejection(&problem) {
                        pending.set(None);
                        rejected.set(true);
                        denied.set(access_denied(&problem));
                        locked.set(false);
                        error.set(Some(rejection_message(&problem)));
                    } else {
                        error.set(Some("The save could not be confirmed. Retry the same save before making another change.".into()));
                    }
                }
            }
            busy.set(false);
        });
    });
    let back = use_callback(move |()| {
        if !preview_pending && !busy() && pending.peek().is_none() {
            on_cancel.call(());
        }
    });
    let finished = saved() || rejected() || preview_failed;
    let label = if busy() {
        "Saving…"
    } else if acknowledged.read().is_some() {
        "Finish recovery cleanup"
    } else if pending.read().is_some() {
        "Retry same save"
    } else if create {
        "Create custom profile"
    } else {
        "Delete profile"
    };
    let preview_failure = preview_failed.then(|| match &*deletion {
        Some(Err(problem)) => load_error(problem),
        _ => "The profile or its affected people changed. Reload permissions before deleting it.",
    });
    if let TemplateIntent::Delete(template) = &intent {
        // Deleting a profile cannot be undone, so it is confirmed in a dialog
        // over the person's permissions; once it settles, only a reload is left.
        // The name shows only with a preview that matches it.
        let leave = if finished { reload } else { back };
        return rsx! {
            p { class: "text-sm text-secondary m-0", "Deleting a custom profile. Confirm or go back in the dialog." }
            ConfirmDialog { id: "permission-template-delete-dialog", labelledby: "permission-template-delete-title",
                open: true, busy: busy(),
                tone: if finished { ConfirmTone::Neutral } else { ConfirmTone::Danger },
                title: "Delete this custom profile?",
                confirm_label: if finished { "Reload and discard unsaved person changes" } else { label },
                confirm_disabled: !finished && (preview_pending || (!ready_to_save && pending.read().is_none())),
                cancel_label: "Back to person permissions",
                on_confirm: if finished { reload } else { save }, on_dismiss: leave,
                if let Some(message) = error() {
                    p { class: "text-danger", role: "alert", "{message}" }
                }
                if saved() {
                    p { role: "status", "Custom profile deleted. Everyone keeps their permissions." }
                } else if let Some(message) = preview_failure {
                    p { role: "alert", class: "text-danger", "{message}" }
                } else if !denied() {
                    p { class: "font-semibold", "{template.name}" }
                    if !preview_ready { Loading { label: "Loading affected people…" } }
                    else if let Some(Ok(Some(value))) = &*deletion {
                        p { "People using this profile: {value.people.len()}. They keep their permissions as individual configurations. Existing work and management responsibilities will not change." }
                        if value.people.is_empty() { p { "No people currently use this profile." } }
                        ul { class: "flex flex-col gap-2 pl-5",
                            for person in &value.people { li { key: "{person.user_id}", "{person.name}" } }
                        }
                    }
                }
                p { "This does not save the person's draft. Reloading discards unsaved person changes." }
            }
        };
    }
    let (primary, secondary) = if finished {
        let reload = ModalAction::click("Reload and discard unsaved person changes", reload)
            .variant(ButtonVariant::Secondary)
            .id(RELOAD_BUTTON);
        (reload, None)
    } else {
        let save = ModalAction::click(label, save)
            .id(SAVE_BUTTON)
            .disabled(busy() || (!ready_to_save && pending.read().is_none()));
        let back = ModalAction::click("Back to person permissions", back)
            .variant(ButtonVariant::Secondary)
            .id(CANCEL_BUTTON)
            .disabled(preview_pending || busy() || pending.read().is_some());
        (save, Some(back))
    };

    rsx! {
        ModalActions { primary, secondary,
            // Hidden while locked, as the dialog's own Close is.
            cancel: (!(preview_pending || busy() || pending.read().is_some()))
                .then(|| ModalCancel::new("Close").id(super::CLOSE_BUTTON)),
        }
        h3 { class: "text-xl mb-3", "Create custom profile" }
        if let Some(message) = error() {
            p { class: "text-sm text-danger mb-4", role: "alert", "{message}" }
        }
        if saved() {
            p { role: "status", class: "text-sm mb-4", "Custom profile created. No person's permissions were changed." }
        } else if !denied() {
            if let TemplateIntent::Create(grants) = &intent {
                FormGroup { label: "Profile name", id: "permission-template-name",
                    hint: "Up to 100 characters. Names must be unique, ignoring case and surrounding spaces.",
                    Input { id: "permission-template-name", value: name(), disabled: freeze,
                        oninput: move |event: FormEvent| { if !freeze { name.set(event.value()); error.set(None); } },
                    }
                }
                p { class: "text-sm text-secondary mb-3", "This reusable profile contains the permissions below, without Administrator access. It will not be applied to anyone automatically." }
                ul { class: "text-sm flex flex-col gap-2 pl-5 mb-4",
                    for grant in grants { li { "{permission_description(*grant)}" } }
                }
            }
        }
        p { class: "text-sm text-secondary mt-4 mb-0", "This action does not save the person's draft. Reloading discards unsaved person changes." }
    }
}
