use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};
use uuid::Uuid;

use super::defaults_form::{DefaultsFields, fields_from, parse_fields};
use crate::components::form::{FormCard, FormGroup, Input};
use crate::components::modal::Modal;
use crate::models::invoice::*;
use crate::server_fns;

#[component]
pub(super) fn DraftEditor(
    id: Uuid,
    mut editing: Signal<bool>,
    busy: Signal<bool>,
    onsaved: EventHandler<()>,
) -> Element {
    let mut editor =
        use_resource(move || async move { server_fns::get_invoice_editor(id.to_string()).await });
    rsx! {
        match &*editor.read() {
            Some(Ok(snapshot)) => rsx! { DraftEditForm { id, snapshot: snapshot.clone(), editing, busy, onsaved } },
            state => rsx! {
                div { class: "card p-5 mb-6", role: "status",
                    if let Some(Err(error)) = state {
                        p { "{error}" }
                        button { class: "btn btn-primary", onclick: move |_| editor.restart(), "Retry loading invoice" }
                    } else { p { "Loading invoice values…" } }
                    button { class: "btn btn-secondary", onclick: move |_| editing.set(false), "Cancel changes" }
                }
            },
        }
    }
}

#[derive(Clone)]
struct FeeInput {
    id: Uuid,
    description: String,
    amount: String,
}

fn parse_edit(
    revision: i64,
    fields: &crate::models::project_creation::InvoiceDefaultsInput,
    fees: &[FeeInput],
) -> Result<InvoiceDraftEdit, String> {
    Ok(InvoiceDraftEdit {
        revision,
        defaults: parse_fields(fields)?,
        fees: fees
            .iter()
            .map(|fee| {
                if fee.description.trim().is_empty()
                    || fee.description.chars().count() > 1000
                    || fee.description.contains('\0')
                {
                    return Err(
                        "Fee descriptions must contain 1–1000 characters without NUL.".into(),
                    );
                }
                let amount_cents = parse_cents(&fee.amount).map_err(|_| {
                    "Enter an exact fee amount with at most two decimal places.".to_owned()
                })?;
                if amount_cents < 0 {
                    return Err("Fee amounts cannot be negative.".into());
                }
                Ok(InvoiceFeeEdit {
                    line_id: fee.id,
                    description: fee.description.clone(),
                    amount_cents,
                })
            })
            .collect::<Result<_, String>>()?,
    })
}

