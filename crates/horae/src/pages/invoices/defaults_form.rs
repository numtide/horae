use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::project::Percentage;

use crate::components::form::{FormGroup, Input};
use crate::components::select_field::SelectField;
use crate::i18n::Tr;
use crate::models::invoice::InvoiceDefaults;
use crate::models::project_creation::{InvoiceDefaultsInput, SecondTaxInput};

pub(super) fn fields_from(defaults: &InvoiceDefaults) -> InvoiceDefaultsInput {
    InvoiceDefaultsInput {
        terms_days: defaults.terms_days.to_string(),
        po_number: defaults.po_number.clone(),
        tax: format_cents_plain(defaults.tax1_bps.into()),
        discount: format_cents_plain(defaults.discount_bps.into()),
        second_tax: defaults
            .tax2_name
            .as_ref()
            .zip(defaults.tax2_bps)
            .map(|(name, bps)| SecondTaxInput {
                name: name.clone(),
                percentage: format_cents_plain(bps.into()),
            }),
    }
}

pub(super) fn parse_fields(
    tr: Tr,
    fields: &InvoiceDefaultsInput,
) -> Result<InvoiceDefaults, String> {
    let terms_days = fields
        .terms_days
        .trim()
        .parse::<i16>()
        .ok()
        .filter(|days| (0..=365).contains(days))
        .ok_or_else(|| tr.invoices_terms_invalid())?;
    if fields.po_number.chars().count() > 200 || fields.po_number.contains('\0') {
        return Err(tr.invoices_po_number_invalid());
    }
    let percentage = |value: &str| -> Result<i16, String> {
        let value = if value.trim().is_empty() { "0" } else { value };
        value
            .parse::<Percentage>()
            .map(|value| value.basis_points() as i16)
            .map_err(|_| tr.invoices_percentage_invalid())
    };
    let (tax2_name, tax2_bps) = match &fields.second_tax {
        Some(tax) => {
            if tax.name.trim().is_empty()
                || tax.name.chars().count() > 100
                || tax.name.contains('\0')
            {
                return Err(tr.invoices_second_tax_name_invalid());
            }
            (
                Some(tax.name.trim().into()),
                Some(percentage(&tax.percentage)?),
            )
        }
        None => (None, None),
    };
    Ok(InvoiceDefaults {
        terms_days,
        po_number: fields.po_number.trim().into(),
        discount_bps: percentage(&fields.discount)?,
        tax1_bps: percentage(&fields.tax)?,
        tax2_name,
        tax2_bps,
    })
}

