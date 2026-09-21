use dioxus::prelude::*;
use horae_core::project::PROJECT_CURRENCIES;
use uuid::Uuid;

use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::modal::Modal;
use crate::models::project_creation::{
    CreationClient, CreationOptions, CreationSearch, ProjectForm,
};
use crate::server_fns;

use super::FormRow;

#[component]
pub(super) fn Basics(
    mut form: Signal<ProjectForm>,
    mut options: Signal<CreationOptions>,
) -> Element {
    let mut client_query = use_signal(String::new);
    let mut client_open = use_signal(|| false);
    let mut tag_input = use_signal(String::new);
    let clients = use_resource(move || {
        let query = client_query();
        async move {
            let mut search = CreationSearch::default();
            search.clients.query = query;
            server_fns::project_creation_options(search).await
        }
    });
    let selected = form.read().client_id;
    let catalog = options.read();
    let client_results = clients.read();
    let mut choices = match &*client_results {
        Some(Ok(results)) => results.clients.clone(),
        _ => catalog.clients.clone(),
    };
    if let Some(client) = catalog
        .clients
        .iter()
        .find(|client| Some(client.id) == selected)
        && !choices.iter().any(|choice| choice.id == client.id)
    {
        choices.insert(0, client.clone());
    }
    let selected_client = choices.iter().find(|client| Some(client.id) == selected);
    let inherited_currency = selected_client
        .map(|client| format!("Client default ({})", client.currency))
        .unwrap_or_else(|| "Client default (select a client)".into());
    let unavailable_client =
        selected.is_some() && selected_client.is_none_or(|client| !client.active);
    let currency_options = std::iter::once((String::new(), inherited_currency))
        .chain(
            PROJECT_CURRENCIES
                .iter()
                .map(|currency| (currency.to_string(), currency.to_string())),
        )
        .collect();
    let missing_client =
        selected.is_some() && !choices.iter().any(|client| Some(client.id) == selected);
    let previous_code = catalog.previous_code.clone();
    let suggested_code = catalog.suggested_code.clone();
    let can_edit_private = catalog.can_edit_private_settings;
    drop(catalog);

    rsx! {
        FormRow { label: "Client", id: "np-client", hint: "Who this project is for",
            div { class: "flex flex-col gap-3",
                label { class: "text-xs text-subtle", r#for: "np-client-search", "Search clients" }
                Input { id: "np-client-search", value: client_query(), placeholder: "Search by client name", oninput: move |event: FormEvent| client_query.set(event.value()) }
                div { class: "flex flex-wrap items-center gap-3",
                    div { class: "flex-1 min-w-0",
                        select { class: "form-select", id: "np-client",
                            onchange: move |event| {
                                let id = Uuid::parse_str(&event.value()).ok();
                                form.write().client_id = id;
                                if let Some(Ok(results)) = &*clients.read()
                                    && let Some(client) = results.clients.iter().find(|client| Some(client.id) == id)
                                    && !options.peek().clients.iter().any(|existing| existing.id == client.id)
                                {
                                    options.write().clients.push(client.clone());
                                }
                            },
                            option { value: "", selected: selected.is_none(), "Select a client…" }
                            if missing_client {
                                option { value: selected.map(|id| id.to_string()).unwrap_or_default(), selected: true, disabled: true, "Previously selected client — search to verify" }
                            }
                            for client in choices {
                                option { key: "{client.id}", value: "{client.id}", selected: Some(client.id) == selected, disabled: !client.active,
                                    "{client.name}"
                                    if !client.active { " (archived)" }
                                }
                            }
                        }
                    }
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| client_open.set(true), "+ New client" }
                }
                match &*client_results {
                    Some(Err(error)) => rsx! { p { class: "text-sm text-danger", role: "alert", "Could not search clients: {error}" } },
                    Some(Ok(results)) if results.more_clients => rsx! { p { class: "form-hint", "Showing the first 50 matches. Refine your search to find another client." } },
                    _ => rsx! {},
                }
                if unavailable_client {
                    p { class: "text-sm text-warning", "This client is archived or unavailable. Choose an active client before saving the project." }
                }
            }
        }
        FormRow { label: "Project name", id: "np-name", hint: "Required",
            Input { id: "np-name", value: form.read().name.clone(), placeholder: "Project name", oninput: move |event: FormEvent| form.write().name = event.value() }
        }
        FormRow { label: "Project code", id: "np-code", hint: "Optional",
            div { class: "max-w-sm",
                Input { id: "np-code", value: form.read().code.clone(), placeholder: "Project code", oninput: move |event: FormEvent| form.write().code = event.value() }
            }
            if let Some(code) = previous_code {
                p { class: "form-hint", "Previous project code: " span { class: "font-mono", "{code}" } }
            }
            if let Some(code) = suggested_code {
                button { r#type: "button", class: "btn btn-ghost btn-sm", onclick: move |_| form.write().code = code.clone(), "Use {code}" }
            }
        }
        FormRow { label: "Dates", hint: "Optional · planning only",
            div { class: "grid md:grid-cols-2 gap-4",
                FormGroup { label: "Start date", id: "np-start",
                    Input { id: "np-start", kind: "date", value: form.read().starts_on.clone(), oninput: move |event: FormEvent| form.write().starts_on = event.value() }
                }
                FormGroup { label: "End date", id: "np-end",
                    Input { id: "np-end", kind: "date", value: form.read().ends_on.clone(), oninput: move |event: FormEvent| form.write().ends_on = event.value() }
                }
            }
            p { class: "form-hint", "Dates are advisory and do not prevent time tracking." }
        }
        FormRow { label: "Tags", id: "np-tags", hint: "Optional · for filtering",
            div { class: "flex flex-wrap gap-2 mb-2",
                for tag in form.read().tags.clone() {
                    span { key: "{tag}", class: "badge badge-neutral inline-flex items-center gap-2",
                        "{tag}"
                        button { r#type: "button", class: "btn btn-ghost btn-sm", aria_label: "Remove tag {tag}", onclick: move |_| form.write().tags.retain(|existing| existing != &tag), "×" }
                    }
                }
            }
            input { id: "np-tags", class: "form-input", value: tag_input(), placeholder: "Type a tag and press Enter",
                oninput: move |event| tag_input.set(event.value()),
                onkeydown: move |event| {
                    if event.key() == Key::Enter || event.key() == Key::Character(",".into()) {
                        event.prevent_default();
                        let raw = tag_input();
                        let mut state = form.write();
                        for tag in raw.split(',').map(str::trim).filter(|tag| !tag.is_empty()) {
                            if state.tags.len() < 50 && !state.tags.iter().any(|existing| existing.to_lowercase() == tag.to_lowercase()) {
                                state.tags.push(tag.to_string());
                            }
                        }
                        tag_input.set(String::new());
                    } else if event.key() == Key::Backspace && tag_input.peek().is_empty() {
                        form.write().tags.pop();
                    }
                },
            }
            p { class: "form-hint", "Enter or comma adds a tag. Backspace on an empty field removes the last tag." }
        }
        FormRow { label: "Currency", id: "np-currency", hint: "For billing and project rates",
            div { class: "max-w-sm",
                Select { id: "np-currency", options: currency_options, selected: form.read().currency.clone().unwrap_or_default(), onchange: move |event: FormEvent| {
                    let value = event.value();
                    form.write().currency = (!value.is_empty()).then_some(value);
                } }
            }
        }
        if can_edit_private {
            FormRow { label: "Notes", id: "np-notes", hint: "Optional · admins only",
                Textarea { id: "np-notes", rows: 3, value: form.read().admin_notes.clone(), placeholder: "Private context for administrators", oninput: move |event: FormEvent| form.write().admin_notes = event.value() }
            }
        }
        NewClientDialog { open: client_open(), currency: options.read().organization_currency.clone(), on_dismiss: move |_| client_open.set(false),
            on_created: move |client: CreationClient| {
                form.write().client_id = Some(client.id);
                options.write().clients.push(client);
                client_open.set(false);
            }
        }
    }
}

#[component]
fn NewClientDialog(
    open: bool,
    currency: String,
    on_dismiss: EventHandler<()>,
    on_created: EventHandler<CreationClient>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut selected_currency = use_signal(|| currency.clone());
    let mut rate = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    use_effect(use_reactive!(|open| {
        if open {
            name.set(String::new());
            rate.set(String::new());
            error.set(None);
        }
    }));
    rsx! {
        Modal { id: "np-client-dialog", labelledby: "np-client-title", open, busy: busy(), on_dismiss,
            div { class: "modal-body",
                h2 { id: "np-client-title", class: "modal-title", "New client" }
                if let Some(message) = error() { div { class: "alert alert-danger", role: "alert", "{message}" } }
                form { onsubmit: move |event| {
                    event.prevent_default();
                    if busy() { return; }
                    busy.set(true);
                    error.set(None);
                    spawn(async move {
                        let result = server_fns::create_project_client(name(), selected_currency(), rate()).await;
                        busy.set(false);
                        match result {
                            Ok(client) => on_created.call(client),
                            Err(problem) => error.set(Some(problem.to_string())),
                        }
                    });
                },
                    FormGroup { label: "Client name", id: "np-new-client-name",
                        Input { id: "np-new-client-name", value: name(), disabled: busy(), oninput: move |event: FormEvent| name.set(event.value()) }
                    }
                    FormGroup { label: "Currency", id: "np-new-client-currency",
                        Select { id: "np-new-client-currency", disabled: busy(), options: PROJECT_CURRENCIES.iter().map(|currency| (currency.to_string(), currency.to_string())).collect(), selected: selected_currency(), onchange: move |event: FormEvent| selected_currency.set(event.value()) }
                    }
                    FormGroup { label: "Default hourly rate", id: "np-new-client-rate", hint: "Optional. Zero is a valid rate.",
                        Input { id: "np-new-client-rate", value: rate(), disabled: busy(), oninput: move |event: FormEvent| rate.set(event.value()) }
                    }
                    p { class: "text-sm text-subtle", "Creates a client immediately, even if you later discard this project draft." }
                    div { class: "modal-actions",
                        button { class: "btn btn-primary", r#type: "submit", disabled: busy() || name.read().trim().is_empty(), if busy() { "Creating…" } else { "Create client" } }
                        button { class: "btn btn-secondary", r#type: "button", disabled: busy(), onclick: move |_| on_dismiss.call(()), "Cancel" }
                    }
                }
            }
        }
    }
}
