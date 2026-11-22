use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use horae_core::permissions::invoices;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::{Failure, loaded};

/// The Invoices title, where focus lands once a failed read loads again.
const INVOICES_HEADING: &str = "invoices-heading";
/// An invoice's title, where focus lands once voiding removes its menu.
const INVOICE_HEADING: &str = "invoice-heading";

/// Focus `id` after the next frame, once the render that shows it has landed.
fn focus_soon(id: &str) {
    let id = serde_json::json!(id);
    document::eval(&format!(
        "requestAnimationFrame(() => document.getElementById({id})?.focus());"
    ));
}
use crate::components::back_link::BackLink;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{
    EmptyState, ErrorState, LOAD_FAILED, Loading, NoAccess, Skeleton,
};
use crate::components::table::DataTable;
use crate::route::Route;
use crate::server_fns;

#[path = "invoices/defaults_form.rs"]
mod defaults_form;
#[path = "invoices/preparation.rs"]
mod preparation;

#[path = "invoices/editing.rs"]
mod editing;
#[path = "invoices/recovery.rs"]
mod recovery;
#[path = "invoices/settings.rs"]
mod settings;

pub use settings::InvoiceSettings;

/// Whether the session's own permissions, from a catalog this build
/// understands, hold every `required` grant: the same sets the server checks.
/// Reading ([`invoices::READ`]) is checked by `RecoveryGate`.
fn holds(session: crate::components::session::Session, required: &[Permission]) -> bool {
    matches!(
        &*session.permissions.read(),
        Some(Ok(own)) if own.catalog_version == PERMISSION_CATALOG_VERSION
            && invoices::holds(|grant| own.grants.contains(&grant), required)
    )
}

/// Whether to offer changing an invoice's status.
fn manages_invoices(session: crate::components::session::Session) -> bool {
    holds(session, invoices::WRITE)
}

/// Whether to offer preparing a new invoice or editing a draft's values.
fn prepares_invoices(session: crate::components::session::Session) -> bool {
    holds(session, invoices::PREPARE)
}

/// The badge class for an invoice status — one convention for list and detail.
pub(super) fn invoice_badge_class(status: InvoiceStatus) -> &'static str {
    match status {
        InvoiceStatus::Paid => "badge badge-success",
        InvoiceStatus::Sent => "badge badge-info",
        InvoiceStatus::Void => "badge badge-neutral",
        InvoiceStatus::Draft => "badge badge-warning",
    }
}

/// The status as a word that starts with a capital ("Draft"), for badges.
pub(super) fn invoice_status_label(tr: crate::i18n::Tr, status: InvoiceStatus) -> String {
    match status {
        InvoiceStatus::Draft => tr.invoices_status_draft(),
        InvoiceStatus::Sent => tr.invoices_status_sent(),
        InvoiceStatus::Paid => tr.invoices_status_paid(),
        InvoiceStatus::Void => tr.invoices_status_void(),
    }
}

#[component]
pub fn NewInvoice() -> Element {
    rsx! { recovery::RecoveryGate { NewInvoiceContent { client_context: None } } }
}

#[component]
pub fn NewInvoiceForClient(client: String) -> Element {
    rsx! { for client in [client] { recovery::RecoveryGate { key: "{client}", NewInvoiceContent { client_context: Some(client.clone()) } } } }
}

