use dioxus::prelude::*;
use horae_core::{duration::format_hhmm, money::format_cents};
use uuid::Uuid;

use crate::components::{badge::Badge, icons::NavIcon};
use crate::route::Route;
use crate::server_fns;

#[component]
pub fn ClientDetail(id: Uuid) -> Element {
    // Route reuse must discard client-local requests, errors and editors together.
    rsx! { for id in [id] { ClientDetailContent { key: "{id}", id } } }
}

#[component]
fn ClientDetailContent(id: Uuid) -> Element {
    let mut show_editor = use_signal(|| false);
    let mut details =
        use_resource(move || async move { server_fns::get_client_details(id.to_string()).await });
    let ready = details.state()() == UseResourceState::Ready;
    let data = details.read();

    rsx! {
        div { class: "client-detail min-w-0",
            Link { id: "client-back", to: Route::ClientList {}, class: "btn btn-ghost text-secondary mb-5 -ml-2.5",
                NavIcon { name: "arrow-left" }
                "Back to Clients"
            }
            if !ready {
                p { role: "status", aria_busy: "true", "Loading client…" }
            } else if let Some(Ok(detail)) = &*data {
                div { class: "page-header border-b pb-6 mb-4",
                    div { class: "min-w-0",
                        p { class: "text-xs text-secondary m-0 mb-2",
                            "Added to Horae {detail.client.created_at.format(\"%d %b %Y\")}"
                        }
                        div { class: "flex flex-wrap items-center gap-3",
                            h1 { class: "page-title text-4xl font-semibold text-strong tracking-tight wrap-anywhere",
                                "{detail.client.name}"
                            }
                            span { class: "font-mono text-xs text-secondary rounded-full border bg-tertiary px-2 py-1",
                                "{detail.client.currency.trim()}"
                            }
                            if !detail.client.active { Badge { variant: "neutral", "Inactive" } }
                        }
                    }
                    if detail.billing.is_some() {
                        div { class: "page-actions",
                            if detail.client.active {
                                Link { to: Route::NewProjectForClient { client: id.to_string() }, class: "btn btn-secondary", "New project" }
                                Link { to: Route::NewInvoiceForClient { client: id.to_string() }, class: "btn btn-primary", "New invoice" }
                            }
                            button { r#type: "button", class: "btn btn-secondary", onclick: move |_| show_editor.set(true), "Edit client" }
                        }
                    }
                }
                div { class: "client-detail-grid grid gap-4 items-start",
                    ClientProjects { client_id: id }
                    div { class: "flex flex-col gap-4 min-w-0",
                        section { class: "border rounded-xl bg-secondary p-5 min-w-0", aria_labelledby: "client-billing-title",
                            h2 { id: "client-billing-title", class: "text-lg font-semibold m-0 mb-3", "Billing" }
                            p { class: "text-sm wrap-anywhere m-0 mb-4",
                                if let Some(address) = detail.client.address.as_deref().filter(|value| !value.trim().is_empty()) {
                                    for line in address.lines() { span { class: "block", "{line}" } }
                                } else { "No billing address set" }
                            }
                            dl { class: "text-sm m-0 flex flex-col gap-3",
                                if let Some(billing) = &detail.billing {
                                    div { class: "flex flex-wrap justify-between gap-2 border-t pt-3",
                                        dt { class: "text-secondary", "Default rate" }
                                        dd { class: "font-mono m-0 wrap-anywhere",
                                            {billing.default_rate_cents.map(|rate| format!("{} / h", format_cents(rate, detail.client.currency.trim()))).unwrap_or_else(|| "Not set".into())}
                                        }
                                    }
                                }
                                div { class: "flex flex-wrap justify-between gap-2",
                                    dt { class: "text-secondary", "Tax ID" }
                                    dd { class: "font-mono m-0 wrap-anywhere",
                                        "{detail.client.tax_id.as_deref().filter(|value| !value.trim().is_empty()).unwrap_or(\"Not set\")}"
                                    }
                                }
                            }
                        }
                        if detail.billing.is_some() { ClientInvoices { client_id: id } }
                    }
                }
            } else {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load this client. It may be unavailable or you may no longer have access. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| details.restart(), "Retry client" }
                }
            }
            super::form::ClientEditor { open: show_editor(), client_id: Some(id), focus_fallback: "client-back",
                on_dismiss: move |_| show_editor.set(false),
                on_saved: move |_| { show_editor.set(false); details.restart(); },
            }
        }
    }
}

