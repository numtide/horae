use dioxus::prelude::*;
use horae_core::importers::harvest::types::PersonErrorKind;

use super::presentation;
use crate::models::MissingPerson;
use crate::models::user::{Activation, NewPerson};
use crate::server_fns;

/// One person a run could not import. When adding them to Horae would let the
/// next run match them, a small prefilled form does it in place.
#[component]
pub(super) fn MissingPersonRow(
    person: MissingPerson,
    added: bool,
    busy: Signal<bool>,
    on_added: EventHandler<String>,
) -> Element {
    let mut open = use_signal(|| false);
    let mut email = use_signal(|| person.email.clone().unwrap_or_default());
    let mut name = use_signal(|| person.name.clone().unwrap_or_default());
    let mut active = use_signal(|| false);
    let mut done = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    // A run matches by email, or for a CSV row without one, by name.
    let addable = matches!(
        person.kind,
        PersonErrorKind::UnmatchedEmail | PersonErrorKind::UnmatchedName
    );
    let email_known = person.email.is_some();
    let label = person
        .email
        .clone()
        .or_else(|| person.name.clone())
        .unwrap_or_default();
    let submit = move |_| async move {
        if *busy.peek() {
            return;
        }
        let new = NewPerson {
            email: email.peek().trim().to_owned(),
            name: name.peek().trim().to_owned(),
        };
        let activation = if *active.peek() {
            Activation::Active
        } else {
            Activation::Deactivated
        };
        busy.set(true);
        error.set(None);
        match server_fns::add_harvest_people(vec![new.clone()], activation).await {
            Ok(_) => {
                done.set(true);
                open.set(false);
                on_added.call(new.email);
            }
            Err(e) => error.set(Some(e.to_string())),
        }
        busy.set(false);
    };
    rsx! {
        tr {
            td {
                div { "{label}" }
                if email_known && let Some(name) = &person.name {
                    div { class: "text-xs text-faint", "{name}" }
                }
                // The action stays in the first column, so it is on screen
                // even when a narrow viewport scrolls the table.
                if added || done() {
                    span { class: "badge badge-success badge-sm", "Added" }
                } else if addable && !open() {
                    button {
                        r#type: "button",
                        class: "btn btn-ghost btn-sm",
                        "data-testid": "add-person-{label}",
                        disabled: busy(),
                        onclick: move |_| open.set(true),
                        "Add to Horae"
                    }
                }
                if open() {
                    div { class: "flex flex-col gap-2",
                        input {
                            class: "input",
                            r#type: "email",
                            aria_label: "Email",
                            "data-testid": "person-email-{label}",
                            readonly: email_known,
                            value: "{email}",
                            oninput: move |e| email.set(e.value()),
                        }
                        input {
                            class: "input",
                            aria_label: "Name",
                            "data-testid": "person-name-{label}",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                        label { class: "text-sm flex items-center gap-2",
                            input {
                                r#type: "checkbox",
                                "data-testid": "person-active-{label}",
                                checked: active(),
                                onchange: move |e| active.set(e.checked()),
                            }
                            "Active: can sign in now"
                        }
                        if let Some(message) = error() {
                            p { class: "text-sm text-danger", role: "alert", "{message}" }
                        }
                        div { class: "flex gap-2",
                            button {
                                r#type: "button",
                                class: "btn btn-primary btn-sm",
                                "data-testid": "confirm-person-{label}",
                                disabled: busy(),
                                onclick: submit,
                                "Add"
                            }
                            button {
                                r#type: "button",
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| { open.set(false); error.set(None); },
                                "Cancel"
                            }
                        }
                    }
                }
            }
            td {
                div { "{presentation::person_kind_label(person.kind)}" }
                if !addable {
                    div { class: "text-xs text-faint", "{presentation::person_fix(person.kind)}" }
                }
            }
            td { class: "text-mono whitespace-nowrap", "{person.entries}" }
        }
    }
}