/// Preparing a new invoice, its own screen under Invoices: "Generate draft"
/// is its one Primary, and the back link leaves it.
#[component]
fn NewInvoiceContent(client_context: Option<String>) -> Element {
    let tr = crate::i18n::use_tr();
    let storage = use_context::<recovery::RecoveryStorage>();
    let mut clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let retry_clients = use_callback(move |_| clients.restart());
    let busy = use_signal(|| false);
    let navigator = use_navigator();
    let session = crate::components::session::use_session();
    let prepares = prepares_invoices(session);
    let retry_session = use_callback(move |()| session.refresh());

    // Client picker options: the placeholder, then every active client.
    let client_opts: Vec<(String, String)> =
        std::iter::once((String::new(), "Select a client…".to_string()))
            .chain(
                clients
                    .read()
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .into_iter()
                    .flatten()
                    .map(|c| (c.id.to_string(), c.name.clone())),
            )
            .collect();

    rsx! {
        div {
            BackLink { to: Route::InvoiceList {}, label: tr.invoices_back() }
            PageHeader { title: tr.invoices_new_title(), title_id: INVOICES_HEADING }
            if session.permissions.read().is_none() {
                Loading {}
            } else if matches!(&*session.permissions.read(), Some(Err(_))) {
                // A failed read says nothing about access, so it offers a retry.
                ErrorState { title: tr.invoices_permissions_failed(), body: LOAD_FAILED, onretry: retry_session,
                    focus_on_recovery: INVOICES_HEADING }
            } else if !prepares {
                NoAccess {}
            } else if storage.ready() {
                if clients.state()() != UseResourceState::Ready && !matches!(&*clients.read(), Some(Err(_))) {
                    Loading { label: "Loading clients…" }
                } else {
                {loaded(&*clients.read(), Failure {
                    what: "clients for invoicing", retry: retry_clients, inset: false,
                    pending: clients.state()() != UseResourceState::Ready, focus: Some(INVOICES_HEADING),
                }, |_| match preparation::initial_client(&client_opts, client_context.as_deref()) {
                    Ok(initial_client) => rsx! {
                        preparation::PrepareInvoice { client_opts: client_opts.clone(), initial_client, busy,
                            oncreated: move |id| { navigator.push(Route::InvoiceDetail { id }); }
                        }
                    },
                    Err(message) => rsx! {
                        div { class: "alert alert-danger", role: "alert", "{message}" }
                        button { class: "btn btn-secondary", onclick: move |_| clients.restart(), "Try again" }
                    }
                })}
                }
            }
        }
    }
}

#[component]
pub fn InvoiceList() -> Element {
    rsx! { recovery::RecoveryGate { InvoiceListContent {} } }
}

