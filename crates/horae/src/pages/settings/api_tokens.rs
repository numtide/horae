//! Personal access tokens: the person's own tokens, a form to create one, a
//! one-time reveal of the new secret, and revoke with confirmation. The
//! administration page reuses [`TokenTable`] for everyone's tokens.

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::badge::Badge;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::form::{FormGroup, Input, Select};
use crate::components::menu::{MenuItem, RowActions};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{EmptyState, LoadError, Loading};
use crate::components::table::{DataTable, NameCell};
use crate::i18n::Tr;
use crate::models::api_tokens::{ApiTokenRow, CreatedApiToken};
use crate::server_fns;

/// Expiry choices: the value in days, empty for a token that never expires.
fn expiries(tr: Tr) -> Vec<(String, String)> {
    let mut options: Vec<(String, String)> = [7, 30, 90]
        .into_iter()
        .map(|days| (days.to_string(), tr.api_tokens_expiry_days(days)))
        .collect();
    options.push(("365".into(), tr.api_tokens_expiry_year()));
    options.push((String::new(), tr.api_tokens_expiry_never()));
    options
}

#[component]
pub fn SettingsApiTokens() -> Element {
    let tr = crate::i18n::use_tr();
    let intro = tr.api_tokens_intro();
    let (intro_before, intro_link, intro_after) = crate::i18n::split_tag(&intro, "link");
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
                Err(_) => revoke_error.set(Some(tr.api_tokens_revoke_failed())),
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
                Err(_) => error.set(Some(tr.api_tokens_create_failed())),
            }
            creating.set(false);
        });
    };
    let expiry_options = expiries(tr);

    rsx! {
        PageHeader { title: tr.api_tokens_title(), title_id: "api-tokens-title" }
        p { class: "text-secondary mb-4",
            "{intro_before}"
            a { id: "api-openapi-link", href: "/api/v1/openapi.json", "{intro_link}" }
            "{intro_after}"
        }
        HarvestAccount {}
        if let Some(token) = created() {
            div { class: "alert alert-success mb-4 max-w-form", role: "status", id: "api-token-created",
                p { class: "font-semibold mb-2", {tr.api_tokens_created(token.row.name.clone())} }
                p { class: "text-sm mb-2", {tr.api_tokens_created_hint()} }
                RevealedToken { token: token.token.clone() }
                div { class: "flex justify-end mt-2",
                    button { id: "api-token-dismiss", r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| created.set(None), {tr.api_tokens_done()}
                    }
                }
            }
        }
        // Name, expiry and the button on one row, aligned on the controls.
        section { class: "card max-w-form mb-6", aria_label: tr.api_tokens_form_label(),
            div { class: "form-row flex flex-wrap items-end gap-3",
                div { class: "flex-1 min-w-0",
                    FormGroup { label: tr.api_tokens_name(), id: "api-token-name",
                        Input { id: "api-token-name", value: name(), disabled: creating(),
                            error_id: error().map(|_| "api-token-error".to_string()),
                            oninput: edit_name,
                        }
                    }
                }
                FormGroup { label: tr.api_tokens_expires(), id: "api-token-expiry",
                    Select { id: "api-token-expiry", options: expiry_options, selected: expiry(),
                        disabled: creating(), onchange: pick_expiry,
                    }
                }
                button { id: "api-token-create", r#type: "button", class: "btn btn-primary",
                    disabled: creating() || name.read().trim().is_empty(),
                    onclick: create,
                    if creating() { {tr.api_tokens_creating()} } else { {tr.api_tokens_create()} }
                }
            }
            p { class: "form-hint mb-0", {tr.api_tokens_name_hint()} }
            if let Some(message) = error() {
                p { id: "api-token-error", class: "alert alert-danger mt-4 mb-0", role: "alert", "{message}" }
            }
        }
        section { aria_label: tr.api_tokens_list_label(),
            if let Some(message) = revoke_error() {
                p { class: "alert alert-danger", role: "alert", "{message}" }
            }
            match &*tokens.read() {
                None => rsx! { Loading { label: tr.api_tokens_loading() } },
                Some(Err(error)) => rsx! {
                    LoadError { error: error.clone(), what: tr.api_tokens_what(), retry,
                        pending: tokens.state()() != UseResourceState::Ready, focus_on_recovery: "api-tokens-title",
                    }
                },
                Some(Ok(rows)) if rows.is_empty() => rsx! {
                    EmptyState { icon: "key", title: tr.api_tokens_empty(),
                        body: tr.api_tokens_empty_hint(),
                    }
                },
                Some(Ok(rows)) => rsx! { TokenTable { rows: rows.clone(), show_owner: false, focus_fallback: "api-tokens-title", on_revoke } },
            }
        }
    }
}

