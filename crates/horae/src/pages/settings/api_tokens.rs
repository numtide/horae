//! Personal access tokens: the person's own tokens, a form to create one, a
//! one-time reveal of the new secret, and revoke with confirmation. The
//! administration page reuses [`TokenTable`] for everyone's tokens.

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::form::{FormGroup, Input, Select};
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::models::api_tokens::{ApiTokenRow, CreatedApiToken};
use crate::server_fns;

/// Expiry choices in days; an empty value never expires.
const EXPIRIES: [(&str, &str); 5] = [
    ("7", "7 days"),
    ("30", "30 days"),
    ("90", "90 days"),
    ("365", "1 year"),
    ("", "No expiry"),
];

#[component]
pub(super) fn ApiTokensSection() -> Element {
    let mut tokens = use_resource(server_fns::list_api_tokens);
    let retry = use_callback(move |_| tokens.restart());
    let mut name = use_signal(String::new);
    let mut expiry = use_signal(|| "90".to_string());
    let mut creating = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut created = use_signal(|| None::<CreatedApiToken>);
    let mut revoke_error = use_signal(|| None::<String>);

    let edit_name = use_callback(move |e: FormEvent| name.set(e.value()));
    let pick_expiry = use_callback(move |e: FormEvent| expiry.set(e.value()));
    let on_revoke = use_callback(move |id: Uuid| {
        revoke_error.set(None);
        spawn(async move {
            match server_fns::revoke_api_token(id.to_string()).await {
                Ok(_) => tokens.restart(),
                Err(_) => revoke_error.set(Some("Could not revoke the token. Try again.".into())),
            }
        });
    });
    let create = move |_| {
        if *creating.peek() {
            return;
        }
        creating.set(true);
        error.set(None);
        let days = expiry.peek().parse::<i64>().ok();
        let expires_at = days.map(|days| Utc::now() + Duration::days(days));
        let token_name = name.peek().clone();
        spawn(async move {
            match server_fns::create_api_token(token_name, expires_at).await {
                Ok(token) => {
                    created.set(Some(token));
                    name.set(String::new());
                    tokens.restart();
                }
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST,
                    message,
                    ..
                }) => error.set(Some(message)),
                Err(_) => error.set(Some("Could not create the token. Try again.".into())),
            }
            creating.set(false);
        });
    };
    let expiry_options: Vec<(String, String)> = EXPIRIES
        .iter()
        .map(|(value, label)| (value.to_string(), label.to_string()))
        .collect();

    rsx! {
        section { class: "card mt-4", aria_labelledby: "api-tokens-title",
            h2 { id: "api-tokens-title", tabindex: "-1", class: "card-title", "API tokens" }
            p { class: "text-secondary text-sm mb-4",
                "A token lets a script or another app use Horae as you, with your permissions. Treat it like a password. "
                "The API is described in "
                a { id: "api-openapi-link", href: "/api/v1/openapi.json", "its OpenAPI document" }
                "."
            }
            HarvestAccount {}
            if let Some(token) = created() {
                div { class: "alert alert-success mb-4", role: "status", id: "api-token-created",
                    p { class: "font-semibold mb-2", "Token “{token.row.name}” created" }
                    p { class: "text-sm mb-2", "Copy it now. Horae keeps no copy and will not show it again." }
                    RevealedToken { token: token.token.clone() }
                    div { class: "flex justify-end mt-2",
                        button { id: "api-token-dismiss", r#type: "button", class: "btn btn-secondary btn-sm",
                            onclick: move |_| created.set(None), "Done"
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-end gap-3",
                div { class: "flex-1",
                    FormGroup { label: "Name", id: "api-token-name", hint: "What will use it, such as “Calendar sync”.",
                        Input { id: "api-token-name", value: name(), disabled: creating(),
                            error_id: error().map(|_| "api-token-error".to_string()),
                            oninput: edit_name,
                        }
                    }
                }
                FormGroup { label: "Expires", id: "api-token-expiry",
                    Select { id: "api-token-expiry", options: expiry_options, selected: expiry(),
                        disabled: creating(), onchange: pick_expiry,
                    }
                }
            }
            if let Some(message) = error() {
                p { id: "api-token-error", class: "alert alert-danger", role: "alert", "{message}" }
            }
            div { class: "flex justify-end mb-4",
                button { id: "api-token-create", r#type: "button", class: "btn btn-primary",
                    disabled: creating() || name.read().trim().is_empty(),
                    onclick: create,
                    if creating() { "Creating…" } else { "Create token" }
                }
            }
            if let Some(message) = revoke_error() {
                p { class: "alert alert-danger", role: "alert", "{message}" }
            }
            match &*tokens.read() {
                None => rsx! { Loading { label: "Loading your tokens…" } },
                Some(Err(error)) => rsx! {
                    LoadError { error: error.clone(), what: "your tokens", retry, inset: true,
                        pending: tokens.state()() != UseResourceState::Ready, focus_on_recovery: "api-tokens-title",
                    }
                },
                Some(Ok(rows)) if rows.is_empty() => rsx! {
                    EmptyState { icon: "settings", title: "No tokens yet", inset: true,
                        body: "Create a token to let a script or another app use Horae on your behalf.",
                    }
                },
                Some(Ok(rows)) => rsx! { TokenTable { rows: rows.clone(), show_owner: false, on_revoke } },
            }
        }
    }
}

/// The account id Harvest clients ask for, while the Harvest-compatible API
/// is on; nothing otherwise.
#[component]
fn HarvestAccount() -> Element {
    let account = use_resource(server_fns::harvest_account_id);
    let Some(Ok(Some(id))) = *account.read() else {
        return rsx! {};
    };
    rsx! {
        p { id: "api-harvest-account", class: "text-secondary text-sm mb-4",
            "Clients of the Harvest API v2 work with these tokens at /harvest/v2. When they ask for an account ID, use "
            span { class: "text-mono", "{id}" }
            "."
        }
    }
}

/// The new secret, read-only, with a copy button.
#[component]
fn RevealedToken(token: String) -> Element {
    let mut copied = use_signal(|| false);
    let copy = {
        let token = token.clone();
        move |_| {
            let js = format!(
                "navigator.clipboard.writeText({});",
                serde_json::to_string(&token).unwrap_or_default()
            );
            document::eval(&js);
            copied.set(true);
        }
    };
    rsx! {
        div { class: "input-group",
            input { id: "api-token-secret", aria_label: "New token",
                class: "input-group-field text-mono text-xs truncate", readonly: true, value: "{token}",
            }
            button { r#type: "button", class: "btn btn-secondary btn-sm",
                aria_label: if copied() { "Token copied" } else { "Copy token" },
                onclick: copy,
                if copied() { "Copied" } else { "Copy" }
            }
        }
    }
}

/// Tokens newest first, with a revoke action on each active one.
#[component]
pub(crate) fn TokenTable(
    rows: Vec<ApiTokenRow>,
    show_owner: bool,
    on_revoke: Callback<Uuid>,
) -> Element {
    // The token whose revocation is being confirmed, with its name.
    let mut revoking = use_signal(|| None::<(Uuid, String)>);
    let on_ask = use_callback(move |token: (Uuid, String)| revoking.set(Some(token)));
    let revoke = use_callback(move |_| {
        if let Some((id, _)) = revoking.take() {
            on_revoke.call(id);
        }
    });
    let keep = use_callback(move |_| revoking.set(None));
    let name = revoking
        .read()
        .as_ref()
        .map(|(_, name)| name.clone())
        .unwrap_or_default();
    rsx! {
        ConfirmDialog { id: "api-token-revoke-dialog", labelledby: "api-token-revoke-title",
            open: revoking.read().is_some(), title: "Revoke “{name}”?", confirm_label: "Revoke",
            on_confirm: revoke, on_dismiss: keep,
            p { "Anything that uses this token loses access at once. A revoked token cannot be restored." }
        }
        div { class: "table-container overflow-x-auto",
            table { class: "table",
                thead {
                    tr {
                        if show_owner { th { scope: "col", "Owner" } }
                        th { scope: "col", "Name" }
                        th { scope: "col", "Token" }
                        th { scope: "col", "Created" }
                        th { scope: "col", "Expires" }
                        th { scope: "col", "Last used" }
                        th { scope: "col", "Status" }
                        th { scope: "col", aria_label: "Action" }
                    }
                }
                tbody {
                    for row in rows {
                        TokenRow { key: "{row.id}", row, show_owner, on_ask }
                    }
                }
            }
        }
    }
}

#[component]
fn TokenRow(row: ApiTokenRow, show_owner: bool, on_ask: Callback<(Uuid, String)>) -> Element {
    let status = status(&row, Utc::now());
    let id = row.id;
    rsx! {
        tr {
            if show_owner { td { class: "text-sm", "{row.owner_name}" } }
            td { class: "text-sm", "{row.name}" }
            td { class: "text-mono text-xs whitespace-nowrap", "horae_pat_{row.display_prefix}…" }
            td { class: "text-sm whitespace-nowrap", {when(row.created_at)} }
            td { class: "text-sm whitespace-nowrap", {row.expires_at.map_or("Never".into(), when)} }
            td { class: "text-sm whitespace-nowrap", {row.last_used_at.map_or("Never".into(), when)} }
            td { class: "whitespace-nowrap",
                span { class: if status == "Active" { "badge badge-success badge-sm" } else { "badge badge-sm" }, "{status}" }
            }
            td { class: "text-right whitespace-nowrap",
                if status == "Active" {
                    button { id: "api-token-revoke-{id}", r#type: "button", class: "btn btn-danger btn-sm",
                        aria_label: "Revoke {row.name}",
                        onclick: {
                            let name = row.name.clone();
                            move |_| on_ask.call((id, name.clone()))
                        },
                        "Revoke"
                    }
                }
            }
        }
    }
}

fn status(row: &ApiTokenRow, now: DateTime<Utc>) -> &'static str {
    if row.revoked_at.is_some() {
        "Revoked"
    } else if row.expires_at.is_some_and(|at| at <= now) {
        "Expired"
    } else {
        "Active"
    }
}

fn when(at: DateTime<Utc>) -> String {
    at.format("%d %b %Y, %H:%M UTC").to_string()
}

const BAD_REQUEST: u16 = 400;
