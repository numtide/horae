use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};

use crate::components::controls::Checkbox;
use crate::components::form::{FormGroup, Input};
use crate::models::invoice::{
    InvoiceFeeBalance, InvoiceFeeSelection, InvoicePreparation, InvoiceSource,
};

#[derive(Clone)]
pub(super) struct FeeInput {
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

pub(super) fn parse(fees: &[FeeInput]) -> Result<Vec<InvoiceFeeSelection>, String> {
    fees.iter()
        .map(|fee| {
            if fee.description.trim().is_empty()
                || fee.description.chars().count() > 1000
                || fee.description.contains('\0')
            {
                return Err("Fee descriptions must contain 1–1000 characters without NUL.".into());
            }
            let amount_cents = parse_cents(&fee.amount).map_err(|_| {
                "Enter an exact fee amount with at most two decimal places.".to_owned()
            })?;
            if amount_cents < 0
                || (fee.selected && amount_cents == 0 && fee.balance.agreed_cents != 0)
            {
                return Err(
                    "Selected fee amounts must be positive; unselected amounts cannot be negative."
                        .into(),
                );
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
    rsx! {
        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled, aria_label: "Fees to invoice",
            h4 { class: "text-sm font-semibold mb-4", "Fees to invoice" }
            p { class: "text-sm text-muted mb-4", "Amounts and descriptions change this invoice only. Balances are from the last review and include drafts, before tax. Settled fees are not selected automatically." }
            for (index, fee) in fees.read().iter().enumerate() {
                div { key: "{fee.source:?}", class: "card p-4 mb-4",
                    Checkbox { label: "Include fee: {fee.label}", checked: fee.selected, disabled,
                        onclick: move |_| { let selected = fees.peek()[index].selected; fees.write()[index].selected = !selected; }
                    }
                    FormGroup { label: "Fee description", id: "prepared-fee-description-{index}",
                        Input { id: "prepared-fee-description-{index}", value: fee.description.clone(), oninput: move |event: FormEvent| fees.write()[index].description = event.value() }
                    }
                    FormGroup { label: "Fee amount ({fee.currency})", id: "prepared-fee-amount-{index}",
                        Input { id: "prepared-fee-amount-{index}", class: "text-mono", value: fee.amount.clone(), oninput: move |event: FormEvent| fees.write()[index].amount = event.value() }
                    }
                    p { class: "text-sm text-muted wrap-anywhere",
                        "Last review · Agreed: {fee.currency} " {format_cents_plain(fee.balance.agreed_cents)}
                        " · Invoiced (including drafts): {fee.currency} " {format_cents_plain(fee.balance.invoiced_cents)}
                        " · Remaining: {fee.currency} " {format_cents_plain(fee.balance.remaining_cents)}
                        if fee.balance.remaining_cents < 0 { strong { " · Over-invoiced" } }
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
    fn fee_inputs_require_exact_amounts_and_an_explicit_positive_charge() {
        let mut fee = FeeInput {
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
        };
        assert!(!parse(&[fee.clone()]).unwrap()[0].selected);
        fee.selected = true;
        assert!(parse(&[fee.clone()]).is_err());
        for invalid in ["-1", "0.001", "NaN", "999999999999999999999999"] {
            fee.amount = invalid.into();
            assert!(parse(&[fee.clone()]).is_err(), "{invalid}");
        }
        fee.amount = "1.25".into();
        assert_eq!(parse(&[fee.clone()]).unwrap()[0].amount_cents, 125);
        fee.description = "  ".into();
        assert!(parse(&[fee]).is_err());
    }
}
