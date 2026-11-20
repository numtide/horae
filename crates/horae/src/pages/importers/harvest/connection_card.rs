use dioxus::prelude::*;
use horae_core::importers::harvest::types::ConnectionStatus;

use super::change_account::ChangeAccount;
use super::{Conn, HarvestLogo};
use crate::components::back_link::BackLink;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::menu::{Menu, MenuDivider};
use crate::components::page_header::PageHeader;
use crate::route::Route;
use crate::server_fns;

/// The page header (Harvest, its connection status and the Connection menu:
/// check, OAuth app, change account, disconnect), then one line of status.
#[component]
pub(super) fn ConnectionCard(
    connection: Conn,
    /// The latest run found that Harvest no longer accepts the connection.
    #[props(default)]
    refused: bool,
    /// A run on this page is queued or running. `active_imports` is read
    /// only with the connection, so it misses a run started since.
    #[props(default)]
    importing: bool,
    on_changed: EventHandler<()>,
) -> Element {
    let navigator = use_navigator();
    let mut busy = use_signal(|| false);
    let mut confirm_disconnect = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let change = use_signal(|| None::<ConnectionStatus>);
    let loaded = connection.ready().cloned().map(|mut status| {
        if importing {
            status.active_imports = status.active_imports.max(1);
        }
        status
    });
    let account = loaded.as_ref().and_then(|s| s.account_id.clone());
    let connected = loaded.as_ref().is_some_and(|s| s.connected);
    let refused = refused && connected;
    let label = if refused {
        "Access refused"
    } else {
        connection.label()
    };
    let tone = if refused {
        "badge-danger"
    } else {
        connection.tone()
    };

    let disconnect = move |_| async move {
        if *busy.peek() {
            return;
        }
        busy.set(true);
        error.set(None);
        match server_fns::harvest_disconnect().await {
            Ok(()) => {
                confirm_disconnect.set(false);
                on_changed.call(());
            }
            Err(e) => error.set(Some(format!(
                "Could not disconnect Harvest. Check the connection before trying again. {e}"
            ))),
        }
        busy.set(false);
    };
    let dismiss = use_callback(move |_| {
        if !*busy.peek() {
            confirm_disconnect.set(false);
        }
    });

    rsx! {
        BackLink { to: Route::ImporterIndex {}, label: "Back to Importers" }
        PageHeader { title: "Harvest",
            leading: rsx! { HarvestLogo {} },
            meta: rsx! {
                span { class: "badge {tone} badge-sm", "data-testid": "connection-status", "{label}" }
            },
            actions: loaded.as_ref().is_some_and(|s| s.configured).then(|| rsx! {
                Menu { id: "harvest-connection", label: "Connection", align_right: true,
                    trigger_class: "text-sm py-3 px-5",
                    button { r#type: "button", role: "menuitem", tabindex: "-1", class: "menu-item",
                        "data-testid": "refresh-connection".to_owned(),
                        onclick: move |_| on_changed.call(()),
                        "Check connection"
                        span { class: "himp-menu-hint", "Read the connection status again" }
                    }
                    button { r#type: "button", role: "menuitem", tabindex: "-1", class: "menu-item",
                        "data-testid": "harvest-oauth-app".to_owned(),
                        onclick: move |_| { navigator.push(Route::HarvestOAuth { focus: Default::default() }); },
                        "OAuth app credentials"
                        span { class: "himp-menu-hint", "Client ID and secret used to connect" }
                    }
                    if let Some(status) = loaded.clone().filter(|s| s.account_id.is_some()) {
                        MenuDivider {}
                        {
                            let blocker = status.change_account_blocker();
                            rsx! {
                                button { r#type: "button", role: "menuitem", tabindex: "-1",
                                    class: if blocker.is_some() { "menu-item disabled" } else { "menu-item" },
                                    "data-testid": "change-account".to_owned(),
                                    // aria-disabled, not disabled: menu.js skips disabled
                                    // items, and the reason would never be read out.
                                    disabled: busy(),
                                    aria_disabled: blocker.is_some(),
                                    onclick: {
                                        let mut change = change;
                                        move |_| {
                                            if blocker.is_none() {
                                                change.set(Some(status.clone()));
                                            }
                                        }
                                    },
                                    "Change account…"
                                    span { class: "himp-menu-hint",
                                        if let Some(reason) = blocker { "Unavailable: {reason}" }
                                        else { "Release this account and connect another" }
                                    }
                                }
                            }
                        }
                        if connected {
                            {
                                let blocker = loaded.as_ref().and_then(ConnectionStatus::disconnect_blocker);
                                rsx! {
                                    button { r#type: "button", role: "menuitem", tabindex: "-1",
                                        class: if blocker.is_some() { "menu-item disabled" } else { "menu-item danger" },
                                        "data-testid": "disconnect-harvest".to_owned(),
                                        disabled: busy(),
                                        aria_disabled: blocker.is_some(),
                                        onclick: move |_| {
                                            if blocker.is_none() {
                                                error.set(None);
                                                confirm_disconnect.set(true);
                                            }
                                        },
                                        "Disconnect…"
                                        span { class: "himp-menu-hint",
                                            if let Some(reason) = blocker { "Unavailable: {reason}" }
                                            else { "Imported data stays" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        }
        p { class: "text-sm text-secondary flex items-center gap-2 flex-wrap mb-6",
            if let Some(account) = &account { span { "Account " span { class: "text-mono", "{account}" } " ·" } }
            if refused {
                span { class: "text-danger", "Reconnect to continue." }
            } else {
                span { "Read-only: Horae never writes to Harvest." }
            }
        }
        if let Some(message) = error() {
            if !confirm_disconnect() {
                div { class: "alert alert-danger mb-4", role: "alert", "{message}" }
            }
        }
        ConfirmDialog { id: "harvest-disconnect", labelledby: "harvest-disconnect-title", open: confirm_disconnect(), busy: busy(),
            title: "Disconnect Harvest?",
            confirm_label: if busy() { "Disconnecting…" } else { "Disconnect Harvest" },
            on_confirm: disconnect,
            on_dismiss: dismiss,
            if confirm_disconnect() {
                p {
                    "Horae deletes its access to Harvest"
                    if let Some(account) = &account { " account " span { class: "text-mono", "{account}" } }
                    ". Clients, projects and time already imported stay as they are."
                }
                p { "You can reconnect later, but only to this same account." }
                if let Some(message) = error() { p { role: "alert", class: "text-danger", "{message}" } }
            }
        }
        ChangeAccount { inspected: change, busy, on_changed }
    }
}