/// The account id Harvest clients ask for, while the Harvest-compatible API
/// is on; nothing otherwise.
#[component]
fn HarvestAccount() -> Element {
    let tr = crate::i18n::use_tr();
    let account = use_resource(server_fns::harvest_account_id);
    let Some(Ok(Some(id))) = *account.read() else {
        return rsx! {};
    };
    // As text, so the account id is not grouped into thousands.
    let text = tr.api_tokens_harvest_account(id.to_string());
    let (before, id, after) = crate::i18n::split_tag(&text, "id");
    rsx! {
        p { id: "api-harvest-account", class: "text-secondary text-sm mb-4",
            "{before}"
            span { class: "text-mono", "{id}" }
            "{after}"
        }
    }
}

/// The new secret, read-only, with a copy button.
#[component]
fn RevealedToken(token: String) -> Element {
    let tr = crate::i18n::use_tr();
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
            input { id: "api-token-secret", aria_label: tr.api_tokens_secret_label(),
                class: "input-group-field text-mono text-xs truncate", readonly: true, value: "{token}",
            }
            button { r#type: "button", class: "btn btn-secondary btn-sm",
                aria_label: if copied() { tr.api_tokens_copied_label() } else { tr.api_tokens_copy_label() },
                onclick: copy,
                if copied() { {tr.api_tokens_copied()} } else { {tr.api_tokens_copy()} }
            }
        }
    }
}

/// Tokens newest first, with a revoke action on each active one.
#[component]
pub(crate) fn TokenTable(
    rows: Vec<ApiTokenRow>,
    show_owner: bool,
    /// Where focus goes after a revoke, since the row's Revoke button goes away.
    focus_fallback: &'static str,
    on_revoke: Callback<Uuid>,
) -> Element {
    let tr = crate::i18n::use_tr();
    // The token whose revocation is being confirmed, with its name.
    let mut revoking = use_signal(|| None::<(Uuid, String)>);
    let on_ask = use_callback(move |token: (Uuid, String)| revoking.set(Some(token)));
    let revoke = use_callback(move |_| {
        if let Some((id, _)) = revoking.take() {
            on_revoke.call(id);
            // After the dialog has closed and returned focus to the doomed button.
            let target = serde_json::json!(focus_fallback);
            document::eval(&format!(
                "requestAnimationFrame(() => requestAnimationFrame(() => document.getElementById({target})?.focus()));"
            ));
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
            open: revoking.read().is_some(), title: tr.api_tokens_revoke_title(name), confirm_label: tr.api_tokens_revoke(),
            on_confirm: revoke, on_dismiss: keep,
            p { {tr.api_tokens_revoke_body()} }
        }
        // My settings already names the section around the table.
        DataTable { label: show_owner.then(|| tr.api_tokens_admin_list_label()),
            table {
                thead {
                    tr {
                        th { scope: "col", {tr.api_tokens_column_name()} }
                        if show_owner { th { scope: "col", {tr.api_tokens_column_owner()} } }
                        th { scope: "col", {tr.api_tokens_column_created()} }
                        th { scope: "col", {tr.api_tokens_column_expires()} }
                        th { scope: "col", {tr.api_tokens_column_last_used()} }
                        th { scope: "col", {tr.api_tokens_column_status()} }
                        th { scope: "col", aria_label: tr.action_menu() }
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
    let tr = crate::i18n::use_tr();
    let status = Status::of(&row, Utc::now());
    let id = row.id;
    let name = row.name.clone();
    let ask = use_callback(move |_: MouseEvent| on_ask.call((id, name.clone())));
    let never = || tr.api_tokens_never();
    rsx! {
        tr {
            NameCell {
                secondary: rsx! { span { class: "text-mono", "horae_pat_{row.display_prefix}…" } },
                "{row.name}"
            }
            if show_owner { td { "{row.owner_name}" } }
            // Mono dates may wrap after the comma, so the row's menu stays in the frame.
            td { class: "text-mono text-secondary", {when(row.created_at)} }
            td { class: "text-mono text-secondary", {row.expires_at.map_or_else(never, when)} }
            td { class: "text-mono text-secondary", {row.last_used_at.map_or_else(never, when)} }
            td { class: "whitespace-nowrap",
                match status {
                    Status::Active => rsx! { Badge { variant: "success", {tr.api_tokens_status_active()} } },
                    Status::Expired => rsx! { Badge { variant: "neutral", {tr.api_tokens_status_expired()} } },
                    Status::Revoked => rsx! { Badge { variant: "neutral", {tr.api_tokens_status_revoked()} } },
                }
            }
            td { class: "text-right",
                if status == Status::Active {
                    RowActions { id: "api-token-actions-{id}", name: row.name.clone(),
                        MenuItem { id: "api-token-revoke-{id}", danger: true, onclick: ask, {tr.api_tokens_revoke_action()} }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Status {
    Active,
    Expired,
    Revoked,
}

impl Status {
    fn of(row: &ApiTokenRow, now: DateTime<Utc>) -> Self {
        if row.revoked_at.is_some() {
            Self::Revoked
        } else if row.expires_at.is_some_and(|at| at <= now) {
            Self::Expired
        } else {
            Self::Active
        }
    }
}

fn when(at: DateTime<Utc>) -> String {
    at.format("%d %b %Y, %H:%M UTC").to_string()
}

const BAD_REQUEST: u16 = 400;
