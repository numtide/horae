use dioxus::prelude::*;

use super::recovery_storage::{
    AcknowledgedPermission, PendingPermission, RecoveryError, RecoveryOutcome,
};
use super::{BAD_REQUEST, CONFLICT, definite_rejection, load_error};
use crate::components::controls::Checkbox;

const RETRY: &str = "permission-recovery-retry";
const DISCARD: &str = "permission-recovery-discard";
const DONE: &str = "permission-recovery-done";

#[component]
pub(super) fn RecoveryForm(
    request: PendingPermission,
    mut locked: Signal<bool>,
    on_finished: EventHandler<Option<bool>>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let acknowledged = use_signal(|| None::<AcknowledgedPermission>);
    let mut rejected = use_signal(|| false);
    let mut checked = use_signal(|| false);
    let mut completed = use_signal(|| false);
    let to_discard = request.clone();
    rsx! {
        if completed() {
            p { role: "status", class: "text-sm mb-4", "Custom profile request completed. No person's permissions were changed." }
            button { id: DONE, r#type: "button", class: "btn btn-primary", onclick: move |_| on_finished.call(None), "Done" }
        } else {
            p { class: "text-sm mb-4", "A previous permission request may have completed. Retry that exact request to check its result before making another change. Nothing is sent automatically." }
            if let Some(message) = error() {
                p { role: "alert", class: "text-sm text-danger mb-4", "{message}" }
            }
            if acknowledged.read().is_some() {
                p { role: "status", class: "text-sm mb-4", "The server confirmed this request. Retry cleanup to remove only this tab's recovery record; the change will not be sent again." }
            }
            button { id: RETRY, r#type: "button", class: "btn btn-primary", disabled: busy(),
                onclick: move |_| {
                    if busy() { return; }
                    busy.set(true); locked.set(true); error.set(None); checked.set(false);
                    let request = request.clone();
                    spawn(async move {
                        match request.attempt(acknowledged).await {
                            Ok(RecoveryOutcome::Person(outcome)) => on_finished.call(Some(outcome.changed)),
                            Ok(RecoveryOutcome::Template) => completed.set(true),
                            Err(RecoveryError::Storage(message) | RecoveryError::Cleanup(message)) => error.set(Some(message)),
                            Err(RecoveryError::Server(problem)) => {
                                rejected.set(definite_rejection(&problem));
                                error.set(Some(match &problem {
                                    ServerFnError::ServerError { code: BAD_REQUEST | CONFLICT, message, .. } => message.clone(),
                                    _ if definite_rejection(&problem) => load_error(&problem).into(),
                                    _ => "The save could not be confirmed. Retry the original request; it may already have completed.".into(),
                                }));
                            },
                        }
                        busy.set(false); locked.set(false);
                    });
                },
                if busy() { "Checking…" } else if acknowledged.read().is_some() { "Finish recovery cleanup" } else { "Retry original request" }
            }
            if rejected() && acknowledged.read().is_none() {
                p { class: "text-sm text-secondary mt-4 mb-3", "This attempt was rejected. An earlier attempt may still have saved. Check the saved permissions before discarding this recovery record. Discarding does not undo or delete any server data." }
                Checkbox { id: "permission-recovery-checked", checked: checked(), disabled: busy(),
                    label: "I have checked the saved permissions and want to discard this recovery record",
                    onclick: move |_| { if !busy() { checked.toggle(); } },
                }
                button { id: DISCARD, r#type: "button", class: "btn btn-secondary mt-4", disabled: busy() || !checked(),
                    onclick: move |_| {
                        if busy() || !checked() { return; }
                        busy.set(true); locked.set(true);
                        let request = to_discard.clone();
                        spawn(async move {
                            match request.clear().await {
                                Ok(()) => on_finished.call(None),
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false); locked.set(false);
                        });
                    }, "Discard recovery record"
                }
            }
        }
    }
}
