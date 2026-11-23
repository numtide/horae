/// Deterministic invoice PDF rendering via Typst (FR-025).
///
/// The template is embedded at compile time; fonts come from typst-kit.
/// Same inputs always produce byte-identical PDF output.
use typst::foundations::{Dict, IntoValue, Value};
use typst_as_lib::TypstEngine;

use crate::models::invoice::{ImportedInvoice, ImportedInvoiceLine, InvoiceLine};
use crate::models::{Invoice, OrgBranding};

/// What an invoice imported from Harvest renders instead of Horae's lines and
/// breakdown: Harvest's lines and the amounts it stated.
#[derive(Clone, Copy)]
pub struct Imported<'a> {
    pub invoice: &'a ImportedInvoice,
    pub lines: &'a [ImportedInvoiceLine],
}

static INVOICE_TEMPLATE: &str = include_str!("../templates/invoice.typ");

fn build_engine() -> TypstEngine<typst_as_lib::TypstTemplateMainFile> {
    TypstEngine::builder()
        .main_file(INVOICE_TEMPLATE)
        .search_fonts_with(
            typst_as_lib::typst_kit_options::TypstKitFontOptions::default()
                .include_system_fonts(false)
                .include_embedded_fonts(true),
        )
        .build()
}

fn opt(s: &Option<String>) -> Value {
    match s {
        Some(v) => v.clone().into_value(),
        None => Value::None,
    }
}

fn build_inputs(
    invoice: &Invoice,
    lines: &[InvoiceLine],
    imported: Option<Imported<'_>>,
    client_name: &str,
    client_address: Option<&str>,
    client_tax_id: Option<&str>,
    branding: &OrgBranding,
) -> Dict {
    let mut dict = Dict::new();

    dict.insert("invoice_number".into(), invoice.number.clone().into_value());
    dict.insert(
        "issued_on".into(),
        invoice.issued_on.to_string().into_value(),
    );
    dict.insert("due_on".into(), invoice.due_on.to_string().into_value());
    dict.insert(
        "currency".into(),
        invoice.currency.trim().to_string().into_value(),
    );
    dict.insert(
        "terms_days".into(),
        i64::from(invoice.terms_days).into_value(),
    );
    dict.insert("po_number".into(), invoice.po_number.clone().into_value());
    dict.insert("subject".into(), invoice.subject.clone().into_value());
    let breakdown = match imported {
        Some(imported) => imported.invoice.breakdown(invoice),
        None => invoice.breakdown(),
    };
    let breakdown: Vec<Value> = breakdown
        .into_iter()
        .map(|(label, cents)| {
            let mut row = Dict::new();
            row.insert("label".into(), label.into_value());
            row.insert("cents".into(), cents.into_value());
            Value::Dict(row)
        })
        .collect();
    dict.insert(
        "breakdown".into(),
        Value::Array(breakdown.as_slice().into()),
    );
    dict.insert("notes".into(), opt(&invoice.notes));

    dict.insert("client_name".into(), client_name.to_string().into_value());
    dict.insert(
        "client_address".into(),
        match client_address {
            Some(a) => a.to_string().into_value(),
            None => Value::None,
        },
    );
    dict.insert(
        "client_tax_id".into(),
        match client_tax_id {
            Some(t) => t.to_string().into_value(),
            None => Value::None,
        },
    );

    dict.insert("provider_name".into(), opt(&branding.provider_name));
    dict.insert("provider_address".into(), opt(&branding.provider_address));
    dict.insert("provider_tax_id".into(), opt(&branding.provider_tax_id));
    dict.insert("provider_email".into(), opt(&branding.provider_email));
    dict.insert("provider_phone".into(), opt(&branding.provider_phone));

    dict.insert("bank_name".into(), opt(&branding.bank_name));
    dict.insert("bank_iban".into(), opt(&branding.bank_iban));
    dict.insert("bank_bic".into(), opt(&branding.bank_bic));
    dict.insert("bank_routing".into(), opt(&branding.bank_routing));
    dict.insert("bank_account".into(), opt(&branding.bank_account));
    dict.insert("payment_terms".into(), opt(&branding.invoice_payment_terms));

    dict.insert("imported".into(), imported.is_some().into_value());
    let imported_lines = imported.map_or(&[][..], |imported| imported.lines);
    let line_values: Vec<Value> = imported_lines
        .iter()
        .map(|l| {
            let mut ld = Dict::new();
            ld.insert("description".into(), l.description.clone().into_value());
            ld.insert("quantity".into(), l.quantity.clone().into_value());
            ld.insert("unit_price".into(), l.unit_price.clone().into_value());
            ld.insert("amount_cents".into(), l.amount_cents.into_value());
            Value::Dict(ld)
        })
        .chain(lines.iter().map(|l| {
            let mut ld = Dict::new();
            ld.insert("description".into(), l.description.clone().into_value());
            ld.insert(
                "minutes".into(),
                l.minutes
                    .map(|minutes| i64::from(minutes).into_value())
                    .unwrap_or(Value::None),
            );
            ld.insert(
                "rate_cents".into(),
                l.rate_cents
                    .map(IntoValue::into_value)
                    .unwrap_or(Value::None),
            );
            ld.insert("amount_cents".into(), l.amount_cents.into_value());
            Value::Dict(ld)
        }))
        .collect();
    dict.insert("lines".into(), Value::Array(line_values.as_slice().into()));

    dict
}

