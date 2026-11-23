use dioxus::prelude::*;

use crate::components::button::{Button, ButtonVariant};
use crate::components::form::FormGroup;
use crate::components::page_header::PageHeader;
use crate::components::select_field::SelectField;
use crate::components::session::use_session;
use crate::server_fns;

/// Workspace-wide settings an Administrator changes: one form, for now with
/// only the time zone. The installed plugins have their own page.
#[component]
pub fn WorkspaceSettings() -> Element {
    let tr = crate::i18n::use_tr();
    let session = use_session();
    let zones = use_resource(server_fns::get_profile_options);
    let saved = session.me.read().workspace_timezone.clone();
    rsx! {
        PageHeader { title: tr.workspace_settings_title() }
        p { class: "text-secondary mb-4", {tr.workspace_settings_intro()} }
        section { class: "card max-w-form", aria_label: tr.workspace_settings_form_label(),
            SettingsForm { key: "{saved}", saved, zones }
        }
    }
}

#[component]
fn SettingsForm(
    saved: String,
    zones: Resource<Result<crate::models::user::ProfileOptions, ServerFnError>>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let session = use_session();
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

    let discard = use_callback(move |_| zone.set(saved.clone()));
    let save = use_callback(move |_| {
        if *saving.peek() {
            return;
        }
        saving.set(true);
        error.set(None);
        let picked = zone();
        spawn(async move {
            match server_fns::set_workspace_timezone(picked).await {
                // The new zone reaches every reader through the session.
                Ok(()) => session.refresh(),
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST | FORBIDDEN,
                    message,
                    ..
                }) => error.set(Some(message)),
                Err(_) => error.set(Some(tr.workspace_save_failed())),
            }
            saving.set(false);
        });
    });

    rsx! {
        FormGroup { label: tr.workspace_zone_label(), id: "workspace-timezone",
            hint: tr.workspace_zone_hint(),
            SelectField { id: "workspace-timezone", label: tr.workspace_zone_choice(), options,
                selected: zone(), query, pending: zones.read().is_none(), onselect: pick,
                error_id: error().map(|_| "workspace-timezone-error".to_string()),
            }
        }
        if let Some(message) = error() {
            p { id: "workspace-timezone-error", class: "alert alert-danger", role: "alert", "{message}" }
        }
        div { class: "flex flex-wrap gap-2",
            Button { id: "workspace-timezone-save", disabled: !changed || saving(), onclick: save,
                label: if saving() { tr.workspace_saving() } else { tr.workspace_save() }
            }
            Button { id: "workspace-timezone-cancel", variant: ButtonVariant::Secondary,
                disabled: !changed || saving(), onclick: discard, label: tr.workspace_discard()
            }
        }
    }
}

const BAD_REQUEST: u16 = 400;
const FORBIDDEN: u16 = 403;
