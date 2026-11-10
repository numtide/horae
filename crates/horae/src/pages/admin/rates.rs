use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};
use horae_core::permissions::rates::RateEdit;
use uuid::Uuid;

use crate::components::{
    form::{FormGroup, Input},
    modal::Modal,
};
use crate::models::permission_editor::PermissionRequester;
use crate::models::user::{PersonRate, PersonRates};
use crate::server_fns;

/// The person whose rates the dialog shows, and who asked.
#[derive(Clone, PartialEq)]
pub(super) struct RatesTarget {
    pub id: Uuid,
    pub name: String,
    pub requester: PermissionRequester,
}

#[component]
pub(super) fn PersonRatesDialog(
    mut target: Signal<Option<RatesTarget>>,
    on_saved: EventHandler<String>,
) -> Element {
    let busy = use_signal(|| false);
    let close = use_callback(move |()| target.set(None));
    rsx! {
        Modal { id: "person-rates", labelledby: "person-rates-title", open: target.read().is_some(), busy: busy(),
            on_dismiss: close,
            if let Some(person) = target() {
                RatesForm { key: "{person.id}", person, busy, on_close: close, on_saved }
            }
        }
    }
}

#[component]
fn RatesForm(
    person: RatesTarget,
    busy: Signal<bool>,
    on_close: EventHandler<()>,
    on_saved: EventHandler<String>,
) -> Element {
    let (id, requester) = (person.id, person.requester);
    let loaded =
        use_resource(move || async move { server_fns::load_person_rates(id, requester).await });
    let rates = match &*loaded.read() {
        None => {
            return rsx! {
                h2 { id: "person-rates-title", class: "modal-title", "Rates for {person.name}" }
                p { class: "modal-body", role: "status", "Loading rates…" }
            };
        }
        Some(Err(_)) => {
            return rsx! {
                h2 { id: "person-rates-title", class: "modal-title", "Rates for {person.name}" }
                div { class: "modal-body",
                    p { role: "alert", class: "alert alert-danger", "You can't view this person's rates." }
                }
                div { class: "px-6 pb-6 flex justify-end",
                    button { r#type: "button", class: "btn btn-secondary", onclick: move |_| on_close.call(()), "Close" }
                }
            };
        }
        Some(Ok(rates)) => rates.clone(),
    };
    rsx! { RatesEditor { person, rates, busy, on_close, on_saved } }
}

#[component]
fn RatesEditor(
    person: RatesTarget,
    rates: PersonRates,
    mut busy: Signal<bool>,
    on_close: EventHandler<()>,
    on_saved: EventHandler<String>,
) -> Element {
    let (id, requester) = (person.id, person.requester);
    let billable = use_signal(|| amount(rates.billable));
    let cost = use_signal(|| amount(rates.cost));
    let mut error = use_signal(|| None::<String>);
    let editable = [rates.billable, rates.cost]
        .iter()
        .any(|rate| rate.is_some_and(|rate| rate.editable));
    let currency = rates.currency.clone();
    let stored = rates.clone();
    rsx! {
        h2 { id: "person-rates-title", class: "modal-title", "Rates for {person.name}" }
        form { id: "person-rates-form", onsubmit: move |event| {
            event.prevent_default();
            if busy() { return; }
            let edits = edit_of(&billable.read(), stored.billable)
                .and_then(|b| edit_of(&cost.read(), stored.cost).map(|c| (b, c)));
            let (billable_edit, cost_edit) = match edits {
                Ok(edits) => edits,
                Err(message) => { error.set(Some(message.into())); return; }
            };
            busy.set(true); error.set(None);
            let currency = stored.currency.clone();
            spawn(async move {
                let result = server_fns::update_person_rates(id, requester, currency, billable_edit, cost_edit).await;
                busy.set(false);
                match result {
                    Ok(_) => { on_saved.call("Rates saved.".into()); on_close.call(()); }
                    Err(failure) => error.set(Some(failure.to_string())),
                }
            });
        },
            div { class: "modal-body",
                if let Some(message) = error() { p { id: "person-rates-error", role: "alert", class: "alert alert-danger", "{message}" } }
                p { class: "form-hint",
                    "Amounts are per hour in {currency}, the workspace currency."
                    if editable { " Leave a field empty for no rate. A change takes effect as soon as you save; there is no rate history." }
                }
                if let Some(rate) = rates.billable {
                    RateField { id: "person-billable-rate", label: format!("Default billable rate ({currency})"), rate, value: billable, busy: busy(),
                        hint: format!("Used for this person's time on projects billed by person rate (or with no rate settings) that have no rate of their own for them and bill in {currency}. Uninvoiced time, budgets and reports use the new rate; invoices already created keep their amounts. New project teams show it as the suggested rate.") }
                }
                if let Some(rate) = rates.cost {
                    RateField { id: "person-cost-rate", label: format!("Cost rate ({currency})"), rate, value: cost, busy: busy(),
                        hint: "Used to cost this person's time in reports on projects without a project cost rate for them, past time included.".to_string() }
                }
            }
            div { class: "px-6 pb-6 flex flex-wrap justify-end gap-3",
                button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| on_close.call(()),
                    if editable { "Cancel" } else { "Close" }
                }
                if editable {
                    button { id: "person-rates-save", r#type: "submit", class: "btn btn-primary", disabled: busy(),
                        if busy() { "Saving…" } else { "Save rates" }
                    }
                }
            }
        }
    }
}

