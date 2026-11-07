use dioxus::prelude::*;
use horae_core::importers::harvest::types::ConnectionStatus;

use crate::server_fns;

/// Release the bound account and authorize a different one, after confirming.
#[component]
pub(super) fn ChangeAccount(
    connection: ConnectionStatus,
    visible: bool,
    mut busy: Signal<bool>,
    on_changed: EventHandler<()>,
) -> Element {
    let mut inspected = use_signal(|| None::<ConnectionStatus>);
    let mut error = use_signal(|| None::<String>);
    let blocker = connection.change_account_blocker();
    let bound = connection.account_id.is_some();
    let confirm = move |_| async move {
        if busy() {
            return;
        }
        let Some(snapshot) = inspected.peek().clone() else {
            return;
        };
        let Some(account) = snapshot.account_id else {
            return;
        };
        busy.set(true);
        error.set(None);
        match server_fns::harvest_change_account(
            account,
            snapshot.account_generation,
            snapshot.connection_revision,
        )
        .await
        {
            Err(e) => {
                error.set(Some(e.to_string()));
                busy.set(false);
            }
            Ok(()) => {
                inspected.set(None);
                on_changed.call(());
                match server_fns::harvest_connect_start().await {
                    Ok(url) => {
                        let _ = document::eval(&format!(
                            "window.location.href = {};",
                            serde_json::json!(url)
                        ))
                        .await;
                    }
                    Err(e) => error.set(Some(format!(
                        "Account released. Use Connect Harvest to try authorization again: {e}"
                    ))),
                }
                busy.set(false);
            }
        }
    };
    let dismiss = use_callback(move |_| {
        if !busy() {
            inspected.set(None);
        }
    });
    rsx! {
        if bound && visible {
            div { class: "mt-4",
                button { r#type: "button", class: "btn btn-secondary btn-sm", "data-testid": "change-account".to_owned(),
                    disabled: busy() || blocker.is_some(),
                    onclick: move |_| { error.set(None); inspected.set(Some(connection.clone())); },
                    "Change account"
                }
                if let Some(reason) = blocker { p { class: "text-sm text-faint", "{reason}" } }
            }
        }
        if inspected().is_none() {
            if let Some(message) = error() { div { role: "alert", class: "alert alert-danger", "{message}" } }
        }
        crate::components::modal::Modal {
            id: "harvest-change-account", labelledby: "harvest-change-account-title", open: inspected().is_some(), busy: busy(),
            on_dismiss: dismiss,
            h2 { id: "harvest-change-account-title", "Change Harvest account?" }
            if let Some(snapshot) = inspected() {
                p { "Disconnect and release account {snapshot.account_id.clone().unwrap_or_default()} before authorizing a different account." }
                p { "Business data and retained reports are kept. Old import attempts cannot be retried against the replacement account. Cancelling authorization afterward leaves Horae disconnected." }
                if let Some(message) = error() { p { role: "alert", "{message}" } }
                div { class: "flex gap-3 flex-wrap mt-4",
                    button { r#type: "button", class: "btn btn-secondary", "data-testid": "cancel-change-account".to_owned(), disabled: busy(),
                        onclick: move |_| { inspected.set(None); error.set(None); }, "Cancel" }
                    button { r#type: "button", class: "btn btn-danger", "data-testid": "confirm-change-account".to_owned(), disabled: busy(), onclick: confirm,
                        if busy() { "Changing account…" } else { "Change account and connect" }
                    }
                }
            }
        }
    }
}
