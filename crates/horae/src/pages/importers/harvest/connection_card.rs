use dioxus::prelude::*;

use super::Conn;
use super::change_account::ChangeAccount;
use crate::route::Route;

/// Compact one-line connection state with an expandable management panel.
#[component]
pub(super) fn ConnectionCard(
    connection: Conn,
    manage_open: Signal<bool>,
    busy: Signal<bool>,
    onconnect: EventHandler<MouseEvent>,
    ondisconnect: EventHandler<MouseEvent>,
    onrefresh: EventHandler<MouseEvent>,
    on_changed: EventHandler<()>,
) -> Element {
    let loaded = connection.ready();
    let account = loaded.and_then(|s| s.account_id.as_deref());
    let connected = loaded.is_some_and(|s| s.connected);
    let expired = loaded.is_some_and(|s| s.connected && s.token_expired);
    let tone = if expired {
        "badge-warning"
    } else if connected {
        "badge-success"
    } else {
        "badge-neutral"
    };
    rsx! {
        div { class: "card",
            div { class: "flex items-center gap-3 flex-wrap",
                span { class: "integration-logo harvest", "h" }
                div { class: "integration-body",
                    div { class: "integration-name", "Harvest" }
                    div { class: "integration-meta",
                        if let Some(account) = account { "Account {account} · " }
                        "Read-only access to Harvest"
                    }
                }
                span { class: "badge {tone} badge-sm", "{connection.label()}" }
                if account.is_some() { button {
                    r#type: "button",
                    class: "btn btn-ghost btn-sm",
                    "data-testid": "manage-connection".to_owned(),
                    aria_expanded: manage_open().to_string(),
                    aria_controls: "harvest-connection-management",
                    disabled: busy(),
                    onclick: move |_| manage_open.set(!manage_open()),
                    "Manage connection"
                } }
            }
            match &connection {
                Conn::Loading => rsx! { p { class: "text-sm text-secondary mt-4", role: "status", "Checking connection… CSV import remains available." } },
                Conn::Err(error) => rsx! {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Could not check Harvest connection. Try checking again." }
                        details { summary { "Details" } p { "{error}" } }
                    }
                },
                Conn::Ready(s) if !s.configured => rsx! {
                    p { class: "text-sm text-secondary mt-4", "Set up the Harvest OAuth app before connecting, or use CSV import." }
                    Link { to: Route::HarvestOAuth {}, class: "btn btn-primary mt-4", "data-testid": "setup-harvest-oauth", "Set up OAuth app" }
                },
                Conn::Ready(s) => rsx! {
                    if !s.connected || s.token_expired {
                        p { class: "text-sm text-secondary mt-4",
                            if s.token_expired { "The stored token has expired. Reconnect the original account, or check the connection again after a refresh." }
                            else if account.is_some() { "Disconnected. Imported data and the original account binding are retained." }
                            else { "Connect your Harvest account. Horae never writes anything back to Harvest." }
                        }
                        button { r#type: "button", class: if s.connected { "btn btn-secondary mt-4" } else { "btn btn-primary mt-4" }, "data-testid": "connect-harvest".to_owned(),
                            disabled: busy(), onclick: move |e| onconnect.call(e),
                            if busy() { "Connecting…" } else if account.is_some() { "Reconnect original account" } else { "Connect Harvest" }
                        }
                    }
                },
            }
            if !matches!(connection, Conn::Loading) {
                button { r#type: "button", class: "btn btn-ghost btn-sm mt-4", "data-testid": "refresh-connection".to_owned(),
                    disabled: busy(), onclick: move |e| onrefresh.call(e), "Check connection again"
                }
            }
            if loaded.is_some_and(|s| s.configured) {
                Link { to: Route::HarvestOAuth {}, class: "btn btn-ghost btn-sm mt-4", "data-testid": "harvest-oauth-app", "OAuth app credentials" }
            }
            div { id: "harvest-connection-management",
                if manage_open() && account.is_some() {
                    div { class: "border-t mt-4 pt-4",
                        p { class: "text-sm text-faint", "Disconnect removes credentials but retains the original account binding and imported data." }
                        if connected { button {
                            r#type: "button",
                            class: "btn btn-danger btn-sm",
                            "data-testid": "disconnect-harvest".to_owned(),
                            disabled: busy(),
                            onclick: move |e| ondisconnect.call(e),
                            if busy() { "Disconnecting…" } else { "Disconnect" }
                        } }
                    }
                }
                ChangeAccount {
                    connection: loaded.cloned().unwrap_or_default(),
                    visible: manage_open(), busy,
                    on_changed,
                }
            }
        }
    }
}