#[component]
fn DraftEditForm(
    id: Uuid,
    snapshot: InvoiceEditor,
    mut editing: Signal<bool>,
    mut busy: Signal<bool>,
    onsaved: EventHandler<()>,
) -> Element {
    let fields = use_signal(|| fields_from(&snapshot.edit.defaults));
    let mut fees = use_signal(|| {
        snapshot
            .edit
            .fees
            .iter()
            .map(|fee| FeeInput {
                id: fee.line_id,
                description: fee.description.clone(),
                amount: format_cents_plain(fee.amount_cents),
            })
            .collect::<Vec<_>>()
    });
    let mut reviewed = use_signal(|| None::<(InvoiceDraftEdit, InvoiceEditReview)>);
    let mut request = use_signal(|| None::<InvoiceDraftSave>);
    let mut uncertain = use_signal(|| false);
    let mut confirm = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let current = parse_edit(snapshot.edit.revision, &fields.read(), &fees.read());
    let ready = reviewed
        .read()
        .as_ref()
        .filter(|(edit, _)| current.as_ref() == Ok(edit))
        .cloned();
    let review_input = current.clone();
    let save_ready = ready.clone();
    let save = EventHandler::new(move |confirm_excess: bool| {
        if busy() && !uncertain() {
            return;
        }
        let next = if uncertain() {
            request.peek().clone()
        } else {
            save_ready.clone().map(|(edit, review)| {
                let confirmed_excess = if confirm_excess {
                    review
                        .fees
                        .iter()
                        .filter(|fee| fee.excess_cents > 0)
                        .map(|fee| InvoiceExcessConfirmation {
                            line_id: fee.line_id,
                            excess_cents: fee.excess_cents,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                InvoiceDraftSave {
                    request_id: Uuid::now_v7(),
                    edit,
                    review,
                    confirmed_excess,
                }
            })
        };
        let Some(next) = next else {
            return;
        };
        request.set(Some(next.clone()));
        uncertain.set(false);
        confirm.set(false);
        busy.set(true);
        error.set(None);
        spawn(async move {
            match server_fns::save_invoice_draft(id.to_string(), next).await {
                Ok(_) => {
                    busy.set(false);
                    editing.set(false);
                    onsaved.call(());
                }
                Err(err) => {
                    if matches!(&err, ServerFnError::ServerError {code, ..} if (400..500).contains(code))
                    {
                        busy.set(false);
                        reviewed.set(None);
                        error.set(Some(err.to_string()));
                    } else {
                        uncertain.set(true);
                        error.set(Some("The save may have completed. Retry the same changes to recover the result safely.".into()));
                    }
                }
            }
        });
    });
    let has_excess = ready
        .as_ref()
        .is_some_and(|(_, review)| review.fees.iter().any(|fee| fee.excess_cents > 0));
    let currency = snapshot.currency.trim();
    rsx! {
        FormCard { title: "Edit invoice values", error,
            p { class: "text-sm text-muted mb-4", "Only this draft changes. Agreed project fees, time entries and other invoices stay unchanged. Other invoiced amounts include draft reservations; balances exclude tax." }
            DefaultsFields { fields, disabled: busy() }
            fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: busy(), aria_label: "Invoice fee lines",
                for (index, fee) in fees.read().iter().enumerate() {
                    div { key: "{fee.id}", class: "card p-4 mb-4",
                        FormGroup { label: "Fee description", id: "fee-description-{fee.id}",
                            Input { id: "fee-description-{fee.id}", value: fee.description.clone(), oninput: move |event: FormEvent| fees.write()[index].description = event.value() }
                        }
                        FormGroup { label: "Fee amount ({currency})", id: "fee-amount-{fee.id}",
                            Input { id: "fee-amount-{fee.id}", class: "text-mono", value: fee.amount.clone(), oninput: move |event: FormEvent| fees.write()[index].amount = event.value() }
                        }
                        if let Some(balance) = snapshot.review.fees.iter().find(|balance| balance.line_id == fee.id) {
                            p { class: "text-sm text-muted",
                                "Agreed: {currency} " {format_cents_plain(balance.agreed_cents)}
                                " · Other invoiced: {currency} " {format_cents_plain(balance.other_invoiced_cents)}
                            }
                        }
                    }
                }
            }
            if let Some((_, review)) = &ready {
                div { class: "mb-4", role: "status",
                    p { "Reviewed total: {currency} " {format_cents_plain(review.amounts.total_cents)} }
                    for fee in &review.fees {
                        p { key: "{fee.line_id}", class: "text-sm",
                            "{fee.period_key} · Net charge: {currency} " {format_cents_plain(fee.net_cents)}
                            " · Remaining: {currency} " {format_cents_plain(fee.remaining_cents)}
                            if fee.excess_cents > 0 { strong { " · Over-invoiced" } }
                        }
                    }
                }
            }
            div { class: "flex flex-wrap gap-3",
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(),
                    onclick: move |_| {
                        let edit = match &review_input { Ok(edit) => edit.clone(), Err(message) => { error.set(Some(message.clone())); return; } };
                        busy.set(true); error.set(None); reviewed.set(None);
                        spawn(async move {
                            match server_fns::review_invoice_edit(id.to_string(), edit.clone()).await {
                                Ok(review) => reviewed.set(Some((edit, review))),
                                Err(err) => error.set(Some(err.to_string())),
                            }
                            busy.set(false);
                        });
                    }, "Review invoice changes"
                }
                button { r#type: "button", class: "btn btn-primary", disabled: (busy() && !uncertain()) || ready.is_none(),
                    onclick: move |_| { if uncertain() { save.call(false); } else if has_excess { confirm.set(true); } else { save.call(false); } },
                    if uncertain() { "Retry invoice save" } else { "Save invoice values" }
                }
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| editing.set(false), "Cancel changes" }
            }
        }
        Modal { id: "invoice-excess", labelledby: "invoice-excess-title", open: confirm(), busy: busy(), on_dismiss: move |_| confirm.set(false),
            h2 { id: "invoice-excess-title", "Confirm over-invoicing" }
            p { class: "text-sm mb-4", "Saving will exceed the agreed fees by the amounts below. Cancelling changes nothing." }
            if let Some((_, review)) = &ready {
                for fee in review.fees.iter().filter(|fee| fee.excess_cents > 0) {
                    p { key: "{fee.line_id}", "{fee.period_key} · Excess: {currency} " {format_cents_plain(fee.excess_cents)} }
                }
            }
            div { class: "flex flex-wrap gap-3 mt-4",
                button { class: "btn btn-danger", disabled: busy(), onclick: move |_| save.call(true), "Confirm excess and save" }
                button { class: "btn btn-secondary", disabled: busy(), onclick: move |_| confirm.set(false), "Cancel over-invoicing" }
            }
        }
    }
}
