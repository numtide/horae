use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::table::DataTable;
use crate::models::Project;
use crate::models::invoice::{InvoiceDefaults, InvoicePreparation};
use crate::models::project_creation::InvoiceDefaultsInput;
use crate::server_fns;

use super::defaults_form::{DefaultsFields, fields_from, parse_fields};

#[derive(Clone, PartialEq, Eq)]
struct InvoiceRequest {
    client: String,
    from: String,
    to: String,
    projects: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
}

// A preview is usable only for the exact values that were reviewed. Invalid
// edits have no request and can never fall back to a previous valid preview.
fn is_reviewed(current: Option<&InvoiceRequest>, reviewed: Option<&InvoiceRequest>) -> bool {
    current
        .zip(reviewed)
        .is_some_and(|(current, reviewed)| current == reviewed)
}

#[component]
pub(super) fn PrepareInvoice(
    client_opts: Vec<(String, String)>,
    mut busy: Signal<bool>,
    oncreated: EventHandler<Uuid>,
) -> Element {
    let mut client = use_signal(String::new);
    let mut from = use_signal(String::new);
    let mut to = use_signal(String::new);
    let mut projects = use_signal(Vec::<Project>::new);
    let mut selected = use_signal(|| None::<Vec<String>>);
    let mut fields = use_signal(InvoiceDefaultsInput::default);
    let mut started = use_signal(|| false);
    let mut preview = use_signal(|| None::<InvoicePreparation>);
    let mut reviewed = use_signal(|| None::<InvoiceRequest>);
    let mut error = use_signal(|| None::<String>);

    let current = if started() {
        parse_fields(&fields.read()).map(Some)
    } else {
        Ok(None)
    }
    .map(|overrides| InvoiceRequest {
        client: client(),
        from: from(),
        to: to(),
        projects: selected(),
        overrides,
    });
    let ready = is_reviewed(current.as_ref().ok(), reviewed.read().as_ref());
    let review_request = current.clone();

    rsx! {
        FormCard { title: "Prepare invoice", error,
            p { class: "text-muted text-sm",
                "Review unbilled time and monthly fees in this period, plus overdue single fees and milestones. The estimate reserves nothing; generation rechecks available charges. Nothing is sent automatically."
            }
            fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: busy(), aria_label: "Invoice sources",
                FormGroup { label: "Client", id: "inv-client",
                    Select { id: "inv-client", options: client_opts, selected: client(),
                        onchange: move |event: FormEvent| {
                            client.set(event.value());
                            projects.set(Vec::new());
                            selected.set(None);
                            fields.set(InvoiceDefaultsInput::default());
                            started.set(false);
                            preview.set(None);
                            reviewed.set(None);
                            error.set(None);
                        }
                    }
                }
                div { class: "grid md:grid-cols-2 gap-3",
                    FormGroup { label: "Period from", id: "inv-from",
                        Input { id: "inv-from", kind: "date", value: from(), oninput: move |event: FormEvent| from.set(event.value()) }
                    }
                    FormGroup { label: "Period to", id: "inv-to",
                        Input { id: "inv-to", kind: "date", value: to(), oninput: move |event: FormEvent| to.set(event.value()) }
                    }
                }
                if !projects.read().is_empty() {
                    div { class: "mb-4",
                        h4 { class: "text-sm font-semibold mb-2", "Projects" }
                        Checkbox { label: "All projects for this client", checked: selected.read().is_none(), disabled: busy(),
                            onclick: move |_| {
                                let all = selected.peek().is_none();
                                selected.set(if all { Some(Vec::new()) } else { None });
                            }
                        }
                        if selected.read().is_some() {
                            div { class: "flex flex-col gap-2 mt-2",
                                for project in projects.read().iter() {
                                    {
                                        let id = project.id.to_string();
                                        let checked = selected.read().as_ref().is_some_and(|ids| ids.contains(&id));
                                        rsx! {
                                            Checkbox { key: "{id}", label: if project.active { project.name.clone() } else { format!("{} (archived)", project.name) }, checked, disabled: busy(),
                                                onclick: move |_| {
                                                    if let Some(ids) = &mut *selected.write() {
                                                        if ids.contains(&id) { ids.retain(|value| value != &id); }
                                                        else { ids.push(id.clone()); ids.sort(); }
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

            if let Some(estimate) = preview.read().as_ref().filter(|estimate| estimate.defaults.is_none()) {
                div { class: "alert alert-warning", role: "status",
                    "These projects have different invoice defaults. Choose project values below or enter explicit values, then review again."
                }
                div { class: "flex flex-wrap gap-3 mb-4",
                    for project in &estimate.projects {
                        {
                            let defaults = project.defaults.clone();
                            rsx! {
                                button { key: "{project.project_id}", r#type: "button", class: "btn btn-secondary btn-sm", disabled: busy(),
                                    onclick: move |_| fields.set(fields_from(&defaults)),
                                    "Use defaults from {project.name}"
                                }
                            }
                        }
                    }
                }
            }
            if started() {
                h4 { class: "text-sm font-semibold mb-4", "Invoice values" }
                DefaultsFields { fields, disabled: busy() }
            }
            div { class: "flex flex-wrap gap-3 mb-4",
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(),
                    onclick: move |_| {
                        if busy() { return; }
                        let request = match review_request.clone() {
                            Ok(request) => request,
                            Err(message) => { error.set(Some(message)); return; }
                        };
                        error.set(None);
                        reviewed.set(None);
                        preview.set(None);
                        busy.set(true);
                        spawn(async move {
                            let result = async {
                                projects.set(server_fns::list_projects(Some(request.client.clone()), true).await?);
                                server_fns::prepare_invoice(request.client.clone(), request.from.clone(), request.to.clone(), request.projects.clone(), request.overrides.clone()).await
                            }.await;
                            match result {
                                Ok(estimate) => {
                                    if let Some(defaults) = &estimate.defaults {
                                        fields.set(fields_from(defaults));
                                        reviewed.set(Some(InvoiceRequest { overrides: Some(defaults.clone()), ..request }));
                                    } else {
                                        fields.set(InvoiceDefaultsInput { terms_days: String::new(), ..Default::default() });
                                    }
                                    started.set(true);
                                    preview.set(Some(estimate));
                                }
                                Err(err) => error.set(Some(err.to_string())),
                            }
                            busy.set(false);
                        });
                    },
                    if busy() { "Working…" } else { "Review invoice" }
                }
                button { r#type: "button", class: "btn btn-primary", disabled: busy() || !ready,
                    onclick: move |_| {
                        if busy() || !is_reviewed(current.as_ref().ok(), reviewed.peek().as_ref()) { return; }
                        let Ok(request) = current.clone() else { return; };
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            let result = server_fns::generate_invoice(request.client, request.from, request.to, request.projects, request.overrides).await;
                            busy.set(false);
                            match result {
                                Ok(data) => oncreated.call(data.invoice.id),
                                Err(err) => {
                                    error.set(Some(err.to_string()));
                                    reviewed.set(None);
                                }
                            }
                        });
                    }, "Generate draft"
                }
            }
            if started() && !ready && !busy() {
                p { class: "text-sm text-muted", role: "status", "Review invoice to refresh the totals before generating a draft." }
            }
            if ready {
                if let Some(estimate) = preview.read().as_ref() {
                    PreparedLines { estimate: estimate.clone() }
                }
            }
        }
    }
}

#[component]
fn PreparedLines(estimate: InvoicePreparation) -> Element {
    rsx! {
        p { class: "text-sm text-muted", "Invoice date: {estimate.issued_on} · Due: " {estimate.due_on.map(|date| date.to_string()).unwrap_or_default()} }
        DataTable {
            table {
                caption { class: "text-sm text-muted text-left", "Estimated invoice charges ({estimate.currency})" }
                thead { tr { th { "Description" } th { class: "text-right", "Hours" } th { class: "text-right", "Rate" } th { class: "text-right", "Amount" } } }
                tbody {
                    for (index, line) in estimate.lines.iter().enumerate() {
                        tr { key: "{index}",
                            td { "{line.description}" }
                            td { class: "text-mono text-right", {line.minutes.map(|minutes| horae_core::duration::format_hhmm(minutes.into())).unwrap_or_else(|| "—".into())} }
                            td { class: "text-mono text-right", {line.rate_cents.map(|rate| format!("{}/hr", format_cents_plain(rate))).unwrap_or_else(|| "—".into())} }
                            td { class: "text-mono text-right", {format_cents_plain(line.amount_cents)} }
                        }
                    }
                }
                tfoot {
                    for (label, amount) in estimate.breakdown().unwrap_or_default() {
                        tr {
                            th { scope: "row", colspan: "3", class: "text-right wrap-anywhere", "{label}" }
                            td { class: "text-mono text-right", "{estimate.currency} " {format_cents_plain(amount)} }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_review_requires_matching_sources_and_valid_values() {
        let request = InvoiceRequest {
            client: "client".into(),
            from: "2026-01-01".into(),
            to: "2026-01-31".into(),
            projects: None,
            overrides: Some(InvoiceDefaults::default()),
        };
        assert!(is_reviewed(Some(&request), Some(&request)));
        assert!(!is_reviewed(None, Some(&request)));
        assert!(!is_reviewed(None, None));
        assert!(!is_reviewed(Some(&request), None));
        let changes = [
            InvoiceRequest {
                client: "different".into(),
                ..request.clone()
            },
            InvoiceRequest {
                from: "2026-01-02".into(),
                ..request.clone()
            },
            InvoiceRequest {
                to: "2026-02-01".into(),
                ..request.clone()
            },
            InvoiceRequest {
                projects: Some(vec!["project".into()]),
                ..request.clone()
            },
            InvoiceRequest {
                overrides: None,
                ..request.clone()
            },
            InvoiceRequest {
                overrides: Some(InvoiceDefaults {
                    tax1_bps: 2100,
                    ..Default::default()
                }),
                ..request.clone()
            },
        ];
        for changed in changes {
            assert!(!is_reviewed(Some(&changed), Some(&request)));
        }
    }
}
