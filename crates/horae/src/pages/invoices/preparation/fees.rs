use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};

use super::super::money;
use crate::components::controls::Checkbox;
use crate::components::form::{FormGroup, Input};
use crate::i18n::{Tr, use_tr};
use crate::models::invoice::{
    InvoiceFeeBalance, InvoiceFeeSelection, InvoicePreparation, InvoiceSource,
};

#[derive(Clone)]
pub(super) struct FeeInput {
    present: bool,
    source: InvoiceSource,
    label: String,
    description: String,
    amount: String,
    selected: bool,
    currency: String,
    balance: InvoiceFeeBalance,
}

pub(super) fn from_review(review: &InvoicePreparation) -> Vec<FeeInput> {
    review
        .lines
        .iter()
        .filter_map(|line| {
            line.fee_balance.map(|balance| FeeInput {
                present: true,
                source: line.source.clone(),
                label: line.description.clone(),
                description: line.description.clone(),
                amount: format_cents_plain(line.amount_cents),
                selected: line.selected,
                currency: line.currency.clone(),
                balance,
            })
        })
        .collect()
}

pub(super) fn refresh(previous: &[FeeInput], current: Vec<FeeInput>) -> Vec<FeeInput> {
    let mut current: std::collections::BTreeMap<_, _> = current
        .into_iter()
        .map(|mut fee| {
            fee.selected = false;
            ((fee.source.clone(), fee.currency.clone()), fee)
        })
        .collect();
    let mut refreshed = Vec::with_capacity(previous.len() + current.len());
    for old in previous {
        let key = (old.source.clone(), old.currency.clone());
        if let Some(mut fee) = current.remove(&key) {
            fee.description = old.description.clone();
            fee.amount = old.amount.clone();
            fee.selected = old.selected;
            refreshed.push(fee);
        } else {
            let mut fee = old.clone();
            fee.present = false;
            refreshed.push(fee);
        }
    }
    refreshed.extend(current.into_values());
    refreshed
}

pub(super) fn parse(tr: Tr, fees: &[FeeInput]) -> Result<Vec<InvoiceFeeSelection>, String> {
    fees.iter()
        .map(|fee| {
            if !fee.present {
                return Err(tr.invoices_fee_unavailable_edits());
            }
            if fee.description.trim().is_empty()
                || fee.description.chars().count() > 1000
                || fee.description.contains('\0')
            {
                return Err(tr.invoices_fee_description_invalid());
            }
            let amount_cents =
                parse_cents(&fee.amount).map_err(|_| tr.invoices_fee_amount_invalid())?;
            if amount_cents < 0
                || (fee.selected && amount_cents == 0 && fee.balance.agreed_cents != 0)
            {
                return Err(tr.invoices_fee_amount_selected());
            }
            Ok(InvoiceFeeSelection {
                source: fee.source.clone(),
                description: fee.description.clone(),
                amount_cents,
                selected: fee.selected,
            })
        })
        .collect()
}

