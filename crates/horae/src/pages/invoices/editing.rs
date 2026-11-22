//! The draft editor: an invoice's values, subject and fee lines, with its
//! totals beside them.

use std::cell::Cell;
use std::rc::Rc;

use dioxus::dioxus_core::Task;
use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};
use uuid::Uuid;

use super::defaults_form::{DefaultsFields, fields_from, parse_fields};
use super::recovery::{PendingInvoice, RecoveryStorage};
use super::{breakdown_label, money, prepares_invoices};
use crate::components::back_link::BackLink;
use crate::components::button::{Button, ButtonVariant};
use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::form::{FormGroup, Input};
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{LoadError, NoAccess, Skeleton};
use crate::components::toast::{Toast, ToastContainer};
use crate::i18n::{Tr, use_tr};
use crate::models::invoice::*;
use crate::route::Route;
use crate::server_fns;

/// How long edits must pause before the totals are reviewed again.
const REVIEW_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(300);

#[component]
pub(super) fn DraftEditorPage(id: Uuid) -> Element {
    let tr = use_tr();
    let session = crate::components::session::use_session();
    let prepares = prepares_invoices(session);
    let mut editor = use_resource(move || async move {
        let (editor, invoice) = futures_util::join!(
            server_fns::get_invoice_editor(id.to_string()),
            server_fns::get_invoice(id.to_string()),
        );
        Ok::<_, ServerFnError>((editor?, invoice?.invoice.number))
    });
    let retry = use_callback(move |()| editor.restart());
    let mut notice = use_signal(|| None::<String>);
    let saved = use_callback(move |()| {
        notice.set(Some(tr.invoices_draft_saved()));
        editor.restart();
    });
    let dismiss = use_callback(move |_: MouseEvent| notice.set(None));
    let title = match &*editor.read() {
        Some(Ok((_, number))) => tr.invoices_edit_title(number.clone()),
        _ => tr.invoices_edit_draft(),
    };
    rsx! {
        div {
            BackLink { to: Route::InvoiceDetail { id }, label: tr.invoices_back_to_invoice() }
            PageHeader { title, title_id: "invoice-editor-heading" }
            if !prepares {
                NoAccess {}
            } else {
                match &*editor.read() {
                    None => rsx! { Skeleton { rows: 6, columns: 2, label: tr.invoices_editor_loading() } },
                    Some(Err(error)) => rsx! {
                        LoadError { error: error.clone(), what: tr.invoices_editor_what(), retry,
                            pending: editor.state()() != UseResourceState::Ready, focus_on_recovery: "invoice-editor-heading" }
                    },
                    Some(Ok((snapshot, _))) => rsx! {
                        // A new revision starts the form again from what was saved.
                        for revision in [snapshot.edit.revision] {
                            DraftEditForm { key: "{revision}", id, snapshot: snapshot.clone(), onsaved: saved }
                        }
                    },
                }
            }
            ToastContainer {
                if let Some(message) = notice() {
                    Toast { message, variant: "success", icon: "✓", dismissible: true, ondismiss: dismiss }
                }
            }
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
    tr: Tr,
    revision: i64,
    subject: &str,
    fields: &crate::models::project_creation::InvoiceDefaultsInput,
    fees: &[FeeInput],
) -> Result<InvoiceDraftEdit, String> {
    let subject = subject.trim();
    if subject.chars().count() > 200 || subject.contains('\0') {
        return Err(tr.invoices_subject_too_long());
    }
    Ok(InvoiceDraftEdit {
        revision,
        defaults: parse_fields(fields)?,
        subject: subject.to_owned(),
        fees: fees
            .iter()
            .map(|fee| {
                if fee.description.trim().is_empty()
                    || fee.description.chars().count() > 1000
                    || fee.description.contains('\0')
                {
                    return Err(tr.invoices_fee_description_invalid());
                }
                let amount_cents =
                    parse_cents(&fee.amount).map_err(|_| tr.invoices_fee_amount_invalid())?;
                if amount_cents < 0 {
                    return Err(tr.invoices_fee_amount_negative());
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
fn DraftEditForm(id: Uuid, snapshot: InvoiceEditor, onsaved: EventHandler<()>) -> Element {
    let tr = use_tr();
    let storage = use_context::<RecoveryStorage>();
    let navigator = use_navigator();
    let fields = use_signal(|| fields_from(&snapshot.edit.defaults));
    let mut subject = use_signal(|| snapshot.edit.subject.clone());
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
    let initial = (snapshot.edit.clone(), snapshot.review.clone());
    let mut reviewed = use_signal(move || Some(initial));
    let mut reviewing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut request = use_signal(|| None::<InvoiceDraftSave>);
    let mut uncertain = use_signal(|| false);
    let mut confirm = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let current = parse_edit(
        tr,
        snapshot.edit.revision,
        &subject.read(),
        &fields.read(),
        &fees.read(),
    );
    let dirty = current.as_ref() != Ok(&snapshot.edit);

    // The totals follow the form once edits pause.
    let waiting = use_hook(|| Rc::new(Cell::new(None::<Task>)));
    let wanted = current.clone();
    use_effect(use_reactive!(|wanted| {
        if let Some(task) = waiting.take() {
            task.cancel();
            reviewing.set(false);
        }
        let Ok(edit) = wanted else {
            return;
        };
        if reviewed
            .peek()
            .as_ref()
            .is_some_and(|(done, _)| *done == edit)
        {
            return;
        }
        waiting.set(Some(spawn(async move {
            crate::components::debounce::sleep(REVIEW_DEBOUNCE).await;
            reviewing.set(true);
            match server_fns::review_invoice_edit(id.to_string(), edit.clone()).await {
                Ok(review) => {
                    error.set(None);
                    reviewed.set(Some((edit, review)));
                }
                Err(failure) => error.set(Some(failure.to_string())),
            }
            reviewing.set(false);
        })));
    }));

    let ready = reviewed
        .read()
        .as_ref()
        .filter(|(edit, _)| current.as_ref() == Ok(edit))
        .cloned();
    let shown = reviewed.read().as_ref().map(|(edit, review)| {
        adjustment_breakdown(
            &review.amounts,
            edit.defaults.discount_bps,
            edit.defaults.tax1_bps,
            edit.defaults
                .tax2_name
                .as_deref()
                .zip(edit.defaults.tax2_bps),
        )
    });
    let save_ready = ready.clone();
    let save = use_callback(move |confirm_excess: bool| {
        if (*busy.peek() && !*uncertain.peek()) || !storage.ready() {
            return;
        }
        let next = if *uncertain.peek() {
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
            let pending = PendingInvoice::Save {
                invoice_id: id,
                request: next,
            };
            if let Err(message) = storage.store(&pending).await {
                busy.set(false);
                error.set(Some(tr.invoices_request_not_sent(message)));
                return;
            }
            match pending.submit().await {
                Ok(_) => match storage.clear().await {
                    Ok(()) => {
                        busy.set(false);
                        onsaved.call(());
                    }
                    Err(message) => {
                        uncertain.set(true);
                        error.set(Some(tr.invoices_recovery_not_cleared(message)));
                    }
                },
                Err(failure) => {
                    if matches!(&failure, ServerFnError::ServerError {code, ..} if (400..500).contains(code))
                    {
                        reviewed.set(None);
                        error.set(Some(failure.to_string()));
                        if let Err(message) = storage.clear().await {
                            error.set(Some(format!("{failure}. {message}")));
                        }
                        busy.set(false);
                    } else {
                        uncertain.set(true);
                        error.set(Some(tr.invoices_save_uncertain()));
                    }
                }
            }
        });
    });
    let has_excess = ready
        .as_ref()
        .is_some_and(|(_, review)| review.fees.iter().any(|fee| fee.excess_cents > 0));
    let currency = snapshot.currency.trim().to_owned();
    let click_save = use_callback(move |_: MouseEvent| {
        if *uncertain.peek() {
            save.call(false);
        } else if has_excess {
            confirm.set(true);
        } else {
            save.call(false);
        }
    });
    let save_excess = use_callback(move |()| save.call(true));
    let keep_editing = use_callback(move |()| confirm.set(false));
    let discard = use_callback(move |_: MouseEvent| {
        navigator.push(Route::InvoiceDetail { id });
    });
    let set_subject = use_callback(move |event: FormEvent| subject.set(event.value()));
    rsx! {
        div { class: "inv-editor", "data-editor-kind": "invoice", "data-editor-state": if busy() { "pending" } else if dirty { "dirty" } else { "clean" },
            div { class: "flex flex-col gap-6 min-w-0",
                if let Err(message) = &current {
                    div { class: "alert alert-danger", role: "alert", "{message}" }
                }
                if let Some(message) = error() {
                    div { class: "alert alert-danger", role: "alert", "{message}" }
                }
                section { class: "card p-5", aria_labelledby: "inv-details-title",
                    h2 { id: "inv-details-title", class: "card-title mb-4", {tr.invoices_details()} }
                    FormGroup { label: tr.invoices_subject(), id: "inv-subject", hint: tr.invoices_subject_hint(),
                        Input { id: "inv-subject", value: subject(), disabled: busy(), oninput: set_subject }
                    }
                    DefaultsFields { fields, disabled: busy() }
                }
                if !fees.read().is_empty() {
                    section { class: "card p-5", aria_labelledby: "inv-fees-title",
                        h2 { id: "inv-fees-title", class: "card-title mb-2", {tr.invoices_fee_lines()} }
                        p { class: "text-sm text-muted mb-4", {tr.invoices_fee_lines_hint()} }
                        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled: busy(), aria_label: tr.invoices_fee_lines(),
                            for (index, fee) in fees.read().iter().enumerate() {
                                div { key: "{fee.id}", class: "grid md:grid-cols-2 gap-3 mb-2",
                                    FormGroup { label: tr.invoices_fee_description(), id: "fee-description-{fee.id}",
                                        Input { id: "fee-description-{fee.id}", value: fee.description.clone(), oninput: move |event: FormEvent| fees.write()[index].description = event.value() }
                                    }
                                    FormGroup { label: tr.invoices_fee_amount(currency.clone()), id: "fee-amount-{fee.id}",
                                        Input { id: "fee-amount-{fee.id}", class: "text-mono", value: fee.amount.clone(), oninput: move |event: FormEvent| fees.write()[index].amount = event.value() }
                                    }
                                    if let Some(balance) = ready.as_ref().and_then(|(_, review)| review.fees.iter().find(|balance| balance.line_id == fee.id)) {
                                        p { class: "text-sm text-muted md:col-span-2 mb-2",
                                            {tr.invoices_fee_edit_balance(money(balance.agreed_cents, &currency), money(balance.other_invoiced_cents, &currency), money(balance.remaining_cents, &currency))}
                                            if balance.excess_cents > 0 { strong { " " {tr.invoices_over_invoiced()} } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            aside { class: "inv-editor-totals card p-5", aria_labelledby: "inv-totals-title",
                h2 { id: "inv-totals-title", class: "card-title mb-4", {tr.invoices_totals()} }
                div { class: "inv-content", aria_busy: reviewing() || (ready.is_none() && current.is_ok()),
                    if let Some(rows) = shown {
                        dl { class: "flex flex-col gap-2 mb-5",
                            for (label, cents) in rows {
                                div { key: "{label}", class: if label == "Total" { "flex justify-between gap-4 font-semibold" } else { "flex justify-between gap-4" },
                                    dt { class: "wrap-anywhere", {breakdown_label(tr, &label)} }
                                    dd { class: "text-mono", {money(cents, &currency)} }
                                }
                            }
                        }
                    }
                }
                div { class: "flex flex-col gap-3",
                    Button { id: "invoice-save", label: if uncertain() { tr.invoices_retry_save() } else { tr.invoices_save_draft() },
                        disabled: (busy() && !uncertain()) || ready.is_none() || (!dirty && !uncertain()),
                        onclick: click_save }
                    Button { variant: ButtonVariant::Secondary, label: tr.invoices_discard_changes(), disabled: busy(), onclick: discard }
                }
            }
        }
        // Over-invoicing can be corrected in the draft afterwards, so it is not
        // a destructive confirmation.
        ConfirmDialog { id: "invoice-excess", labelledby: "invoice-excess-title", open: confirm(), busy: busy(),
            tone: ConfirmTone::Neutral, title: tr.invoices_excess_title(),
            confirm_label: tr.invoices_excess_save(), cancel_label: tr.invoices_excess_cancel(),
            on_confirm: save_excess, on_dismiss: keep_editing,
            p { {tr.invoices_excess_save_body()} }
            if let Some((_, review)) = &ready {
                for fee in review.fees.iter().filter(|fee| fee.excess_cents > 0) {
                    p { key: "{fee.line_id}", {tr.invoices_excess_line(money(fee.excess_cents, &currency), fee.period_key.clone())} }
                }
            }
        }
    }
}
