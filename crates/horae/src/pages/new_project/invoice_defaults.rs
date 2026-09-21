use dioxus::prelude::*;
use horae_core::types::ProjectType;

use crate::components::form::{FormGroup, Input};
use crate::components::select_field::SelectField;
use crate::models::project_creation::{ProjectForm, SecondTaxInput};

use super::FormRow;

const TERMS: [&str; 5] = ["0", "15", "30", "45", "60"];

#[component]
pub(super) fn InvoiceDefaults(mut form: Signal<ProjectForm>) -> Element {
    let mut custom_terms =
        use_signal(|| !TERMS.contains(&form.peek().invoice_defaults.terms_days.as_str()));
    if form.read().project_type == ProjectType::NonBillable {
        return rsx! {};
    }
    let terms_options = TERMS
        .iter()
        .map(|days| {
            (
                days.to_string(),
                if *days == "0" {
                    "Due on receipt".into()
                } else {
                    format!("Net {days}")
                },
            )
        })
        .chain(std::iter::once(("custom".into(), "Custom days".into())))
        .collect();
    rsx! {
        section { class: "mt-8", aria_labelledby: "np-invoice-heading",
            div { class: "flex flex-wrap items-baseline gap-3 pb-2 border-b border-light",
                h2 { id: "np-invoice-heading", class: "text-xl font-semibold m-0", "Invoice defaults" }
                p { class: "text-xs text-subtle m-0", "Pre-filled when preparing an invoice — editable per invoice" }
            }
            FormRow { label: "Payment terms", id: "np-terms",
                div { class: "w-60 max-w-full",
                    SelectField { id: "np-terms", label: "Payment terms", options: terms_options,
                        selected: if custom_terms() { "custom".into() } else { form.read().invoice_defaults.terms_days.clone() },
                        onselect: move |value: String| {
                            custom_terms.set(value == "custom");
                            if value != "custom" { form.write().invoice_defaults.terms_days = value; }
                        }
                    }
                }
                if custom_terms() {
                    div { class: "mt-3",
                        FormGroup { label: "Days until payment is due", id: "np-terms-days", hint: "From 0 to 365 days after the invoice date.",
                            Input { id: "np-terms-days", class: "w-24 max-w-full font-mono text-right", value: form.read().invoice_defaults.terms_days.clone(), oninput: move |event: FormEvent| form.write().invoice_defaults.terms_days = event.value() }
                        }
                    }
                }
            }
            FormRow { label: "PO number", id: "np-po-number", hint: "Optional",
                Input { id: "np-po-number", class: "w-60 max-w-full font-mono", value: form.read().invoice_defaults.po_number.clone(), oninput: move |event: FormEvent| form.write().invoice_defaults.po_number = event.value() }
            }
            FormRow { label: "Tax (%)", id: "np-tax", hint: "Optional · up to two decimal places",
                div { class: "flex flex-wrap items-center gap-3",
                    div { class: "flex items-center gap-2",
                        Input { id: "np-tax", class: "w-24 font-mono text-right", value: form.read().invoice_defaults.tax.clone(), oninput: move |event: FormEvent| form.write().invoice_defaults.tax = event.value() }
                        span { class: "text-sm text-subtle", "%" }
                    }
                    if let Some(tax) = form.read().invoice_defaults.second_tax.clone() {
                        div { class: "flex flex-wrap items-center gap-2",
                            span { class: "text-sm text-faint", "+" }
                            Input { id: "np-second-tax-name", label: "Second tax name", placeholder: "Second tax name", class: "w-40 max-w-full", value: tax.name,
                                oninput: move |event: FormEvent| { if let Some(tax) = &mut form.write().invoice_defaults.second_tax { tax.name = event.value(); } }
                            }
                            div { class: "flex items-center gap-2",
                                Input { id: "np-second-tax", label: "Second tax (%)", class: "w-24 font-mono text-right", value: tax.percentage,
                                    oninput: move |event: FormEvent| { if let Some(tax) = &mut form.write().invoice_defaults.second_tax { tax.percentage = event.value(); } }
                                }
                                span { class: "text-sm text-subtle", "%" }
                            }
                            button { r#type: "button", class: "btn btn-icon btn-ghost", aria_label: "Remove second tax",
                                onclick: move |_| { form.write().invoice_defaults.second_tax = None; document::eval("document.getElementById('np-tax')?.focus();"); }, "×"
                            }
                        }
                    } else {
                        button { r#type: "button", class: "btn btn-ghost btn-sm", onclick: move |_| form.write().invoice_defaults.second_tax = Some(SecondTaxInput { name: String::new(), percentage: String::new() }), "Add a second tax" }
                    }
                }
            }
            FormRow { label: "Discount (%)", id: "np-discount", hint: "Optional",
                div { class: "flex items-center gap-2",
                    Input { id: "np-discount", class: "w-24 font-mono text-right", value: form.read().invoice_defaults.discount.clone(), oninput: move |event: FormEvent| form.write().invoice_defaults.discount = event.value() }
                    span { class: "text-sm text-subtle", "%" }
                }
                p { class: "form-hint", "Discount is applied before tax. Both taxes use the discounted subtotal without compounding." }
            }
        }
    }
}
