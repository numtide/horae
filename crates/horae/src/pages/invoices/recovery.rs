use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::invoice::{InvoiceDefaults, InvoiceDraftSave, InvoiceGenerationRequest};
use crate::route::Route;
use crate::server_fns;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) enum PendingInvoice {
    Generate {
        client: String,
        from: String,
        to: String,
        projects: Option<Vec<String>>,
        overrides: Option<InvoiceDefaults>,
        request: Box<InvoiceGenerationRequest>,
    },
    Save {
        invoice_id: Uuid,
        request: InvoiceDraftSave,
    },
}

impl PendingInvoice {
    pub(super) async fn submit(self) -> Result<Uuid, ServerFnError> {
        match self {
            Self::Generate {
                client,
                from,
                to,
                projects,
                overrides,
                request,
            } => server_fns::generate_invoice(client, from, to, projects, overrides, *request)
                .await
                .map(|data| data.invoice.id),
            Self::Save {
                invoice_id,
                request,
            } => {
                server_fns::save_invoice_draft(invoice_id.to_string(), request).await?;
                Ok(invoice_id)
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct RecoveryStorage {
    key: Signal<String>,
    loaded: Signal<bool>,
}

impl RecoveryStorage {
    pub(super) fn ready(self) -> bool {
        (self.loaded)()
    }
    pub(super) async fn store(self, request: &PendingInvoice) -> Result<(), String> {
        let value = serde_json::to_string(request)
            .map_err(|_| "Cannot encode invoice recovery data.".to_owned())?;
        if value.len() > 524288 {
            return Err("Invoice recovery data exceeds 512 KiB.".into());
        }
        self.write(Some(value)).await
    }

    pub(super) async fn clear(self) -> Result<(), String> {
        self.write(None).await
    }

    async fn write(self, value: Option<String>) -> Result<(), String> {
        let mut eval = document::eval(
            r#"
            const [key, value] = await dioxus.recv();
            try {
                if (value === null) sessionStorage.removeItem(key);
                else sessionStorage.setItem(key, value);
                dioxus.send({Ok: null});
            } catch (_) { dioxus.send({Err: 'Browser session storage is unavailable.'}); }
        "#,
        );
        eval.send((self.key.peek().clone(), value))
            .map_err(|_| "Cannot access invoice recovery storage.".to_owned())?;
        eval.recv::<Result<(), String>>()
            .await
            .map_err(|_| "Cannot acknowledge invoice recovery storage.".to_owned())?
    }

    async fn load(self) -> Result<Option<PendingInvoice>, String> {
        let mut eval = document::eval(
            r#"
            const key = await dioxus.recv();
            try { dioxus.send({Ok: sessionStorage.getItem(key)}); }
            catch (_) { dioxus.send({Err: 'Browser session storage is unavailable.'}); }
        "#,
        );
        eval.send(self.key.peek().clone())
            .map_err(|_| "Cannot access invoice recovery storage.".to_owned())?;
        let value = eval
            .recv::<Result<Option<String>, String>>()
            .await
            .map_err(|_| "Cannot read invoice recovery storage.".to_owned())??;
        value.map(|value| {
            if value.len() > 524288 { return Err("Invoice recovery data exceeds 512 KiB.".into()); }
            serde_json::from_str(&value).map_err(|_| "Invoice recovery data is invalid. Keep this tab and check saved invoices before clearing browser data.".into())
        }).transpose()
    }
}

pub(super) async fn release_navigation() {
    let _ = document::eval("document.querySelector('[data-editor-kind=\"invoice\"]')?.setAttribute('data-editor-leaving', '');").await;
}

/// Check this tab's current user's unresolved mutation before exposing invoice actions.
#[component]
pub(super) fn RecoveryGate(children: Element) -> Element {
    let mut storage_key = use_signal(String::new);
    let mut pending = use_signal(|| None::<PendingInvoice>);
    let mut loaded = use_signal(|| false);
    let storage = use_context_provider(|| RecoveryStorage {
        key: storage_key,
        loaded,
    });
    let mut loading_error = use_signal(|| None::<String>);
    let mut attempts = use_signal(|| 0_u64);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut rejected = use_signal(|| false);
    let navigator = use_navigator();
    use_effect(move || {
        let _ = attempts();
        spawn(async move {
            loaded.set(false);
            loading_error.set(None);
            let result = async {
                let user = server_fns::get_me()
                    .await
                    .map_err(|error| error.to_string())?;
                if !user.is_manager_or_above() {
                    return Err("Invoices require manager access.".to_owned());
                }
                storage_key.set(format!(
                    "horae-invoice-request:v1:{}:{}",
                    user.org_id, user.id
                ));
                storage.load().await
            }
            .await;
            match result {
                Ok(request) => {
                    pending.set(request);
                    loaded.set(true);
                }
                Err(message) => loading_error.set(Some(message)),
            }
        });
    });
    if let Some(request) = pending() {
        return rsx! { div { class: "card p-5", "data-editor-kind": "invoice", "data-editor-state": if rejected() && !busy() { "clean" } else { "pending" },
            h1 { class: "page-title", "Recover invoice request" }
            p { class: "mb-4", "A previous invoice request may have completed. Recover the exact submitted request before starting another. No new invoice is created by an acknowledged retry." }
            if let Some(message) = error() { div { class: "alert alert-danger", role: "alert", "{message}" } }
            button { class: "btn btn-primary", disabled: busy(),
                onclick: move |_| {
                    if busy() { return; }
                    busy.set(true); error.set(None);
                    let request = request.clone();
                    spawn(async move {
                        match request.submit().await {
                            Ok(id) => match storage.clear().await {
                                Ok(()) => { release_navigation().await; pending.set(None); navigator.push(Route::InvoiceDetail { id }); }
                                Err(message) => error.set(Some(format!("The invoice is saved, but recovery could not be cleared: {message}"))),
                            },
                            Err(err) => {
                                rejected.set(matches!(&err, ServerFnError::ServerError { code, .. } if (400..500).contains(code)));
                                error.set(Some(err.to_string()));
                            }
                        }
                        busy.set(false);
                    });
                }, "Recover invoice request"
            }
            if rejected() {
                p { class: "text-sm text-muted mt-4", "The server rejected this request. Check the saved invoices before discarding this tab's recovery record; discarding it never deletes an invoice." }
                button { class: "btn btn-secondary", disabled: busy(), onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match storage.clear().await { Ok(()) => pending.set(None), Err(message) => error.set(Some(message)) }
                        busy.set(false);
                    });
                }, "Discard rejected recovery request" }
            }
        } };
    }
    rsx! {
        if !loaded() {
            div { role: "status",
                if let Some(message) = loading_error() {
                    p { "{message}" }
                    button { class: "btn btn-secondary", onclick: move |_| attempts += 1, "Retry loading invoices" }
                } else { p { "Checking invoice recovery…" } }
            }
        }
        {children}
    }
}