#[component]
pub(super) fn DefaultsFields(mut fields: Signal<InvoiceDefaultsInput>, disabled: bool) -> Element {
    const TERMS: [&str; 5] = ["0", "15", "30", "45", "60"];
    let tr = crate::i18n::use_tr();
    let mut custom = use_signal(|| false);
    let custom_terms = custom() || !TERMS.contains(&fields.read().terms_days.as_str());
    let options = TERMS
        .iter()
        .map(|days| {
            (
                days.to_string(),
                if *days == "0" {
                    tr.invoices_terms_on_receipt()
                } else {
                    tr.invoices_terms_net(*days)
                },
            )
        })
        .chain(std::iter::once((
            "custom".into(),
            tr.invoices_terms_custom(),
        )))
        .collect();
    let pick_terms = use_callback(move |value: String| {
        custom.set(value == "custom");
        if value != "custom" {
            fields.write().terms_days = value;
        }
    });
    rsx! {
        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled, aria_label: tr.invoices_values(),
            FormGroup { label: tr.invoices_terms(), id: "inv-terms",
                SelectField { id: "inv-terms", label: tr.invoices_terms(), options,
                    selected: if custom_terms { "custom".into() } else { fields.read().terms_days.clone() },
                    onselect: pick_terms,
                }
            }
            if custom_terms {
                FormGroup { label: tr.invoices_terms_days_label(), id: "inv-terms-days", hint: tr.invoices_terms_days_hint(),
                    Input { id: "inv-terms-days", value: fields.read().terms_days.clone(), oninput: move |event: FormEvent| fields.write().terms_days = event.value() }
                }
            }
            FormGroup { label: tr.invoices_po_number(), id: "inv-po-number", hint: tr.invoices_optional(),
                Input { id: "inv-po-number", value: fields.read().po_number.clone(), oninput: move |event: FormEvent| fields.write().po_number = event.value() }
            }
            FormGroup { label: tr.invoices_tax(), id: "inv-tax", hint: tr.invoices_tax_hint(),
                Input { id: "inv-tax", value: fields.read().tax.clone(), oninput: move |event: FormEvent| fields.write().tax = event.value() }
            }
            if let Some(tax) = fields.read().second_tax.clone() {
                div { class: "grid md:grid-cols-2 gap-3",
                    FormGroup { label: tr.invoices_second_tax_name(), id: "inv-second-tax-name",
                        Input { id: "inv-second-tax-name", value: tax.name, oninput: move |event: FormEvent| {
                            if let Some(tax) = &mut fields.write().second_tax { tax.name = event.value(); }
                        } }
                    }
                    FormGroup { label: tr.invoices_second_tax(), id: "inv-second-tax",
                        Input { id: "inv-second-tax", value: tax.percentage, oninput: move |event: FormEvent| {
                            if let Some(tax) = &mut fields.write().second_tax { tax.percentage = event.value(); }
                        } }
                    }
                }
                button { r#type: "button", class: "btn btn-ghost btn-sm mb-4", onclick: move |_| fields.write().second_tax = None, {tr.invoices_second_tax_remove()} }
            } else {
                button { r#type: "button", class: "btn btn-ghost btn-sm mb-4", onclick: move |_| fields.write().second_tax = Some(SecondTaxInput { name: String::new(), percentage: String::new() }), {tr.invoices_second_tax_add()} }
            }
            FormGroup { label: tr.invoices_discount(), id: "inv-discount", hint: tr.invoices_discount_hint(),
                Input { id: "inv-discount", value: fields.read().discount.clone(), oninput: move |event: FormEvent| fields.write().discount = event.value() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_fields_preserve_exact_percentages_and_custom_terms() {
        let defaults = InvoiceDefaults {
            terms_days: 21,
            po_number: "PO-123".into(),
            tax1_bps: 2100,
            discount_bps: 1250,
            tax2_name: Some("Local tax".into()),
            tax2_bps: Some(150),
        };
        assert_eq!(
            parse_fields(Tr::default(), &fields_from(&defaults)).unwrap(),
            defaults
        );
    }

    #[test]
    fn invoice_fields_reject_invalid_percentages_terms_and_second_tax() {
        for value in ["-1", "100.01", "1.001", "NaN"] {
            let fields = InvoiceDefaultsInput {
                tax: value.into(),
                ..Default::default()
            };
            assert!(parse_fields(Tr::default(), &fields).is_err());
        }
        for value in ["", "-1", "366", "1.5"] {
            let fields = InvoiceDefaultsInput {
                terms_days: value.into(),
                ..Default::default()
            };
            assert!(parse_fields(Tr::default(), &fields).is_err());
        }
        let fields = InvoiceDefaultsInput {
            second_tax: Some(SecondTaxInput {
                name: " ".into(),
                percentage: "1.5".into(),
            }),
            ..Default::default()
        };
        assert!(parse_fields(Tr::default(), &fields).is_err());
    }

    #[test]
    fn invoice_fields_bound_metadata_by_characters_and_reject_nul() {
        let mut fields = InvoiceDefaultsInput {
            po_number: "é".repeat(200),
            second_tax: Some(SecondTaxInput {
                name: "é".repeat(100),
                percentage: "100".into(),
            }),
            ..Default::default()
        };
        assert!(parse_fields(Tr::default(), &fields).is_ok());
        fields.po_number.push('é');
        assert!(parse_fields(Tr::default(), &fields).is_err());
        fields.po_number = "PO\0number".into();
        assert!(parse_fields(Tr::default(), &fields).is_err());
        fields.po_number.clear();
        fields.second_tax.as_mut().unwrap().name.push('é');
        assert!(parse_fields(Tr::default(), &fields).is_err());
        fields.second_tax.as_mut().unwrap().name = "Tax\0name".into();
        assert!(parse_fields(Tr::default(), &fields).is_err());
    }

    #[test]
    fn invoice_fields_preserve_receipt_terms_and_empty_optional_percentages() {
        let fields = InvoiceDefaultsInput {
            terms_days: "0".into(),
            ..Default::default()
        };
        assert_eq!(
            parse_fields(Tr::default(), &fields).unwrap(),
            InvoiceDefaults {
                terms_days: 0,
                ..Default::default()
            }
        );
    }
}
