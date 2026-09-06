use std::collections::HashMap;

use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::{loaded, run_action};
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::table::DataTable;
use crate::route::Route;
use crate::server_fns;

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
    let mut selected_client = use_signal(String::new);
    let mut period_from = use_signal(String::new);
    let mut period_to = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoices" }
                div { class: "page-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            if show_form() {
                                show_form.set(false);
                                error.set(None);
                            } else {
                                show_form.set(true);
                            }
                        },
                        if show_form() { "Cancel" } else { "New Invoice" }
                    }
                }
            }

            if show_form() {
                FormCard { title: "Generate Invoice", error,
                    FormGroup { label: "Client", id: "inv-client",
                        Select {
                            id: "inv-client",
                            options: client_opts,
                            selected: selected_client(),
                            onchange: move |e: FormEvent| selected_client.set(e.value()),
                        }
                    }
                    FormGroup { label: "Period from", id: "inv-from",
                        Input {
                            id: "inv-from",
                            kind: "date",
                            value: "{period_from}",
                            oninput: move |e: FormEvent| period_from.set(e.value()),
                        }
                    }
                    FormGroup { label: "Period to", id: "inv-to",
                        Input {
                            id: "inv-to",
                            kind: "date",
                            value: "{period_to}",
                            oninput: move |e: FormEvent| period_to.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let client = selected_client();
                            let from = period_from();
                            let to = period_to();
                            run_action(
                                server_fns::generate_invoice(client, from, to),
                                invoices,
                                error,
                                move || show_form.set(false),
                            );
                        },
                        "Generate Invoice"
                    }
                }
            }

            div { class: "card",
                {loaded(&*invoices.read(), |list| rsx! {
                    if list.is_empty() {
                        div { class: "text-muted text-sm p-5",
                            "No invoices yet. Generate one from billable time."
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
    let invoice_data =
        use_resource(move || async move { server_fns::get_invoice(id.to_string()).await });
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let error = use_signal(|| None::<String>);

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
        rsx! {
            button {
                class: if primary { "btn btn-primary" } else { "btn btn-secondary" },
                style: if !primary { "margin-left: 0.5rem;" },
                onclick: move |_| run_action(
                    server_fns::update_invoice_status(id.to_string(), to.to_string()),
                    invoice_data,
                    error,
                    || (),
                ),
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
                                    div { class: "text-sm text-muted", "Total" }
                                    div { class: "text-mono",
                                        { format!("{} {}", inv.currency.trim(), format_cents_plain(inv.total_cents)) }
                                    }
                                }
                            }
                        }
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
                                                { horae_core::duration::format_hhmm(line.minutes.into()) }
                                            }
                                            td { class: "text-mono text-right",
                                                { format!("{}/hr", format_cents_plain(line.rate_cents)) }
                                            }
                                            td { class: "text-mono text-right",
                                                { format_cents_plain(line.amount_cents) }
                                            }
                                        }
                                    }
                                }
                                tfoot {
                                    tr {
                                        td { colspan: "3", class: "text-right font-semibold", "Total" }
                                        td { class: "text-mono text-right font-semibold",
                                            { format!("{} {}", inv.currency.trim(), format_cents_plain(inv.total_cents)) }
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