/// Render an invoice to a PDF byte vector.
///
/// Deterministic: identical inputs produce byte-identical output.
#[allow(clippy::too_many_arguments)]
pub fn render_invoice_pdf(
    invoice: &Invoice,
    lines: &[InvoiceLine],
    imported: Option<Imported<'_>>,
    client_name: &str,
    client_address: Option<&str>,
    client_tax_id: Option<&str>,
    branding: &OrgBranding,
) -> anyhow::Result<Vec<u8>> {
    let engine = build_engine();
    let inputs = build_inputs(
        invoice,
        lines,
        imported,
        client_name,
        client_address,
        client_tax_id,
        branding,
    );

    let doc: typst_layout::PagedDocument = engine
        .compile_with_input(inputs)
        .output
        .map_err(|e| anyhow::anyhow!("Typst compilation failed: {e}"))?;

    let options = typst_pdf::PdfOptions {
        timestamp: None,
        ..Default::default()
    };

    let pdf_bytes =
        typst_pdf::pdf(&doc, &options).map_err(|e| anyhow::anyhow!("PDF export failed: {e:?}"))?;

    Ok(pdf_bytes)
}

#[cfg(any(test, feature = "test-support"))]
pub fn invoice_text(
    invoice: &Invoice,
    lines: &[InvoiceLine],
    imported: Option<Imported<'_>>,
    branding: &OrgBranding,
) -> String {
    fn visit(frame: &typst::layout::Frame, text: &mut String) {
        for (_, item) in frame.items() {
            match item {
                typst::layout::FrameItem::Group(group) => visit(&group.frame, text),
                typst::layout::FrameItem::Text(run) => {
                    text.push_str(&run.text);
                    text.push('\n');
                }
                _ => {}
            }
        }
    }
    let document: typst_layout::PagedDocument = build_engine()
        .compile_with_input(build_inputs(
            invoice, lines, imported, "Client", None, None, branding,
        ))
        .output
        .unwrap();
    let mut text = String::new();
    for page in document.pages() {
        visit(&page.frame, &mut text);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_imported_invoice_renders_its_lines_and_stated_amounts() {
        let invoice = Invoice {
            id: uuid::Uuid::nil(),
            org_id: uuid::Uuid::nil(),
            client_id: uuid::Uuid::nil(),
            number: "1001".into(),
            status: horae_core::types::InvoiceStatus::Sent,
            issued_on: chrono::NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            due_on: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            currency: "USD".into(),
            total_cents: 105_000,
            terms_days: 28,
            po_number: String::new(),
            discount_bps: 0,
            tax1_bps: 0,
            tax2_name: None,
            tax2_bps: None,
            subtotal_cents: 110_000,
            discount_cents: 11_000,
            tax1_cents: 5_000,
            tax2_cents: 1_000,
            notes: None,
            subject: String::new(),
            created_at: chrono::DateTime::UNIX_EPOCH,
        };
        let imported = ImportedInvoice {
            due_cents: 45_000,
            paid_on: None,
            discount_rate: Some("10".into()),
            tax1_rate: Some("8.875".into()),
            tax2_rate: Some("2".into()),
            period_start: None,
            period_end: None,
        };
        let line = |description: &str, amount_cents| ImportedInvoiceLine {
            kind: "Service".into(),
            description: description.into(),
            quantity: "1.5".into(),
            unit_price: "95.2381".into(),
            amount_cents,
            taxed: true,
            taxed2: false,
            project_id: None,
        };
        let lines = [line("Design", 120_000), line("Credit", -10_000)];
        let text = invoice_text(
            &invoice,
            &[],
            Some(Imported {
                invoice: &imported,
                lines: &lines,
            }),
            &OrgBranding::default(),
        );
        for expected in [
            "Quantity",
            "Unit price",
            "Design",
            "1.5",
            "USD 95.2381",
            "USD 1200.00",
            "−USD 100.00",
            "Discount (10%)",
            "Tax (8.875%)",
            "Second tax (2%)",
            "USD 1050.00",
            "Paid",
            "−USD 600.00",
            "Due",
            "USD 450.00",
        ] {
            assert!(text.contains(expected), "{expected} missing from {text}");
        }
    }

    #[test]
    fn invoice_template_formats_large_minor_units_exactly() {
        // Evaluate the actual template helpers without the document's table.
        let (helpers, _) = INVOICE_TEMPLATE.split_once("#grid(").unwrap();
        let source = format!(
            "{helpers}\n\
             #assert.eq(fmt-money(9223372036854775807, \"EUR\"), \"EUR 92233720368547758.07\")\n\
             #assert.eq(fmt-money(9007199254740999, \"USD\"), \"USD 90071992547409.99\")\n\
             #assert.eq(fmt-money(99, \"EUR\"), \"EUR 0.99\")\n\
             #assert.eq(fmt-money(0, \"EUR\"), \"EUR 0.00\")"
        );
        let mut inputs = Dict::new();
        inputs.insert("invoice_number".into(), "format-test".into_value());
        let engine = TypstEngine::builder().main_file(source.as_str()).build();
        let _: typst_layout::PagedDocument = engine.compile_with_input(inputs).output.unwrap();
    }
}
