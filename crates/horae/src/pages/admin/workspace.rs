use dioxus::prelude::*;

use crate::components::form::FormGroup;
use crate::components::select_field::SelectField;
use crate::components::session::use_session;
use crate::server_fns;

/// Workspace-wide settings an Administrator changes: for now, the time zone.
#[component]
pub fn WorkspaceSettings() -> Element {
    let session = use_session();
    let zones = use_resource(server_fns::get_profile_options);
    let saved = match &*session.me.read() {
        Some(Ok(me)) => Some(me.workspace_timezone.clone()),
        _ => None,
    };
    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "Workspace settings" }
        }
        section { class: "card max-w-lg", aria_labelledby: "workspace-timezone-title",
            h2 { id: "workspace-timezone-title", class: "card-title", "Time zone" }
            match saved {
                None => rsx! { p { class: "text-muted text-sm", role: "status", "Loading…" } },
                Some(saved) => rsx! { TimezoneForm { key: "{saved}", saved, zones } },
            }
        }
    }
}

#[component]
fn TimezoneForm(
    saved: String,
    zones: Resource<Result<crate::models::user::ProfileOptions, ServerFnError>>,
) -> Element {
    let mut me = use_session().me;
    let mut zone = use_signal(|| saved.clone());
    let query = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let pick = use_callback(move |value: String| zone.set(value));
    let options: Vec<(String, String)> = match &*zones.read() {
        Some(Ok(options)) => options
            .zones
            .iter()
            .map(|z| (z.clone(), z.clone()))
            .collect(),
        _ => Vec::new(),
    };
    let changed = *zone.read() != saved;

    let save = move |_| {
        if *saving.peek() {
            return;
        }
        saving.set(true);
        error.set(None);
        let picked = zone();
        spawn(async move {
            match server_fns::set_workspace_timezone(picked).await {
                // The new zone reaches every reader through the session.
                Ok(()) => me.restart(),
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST | FORBIDDEN,
                    message,
                    ..
                }) => error.set(Some(message)),
                Err(_) => error.set(Some("Could not save the time zone. Try again.".into())),
            }
            saving.set(false);
        });
    };

    rsx! {
        FormGroup { label: "Workspace time zone", id: "workspace-timezone",
            hint: "People who have not chosen their own time zone use this one. It decides which day timers and monthly budgets fall on.",
            SelectField { id: "workspace-timezone", label: "workspace time zone", options,
                selected: zone(), query, pending: zones.read().is_none(), onselect: pick,
                error_id: error().map(|_| "workspace-timezone-error".to_string()),
            }
        }
        if let Some(message) = error() {
            p { id: "workspace-timezone-error", class: "alert alert-danger", role: "alert", "{message}" }
        }
        div { class: "flex justify-end gap-2",
            button { id: "workspace-timezone-cancel", r#type: "button", class: "btn btn-secondary", disabled: !changed || saving(),
                onclick: move |_| zone.set(saved.clone()), "Cancel"
            }
            button { id: "workspace-timezone-save", r#type: "button", class: "btn btn-primary", disabled: !changed || saving(),
                onclick: save, if saving() { "Saving…" } else { "Save" }
            }
        }
    }
}

const BAD_REQUEST: u16 = 400;
const FORBIDDEN: u16 = 403;
