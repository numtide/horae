use dioxus::prelude::*;

use crate::server_fns;

/// Profiles an administrator may give a new person. Administrators are chosen
/// in the permission editor, never here; everyone else adds Members.
const PROFILES: &[(&str, &str)] = &[
    ("member", "Member"),
    ("project_manager", "Project Manager"),
    ("accounting", "Accounting"),
    ("people_admin", "People Admin"),
    ("executive_manager", "Executive Manager"),
];

/// Adds one active person. `on_added` reloads the directory.
#[component]
pub(super) fn AddPerson(choose_profile: bool, on_added: EventHandler<String>) -> Element {
    let mut open = use_signal(|| false);
    let mut email = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut profile = use_signal(|| "member".to_string());
    let mut error = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);
    let submit = move |event: FormEvent| {
        event.prevent_default();
        if saving() {
            return;
        }
        saving.set(true);
        let (added_email, added_name, chosen) = (email(), name(), profile());
        spawn(async move {
            match server_fns::create_user(added_email, added_name, chosen).await {
                Ok(user) => {
                    error.set(None);
                    email.set(String::new());
                    name.set(String::new());
                    profile.set("member".into());
                    open.set(false);
                    on_added.call(user.name);
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
            saving.set(false);
        });
    };

    rsx! {
        if !open() {
            button { id: "people-add", r#type: "button", class: "btn btn-primary",
                onclick: move |_| open.set(true), "Add person"
            }
        } else {
            form { id: "people-add-form", class: "card p-5 mb-4", aria_label: "Add person", onsubmit: submit,
                if let Some(message) = error() {
                    p { class: "alert alert-danger", role: "alert", "{message}" }
                }
                div { class: "form-group",
                    label { class: "form-label", r#for: "people-add-email", "Email" }
                    input { id: "people-add-email", class: "form-input", r#type: "email", required: true,
                        value: "{email}", oninput: move |event| email.set(event.value()),
                    }
                }
                div { class: "form-group",
                    label { class: "form-label", r#for: "people-add-name", "Name" }
                    input { id: "people-add-name", class: "form-input",
                        value: "{name}", oninput: move |event| name.set(event.value()),
                    }
                }
                if choose_profile {
                    div { class: "form-group",
                        label { class: "form-label", r#for: "people-add-profile", "Profile" }
                        select { id: "people-add-profile", class: "form-select",
                            onchange: move |event| profile.set(event.value()),
                            for (value, label) in PROFILES {
                                option { value: *value, selected: profile() == *value, "{label}" }
                            }
                        }
                    }
                }
                div { class: "flex flex-wrap gap-2",
                    button { r#type: "submit", class: "btn btn-primary", disabled: saving(), "Add person" }
                    button { r#type: "button", class: "btn btn-secondary", disabled: saving(),
                        onclick: move |_| { error.set(None); open.set(false); }, "Cancel"
                    }
                }
            }
        }
    }
}