#[component]
fn RateField(
    id: &'static str,
    label: String,
    rate: PersonRate,
    mut value: Signal<String>,
    busy: bool,
    hint: String,
) -> Element {
    let oninput = use_callback(move |event: FormEvent| value.set(event.value()));
    rsx! {
        if rate.editable {
            FormGroup { id, label, hint,
                Input { id, value: value(), disabled: busy, placeholder: "No rate", oninput }
            }
        } else {
            div { class: "form-group",
                p { class: "form-label", "{label}" }
                p { id, "{display(rate.cents)}" }
                p { class: "form-hint", "{hint}" }
            }
        }
    }
}

fn amount(rate: Option<PersonRate>) -> String {
    rate.and_then(|rate| rate.cents)
        .map(format_cents_plain)
        .unwrap_or_default()
}

fn display(cents: Option<i64>) -> String {
    cents.map_or_else(|| "No rate".into(), format_cents_plain)
}

/// The edit a field asks for: nothing when it is read-only or still shows the
/// stored amount, a reset when emptied, otherwise the exact amount in cents.
fn edit_of(input: &str, rate: Option<PersonRate>) -> Result<RateEdit, &'static str> {
    let Some(rate) = rate.filter(|rate| rate.editable) else {
        return Ok(RateEdit::Unchanged);
    };
    let input = input.trim();
    if input.is_empty() {
        return Ok(if rate.cents.is_some() {
            RateEdit::Reset
        } else {
            RateEdit::Unchanged
        });
    }
    let cents = parse_cents(input).map_err(|_| "Enter a rate with at most two decimal places.")?;
    if cents < 0 {
        return Err("A rate cannot be negative.");
    }
    Ok(if rate.cents == Some(cents) {
        RateEdit::Unchanged
    } else {
        RateEdit::Set(cents)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: Option<PersonRate> = Some(PersonRate {
        cents: Some(9550),
        editable: true,
    });
    const UNSET: Option<PersonRate> = Some(PersonRate {
        cents: None,
        editable: true,
    });

    #[test]
    fn amounts_parse_to_exact_cents() {
        assert_eq!(edit_of("120", UNSET), Ok(RateEdit::Set(12000)));
        assert_eq!(edit_of(" 0.1 ", UNSET), Ok(RateEdit::Set(10)));
        assert_eq!(edit_of("1,234.56", UNSET), Ok(RateEdit::Set(123456)));
        assert_eq!(edit_of("0", SET), Ok(RateEdit::Set(0)));
    }

    #[test]
    fn imprecise_or_negative_amounts_are_refused() {
        assert!(edit_of("10.005", SET).is_err());
        assert!(edit_of("ten", SET).is_err());
        assert!(edit_of("-1", SET).is_err());
    }

    #[test]
    fn empty_means_no_rate_and_the_shown_amount_means_unchanged() {
        assert_eq!(edit_of("", SET), Ok(RateEdit::Reset));
        assert_eq!(edit_of("", UNSET), Ok(RateEdit::Unchanged));
        assert_eq!(edit_of("95.50", SET), Ok(RateEdit::Unchanged));
    }

    #[test]
    fn read_only_or_withheld_fields_never_edit() {
        let read_only = Some(PersonRate {
            cents: Some(9550),
            editable: false,
        });
        assert_eq!(edit_of("1", read_only), Ok(RateEdit::Unchanged));
        assert_eq!(edit_of("1", None), Ok(RateEdit::Unchanged));
    }
}
