use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::modal::Modal;
use crate::components::table::DataTable;
use crate::models::Project;
use crate::models::invoice::{
    InvoiceDefaults, InvoiceFeeSelection, InvoiceGenerationRequest, InvoicePreparation,
};
use crate::models::project_creation::InvoiceDefaultsInput;
use crate::server_fns;

use super::defaults_form::{DefaultsFields, fields_from, parse_fields};

#[path = "preparation/fees.rs"]
mod fees;

#[derive(Clone, PartialEq, Eq)]
struct InvoiceRequest {
    client: String,
    from: String,
    to: String,
    projects: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
    fees: Option<Vec<InvoiceFeeSelection>>,
}

impl InvoiceRequest {
    fn same_sources(&self, other: &Self) -> bool {
        self.client == other.client
            && self.from == other.from
            && self.to == other.to
            && self.projects == other.projects
    }
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
    let mut generation = use_signal(|| None::<(InvoiceRequest, InvoiceGenerationRequest)>);
    let mut uncertain = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut fees = use_signal(Vec::<fees::FeeInput>::new);
    let mut loaded = use_signal(|| None::<InvoiceRequest>);
    let mut confirm = use_signal(|| false);

    let source_request = InvoiceRequest {
        client: client(),
        from: from(),
        to: to(),
        projects: selected(),
        overrides: None,
        fees: None,
    };
    let same_sources = loaded
        .read()
        .as_ref()
        .is_some_and(|last| last.same_sources(&source_request));
    let current = if started() {
        parse_fields(&fields.read()).map(Some)
    } else {
        Ok(None)
    }
    .and_then(|overrides| {
        let selected_fees = if same_sources {
            Some(fees::parse(&fees.read())?)
        } else {
            None
        };
        Ok(InvoiceRequest {
            overrides,
            fees: selected_fees,
            ..source_request
        })
    });
    let ready = is_reviewed(current.as_ref().ok(), reviewed.read().as_ref());
    let review_request = current.clone();
    let has_selection = preview
        .read()
        .as_ref()
        .is_some_and(|review| review.lines.iter().any(|line| line.selected));
    let has_excess = ready
        && preview
            .read()
            .as_ref()
            .is_some_and(|review| !review.excess.is_empty());
    let generate = EventHandler::new(move |confirm_excess: bool| {
        if (busy() && !uncertain())
            || !is_reviewed(current.as_ref().ok(), reviewed.peek().as_ref())
            || !has_selection
        {
            return;
        }
        let Ok(request) = current.clone() else {
            return;
        };
        let mutation = if uncertain() {
            generation
                .peek()
                .as_ref()
                .filter(|(previous, _)| previous == &request)
                .map(|(_, mutation)| mutation.clone())
        } else {
            preview.peek().clone().map(|review| {
                let confirmed_excess = if confirm_excess {
                    review.excess.clone()
                } else {
                    Vec::new()
                };
                InvoiceGenerationRequest {
                    request_id: Uuid::now_v7(),
                    review,
                    confirmed_excess,
                }
            })
        };
        let Some(mutation) = mutation else {
            return;
        };
        generation.set(Some((request.clone(), mutation.clone())));
        uncertain.set(false);
        confirm.set(false);
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result = server_fns::generate_invoice(
                request.client,
                request.from,
                request.to,
                request.projects,
                request.overrides,
                mutation,
            )
            .await;
            busy.set(false);
            match result {
                Ok(data) => oncreated.call(data.invoice.id),
                Err(err) => {
                    if matches!(&err, ServerFnError::ServerError { code, .. } if (400..500).contains(code))
                    {
                        error.set(Some(err.to_string()));
                        reviewed.set(None);
                    } else {
                        error.set(Some("The invoice may have been saved. Retry the same request to recover it without creating another draft.".into()));
                        uncertain.set(true);
                        busy.set(true);
                    }
                }
            }
        });
    });

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
                            fees.set(Vec::new());
                            loaded.set(None);
                            confirm.set(false);
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
            if same_sources && !fees.read().is_empty() {
                fees::FeeFields { fees, disabled: busy() }
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
                        generation.set(None);
                        confirm.set(false);
                        busy.set(true);
                        spawn(async move {
                            let result = async {
                                projects.set(server_fns::list_projects(Some(request.client.clone()), true).await?);
                                server_fns::prepare_invoice(request.client.clone(), request.from.clone(), request.to.clone(), request.projects.clone(), request.overrides.clone(), request.fees.clone()).await
                            }.await;
                            match result {
                                Ok(estimate) => {
                                    fees.set(fees::from_review(&estimate));
                                    loaded.set(Some(request.clone()));
                                    if let Some(defaults) = &estimate.defaults {
                                        fields.set(fields_from(defaults));
                                        reviewed.set(Some(InvoiceRequest { overrides: Some(defaults.clone()), fees: Some(estimate.fee_selection()), ..request }));
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
                button { r#type: "button", class: "btn btn-primary", disabled: (busy() && !uncertain()) || !ready || !has_selection,
                    onclick: move |_| { if uncertain() { generate.call(false); } else if has_excess { confirm.set(true); } else { generate.call(false); } },
                    if uncertain() { "Retry generation" } else { "Generate draft" }
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
        Modal { id: "generation-excess", labelledby: "generation-excess-title", open: confirm() && ready, busy: busy(), on_dismiss: move |_| confirm.set(false),
            h2 { id: "generation-excess-title", "Confirm over-invoicing" }
            p { class: "text-sm mb-4", "Generating this draft will exceed the agreed fees by the amounts below, before tax. Cancelling creates nothing." }
            if let Some(review) = preview.read().as_ref().filter(|_| ready) {
                for excess in &review.excess {
                    if let Some(line) = review.lines.iter().find(|line| line.source == excess.source) {
                        p { key: "{excess.source:?}", class: "wrap-anywhere", "{line.description} · Excess: {review.currency} " {format_cents_plain(excess.excess_cents)} }
                    }
                }
            }
            div { class: "flex flex-wrap gap-3 mt-4",
                button { r#type: "button", class: "btn btn-danger", disabled: busy() || !ready, onclick: move |_| generate.call(true), "Confirm excess and generate" }
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| confirm.set(false), "Cancel over-invoicing" }
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
                    for (index, line) in estimate.lines.iter().enumerate().filter(|(_, line)| line.selected) {
                        tr { key: "{index}",
                            td {
                                "{line.description}"
                                if let Some(balance) = line.fee_balance {
                                    div { class: "text-sm text-muted mt-2 wrap-anywhere",
                                        p { "Agreed: {estimate.currency} " {format_cents_plain(balance.agreed_cents)} }
                                        p { "Invoiced (including drafts): {estimate.currency} " {format_cents_plain(balance.invoiced_cents)} }
                                        p {
                                            if balance.remaining_cents < 0 { "Over-invoiced: " } else { "Available: " }
                                            "{estimate.currency} " {format_cents_plain(balance.remaining_cents)}
                                        }
                                        if let Some(net) = line.net_before_tax_cents {
                                            p { "Proposed net before tax: {estimate.currency} " {format_cents_plain(net)} }
                                        }
                                    }
                                }
                            }
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
            fees: None,
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
                fees: Some(Vec::new()),
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