#[component]
fn InvoiceListContent() -> Element {
    let tr = crate::i18n::use_tr();
    let storage = use_context::<recovery::RecoveryStorage>();
    let mut invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    let retry_invoices = use_callback(move |_| invoices.restart());
    let session = crate::components::session::use_session();
    let configures = matches!(
        &*session.permissions.read(),
        Some(Ok(own)) if settings::edits_invoice_settings(own)
    );
    let prepares = prepares_invoices(session);

    rsx! {
        div {
            PageHeader { title: "Invoices", title_id: INVOICES_HEADING,
                actions: rsx! {
                    if configures {
                        Link { to: Route::InvoiceSettings {}, class: "btn btn-secondary", "Settings" }
                    }
                    if prepares && storage.ready() {
                        Link { id: "invoices-new", to: Route::NewInvoice {}, class: "btn btn-primary", {tr.invoices_new()} }
                    }
                },
            }

            if invoices.read().is_none() {
                Skeleton { rows: 5, columns: 6, label: "Loading invoices…" }
            } else {
            {loaded(&*invoices.read(), Failure {
                what: "invoices", retry: retry_invoices, inset: false,
                pending: invoices.state()() != UseResourceState::Ready, focus: Some(INVOICES_HEADING),
            }, |list| rsx! {
                    if list.is_empty() {
                        EmptyState {
                            icon: "invoices",
                            title: "No invoices yet",
                            body: "Invoices are generated from a client's billable time or project fees.",
                        }
                    } else {
                    div { class: "card",
                        DataTable {
                            table {
                                thead {
                                    tr {
                                        th { "Number" }
                                        th { "Client" }
                                        th { "Status" }
                                        th { "Issued" }
                                        th { "Due" }
                                        th { class: "text-right", "Total" }
                                    }
                                }
                                tbody {
                                    for item in list.iter() {
                                        {
                                            let inv = item.invoice.clone();
                                            rsx! {
                                                // The number's link covers the whole row.
                                                tr { key: "{inv.id}", class: "row-link",
                                                    td { class: "text-mono",
                                                        Link { to: Route::InvoiceDetail { id: inv.id }, class: "font-semibold text-strong", "{inv.number}" }
                                                    }
                                                    td {
                                                        "{item.client_name}"
                                                    }
                                                    td {
                                                        span { class: invoice_badge_class(inv.status), {invoice_status_label(tr, inv.status)} }
                                                    }
                                                    td { class: "text-mono", "{inv.issued_on}" }
                                                    td { class: "text-mono", "{inv.due_on}" }
                                                    td { class: "text-mono text-right",
                                                        { format!("{} {}", inv.currency.trim(), format_cents_plain(inv.total_cents)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    }
                })}
            }
        }
    }
}

#[component]
pub fn InvoiceDetail(id: Uuid) -> Element {
    // A keyed fragment resets invoice-local resources and actions when the
    // router reuses this page for another ID.
    rsx! { for id in [id] { recovery::RecoveryGate { key: "{id}", InvoiceDetailContent { id } } } }
}

#[component]
fn InvoiceDetailContent(id: Uuid) -> Element {
    let storage = use_context::<recovery::RecoveryStorage>();
    let mut invoice_data =
        use_resource(move || async move { server_fns::get_invoice(id.to_string()).await });
    let retry_invoice = use_callback(move |_| invoice_data.restart());
    let mut error = use_signal(|| None::<String>);
    let mut editing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut confirm_void = use_signal(|| false);
    let session = crate::components::session::use_session();
    let manages = manages_invoices(session);
    let prepares = prepares_invoices(session);
    let tr = crate::i18n::use_tr();

    // A failed void stays inside its confirmation, apart from the page's error.
    let mut void_error = use_signal(|| None::<String>);
    // Voiding removes the Actions menu that held it, so focus goes to the title
    // once the reloaded invoice has rendered.
    let mut focus_title = use_signal(|| false);
    use_effect(move || {
        if invoice_data.state()() == UseResourceState::Ready && *focus_title.peek() {
            focus_title.set(false);
            focus_soon(INVOICE_HEADING);
        }
    });
    // Opening the values editor moves focus into it; closing it, saved or not,
    // returns focus to the menu that opened it.
    let mut was_editing = use_signal(|| false);
    use_effect(move || {
        let now = editing();
        if now == *was_editing.peek() {
            return;
        }
        was_editing.set(now);
        if now {
            // The editor loads the draft first, so wait for an enabled field,
            // unless the person has already moved focus somewhere else.
            document::eval(
                "let tries = 0;
                 (function first() {
                     const active = document.activeElement;
                     if (active && active !== document.body && active.id !== 'invoice-actions-trigger') return;
                     const editor = document.querySelector('[data-editor-kind=\"invoice\"]');
                     const field = editor?.querySelector('input:not(:disabled), select:not(:disabled), textarea:not(:disabled)');
                     field?.focus();
                     if (!editor?.contains(document.activeElement) && ++tries < 300) requestAnimationFrame(first);
                 })();",
            );
        } else {
            focus_soon("invoice-actions-trigger");
        }
    });

    let mut change_status = move |to: &'static str| {
        if busy() || editing() || !storage.ready() {
            return;
        }
        error.set(None);
        void_error.set(None);
        busy.set(true);
        spawn(async move {
            match server_fns::update_invoice_status(id.to_string(), to.to_string()).await {
                Ok(_) => {
                    if to == "void" {
                        focus_title.set(true);
                    }
                    confirm_void.set(false);
                    invoice_data.restart();
                }
                Err(err) if to == "void" => void_error.set(Some(err.to_string())),
                Err(err) => error.set(Some(err.to_string())),
            }
            busy.set(false);
        });
    };
    let void = use_callback(move |()| change_status("void"));
    let keep = use_callback(move |()| {
        if !busy() {
            void_error.set(None);
            confirm_void.set(false);
        }
    });
    let edit_values = use_callback(move |_: MouseEvent| {
        if storage.ready() && !busy() {
            editing.set(true);
        }
    });
    let ask_void = use_callback(move |_: MouseEvent| {
        void_error.set(None);
        confirm_void.set(true);
    });

    // The one Primary moves the invoice on: sent, then paid.
    let status_btn = move |to: &'static str, label: &'static str| {
        if !manages || !storage.ready() {
            return rsx! {};
        }
        rsx! {
            button {
                class: "btn btn-primary",
                disabled: busy() || editing(),
                onclick: move |_| change_status(to),
                {label}
            }
        }
    };

    rsx! {
        div {
            BackLink { to: Route::InvoiceList {}, label: tr.invoices_back() }
            // The number names the title once the invoice has loaded.
            if !matches!(&*invoice_data.read(), Some(Ok(_))) {
                PageHeader { title: "Invoice" }
            }
            {loaded(&*invoice_data.read(), Failure {
                what: "this invoice", retry: retry_invoice, inset: false,
                pending: invoice_data.state()() != UseResourceState::Ready, focus: Some("page-back"),
            }, |data| {
                let inv = data.invoice.clone();
                let lines = data.lines.clone();
                let cname = &data.client_name;
                let open = matches!(inv.status, InvoiceStatus::Draft | InvoiceStatus::Sent);
                let edits = prepares && inv.status == InvoiceStatus::Draft && storage.ready();
                let voids = manages && open && storage.ready();

                rsx! {
                    PageHeader { title: format!("Invoice {}", inv.number), title_id: INVOICE_HEADING,
                        actions: rsx! {
                            match inv.status {
                                InvoiceStatus::Draft => status_btn("sent", "Mark Sent"),
                                InvoiceStatus::Sent => status_btn("paid", "Mark Paid"),
                                _ => rsx! {},
                            }
                            a {
                                class: "btn btn-secondary",
                                href: "/api/invoices/{inv.id}/export/pdf",
                                target: "_blank",
                                "Download PDF"
                            }
                            if edits || voids {
                                Menu { id: "invoice-actions", label: tr.action_menu(), align_right: true, trigger_class: "text-sm py-3 px-5",
                                    if edits {
                                        MenuItem { id: "invoice-edit-values", disabled: busy() || editing(), onclick: edit_values, {tr.invoices_edit_values()} }
                                    }
                                    if edits && voids { MenuDivider {} }
                                    if voids {
                                        MenuItem { id: "invoice-void", danger: true, disabled: busy() || editing(), onclick: ask_void, {tr.invoices_void_ellipsis()} }
                                    }
                                }
                            }
                        },
                    }
                    ConfirmDialog { id: "invoice-void-dialog", labelledby: "invoice-void-title",
                        open: confirm_void(), busy: busy(), focus_fallback: Some(INVOICE_HEADING),
                        title: tr.invoices_void_title(inv.number.clone()),
                        confirm_label: tr.invoices_void_confirm(),
                        on_confirm: void, on_dismiss: keep,
                        p { {tr.invoices_void_body()} }
                        if let Some(err) = &*void_error.read() {
                            p { class: "alert alert-danger", role: "alert", "{err}" }
                        }
                    }

                    if let Some(err) = &*error.read() {
                        div { class: "alert alert-danger", "{err}" }
                    }

                    div { class: "card mb-6",
                        div { class: "p-5",
                            div { class: "grid gap-4 inv-meta",
                                div {
                                    div { class: "text-sm text-muted", "Client" }
                                    div { "{cname}" }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Status" }
                                    span { class: invoice_badge_class(inv.status), {invoice_status_label(tr, inv.status)} }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Issued" }
                                    div { class: "text-mono", "{inv.issued_on}" }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Due" }
                                    div { class: "text-mono", "{inv.due_on}" }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Payment terms" }
                                    div { "{inv.terms_days} days" }
                                }
                                if !inv.po_number.is_empty() {
                                    div {
                                        div { class: "text-sm text-muted", "Purchase order" }
                                        div { class: "wrap-anywhere", "{inv.po_number}" }
                                    }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Total" }
                                    div { class: "text-mono",
                                        { format!("{} {}", inv.currency.trim(), format_cents_plain(inv.total_cents)) }
                                    }
                                }
                            }
                        }
                    }

                    if prepares && inv.status == InvoiceStatus::Draft {
                        defaults_form::DraftDefaults { invoice: inv.clone(), editing, busy, onsaved: move |_| invoice_data.restart() }
                    }

                    div { class: "card",
                        DataTable {
                            table {
                                thead {
                                    tr {
                                        th { "Description" }
                                        th { class: "text-right", "Hours" }
                                        th { class: "text-right", "Rate" }
                                        th { class: "text-right", "Amount" }
                                    }
                                }
                                tbody {
                                    for line in lines.iter() {
                                        tr { key: "{line.id}",
                                            td { "{line.description}" }
                                            td { class: "text-mono text-right",
                                                { line.minutes.map(|minutes| horae_core::duration::format_hhmm(minutes.into())).unwrap_or_else(|| "—".into()) }
                                            }
                                            td { class: "text-mono text-right",
                                                { line.rate_cents.map(|rate| format!("{}/hr", format_cents_plain(rate))).unwrap_or_else(|| "—".into()) }
                                            }
                                            td { class: "text-mono text-right",
                                                { format_cents_plain(line.amount_cents) }
                                            }
                                        }
                                    }
                                }
                                tfoot {
                                    for (label, cents) in inv.breakdown() {
                                        tr {
                                            th { scope: "row", colspan: "3", class: "text-right wrap-anywhere", "{label}" }
                                            td { class: if label == "Total" { "text-mono text-right font-semibold" } else { "text-mono text-right" },
                                                { format!("{} {}", inv.currency.trim(), format_cents_plain(cents)) }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            })}
        }
    }
}
