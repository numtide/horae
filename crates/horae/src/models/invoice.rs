use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::InvoiceStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Invoice {
    pub id: Uuid,
    pub org_id: Uuid,
    pub client_id: Uuid,
    pub number: String,
    pub status: InvoiceStatus,
    pub issued_on: NaiveDate,
    pub due_on: NaiveDate,
    pub currency: String,
    pub total_cents: i64,
    pub terms_days: i32,
    pub po_number: String,
    pub discount_bps: i16,
    pub tax1_bps: i16,
    pub tax2_name: Option<String>,
    pub tax2_bps: Option<i16>,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax1_cents: i64,
    pub tax2_cents: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Invoice {
    /// Stored invoice components, in display order; no project settings are read.
    pub fn breakdown(&self) -> Vec<(String, i64)> {
        adjustment_breakdown(
            &horae_core::invoice::InvoiceAmounts {
                subtotal_cents: self.subtotal_cents,
                discount_cents: self.discount_cents,
                tax1_cents: self.tax1_cents,
                tax2_cents: self.tax2_cents,
                total_cents: self.total_cents,
            },
            self.discount_bps,
            self.tax1_bps,
            self.tax2_name.as_deref().zip(self.tax2_bps),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct InvoiceLine {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub time_entry_id: Option<Uuid>,
    pub fee_occurrence_id: Option<Uuid>,
    pub description: String,
    pub minutes: Option<i32>,
    pub rate_cents: Option<i64>,
    pub amount_cents: i64,
}

/// Invoice with its line items, returned by `get_invoice`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceWithLines {
    pub invoice: Invoice,
    pub lines: Vec<InvoiceLine>,
}

/// Editable invoice values, copied from projects rather than linked to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct InvoiceDefaults {
    pub terms_days: i16,
    pub po_number: String,
    pub discount_bps: i16,
    pub tax1_bps: i16,
    pub tax2_name: Option<String>,
    pub tax2_bps: Option<i16>,
}

impl Default for InvoiceDefaults {
    fn default() -> Self {
        Self {
            terms_days: 30,
            po_number: String::new(),
            discount_bps: 0,
            tax1_bps: 0,
            tax2_name: None,
            tax2_bps: None,
        }
    }
}

/// Read-only estimate. Generation rechecks all sources and settings before claiming them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoicePreparation {
    pub issued_on: NaiveDate,
    pub due_on: Option<NaiveDate>,
    pub currency: String,
    pub projects: Vec<InvoiceProjectDefaults>,
    /// Absent when selected projects require explicit conflict resolution.
    pub defaults: Option<InvoiceDefaults>,
    pub lines: Vec<InvoicePreviewLine>,
    pub subtotal_cents: i64,
    pub amounts: Option<horae_core::invoice::InvoiceAmounts>,
}

impl InvoicePreparation {
    /// Estimated components in the same order as frozen invoices; unresolved
    /// defaults have no calculated breakdown.
    pub fn breakdown(&self) -> Option<Vec<(String, i64)>> {
        let defaults = self.defaults.as_ref()?;
        Some(adjustment_breakdown(
            self.amounts.as_ref()?,
            defaults.discount_bps,
            defaults.tax1_bps,
            defaults.tax2_name.as_deref().zip(defaults.tax2_bps),
        ))
    }
}

fn adjustment_breakdown(
    amounts: &horae_core::invoice::InvoiceAmounts,
    discount_bps: i16,
    tax1_bps: i16,
    second_tax: Option<(&str, i16)>,
) -> Vec<(String, i64)> {
    use horae_core::money::format_cents_plain;

    let mut rows = vec![("Subtotal".into(), amounts.subtotal_cents)];
    if discount_bps != 0 {
        rows.push((
            format!("Discount ({}%)", format_cents_plain(discount_bps.into())),
            -amounts.discount_cents,
        ));
    }
    if tax1_bps != 0 {
        rows.push((
            format!("Tax ({}%)", format_cents_plain(tax1_bps.into())),
            amounts.tax1_cents,
        ));
    }
    if let Some((name, bps)) = second_tax {
        rows.push((
            format!("{name} ({}%)", format_cents_plain(bps.into())),
            amounts.tax2_cents,
        ));
    }
    rows.push(("Total".into(), amounts.total_cents));
    rows
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceProjectDefaults {
    pub project_id: Uuid,
    pub name: String,
    pub defaults: InvoiceDefaults,
}

/// Fee previews have neither a time quantity nor an hourly rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoicePreviewLine {
    pub project_id: Uuid,
    pub currency: String,
    pub description: String,
    pub minutes: Option<i32>,
    pub rate_cents: Option<i64>,
    pub amount_cents: i64,
}

/// Only existing fee lines are editable; time quantities and rates stay frozen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceFeeEdit {
    pub line_id: Uuid,
    pub description: String,
    pub amount_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceDraftEdit {
    pub revision: i64,
    pub defaults: InvoiceDefaults,
    pub fees: Vec<InvoiceFeeEdit>,
}

/// Availability excludes this draft's old contribution before replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceFeeReview {
    pub line_id: Uuid,
    pub project_id: Uuid,
    pub period_key: String,
    pub agreed_cents: i64,
    pub other_invoiced_cents: i64,
    pub net_cents: i64,
    pub remaining_cents: i64,
    pub excess_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceEditReview {
    pub fees: Vec<InvoiceFeeReview>,
    pub amounts: horae_core::invoice::InvoiceAmounts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceEditor {
    pub edit: InvoiceDraftEdit,
    pub review: InvoiceEditReview,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceExcessConfirmation {
    pub line_id: Uuid,
    pub excess_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceDraftSave {
    pub request_id: Uuid,
    pub edit: InvoiceDraftEdit,
    pub review: InvoiceEditReview,
    pub confirmed_excess: Vec<InvoiceExcessConfirmation>,
}
