use dioxus::prelude::*;
use horae_core::permissions::catalog::PERMISSION_CATALOG_VERSION;
use horae_core::permissions::invoices;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::components::page_header::PageHeader;
use crate::components::screen_state::{ErrorState, LOAD_FAILED, NoAccess};
use crate::i18n::{Tr, use_tr};
use crate::models::invoice::{InvoiceDefaults, InvoiceDraftSave, InvoiceGenerationRequest};
use crate::models::own_permissions::OwnPermissions;
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
    /// Its errors are shown on the page, so they come from the catalog.
    tr: Tr,
}

impl RecoveryStorage {
    pub(super) fn ready(self) -> bool {
        (self.loaded)()
    }
    pub(super) async fn store(self, request: &PendingInvoice) -> Result<(), String> {
        let value =
            serde_json::to_string(request).map_err(|_| self.tr.invoices_recovery_encode())?;
        if value.len() > 524288 {
            return Err(self.tr.invoices_recovery_too_large());
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
            } catch (_) { dioxus.send({Err: null}); }
        "#,
        );
        eval.send((self.key.peek().clone(), value))
            .map_err(|_| self.tr.invoices_recovery_unavailable())?;
        eval.recv::<Result<(), ()>>()
            .await
            .map_err(|_| self.tr.invoices_recovery_unavailable())?
            .map_err(|()| self.tr.invoices_recovery_unavailable())
    }

    async fn load(self) -> Result<Option<PendingInvoice>, String> {
        let mut eval = document::eval(
            r#"
            const key = await dioxus.recv();
            try { dioxus.send({Ok: sessionStorage.getItem(key)}); }
            catch (_) { dioxus.send({Err: null}); }
        "#,
        );
        eval.send(self.key.peek().clone())
            .map_err(|_| self.tr.invoices_recovery_unavailable())?;
        let value = eval
            .recv::<Result<Option<String>, ()>>()
            .await
            .map_err(|_| self.tr.invoices_recovery_unavailable())?
            .map_err(|()| self.tr.invoices_recovery_unavailable())?;
        value
            .map(|value| {
                if value.len() > 524288 {
                    return Err(self.tr.invoices_recovery_too_large());
                }
                serde_json::from_str(&value).map_err(|_| self.tr.invoices_recovery_invalid())
            })
            .transpose()
    }
}

pub(super) async fn release_navigation() {
    let _ = document::eval("document.querySelector('[data-editor-kind=\"invoice\"]')?.setAttribute('data-editor-leaving', '');").await;
}

/// Whether the person may open invoices at all: the grant `list_invoices` and
/// `get_invoice` require, from a catalog this build understands.
fn reads_invoices(own: &OwnPermissions) -> bool {
    own.catalog_version == PERMISSION_CATALOG_VERSION
        && invoices::holds(|grant| own.grants.contains(&grant), invoices::READ)
}

/// Check this tab's current user's unresolved mutation before exposing invoice actions.
#[component]
pub(super) fn RecoveryGate(children: Element) -> Element {
    let tr = use_tr();
    let mut storage_key = use_signal(String::new);
    let mut pending = use_signal(|| None::<PendingInvoice>);
    let mut loaded = use_signal(|| false);
    let storage = use_context_provider(|| RecoveryStorage {
        key: storage_key,
        loaded,
        tr,
    });
    let mut loading_error = use_signal(|| None::<String>);
    // The stored permissions hold no invoice access: a retry cannot help.
    let mut denied = use_signal(|| false);
    let mut checking = use_signal(|| false);
    let mut attempts = use_signal(|| 0_u64);
    let retry = use_callback(move |_| attempts += 1);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut rejected = use_signal(|| false);
    let navigator = use_navigator();
    use_effect(move || {
        let _ = attempts();
        spawn(async move {
            // A previous failure stays on screen, its retry disabled, until this check answers.
            loaded.set(false);
            checking.set(true);
            let result = async {
                let (user, own) =
                    futures_util::join!(server_fns::get_me(), server_fns::get_my_permissions());
                let user = user.map_err(|error| error.to_string())?;
                let own = own.map_err(|error| error.to_string())?;
                let refused = !reads_invoices(&own);
                denied.set(refused);
                if refused {
                    return Err(tr.invoices_access_required());
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
                    loading_error.set(None);
                    loaded.set(true);
                }
                Err(message) => loading_error.set(Some(message)),
            }
            checking.set(false);
        });
    });
    if let Some(request) = pending() {
        return rsx! { div { class: "card p-5", "data-editor-kind": "invoice", "data-editor-state": if rejected() && !busy() { "clean" } else { "pending" },
            PageHeader { title: tr.invoices_recover_title() }
            p { class: "mb-4", {tr.invoices_recover_body()} }
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
                                Err(message) => error.set(Some(tr.invoices_recovery_not_cleared(message))),
                            },
                            Err(err) => {
                                rejected.set(matches!(&err, ServerFnError::ServerError { code, .. } if (400..500).contains(code)));
                                error.set(Some(err.to_string()));
                            }
                        }
                        busy.set(false);
                    });
                }, {tr.invoices_recover()}
            }
            if rejected() {
                p { class: "text-sm text-muted mt-4", {tr.invoices_recover_rejected()} }
                button { class: "btn btn-secondary", disabled: busy(), onclick: move |_| {
                    busy.set(true);
                    spawn(async move {
                        match storage.clear().await { Ok(()) => pending.set(None), Err(message) => error.set(Some(message)) }
                        busy.set(false);
                    });
                }, {tr.invoices_recover_discard()} }
            }
        } };
    }
    // No invoice access replaces the screen, below its header, instead of
    // sitting above a list that would be refused beside it.
    if !loaded() && denied() {
        return rsx! {
            PageHeader { title: tr.invoices_title() }
            NoAccess {}
        };
    }
    // A failed check leaves the page usable without invoice actions, as before.
    // While it runs, the screen shows as it is and its actions wait, so nothing
    // sits above the page header.
    rsx! {
        if !loaded() && loading_error().is_some() {
            ErrorState {
                title: tr.invoices_open_failed(),
                body: LOAD_FAILED,
                onretry: retry,
                retry_disabled: checking(),
                focus_on_recovery: "invoices-heading",
            }
        }
        {children}
    }
}
