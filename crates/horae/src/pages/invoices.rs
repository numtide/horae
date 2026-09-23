use std::collections::HashMap;

use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::loaded;
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

/// The badge class for an invoice status — one convention for list and detail.
fn invoice_badge_class(status: InvoiceStatus) -> &'static str {
    match status {
        InvoiceStatus::Paid => "badge badge-success",
        InvoiceStatus::Sent => "badge badge-info",
        InvoiceStatus::Void => "badge badge-neutral",
        InvoiceStatus::Draft => "badge badge-warning",
    }
}

#[component]
pub fn InvoiceList() -> Element {
    rsx! { recovery::RecoveryGate { InvoiceListContent {} } }
}

#[component]
fn InvoiceListContent() -> Element {
    let storage = use_context::<recovery::RecoveryStorage>();
    let invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });

    let client_names: HashMap<Uuid, String> = clients
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|cs| cs.iter().map(|c| (c.id, c.name.clone())).collect())
        .unwrap_or_default();

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

    let mut show_form = use_signal(|| false);
    let busy = use_signal(|| false);
    let navigator = use_navigator();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoices" }
                div { class: "page-actions",
                    if storage.ready() {
                    button {
                        class: "btn btn-primary",
                        disabled: busy(),
                        onclick: move |_| {
                            if !busy() && storage.ready() { show_form.toggle(); }
                        },
                        if show_form() { "Cancel" } else { "New Invoice" }
                    }
                    }
                }
            }

            if show_form() {
                {loaded(&*clients.read(), |_| rsx! {
                    preparation::PrepareInvoice { client_opts: client_opts.clone(), busy,
                        oncreated: move |id| {
                            show_form.set(false);
                            navigator.push(Route::InvoiceDetail { id });
                        }
                    }
                })}
            }

            div { class: "card",
                {loaded(&*invoices.read(), |list| rsx! {
                    if list.is_empty() {
                        div { class: "text-muted text-sm p-5",
                            "No invoices yet. Generate one from billable time or project fees."
                        }
                    } else {
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
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for invoice in list.iter() {
                                        {
                                            let inv = invoice.clone();
                                            rsx! {
                                                tr { key: "{inv.id}",
                                                    td { class: "text-mono", "{inv.number}" }
                                                    td {
                                                        {client_names.get(&inv.client_id)
                                                            .cloned()
                                                            .unwrap_or_else(|| inv.client_id.to_string())}
                                                    }
                                                    td {
                                                        span { class: invoice_badge_class(inv.status), "{inv.status}" }
                                                    }
                                                    td { class: "text-mono", "{inv.issued_on}" }
                                                    td { class: "text-mono", "{inv.due_on}" }
                                                    td { class: "text-mono text-right",
                                                        { format!("{} {}", inv.currency.trim(), format_cents_plain(inv.total_cents)) }
                                                    }
                                                    td {
                                                        Link {
                                                            to: Route::InvoiceDetail { id: inv.id },
                                                            class: "btn btn-secondary btn-sm",
                                                            "View"
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
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let mut error = use_signal(|| None::<String>);
    let editing = use_signal(|| false);
    let mut busy = use_signal(|| false);

    let client_name = |cid: Uuid| -> String {
        clients
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .and_then(|cs| cs.iter().find(|c| c.id == cid))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| cid.to_string())
    };

    // One button per status transition; the four share everything but the
    // target status, label, and emphasis.
    let status_btn = move |to: &'static str, label: &'static str, primary: bool| {
        if !storage.ready() {
            return rsx! {};
        }
        rsx! {
            button {
                class: if primary { "btn btn-primary" } else { "btn btn-secondary" },
                style: if !primary { "margin-left: 0.5rem;" },
                disabled: busy() || editing(),
                onclick: move |_| {
                    if busy() || editing() || !storage.ready() { return; }
                    error.set(None);
                    busy.set(true);
                    spawn(async move {
                        match server_fns::update_invoice_status(id.to_string(), to.to_string()).await {
                            Ok(_) => invoice_data.restart(),
                            Err(err) => error.set(Some(err.to_string())),
                        }
                        busy.set(false);
                    });
                },
                {label}
            }
        }
    };

    rsx! {
        div {
            {loaded(&*invoice_data.read(), |data| {
                let inv = data.invoice.clone();
                let lines = data.lines.clone();
                let cname = client_name(inv.client_id);

                rsx! {
                    div { class: "page-header",
                        h1 { class: "page-title", "Invoice {inv.number}" }
                        div { class: "page-actions",
                            match inv.status {
                                InvoiceStatus::Draft => rsx! {
                                    {status_btn("sent", "Mark Sent", true)}
                                    {status_btn("void", "Void", false)}
                                },
                                InvoiceStatus::Sent => rsx! {
                                    {status_btn("paid", "Mark Paid", true)}
                                    {status_btn("void", "Void", false)}
                                },
                                _ => rsx! {},
                            }
                            a {
                                class: "btn btn-secondary",
                                style: "margin-left: 0.5rem;",
                                href: "/api/invoices/{inv.id}/export/pdf",
                                target: "_blank",
                                "Download PDF"
                            }
                        }
                    }

                    if let Some(err) = &*error.read() {
                        div { class: "alert alert-danger", "{err}" }
                    }

                    div { class: "card mb-6",
                        div { class: "p-5",
                            div { class: "grid gap-4", style: "grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));",
                                div {
                                    div { class: "text-sm text-muted", "Client" }
                                    div { "{cname}" }
                                }
                                div {
                                    div { class: "text-sm text-muted", "Status" }
                                    span { class: invoice_badge_class(inv.status), "{inv.status}" }
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

                    if inv.status == InvoiceStatus::Draft {
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