#[component]
fn ClientProjects(client_id: Uuid) -> Element {
    let mut projects = use_resource(move || async move {
        server_fns::list_projects(Some(client_id.to_string()), true).await
    });
    let mut spend = use_resource(|| async { server_fns::list_project_spend(None).await });
    let ready = projects.state()() == UseResourceState::Ready;
    let spend_ready = spend.state()() == UseResourceState::Ready;
    let data = projects.read();
    let spend_data = spend.read();
    let totals = spend_ready
        .then(|| spend_data.as_ref().and_then(|result| result.as_ref().ok()))
        .flatten();

    rsx! {
        section { class: "border rounded-xl bg-secondary min-w-0 overflow-hidden", aria_labelledby: "client-projects-title",
            div { class: "flex flex-wrap items-center gap-3 px-5 py-4 border-b",
                h2 { id: "client-projects-title", class: "text-lg font-semibold m-0", "Projects" }
                if ready {
                    if let Some(Ok(rows)) = &*data {
                        span { class: "text-xs text-secondary",
                            "{rows.iter().filter(|project| project.active).count()} active · {rows.iter().filter(|project| !project.active).count()} inactive"
                        }
                    }
                }
                Link { to: Route::ProjectsForClient { client: client_id.to_string() }, class: "text-sm font-semibold ml-auto", "View in Projects" }
            }
            if !ready {
                p { class: "p-5 m-0", role: "status", aria_busy: "true", "Loading projects…" }
            } else if let Some(Ok(rows)) = &*data {
                if rows.is_empty() {
                    p { class: "p-5 m-0 text-secondary", "No projects available to you for this client." }
                } else {
                    if !spend_ready {
                        p { class: "px-5 text-sm text-secondary", role: "status", "Loading project totals…" }
                    } else if let Some(Err(_)) = &*spend_data {
                        div { class: "alert alert-danger m-4", role: "alert",
                            "Could not load project totals. "
                            button { r#type: "button", class: "btn btn-secondary btn-sm",
                                onclick: move |_| spend.restart(), "Retry totals" }
                        }
                    }
                    div { class: "overflow-x-auto", role: "region", aria_label: "Client projects", tabindex: "0",
                        table { class: "client-table",
                            thead { tr {
                                th { scope: "col", class: "text-secondary", "Project" }
                                th { scope: "col", class: "text-right text-secondary", "Hours" }
                                th { scope: "col", class: "text-right text-secondary", "Spent" }
                                th { scope: "col", class: "text-secondary", "Status" }
                            } }
                            tbody {
                                for project in rows {
                                    {
                                        let total = totals.and_then(|rows| rows.iter().find(|row| row.project_id == project.id));
                                        let hours = total.map(|row| format_hhmm(row.spent_minutes)).unwrap_or_else(|| "—".into());
                                        let amount = total.and_then(|row| row.spent_cents).map(|cents| format_cents(cents, project.currency.trim())).unwrap_or_else(|| "—".into());
                                        rsx! { tr { key: "{project.id}",
                                            td {
                                                Link { to: Route::ProjectDetail { id: project.id }, class: "font-semibold text-strong wrap-anywhere",
                                                    if let Some(code) = &project.code { "[{code}] " }
                                                    "{project.name}"
                                                }
                                                div { class: "text-xs text-secondary mt-1", "{project.project_type.label()} · {project.currency.trim()}" }
                                            }
                                            td { class: "font-mono text-right whitespace-nowrap", "{hours}" }
                                            td { class: "font-mono text-right whitespace-nowrap", "{amount}" }
                                            td { if project.active { Badge { variant: "success", "Active" } } else { Badge { variant: "neutral", "Inactive" } } }
                                        } }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "alert alert-danger m-4", role: "alert",
                    "Could not load projects. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| projects.restart(), "Retry projects" }
                }
            }
        }
    }
}

#[component]
fn ClientInvoices(client_id: Uuid) -> Element {
    let mut invoices = use_resource(move || async move {
        server_fns::list_client_invoices(client_id.to_string()).await
    });
    let ready = invoices.state()() == UseResourceState::Ready;
    let data = invoices.read();
    rsx! {
        section { class: "border rounded-xl bg-secondary p-5 min-w-0", aria_labelledby: "client-invoices-title",
            h2 { id: "client-invoices-title", class: "text-lg font-semibold m-0 mb-3", "Invoices" }
            if !ready {
                p { role: "status", aria_busy: "true", "Loading invoices…" }
            } else if let Some(Ok(rows)) = &*data {
                if rows.is_empty() {
                    p { class: "text-sm text-secondary m-0", "No invoices for this client." }
                } else {
                    ul { role: "list", class: "m-0 p-0 flex flex-col gap-3",
                        for invoice in rows {
                            li { key: "{invoice.id}", class: "flex flex-wrap items-center justify-between gap-2 text-sm",
                                Link { to: Route::InvoiceDetail { id: invoice.id }, class: "font-semibold wrap-anywhere", "{invoice.number}" }
                                span { class: "font-mono text-secondary wrap-anywhere", "{format_cents(invoice.total_cents, invoice.currency.trim())}" }
                                span { class: super::super::invoices::invoice_badge_class(invoice.status), "{invoice.status}" }
                            }
                        }
                    }
                }
            } else {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load invoices. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| invoices.restart(), "Retry invoices" }
                }
            }
        }
    }
}
