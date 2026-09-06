use dioxus::prelude::*;
use uuid::Uuid;

use super::{is_manager, loaded, run_action};
use crate::components::badge::Badge;
use crate::components::form::{FormCard, FormGroup, Input, Textarea};
use crate::components::table::DataTable;
use crate::server_fns;

#[component]
pub fn ClientList() -> Element {
    // Management view: include inactive clients so managers can reactivate them.
    let clients = use_resource(|| async move { server_fns::list_clients(true).await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing client, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut name = use_signal(String::new);
    let mut currency = use_signal(|| "USD".to_string());
    let mut address = use_signal(String::new);
    let mut tax_id = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let is_manager = is_manager(&me);

    let mut reset_form = move || {
        editing_id.set(None);
        name.set(String::new());
        currency.set("USD".to_string());
        address.set(String::new());
        tax_id.set(String::new());
        error.set(None);
        show_form.set(false);
    };

    let form_title = if editing_id().is_some() {
        "Edit Client"
    } else {
        "New Client"
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Clients" }
                div { class: "page-actions",
                    if is_manager {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                if show_form() {
                                    reset_form();
                                } else {
                                    editing_id.set(None);
                                    show_form.set(true);
                                }
                            },
                            if show_form() { "Cancel" } else { "Add Client" }
                        }
                    }
                }
            }

            if show_form() && is_manager {
                FormCard { title: "{form_title}", error,
                    FormGroup { label: "Name", id: "client-name",
                        Input {
                            id: "client-name",
                            placeholder: "Client name",
                            value: "{name}",
                            oninput: move |e: FormEvent| name.set(e.value()),
                        }
                    }
                    FormGroup { label: "Currency", id: "client-currency",
                        Input {
                            id: "client-currency",
                            placeholder: "USD",
                            value: "{currency}",
                            oninput: move |e: FormEvent| currency.set(e.value()),
                        }
                    }
                    FormGroup { label: "Address (optional)", id: "client-address",
                        Textarea {
                            id: "client-address",
                            placeholder: "Client address",
                            value: "{address}",
                            oninput: move |e: FormEvent| address.set(e.value()),
                        }
                    }
                    FormGroup { label: "Tax ID (optional)", id: "client-taxid",
                        Input {
                            id: "client-taxid",
                            placeholder: "Tax ID",
                            value: "{tax_id}",
                            oninput: move |e: FormEvent| tax_id.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let editing = editing_id();
                            let n = name();
                            let c = currency();
                            let a = address();
                            let t = tax_id();
                            run_action(
                                async move {
                                    let addr = if a.is_empty() { None } else { Some(a) };
                                    let tid = if t.is_empty() { None } else { Some(t) };
                                    match editing {
                                        Some(id) => {
                                            server_fns::update_client(id.to_string(), n, c, addr, tid).await
                                        }
                                        None => server_fns::create_client(n, c, addr, tid).await,
                                    }
                                },
                                clients,
                                error,
                                reset_form,
                            );
                        },
                        if editing_id().is_some() { "Save Changes" } else { "Create Client" }
                    }
                }
            }

            div { class: "card",
                {loaded(&*clients.read(), |list| rsx! {
                    DataTable {
                        table {
                            thead {
                                tr {
                                    th { "Name" }
                                    th { "Currency" }
                                    th { "Address" }
                                    th { "Status" }
                                    th { "Actions" }
                                }
                            }
                            tbody {
                                for client in list.iter() {
                                    {
                                        let c = client.clone();
                                        rsx! {
                                            tr { key: "{c.id}",
                                                td { "{c.name}" }
                                                td { "{c.currency}" }
                                                td { "{c.address.as_deref().unwrap_or(\"-\")}" }
                                                td {
                                                    if c.active {
                                                        Badge { variant: "success", "Active" }
                                                    } else {
                                                        Badge { variant: "neutral", "Inactive" }
                                                    }
                                                }
                                                td {
                                                    if is_manager {
                                                        button {
                                                            class: "btn btn-secondary btn-sm",
                                                            onclick: {
                                                                let c = c.clone();
                                                                move |_| {
                                                                    editing_id.set(Some(c.id));
                                                                    name.set(c.name.clone());
                                                                    currency.set(c.currency.clone());
                                                                    address.set(c.address.clone().unwrap_or_default());
                                                                    tax_id.set(c.tax_id.clone().unwrap_or_default());
                                                                    error.set(None);
                                                                    show_form.set(true);
                                                                }
                                                            },
                                                            "Edit"
                                                        }
                                                        button {
                                                            class: "btn btn-secondary btn-sm",
                                                            style: "margin-left: 0.5rem;",
                                                            onclick: {
                                                                let id = c.id;
                                                                let next_active = !c.active;
                                                                move |_| run_action(
                                                                    server_fns::set_client_active(id.to_string(), next_active),
                                                                    clients,
                                                                    error,
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
                })}
            }
        }
    }
}

#[component]
pub fn ClientDetail(id: Uuid) -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Client" }
            }
            div { class: "card",
                p { class: "text-muted", "Client detail for {id}" }
            }
        }
    }
}
