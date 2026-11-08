use dioxus::prelude::*;
use horae_core::importers::harvest::types::ConnectionStatus;

use super::go_to_harvest;

/// Confirm releasing the bound account, then authorize a different one. Open
/// while `inspected` holds the connection the administrator chose to change.
#[component]
pub(super) fn ChangeAccount(
    inspected: Signal<Option<ConnectionStatus>>,
    mut busy: Signal<bool>,
    on_changed: EventHandler<()>,
) -> Element {
    let mut inspected = inspected;
    let mut error = use_signal(|| None::<String>);
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
        match crate::server_fns::harvest_change_account(
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
                if let Err(e) = go_to_harvest().await {
                    error.set(Some(format!(
                        "Account released. Use Connect Harvest to try authorization again: {e}"
                    )));
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
        if inspected().is_none() {
            if let Some(message) = error() { div { role: "alert", class: "alert alert-danger mb-4", "{message}" } }
        }
        crate::components::modal::Modal {
            id: "harvest-change-account", labelledby: "harvest-change-account-title", open: inspected().is_some(), busy: busy(),
            on_dismiss: dismiss,
            h2 { id: "harvest-change-account-title", "Change Harvest account?" }
            if let Some(snapshot) = inspected() {
                p { "Horae disconnects and releases account {snapshot.account_id.clone().unwrap_or_default()}, then asks Harvest to authorize a different one." }
                p { "Business data and retained reports are kept. Old import attempts cannot be retried against the new account. Cancelling the authorization leaves Horae disconnected." }
                if let Some(message) = error() { p { role: "alert", "{message}" } }
                div { class: "flex gap-3 flex-wrap justify-end mt-4",
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
