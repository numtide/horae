use dioxus::prelude::*;

use crate::components::button::{Button, ButtonVariant};
use crate::components::form::FormGroup;
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, Failure, loaded};
use crate::components::select_field::SelectField;
use crate::components::session::use_session;
use crate::server_fns;

/// Workspace-wide settings an Administrator changes: for now, the time zone.
#[component]
pub fn WorkspaceSettings() -> Element {
    let session = use_session();
    let zones = use_resource(server_fns::get_profile_options);
    let saved = session.me.read().workspace_timezone.clone();
    rsx! {
        PageHeader { title: "Workspace settings" }
        section { class: "card max-w-lg", aria_labelledby: "workspace-timezone-title",
            h2 { id: "workspace-timezone-title", class: "card-title", "Time zone" }
            TimezoneForm { key: "{saved}", saved, zones }
        }
        Plugins {}
    }
}

/// The plugins this server loaded at startup. The server installation chooses
/// them, so they are listed here and not in anyone's own settings.
#[component]
fn Plugins() -> Element {
    let tr = crate::i18n::use_tr();
    let mut plugins = use_resource(server_fns::list_plugins);
    let retry = use_callback(move |_| plugins.restart());
    rsx! {
        section { class: "card mt-4", aria_labelledby: "plugins-title",
            h2 { id: "plugins-title", tabindex: "-1", class: "card-title", {tr.workspace_plugins_title()} }
            {loaded(&plugins.read(), Failure {
                what: "plugins", retry, inset: true,
                pending: plugins.state()() != UseResourceState::Ready, focus: Some("plugins-title"),
            }, |items| rsx! {
                if items.is_empty() {
                    EmptyState { icon: "plugin", title: tr.workspace_plugins_empty(), inset: true,
                        body: tr.workspace_plugins_empty_hint(),
                    }
                } else {
                    div { class: "text-sm",
                        for (index, plugin) in items.iter().enumerate() {
                            div { key: "{plugin.name}", class: if index == 0 { "py-3" } else { "py-3 border-t" },
                                div { class: "flex items-center justify-between gap-2",
                                    strong { "{plugin.name}" }
                                    span { class: "text-muted", {tr.workspace_plugin_version(plugin.version.clone())} }
                                }
                                p { class: "text-muted", {tr.workspace_plugin_hooks(plugin.hooks.clone())} }
                            }
                        }
                    }
                }
            })}
        }
    }
}

#[component]
fn TimezoneForm(
    saved: String,
    zones: Resource<Result<crate::models::user::ProfileOptions, ServerFnError>>,
) -> Element {
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

    let cancel = use_callback(move |_| zone.set(saved.clone()));
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
                Err(_) => error.set(Some("Could not save the time zone. Try again.".into())),
            }
            saving.set(false);
        });
    });

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
            Button { id: "workspace-timezone-cancel", variant: ButtonVariant::Secondary,
                disabled: !changed || saving(), onclick: cancel, label: "Cancel"
            }
            Button { id: "workspace-timezone-save", disabled: !changed || saving(), onclick: save,
                label: if saving() { "Saving…" } else { "Save" }
            }
        }
    }
}

const BAD_REQUEST: u16 = 400;
const FORBIDDEN: u16 = 403;
