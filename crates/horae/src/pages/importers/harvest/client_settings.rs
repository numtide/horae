//! The "Harvest OAuth app" view: the client ID and write-only secret an
//! administrator saves for this organization, or the environment's client
//! shown read-only (specs/018-harvest-oauth-client-settings).

use dioxus::prelude::*;
use horae_core::importers::harvest::client::{
    CALLBACK_PATH, ClientSettings, ClientSource, DISCONNECT_UNCONFIRMED, save_disconnects,
};

use crate::route::Route;
use crate::server_fns;

const LABEL: &str = "block text-xs uppercase tracking-eyebrow text-faint mb-2";

/// A change waiting for confirmation because it disconnects Harvest.
#[derive(Clone, Copy, PartialEq)]
enum Change {
    Save,
    Remove,
}

#[component]
pub(super) fn HarvestOAuthApp(
    /// Whether Harvest is connected, so a disconnecting change asks first.
    connected: bool,
    on_changed: EventHandler<()>,
) -> Element {
    let mut settings = use_resource(server_fns::harvest_client_settings);
    let mut origin = use_signal(String::new);
    let mut client_id = use_signal(String::new);
    let mut secret = use_signal(String::new);
    let mut replacing = use_signal(|| false);
    let mut confirming = use_signal(|| None::<Change>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut copied = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            if let Ok(current) = document::eval("return window.location.origin;")
                .join::<String>()
                .await
            {
                origin.set(current);
            }
        });
    });
    // Start the form from the saved client ID, and again after each change.
    use_effect(move || {
        if let Some(Ok(loaded)) = &*settings.read() {
            client_id.set(loaded.client_id.clone().unwrap_or_default());
        }
    });

    // The value Harvest must have registered: the one in effect, or this
    // instance's own callback for a first save.
    let redirect = move |current: &ClientSettings| {
        current
            .redirect_url
            .clone()
            .unwrap_or_else(|| format!("{}{CALLBACK_PATH}", origin()))
    };

    let submit = move |change: Change, current: ClientSettings| async move {
        if *busy.peek() {
            return;
        }
        let id = client_id.peek().trim().to_owned();
        let disconnects = connected
            && match change {
                Change::Remove => true,
                Change::Save => save_disconnects(current.client_id.as_deref(), &id),
            };
        let confirmed = *confirming.peek() == Some(change);
        if disconnects && !confirmed {
            confirming.set(Some(change));
            return;
        }
        confirming.set(None);
        busy.set(true);
        error.set(None);
        let result = match change {
            Change::Save => {
                let entered = secret.peek().clone();
                let new_secret = (!entered.trim().is_empty()).then_some(entered);
                server_fns::save_harvest_client(id, new_secret, redirect(&current), confirmed).await
            }
            Change::Remove => server_fns::remove_harvest_client(confirmed).await,
        };
        busy.set(false);
        match result {
            Ok(_) => {
                secret.set(String::new());
                replacing.set(false);
                settings.restart();
                on_changed.call(());
            }
            // Harvest was connected after this page loaded: ask now.
            Err(failure) if failure.to_string().contains(DISCONNECT_UNCONFIRMED) => {
                confirming.set(Some(change));
                on_changed.call(());
            }
            Err(failure) => error.set(Some(failure.to_string())),
        }
    };

    let loaded = settings.read().clone();
    let current = match loaded {
        None => {
            return rsx! { p { class: "text-sm text-secondary", role: "status", "Loading the OAuth app…" } };
        }
        Some(Err(failure)) => {
            return rsx! {
                div { class: "alert alert-danger", role: "alert",
                    p { "Could not load the Harvest OAuth app." }
                    details { summary { "Details" } p { "{failure}" } }
                }
            };
        }
        Some(Ok(current)) => current,
    };
    let editable = current.editable();
    let environment = current.source == ClientSource::Environment;
    let (tone, status) = match current.source {
        ClientSource::Environment => ("badge-success", "Configured by the environment"),
        ClientSource::Stored if !current.secret_readable => ("badge-warning", "Secret unreadable"),
        ClientSource::Stored if current.needs_redirect() => {
            ("badge-warning", "Redirect URL needed")
        }
        ClientSource::Stored => ("badge-success", "Configured"),
        ClientSource::None => ("badge-neutral", "Not configured"),
    };
    let redirect_url = redirect(&current);
    let stored_on = current
        .stored_at
        .as_deref()
        .and_then(|at| at.get(..10))
        .map(|date| format!("stored {date}"));
    let show_stored_secret = current.secret_set && current.secret_readable && !replacing();
    let copy_url = redirect_url.clone();
    let save_current = current.clone();
    let remove_current = current.clone();
    let confirm_current = current.clone();

    rsx! {
        div { class: "flex flex-col gap-5",
            div {
                Link {
                    to: Route::HarvestOverview {},
                    class: "imp-back text-primary text-sm",
                    "data-testid": "harvest-oauth-back",
                    "← Back to Harvest"
                }
            }
            div { class: "alert alert-warning mb-0",
                p { class: "font-semibold", "One-time setup" }
                p { class: "text-sm mt-1",
                    "Register Horae once as an OAuth2 app in Harvest, then connect your Harvest account from the importer."
                }
            }
            section { class: "card p-0 overflow-hidden", aria_label: "Harvest OAuth2 app",
                div { class: "flex items-center gap-3 flex-wrap px-5 py-4 border-b",
                    span { class: "integration-logo harvest", "h" }
                    h2 { class: "text-sm font-semibold", "Harvest OAuth2 app" }
                    span { class: "badge {tone} badge-sm", "{status}" }
                }
                div { class: "flex flex-col gap-5 p-5",
                    if !current.encryption_key_set {
                        div { class: "alert alert-warning mb-0", role: "status",
                            "Horae cannot store a client secret yet. Ask the operator to set "
                            code { "HORAE_HARVEST_ENC_KEY" }
                            " (64 hex characters) and restart the server."
                        }
                    }
                    div {
                        label { class: LABEL, r#for: "harvest-redirect-url", "Redirect URI" }
                        div { class: "input-group",
                            input {
                                id: "harvest-redirect-url",
                                class: "input-group-field text-mono text-xs truncate",
                                readonly: true,
                                value: "{redirect_url}",
                            }
                            button {
                                r#type: "button",
                                class: "btn btn-secondary btn-sm",
                                onclick: move |_| {
                                    let js = format!(
                                        "navigator.clipboard.writeText({});",
                                        serde_json::to_string(&copy_url).unwrap_or_default()
                                    );
                                    document::eval(&js);
                                    copied.set(true);
                                },
                                if copied() { "Copied" } else { "Copy" }
                            }
                        }
                        p { class: "form-hint",
                            if current.redirect_from_environment {
                                "Set by HORAE_HARVEST_REDIRECT_URL. Register it in Harvest exactly as shown."
                            } else if environment {
                                "The environment client also needs HORAE_HARVEST_REDIRECT_URL. Ask the operator to set it to this address."
                            } else {
                                "Paste this into Harvest first. It must match exactly, or the callback is rejected."
                            }
                        }
                        if current.needs_redirect() {
                            div { class: "alert alert-warning mt-3 mb-0", role: "status",
                                "Redirect URL needed. This client was saved while "
                                code { "HORAE_HARVEST_REDIRECT_URL" }
                                " supplied it, and that is no longer set. Save again to store the address above."
                            }
                        }
                    }
                    div {
                        label { class: LABEL, r#for: "harvest-client-id", "Client ID" }
                        input {
                            id: "harvest-client-id".to_owned(),
                            class: "form-input text-mono",
                            autocomplete: "off",
                            placeholder: "1234567.pt.abcdEFGH…",
                            disabled: !editable || busy(),
                            value: "{client_id}",
                            oninput: move |event| {
                                confirming.set(None);
                                client_id.set(event.value());
                            },
                        }
                    }
                    div {
                        label { class: LABEL, r#for: "harvest-client-secret", "Client secret" }
                        if show_stored_secret {
                            div { class: "flex items-center gap-3 flex-wrap px-4 py-3 bg-secondary border rounded-lg",
                                span { class: "text-mono text-sm text-secondary", aria_label: "Secret set", "••••••••••••••••••••" }
                                span { class: "text-xs text-faint",
                                    if environment { "set in the environment" } else if let Some(stored_on) = &stored_on { "{stored_on}" }
                                }
                                span { class: "flex-1" }
                                if editable {
                                    button {
                                        r#type: "button",
                                        class: "btn btn-ghost btn-sm",
                                        "data-testid": "replace-harvest-secret".to_owned(),
                                        disabled: busy(),
                                        onclick: move |_| replacing.set(true),
                                        "Replace secret"
                                    }
                                }
                            }
                        } else {
                            input {
                                id: "harvest-client-secret".to_owned(),
                                class: "form-input text-mono",
                                r#type: "password",
                                autocomplete: "new-password",
                                placeholder: "Paste the secret, shown only once by Harvest",
                                disabled: !editable || busy(),
                                value: "{secret}",
                                oninput: move |event| secret.set(event.value()),
                            }
                        }
                        p { class: "form-hint",
                            if current.secret_set && !current.secret_readable {
                                "The stored secret no longer opens with the current encryption key. Enter it again. "
                            }
                            "Encrypted at rest and never returned by the API. Horae can overwrite it, never read it back."
                        }
                    }
                    if let Some(change) = confirming() {
                        div { class: "alert alert-warning mb-0", role: "alert",
                            p {
                                if change == Change::Remove {
                                    "Removing the OAuth app disconnects Harvest."
                                } else {
                                    "A different client ID disconnects Harvest: its tokens belong to the current app."
                                }
                                " The account binding, import history and imported data are kept; reconnect the same Harvest account afterwards."
                            }
                            div { class: "flex items-center gap-3 flex-wrap mt-3",
                                button {
                                    r#type: "button",
                                    class: "btn btn-danger btn-sm",
                                    "data-testid": "confirm-harvest-client-change".to_owned(),
                                    disabled: busy(),
                                    onclick: move |_| submit(change, confirm_current.clone()),
                                    if change == Change::Remove { "Remove and disconnect" } else { "Save and disconnect" }
                                }
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost btn-sm",
                                    onclick: move |_| confirming.set(None),
                                    "Cancel"
                                }
                            }
                        }
                    }
                    if let Some(failure) = error() {
                        div { class: "alert alert-danger mb-0", role: "alert", "{failure}" }
                    }
                    // Removing needs no encryption key, so a client whose
                    // secret no longer opens can still be removed.
                    if editable || current.source == ClientSource::Stored {
                        div { class: "flex items-center gap-3 flex-wrap",
                            if editable {
                                button {
                                    r#type: "button",
                                    class: "btn btn-primary",
                                    "data-testid": "save-harvest-client".to_owned(),
                                    disabled: busy(),
                                    onclick: move |_| submit(Change::Save, save_current.clone()),
                                    if busy() { "Saving…" } else { "Save credentials" }
                                }
                            }
                            if current.source == ClientSource::Stored {
                                button {
                                    r#type: "button",
                                    class: "btn btn-ghost btn-sm text-danger",
                                    "data-testid": "remove-harvest-client".to_owned(),
                                    disabled: busy(),
                                    onclick: move |_| submit(Change::Remove, remove_current.clone()),
                                    "Remove"
                                }
                            }
                        }
                    }
                }
                div { class: "px-5 py-4 border-t bg-secondary",
                    p { class: "text-xs text-faint",
                        if environment {
                            "Locked: set by "
                            code { "HORAE_HARVEST_CLIENT_ID" }
                            " and "
                            code { "HORAE_HARVEST_CLIENT_SECRET" }
                            ". Unset them to manage the client here."
                            if current.stored_ignored { " A client saved here earlier is ignored while they are set." }
                        } else {
                            "Prefer not to keep secrets in the database? Set "
                            code { "HORAE_HARVEST_CLIENT_ID" }
                            " and "
                            code { "HORAE_HARVEST_CLIENT_SECRET" }
                            " in the environment; this panel then reads as locked."
                        }
                    }
                }
            }
        }
    }
}
