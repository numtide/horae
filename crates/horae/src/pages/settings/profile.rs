use dioxus::prelude::*;

use crate::components::form::{FormGroup, Input};
use crate::components::select_field::SelectField;
use crate::components::session::use_session;
use crate::models::user::{CurrentUser, ProfileOptions};
use crate::server_fns;

/// The signed-in person's name and time zone, with the identity they cannot
/// change here shown beside them.
#[component]
pub(super) fn ProfileSection() -> Element {
    let session = use_session();
    let options = use_resource(server_fns::get_profile_options);
    // Policy zero has no permission panel to name the person's access, so the
    // legacy role is shown here instead.
    let show_role = matches!(&*session.permissions.read(), Some(Ok(None)));
    // Only the first load is gated: re-reading the session after a save keeps
    // the previous value, so the form stays mounted.
    let me = session.me.read();
    rsx! {
        section { class: "card mb-4", aria_labelledby: "profile-title",
            h2 { id: "profile-title", class: "card-title", "Profile" }
            match &*me {
                None => rsx! { p { class: "text-muted text-sm", role: "status", "Loading your profile…" } },
                Some(Err(_)) => rsx! {
                    p { class: "alert alert-danger", role: "alert", "Could not load your profile. Reload the page to try again." }
                },
                Some(Ok(me)) => rsx! { ProfileForm { key: "{me.id}", me: me.clone(), options, show_role } },
            }
        }
    }
}

#[component]
fn ProfileForm(
    me: CurrentUser,
    options: Resource<Result<ProfileOptions, ServerFnError>>,
    show_role: bool,
) -> Element {
    let mut session_me = use_session().me;
    let saved_zone = me.timezone.clone().unwrap_or_default();
    let mut name = use_signal(|| me.name.clone());
    let mut zone = use_signal(|| saved_zone.clone());
    let query = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut saved = use_signal(|| false);
    let changed = *name.read() != me.name || *zone.read() != saved_zone;

    let (sign_in, zones) = match &*options.read() {
        Some(Ok(options)) => (
            options
                .sign_in
                .clone()
                .unwrap_or_else(|| "Development sign-in".into()),
            options.zones.clone(),
        ),
        Some(Err(_)) => ("Unavailable".into(), Vec::new()),
        None => ("Loading…".into(), Vec::new()),
    };
    let pending = options.read().is_none();
    let zone_options: Vec<(String, String)> = std::iter::once((
        String::new(),
        format!("Workspace default ({})", me.workspace_timezone),
    ))
    .chain(zones.into_iter().map(|zone| (zone.clone(), zone)))
    .collect();
    let role = me.org_role.to_string();
    let role = role[..1].to_uppercase() + &role[1..];
    let edit_name = use_callback(move |e: FormEvent| {
        name.set(e.value());
        saved.set(false);
    });
    let pick_zone = use_callback(move |value: String| {
        zone.set(value);
        saved.set(false);
    });

    let save = move |_| {
        if *saving.peek() {
            return;
        }
        saving.set(true);
        error.set(None);
        saved.set(false);
        let (new_name, new_zone) = (name(), zone());
        spawn(async move {
            let new_zone = (!new_zone.is_empty()).then_some(new_zone);
            match server_fns::update_own_profile(new_name, new_zone).await {
                Ok(()) => {
                    saved.set(true);
                    session_me.restart();
                }
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST,
                    message,
                    ..
                }) => error.set(Some(message)),
                Err(_) => error.set(Some("Could not save your profile. Try again.".into())),
            }
            saving.set(false);
        });
    };
    let reset = {
        let (saved_name, saved_zone) = (me.name.clone(), saved_zone.clone());
        move |_| {
            name.set(saved_name.clone());
            zone.set(saved_zone.clone());
            error.set(None);
        }
    };

    rsx! {
        FormGroup { label: "Name", id: "profile-name",
            Input { id: "profile-name", value: name(), disabled: saving(),
                error_id: error().map(|_| "profile-error".to_string()),
                oninput: edit_name,
            }
        }
        FormGroup { label: "Work email", id: "profile-email", hint: "Managed by your sign-in provider.",
            Input { id: "profile-email", value: me.email.clone(), readonly: true }
        }
        FormGroup { label: "Signed in with", id: "profile-sign-in",
            Input { id: "profile-sign-in", value: sign_in, readonly: true }
        }
        if show_role {
            FormGroup { label: "Role", id: "profile-role", hint: "Only an Administrator can change your role.",
                Input { id: "profile-role", value: role, readonly: true }
            }
        }
        FormGroup { label: "Time zone", id: "profile-timezone",
            hint: "Decides which day is today on your timesheet and when you start a timer.",
            SelectField { id: "profile-timezone", label: "time zone", options: zone_options,
                selected: zone(), query, pending, onselect: pick_zone,
            }
        }
        if let Some(message) = error() {
            p { id: "profile-error", class: "alert alert-danger", role: "alert", "{message}" }
        }
        div { class: "flex flex-wrap items-center justify-end gap-2",
            if saved() && !changed {
                span { class: "text-sm text-success", role: "status", "Profile saved." }
            }
            button { id: "profile-cancel", r#type: "button", class: "btn btn-secondary", disabled: !changed || saving(),
                onclick: reset, "Cancel"
            }
            button { id: "profile-save", r#type: "button", class: "btn btn-primary", disabled: !changed || saving(),
                onclick: save, if saving() { "Saving…" } else { "Save" }
            }
        }
    }
}

const BAD_REQUEST: u16 = 400;
