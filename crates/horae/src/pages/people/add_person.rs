use dioxus::prelude::*;

use crate::components::controls::Radio;
use crate::components::form::{FormGroup, Input};
use crate::components::modal::{Modal, ModalAction, ModalBody, ModalHeader};
use crate::i18n::{Tr, use_tr};
use crate::server_fns;

/// Profiles an administrator may give a new person: the stored name, the
/// label and what each adds. Administrators are chosen in the permission
/// editor, never here; everyone else adds Members.
fn profiles(tr: Tr) -> [(&'static str, String, String); 5] {
    [
        (
            "member",
            tr.people_profile_member(),
            tr.people_profile_member_hint(),
        ),
        (
            "project_manager",
            tr.people_profile_project_manager(),
            tr.people_profile_project_manager_hint(),
        ),
        (
            "accounting",
            tr.people_profile_accounting(),
            tr.people_profile_accounting_hint(),
        ),
        (
            "people_admin",
            tr.people_profile_people_admin(),
            tr.people_profile_people_admin_hint(),
        ),
        (
            "executive_manager",
            tr.people_profile_executive_manager(),
            tr.people_profile_executive_manager_hint(),
        ),
    ]
}

/// The dialog that adds one active person. `on_added` gets their name.
#[component]
pub(super) fn AddPerson(
    open: bool,
    choose_profile: bool,
    on_added: EventHandler<String>,
    on_dismiss: EventHandler<()>,
) -> Element {
    let tr = use_tr();
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
    let dismiss = use_callback(move |()| {
        error.set(None);
        on_dismiss.call(());
    });
    let type_name = use_callback(move |event: FormEvent| name.set(event.value()));
    let type_email = use_callback(move |event: FormEvent| email.set(event.value()));

    rsx! {
        Modal { id: "people-add-dialog", labelledby: "people-add-title", open, busy: saving(),
            focus_fallback: Some("people-add"),
            primary: ModalAction::submit(if saving() { tr.people_adding() } else { tr.people_add() }, "people-add-form")
                .disabled(saving()),
            on_dismiss: dismiss,
            ModalHeader { id: "people-add-title", title: tr.people_add_title() }
            form { id: "people-add-form", onsubmit: submit,
                ModalBody {
                    if let Some(message) = error() {
                        p { class: "alert alert-danger", role: "alert", "{message}" }
                    }
                    FormGroup { label: tr.people_add_name(), id: "people-add-name",
                        Input { id: "people-add-name", value: name(), disabled: saving(), oninput: type_name }
                    }
                    FormGroup { label: tr.people_add_email(), id: "people-add-email", hint: tr.people_add_email_hint(),
                        Input { id: "people-add-email", kind: "email", value: email(), disabled: saving(), oninput: type_email }
                    }
                    if choose_profile {
                        div { class: "form-group",
                            p { id: "people-add-profile", class: "form-label", {tr.people_add_profile()} }
                            div { class: "flex flex-col gap-2", role: "radiogroup", aria_labelledby: "people-add-profile",
                                for (value, label, hint) in profiles(tr) {
                                    ProfileChoice { key: "{value}", value, label, hint, profile }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProfileChoice(
    value: &'static str,
    label: String,
    hint: String,
    profile: Signal<&'static str>,
) -> Element {
    let mut profile = profile;
    let choose = use_callback(move |_: MouseEvent| profile.set(value));
    rsx! {
        Radio { id: "people-add-profile-{value}", selected: profile() == value, label, hint, onclick: choose }
    }
}
