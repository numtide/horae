use dioxus::prelude::*;

use crate::components::controls::Radio;
use crate::components::form::{FormGroup, Input};
use crate::components::modal::{Modal, ModalAction, ModalBody, ModalHeader};
use crate::server_fns;

/// Profiles an administrator may give a new person, with what each adds.
/// Administrators are chosen in the permission editor, never here; everyone
/// else adds Members.
const PROFILES: &[(&str, &str, &str)] = &[
    ("member", "Member", "Tracks their own time."),
    ("project_manager", "Project Manager", "Also manages the projects you assign."),
    ("accounting", "Accounting", "Also sees everyone's time, rates and invoices."),
    ("people_admin", "People Admin", "Also manages people and everyone's time."),
    ("executive_manager", "Executive Manager", "Also manages projects, people and invoicing."),
];

/// The dialog that adds one active person. `on_added` gets their name.
#[component]
pub(super) fn AddPerson(
    open: bool,
    choose_profile: bool,
    on_added: EventHandler<String>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let mut email = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut profile = use_signal(|| "member");
    let mut error = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if saving() {
            return;
        }
        saving.set(true);
        let (added_email, added_name, chosen) = (email(), name(), profile().to_string());
        spawn(async move {
            match server_fns::create_user(added_email, added_name, chosen).await {
                Ok(user) => {
                    error.set(None);
                    email.set(String::new());
                    name.set(String::new());
                    profile.set("member");
                    on_added.call(user.name);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
            saving.set(false);
        });
    };
    let dismiss = move |_| {
        error.set(None);
        on_dismiss.call(());
    };

    rsx! {
        Modal { id: "people-add-dialog", labelledby: "people-add-title", open, busy: saving(),
            focus_fallback: Some("people-add"),
            primary: ModalAction::submit(if saving() { "Adding person…" } else { "Add person" }, "people-add-form")
                .disabled(saving()),
            on_dismiss: dismiss,
            ModalHeader { id: "people-add-title", title: "Add person" }
            form { id: "people-add-form", onsubmit: submit,
                ModalBody {
                    if let Some(message) = error() {
                        p { class: "alert alert-danger", role: "alert", "{message}" }
                    }
                    FormGroup { label: "Name", id: "people-add-name",
                        Input { id: "people-add-name", value: name(), disabled: saving(),
                            oninput: move |event: FormEvent| name.set(event.value()),
                        }
                    }
                    FormGroup { label: "Email", id: "people-add-email", hint: "They sign in with this address.",
                        Input { id: "people-add-email", kind: "email", value: email(), disabled: saving(),
                            oninput: move |event: FormEvent| email.set(event.value()),
                        }
                    }
                    if choose_profile {
                        div { class: "form-group",
                            p { id: "people-add-profile", class: "form-label", "Profile" }
                            div { class: "flex flex-col gap-2", role: "radiogroup", aria_labelledby: "people-add-profile",
                                for (value, label, hint) in PROFILES {
                                    Radio { key: "{value}", id: "people-add-profile-{value}", selected: profile() == *value,
                                        label: label.to_string(), hint: hint.to_string(),
                                        onclick: move |_| profile.set(*value),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
