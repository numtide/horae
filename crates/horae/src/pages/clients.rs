use dioxus::prelude::*;
use std::collections::BTreeSet;
use uuid::Uuid;

use super::{is_admin, is_manager, run_action};
use crate::components::badge::Badge;
use crate::components::icons::NavIcon;
use crate::components::menu::{Menu, MenuItem};
use crate::route::Route;
use crate::server_fns;

#[path = "clients/filters.rs"]
mod filters;
use filters::{ClientScope, matches_query_currency};

#[path = "clients/detail.rs"]
mod detail;
pub use detail::ClientDetail;

#[path = "clients/form.rs"]
mod form;

#[component]
pub fn ClientList() -> Element {
    // Management view: include inactive clients so managers can reactivate them.
    let mut clients = use_resource(|| async move { server_fns::list_client_summaries().await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut query = use_signal(String::new);
    let mut scope = use_signal(ClientScope::default);
    let mut currency_filter = use_signal(String::new);

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing client, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let action_error = use_signal(|| None::<String>);

    let is_manager = is_manager(&me);
    let can_import = is_admin(&me);
    let ready = clients.state()() == UseResourceState::Ready;
    let data = clients.read();
    let list = data.as_ref().and_then(|result| result.as_ref().ok());
    let matching: Vec<_> = list
        .into_iter()
        .flatten()
        .filter(|row| matches_query_currency(row, &query(), &currency_filter()))
        .collect();
    let count_scope = |status: ClientScope| {
        matching
            .iter()
            .filter(|row| status.matches(row.client.active))
            .count()
    };
    let active_count = count_scope(ClientScope::Active);
    let inactive_count = count_scope(ClientScope::Inactive);
    let all_count = matching.len();
    let scope_label = format!("{} ({})", scope().label(), count_scope(scope()));
    let currencies: BTreeSet<String> = list
        .into_iter()
        .flatten()
        .flat_map(|row| std::iter::once(&row.client.currency).chain(row.project_currencies.iter()))
        .map(|currency| currency.trim().to_owned())
        .collect();
    let currency_label = if currency_filter().is_empty() {
        "All currencies".to_owned()
    } else {
        currency_filter()
    };
    let visible: Vec<_> = matching
        .into_iter()
        .filter(|row| scope().matches(row.client.active))
        .collect();

    let mut reset_form = move || {
        editing_id.set(None);
        show_form.set(false);
    };

    rsx! {
        div {
            div { class: "page-header mb-5",
                h1 { id: "clients-heading", tabindex: "-1", class: "page-title text-4xl font-semibold text-strong tracking-tight", "Clients" }
                div { class: "page-actions items-center gap-4",
                    if is_manager {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| { editing_id.set(None); show_form.set(true); },
                            "New client"
                        }
                    }
                    if can_import {
                        Link { to: Route::HarvestImport {}, class: "btn btn-secondary", "Import" }
                    }
                    input {
                        class: "form-input client-search w-form-select max-w-full",
                        r#type: "search",
                        aria_label: "Search clients by name",
                        placeholder: "Search clients by name",
                        value: "{query}",
                        oninput: move |event| query.set(event.value()),
                    }
                }
            }

            if ready && list.is_some() {
                div { class: "flex flex-wrap items-center gap-4 mb-5",
                    Menu { id: "client-scope-menu", label: scope_label, trigger_class: "text-sm px-4",
                        MenuItem { selected: scope() == ClientScope::Active,
                            onclick: move |_| scope.set(ClientScope::Active), "Active clients ({active_count})" }
                        MenuItem { selected: scope() == ClientScope::Inactive,
                            onclick: move |_| scope.set(ClientScope::Inactive), "Inactive clients ({inactive_count})" }
                        MenuItem { selected: scope() == ClientScope::All,
                            onclick: move |_| scope.set(ClientScope::All), "All clients ({all_count})" }
                    }
                    div { class: "flex-1" }
                    Menu { id: "client-currency-menu", label: currency_label, align_right: true, trigger_class: "text-sm px-4",
                        MenuItem { selected: currency_filter().is_empty(),
                            onclick: move |_| currency_filter.set(String::new()), "All currencies" }
                        for code in currencies {
                            MenuItem { key: "{code}", selected: currency_filter() == code,
                                onclick: { let code = code.clone(); move |_| currency_filter.set(code.clone()) },
                                "{code}"
                            }
                        }
                    }
                }
            }

            form::ClientEditor { open: show_form() && is_manager, client_id: editing_id(), focus_fallback: "clients-heading",
                on_dismiss: move |_| reset_form(),
                on_saved: move |_| { reset_form(); clients.restart(); },
            }

            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "Could not change client status: {message}" }
            }

            if !ready {
                p { class: "text-sm text-secondary", role: "status", aria_busy: "true", "Loading clients…" }
            } else if let Some(Err(_)) = &*data {
                div { class: "alert alert-danger", role: "alert",
                    "Could not load clients. Retry to refresh the list. "
                    button { r#type: "button", class: "btn btn-secondary btn-sm",
                        onclick: move |_| clients.restart(), "Retry clients" }
                }
            } else if let Some(list) = list {
                if visible.is_empty() {
                    div { class: "empty-state rounded-xl",
                        div { class: "empty-state-icon-tile", aria_hidden: "true", NavIcon { name: "clients" } }
                        if list.is_empty() {
                            h2 { class: "empty-state-title text-xl m-0", "No clients yet" }
                            p { class: "empty-state-text m-0", "Clients group your projects and billing information." }
                            div { class: "flex flex-wrap justify-center gap-3",
                                if is_manager {
                                    button { class: "btn btn-primary", onclick: move |_| { reset_form(); show_form.set(true); }, "New client" }
                                }
                                if can_import {
                                    Link { to: Route::HarvestImport {}, class: "btn btn-secondary", "Import from Harvest" }
                                }
                            }
                        } else {
                            h2 { class: "empty-state-title text-xl m-0", "No clients match your filters" }
                            p { class: "empty-state-text m-0", "Try another name, status or currency." }
                            button { class: "btn btn-secondary", onclick: move |_| {
                                query.set(String::new()); currency_filter.set(String::new()); scope.set(ClientScope::All);
                            }, "Clear filters" }
                        }
                    }
                } else {
                    div { class: "table-container rounded-xl bg-secondary", role: "region", aria_label: "Clients", tabindex: "0",
                        table { class: "client-table",
                            thead {
                                tr {
                                    th { scope: "col", class: "text-secondary", "Client" }
                                    th { scope: "col", class: "text-right text-secondary", "Currency" }
                                    th { scope: "col", class: "text-right text-secondary", "Projects" }
                                    th { scope: "col", class: "text-secondary", "Status" }
                                    if is_manager { th { scope: "col", class: "text-secondary", "Actions" } }
                                }
                            }
                            tbody {
                                for summary in visible {
                                    {
                                        let c = summary.client.clone();
                                        let row_currencies: BTreeSet<_> = std::iter::once(c.currency.trim())
                                            .chain(summary.project_currencies.iter().map(|c| c.trim())).collect();
                                        let row_currencies = row_currencies.into_iter().collect::<Vec<_>>().join(" / ");
                                        rsx! {
                                            tr { key: "{c.id}",
                                                td { Link { to: Route::ClientDetail { id: c.id }, class: "font-semibold text-strong wrap-anywhere", "{c.name}" } }
                                                td { class: "text-right font-mono text-secondary", "{row_currencies}" }
                                                td { class: "text-right font-mono whitespace-nowrap",
                                                    span { aria_label: "{summary.active_projects} active of {summary.total_projects} visible projects",
                                                        "{summary.active_projects} / {summary.total_projects}"
                                                    }
                                                }
                                                td {
                                                    if c.active {
                                                        Badge { variant: "success", "Active" }
                                                    } else {
                                                        Badge { variant: "neutral", "Inactive" }
                                                    }
                                                }
                                                if is_manager {
                                                    td {
                                                        div { class: "flex items-center gap-2",
                                                        button {
                                                            class: "btn btn-secondary btn-sm",
                                                            onclick: {
                                                                let c = c.clone();
                                                                move |_| {
                                                                    editing_id.set(Some(c.id));
                                                                    show_form.set(true);
                                                                }
                                                            },
                                                            "Edit"
                                                        }
                                                        button {
                                                            class: "btn btn-secondary btn-sm",
                                                            onclick: {
                                                                let id = c.id;
                                                                let next_active = !c.active;
                                                                move |_| run_action(
                                                                    server_fns::set_client_active(id.to_string(), next_active),
                                                                    clients,
                                                                    action_error,
                                                                    || (),
                                                                )
                                                            },
                                                            if c.active { "Deactivate" } else { "Activate" }
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
                    }
                }
            }
        }
    }
}
