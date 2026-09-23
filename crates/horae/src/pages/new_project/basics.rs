use dioxus::prelude::*;
use horae_core::project::PROJECT_CURRENCIES;
use uuid::Uuid;

use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::modal::Modal;
use crate::components::select_field::SelectField;
use crate::models::project_creation::{
    CreationClient, CreationOptions, CreationSearch, ProjectForm, ProjectFormField,
};
use crate::server_fns;

use super::FormRow;
use super::date_field::DateField;

#[component]
pub(super) fn Basics(
    mut form: Signal<ProjectForm>,
    mut options: Signal<CreationOptions>,
    #[props(default)] editing: bool,
    #[props(default)] invalid_field: Option<ProjectFormField>,
    #[props(default)] error_message: Option<String>,
) -> Element {
    let error_id =
        |field| (invalid_field == Some(field)).then(|| "np-basic-field-error".to_owned());
    let error_for = |fields: &[ProjectFormField]| {
        invalid_field
            .filter(|field| fields.contains(field))
            .and(error_message.clone())
    };
    let client_query = use_signal(String::new);
    let mut client_open = use_signal(|| false);
    let mut tag_input = use_signal(String::new);
    let mut tag_error = use_signal(|| None::<&'static str>);
    let clients = use_resource(move || {
        let query = client_query();
        async move {
            let mut search = CreationSearch::default();
            search.clients.query = query;
            server_fns::project_creation_options(search).await
        }
    });
    let clients_pending = clients.state()() != UseResourceState::Ready;
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
    let currency_options = if editing {
        form.read()
            .currency
            .iter()
            .map(|currency| (currency.clone(), currency.clone()))
            .collect()
    } else {
        std::iter::once((String::new(), inherited_currency))
            .chain(
                PROJECT_CURRENCIES
                    .iter()
                    .map(|currency| (currency.to_string(), currency.to_string())),
            )
            .collect()
    };
    let missing_client =
        selected.is_some() && !choices.iter().any(|client| Some(client.id) == selected);
    let previous_code = catalog.previous_code.clone();
    let suggested_code = catalog.suggested_code.clone();
    let can_edit_private = catalog.can_edit_private_settings;
    let organization_currency = catalog.organization_currency.clone();
    drop(catalog);
    let mut client_choices = vec![(String::new(), "Select a client…".into())];
    let mut disabled_clients = Vec::new();
    if missing_client {
        let id = selected.map(|id| id.to_string()).unwrap_or_default();
        client_choices.push((
            id.clone(),
            "Previously selected client — search to verify".into(),
        ));
        disabled_clients.push(id);
    }
    for client in choices {
        let label = if client.active {
            client.name
        } else {
            format!("{} (archived)", client.name)
        };
        if !client.active {
            disabled_clients.push(client.id.to_string());
        }
        client_choices.push((client.id.to_string(), label));
    }

    rsx! {
        FormRow { label: "Client", id: "np-client", hint: "Who this project is for",
            div { class: "flex flex-col gap-3",
                div { class: "flex flex-wrap items-center gap-3",
                    div { class: "flex-1 basis-form-select min-w-0",
                        SelectField { id: "np-client", label: "Client", options: client_choices,
                            error_id: error_id(ProjectFormField::Client),
                            selected: selected.map(|id| id.to_string()).unwrap_or_default(),
                            disabled_values: disabled_clients, query: Some(client_query),
                            pending: clients_pending,
                            onselect: move |value: String| {
                                let id = Uuid::parse_str(&value).ok();
                                form.write().client_id = id;
                                if let Some(Ok(results)) = &*clients.read()
                                    && let Some(client) = results.clients.iter().find(|client| Some(client.id) == id)
                                    && !options.peek().clients.iter().any(|existing| existing.id == client.id)
                                {
                                    options.write().clients.push(client.clone());
                                }
                            },
                            if !clients_pending {
                                match &*client_results {
                                    Some(Err(error)) => rsx! { p { class: "text-sm text-danger p-2", role: "alert", "Could not search clients: {error}" } },
                                    Some(Ok(results)) if results.clients.is_empty() => rsx! { p { class: "text-sm text-subtle p-2 m-0", role: "status", "No matching clients." } },
                                    Some(Ok(results)) if results.more_clients => rsx! { p { class: "form-hint p-2", "Showing the first 50 matches. Refine your search to find another client." } },
                                    _ => rsx! {},
                                }
                            }
                        }
                    }
                    span { class: "text-sm text-label", "or" }
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| client_open.set(true), "+ New client" }
                }
                if unavailable_client {
                    p { class: "text-sm text-warning", if editing { "This client is archived. You can keep the current client or select an active client." } else { "This client is archived or unavailable. Choose an active client before saving the project." } }
                }
                if let Some(message) = error_for(&[ProjectFormField::Client]) {
                    p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
                }
            }
        }
        FormRow { label: "Project name", id: "np-name", hint: "Required",
            Input { id: "np-name", error_id: error_id(ProjectFormField::Name), value: form.read().name.clone(), placeholder: "Project name", oninput: move |event: FormEvent| form.write().name = event.value() }
            if let Some(message) = error_for(&[ProjectFormField::Name]) {
                p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
            }
        }
        FormRow { label: "Project code", id: "np-code", hint: "Optional",
            div { class: "flex flex-wrap items-center gap-3",
                input { id: "np-code", class: "np-code form-input font-mono", value: form.read().code.clone(), placeholder: "Project code",
                    aria_invalid: error_id(ProjectFormField::Code).map(|_| "true"),
                    aria_describedby: error_id(ProjectFormField::Code),
                    oninput: move |event| form.write().code = event.value() }
                if let Some(code) = previous_code {
                    span { class: "text-sm text-subtle", "Last code: " span { class: "font-mono", "{code}" } }
                }
                if let Some(code) = suggested_code {
                    button { r#type: "button", class: "btn btn-ghost btn-sm font-semibold", onclick: move |_| form.write().code = code.clone(), "Use {code}" }
                }
            }
            p { class: "form-hint", "An optional reference for this project. Numbers or letters, up to 100 characters." }
            if let Some(message) = error_for(&[ProjectFormField::Code]) {
                p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
            }
        }
        FormRow { label: "Dates", hint: "Optional · planning only",
            div { class: "flex flex-wrap items-center gap-3",
                FormGroup { label: "Start date", id: "np-start",
                    DateField { id: "np-start", error_id: error_id(ProjectFormField::StartsOn), label: "Start date", placeholder: "Starts on", value: form.read().starts_on.clone(), onchange: move |value| form.write().starts_on = value }
                }
                FormGroup { label: "End date", id: "np-end",
                    DateField { id: "np-end", error_id: error_id(ProjectFormField::EndsOn), label: "End date", placeholder: "Ends on", value: form.read().ends_on.clone(), onchange: move |value| form.write().ends_on = value }
                }
            }
            p { class: "form-hint", "Dates are advisory and do not prevent time tracking." }
            if let Some(message) = error_for(&[ProjectFormField::StartsOn, ProjectFormField::EndsOn]) {
                p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
            }
        }
        FormRow { label: "Tags", id: "np-tags", hint: "Optional · for filtering",
            div { class: "chip-input chip-input-neutral np-tags py-1 cursor-text",
                onclick: move |_| { document::eval("document.getElementById('np-tags')?.focus();"); },
                for tag in form.read().tags.clone() {
                    span { key: "{tag}", class: "chip min-w-0",
                        span { class: "truncate", title: "{tag}", "{tag}" }
                        button { r#type: "button", class: "chip-input-x", aria_label: "Remove tag {tag}", onclick: move |_| {
                            form.write().tags.retain(|existing| existing != &tag);
                            tag_error.set(None);
                        }, "×" }
                    }
                }
                input { id: "np-tags", class: "chip-input-field", value: tag_input(), placeholder: if form.read().tags.is_empty() { "Add tags…" } else { "" },
                    aria_describedby: if tag_error().is_some() { "np-tags-hint np-tags-error" } else { "np-tags-hint" },
                    aria_invalid: tag_error().is_some(),
                    oninput: move |event| { tag_input.set(event.value()); tag_error.set(None); },
                    onkeydown: move |event| {
                        if event.is_composing() { return; }
                        if event.key() == Key::Enter || event.key() == Key::Character(",".into()) {
                            event.prevent_default();
                            let result = add_tags(&form.read().tags, &tag_input());
                            match result {
                                Ok(tags) => {
                                    form.write().tags = tags;
                                    tag_input.set(String::new());
                                    tag_error.set(None);
                                }
                                Err(message) => tag_error.set(Some(message)),
                            }
                        } else if event.key() == Key::Backspace && tag_input.peek().is_empty() {
                            form.write().tags.pop();
                            tag_error.set(None);
                        }
                    },
                }
            }
            p { id: "np-tags-hint", class: "form-hint", "Enter or comma adds a tag. Backspace on an empty field removes the last tag. Removing a tag here does not delete it from other projects." }
            if let Some(message) = tag_error() { p { id: "np-tags-error", class: "text-sm text-danger", role: "alert", "{message}" } }
        }
        FormRow { label: "Currency", id: "np-currency", hint: "For billing and project rates",
            div { class: "w-form-select max-w-full",
                fieldset { class: "border-0 p-0 m-0", disabled: editing,
                SelectField { id: "np-currency", error_id: error_id(ProjectFormField::Currency), label: "Currency", options: currency_options, selected: form.read().currency.clone().unwrap_or_default(), onselect: move |value: String| {
                    form.write().currency = (!value.is_empty()).then_some(value);
                } } }
            }
            p { class: "form-hint", "Sets the currency for rates, fees and budgets. Costs use the workspace currency ({organization_currency})." }
            if editing { p { class: "form-hint", "Changing an existing project's currency requires a data migration." } }
            if let Some(message) = error_for(&[ProjectFormField::Currency]) {
                p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
            }
        }
        if can_edit_private {
            FormRow { label: "Notes", id: "np-notes", hint: "Optional · admins only",
                Textarea { id: "np-notes", error_id: error_id(ProjectFormField::AdminNotes), rows: 3, value: form.read().admin_notes.clone(), placeholder: "Private context for administrators", oninput: move |event: FormEvent| form.write().admin_notes = event.value() }
                if let Some(message) = error_for(&[ProjectFormField::AdminNotes]) {
                    p { id: "np-basic-field-error", class: "text-sm text-danger", "{message}" }
                }
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

fn add_tags(current: &[String], input: &str) -> Result<Vec<String>, &'static str> {
    let mut tags = current.to_vec();
    for tag in input
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if tag.chars().count() > 50 || tag.contains('\0') {
            return Err("Each tag must contain at most 50 characters and no null character.");
        }
        let key = tag.to_lowercase();
        if tags
            .iter()
            .any(|existing| existing.trim().to_lowercase() == key)
        {
            continue;
        }
        if tags.len() >= 50 {
            return Err("A project can have at most 50 tags. Remove a tag before adding another.");
        }
        tags.push(tag.to_owned());
    }
    Ok(tags)
}

#[cfg(test)]
mod tests {
    use super::add_tags;

    #[test]
    fn tags_trim_and_deduplicate_without_changing_existing_spelling() {
        assert_eq!(
            add_tags(&["Équipe".into()], " équipe , Q3, q3, , platform ").unwrap(),
            ["Équipe", "Q3", "platform"]
        );
    }

    #[test]
    fn a_full_tag_list_still_accepts_a_duplicate() {
        let current: Vec<_> = (0..50).map(|index| format!("tag-{index}")).collect();
        assert_eq!(add_tags(&current, " TAG-0 ").unwrap(), current);
    }

    #[test]
    fn a_batch_that_would_exceed_the_tag_limit_is_rejected() {
        let current: Vec<_> = (0..49).map(|index| format!("tag-{index}")).collect();
        assert_eq!(
            add_tags(&current, "last, overflow"),
            Err("A project can have at most 50 tags. Remove a tag before adding another.")
        );
    }

    #[test]
    fn tag_length_counts_characters_instead_of_bytes() {
        let tag = "é".repeat(50);
        assert_eq!(add_tags(&[], &tag).unwrap(), [tag]);
    }

    #[test]
    fn overlong_tags_are_rejected_without_truncation() {
        assert_eq!(
            add_tags(&[], &"é".repeat(51)),
            Err("Each tag must contain at most 50 characters and no null character.")
        );
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