#[component]
pub(super) fn FeeFields(mut fees: Signal<Vec<FeeInput>>, disabled: bool) -> Element {
    let tr = use_tr();
    rsx! {
        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled, aria_label: tr.invoices_fees_to_invoice(),
            h4 { class: "text-sm font-semibold mb-4", {tr.invoices_fees_to_invoice()} }
            p { class: "text-sm text-muted mb-4", {tr.invoices_fees_to_invoice_hint()} }
            for (index, fee) in fees.read().iter().enumerate() {
                div { key: "{fee.source:?}:{fee.currency}", class: "card p-4 mb-4", role: "group",
                    aria_label: tr.invoices_fee_group(fee.currency.clone(), fee.label.clone()),
                    if !fee.present {
                        p { class: "alert alert-warning", role: "status", {tr.invoices_fee_unavailable()} }
                        button { r#type: "button", class: "btn btn-secondary btn-sm mb-4", disabled,
                            onclick: move |_| { fees.write().remove(index); }, {tr.invoices_fee_remove_unavailable()}
                        }
                    }
                    Checkbox { label: tr.invoices_fee_include(fee.label.clone()), checked: fee.selected, disabled: disabled || !fee.present,
                        onclick: move |_| { let selected = fees.peek()[index].selected; fees.write()[index].selected = !selected; }
                    }
                    FormGroup { label: tr.invoices_fee_description(), id: "prepared-fee-description-{index}",
                        Input { id: "prepared-fee-description-{index}", value: fee.description.clone(), oninput: move |event: FormEvent| fees.write()[index].description = event.value() }
                    }
                    FormGroup { label: tr.invoices_fee_amount(fee.currency.clone()), id: "prepared-fee-amount-{index}",
                        Input { id: "prepared-fee-amount-{index}", class: "text-mono", value: fee.amount.clone(), oninput: move |event: FormEvent| fees.write()[index].amount = event.value() }
                    }
                    p { class: "text-sm text-muted wrap-anywhere",
                        {tr.invoices_fee_last_review(
                            money(fee.balance.agreed_cents, &fee.currency),
                            money(fee.balance.invoiced_cents, &fee.currency),
                            money(fee.balance.remaining_cents, &fee.currency),
                        )}
                        if fee.balance.remaining_cents < 0 { strong { " " {tr.invoices_over_invoiced()} } }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fee() -> FeeInput {
        FeeInput {
            present: true,
            source: InvoiceSource::Fee {
                project_id: uuid::Uuid::now_v7(),
                period_key: "single".into(),
            },
            label: "Fee".into(),
            description: "Fee".into(),
            amount: "0.00".into(),
            selected: false,
            currency: "EUR".into(),
            balance: InvoiceFeeBalance {
                agreed_cents: 100,
                invoiced_cents: 100,
                remaining_cents: 0,
            },
        }
    }

    #[test]
    fn refresh_keeps_raw_edits_and_does_not_select_new_sources() {
        let mut old = fee();
        old.amount = "invalid input kept".into();
        old.description = "My edited description".into();
        old.selected = true;
        let mut current = old.clone();
        current.balance.remaining_cents = -50;
        let mut added = fee();
        added.selected = true;
        let refreshed = refresh(&[old.clone()], vec![current, added]);
        assert_eq!(refreshed[0].amount, old.amount);
        assert_eq!(refreshed[0].description, old.description);
        assert!(refreshed[0].selected);
        assert_eq!(refreshed[0].balance.remaining_cents, -50);
        assert!(!refreshed[1].selected);
        assert!(parse(Tr::default(), &refreshed).is_err());
    }

    #[test]
    fn refresh_preserves_missing_or_relabelled_edits_until_explicit_removal() {
        let mut old = fee();
        old.amount = "9.99".into();
        old.selected = true;
        let mut changed_currency = old.clone();
        changed_currency.currency = "USD".into();
        let refreshed = refresh(&[old.clone()], vec![changed_currency]);
        assert_eq!(refreshed.len(), 2);
        assert_eq!(refreshed[0].currency, "EUR");
        assert_eq!(refreshed[0].amount, "9.99");
        assert!(
            parse(Tr::default(), &refreshed)
                .unwrap_err()
                .contains("unavailable")
        );
        assert!(!refreshed[1].selected);
        let restored = refresh(&refreshed[..1], vec![old]);
        assert_eq!(
            parse(Tr::default(), &restored).unwrap()[0].amount_cents,
            999
        );
        assert!(parse(Tr::default(), &refresh(&restored, Vec::new())).is_err());
    }

    #[test]
    fn fee_inputs_require_exact_amounts_and_an_explicit_positive_charge() {
        let mut fee = fee();
        assert!(!parse(Tr::default(), &[fee.clone()]).unwrap()[0].selected);
        fee.selected = true;
        assert!(parse(Tr::default(), &[fee.clone()]).is_err());
        for invalid in ["-1", "0.001", "NaN", "999999999999999999999999"] {
            fee.amount = invalid.into();
            assert!(parse(Tr::default(), &[fee.clone()]).is_err(), "{invalid}");
        }
        fee.amount = "1.25".into();
        assert_eq!(
            parse(Tr::default(), &[fee.clone()]).unwrap()[0].amount_cents,
            125
        );
        fee.description = "  ".into();
        assert!(parse(Tr::default(), &[fee]).is_err());
    }
}
