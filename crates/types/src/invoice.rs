use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::InvoiceStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// One optional line naming what the invoice is for; empty when unset.
    pub subject: String,
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

/// Invoice with its line items, returned by invoice writes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceWithLines {
    pub invoice: Invoice,
    pub lines: Vec<InvoiceLine>,
}

/// List projection with the current client name, independent of picker activity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceListItem {
    pub invoice: Invoice,
    pub client_name: String,
    /// Sent, unpaid and past due on the organization's today.
    pub overdue: bool,
}

/// Invoice detail and client identity materialized under the same read authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceDetailView {
    pub invoice: Invoice,
    pub lines: Vec<InvoiceLine>,
    pub client_name: String,
    /// Sent, unpaid and past due on the organization's today.
    pub overdue: bool,
}

/// An amount in one currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyAmount {
    pub currency: String,
    pub cents: i64,
}

/// A client's latest invoice that is not void, and the last day it billed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastInvoice {
    pub id: Uuid,
    pub number: String,
    pub issued_on: NaiveDate,
    /// The latest day of time or fee due date on its lines, or its issue date
    /// when it bills neither.
    pub covered_to: NaiveDate,
}

impl LastInvoice {
    /// Where a new invoice for the same client starts: the day after the
    /// last one's period.
    pub fn next_start(&self) -> Option<NaiveDate> {
        self.covered_to.succ_opt()
    }
}

/// A client with unbilled time or fees due up to today, as preparing an
/// invoice for it would find them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UninvoicedClient {
    pub client_id: Uuid,
    pub client_name: String,
    pub unbilled_minutes: i64,
    /// Unbilled time priced at today's billable rates, per currency.
    pub time: Vec<CurrencyAmount>,
    /// Fees due and not yet invoiced, per currency.
    pub fees: Vec<CurrencyAmount>,
    pub last_invoice: Option<LastInvoice>,
}

/// What Duplicate copies from an invoice into a new one: its client, the
/// projects it billed and its values. Its subject and notes are copied when
/// the new draft is created (`InvoiceGenerationRequest::duplicate_of`); its
/// lines and time never are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDuplicate {
    pub source_id: Uuid,
    pub source_number: String,
    pub client_id: Uuid,
    pub project_ids: Vec<Uuid>,
    pub defaults: InvoiceDefaults,
}

/// Editable invoice values, copied from projects rather than linked to them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// Exact source-level excesses computed by the server for this review.
    pub excess: Vec<InvoiceSourceExcess>,
    /// Billable time in the period still waiting for approval. It stays out
    /// of the invoice until it is approved. Left out of the serialized form
    /// when zero, so a reviewed request stored before this field existed
    /// still replays as the same request.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub awaiting_approval_minutes: i64,
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

pub fn adjustment_breakdown(
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

/// Stable before a fee occurrence is materialized. Variant and field order also
/// define discount tie-breaking: time UUIDs, then project UUID and period key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum InvoiceSource {
    Time {
        entry_id: Uuid,
    },
    Fee {
        project_id: Uuid,
        period_key: String,
    },
}

/// Non-void invoices, including draft reservations, consume the agreed fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceFeeBalance {
    pub agreed_cents: i64,
    pub invoiced_cents: i64,
    /// Signed balance before applying the proposed invoice.
    pub remaining_cents: i64,
}

/// Fee previews have neither a time quantity nor an hourly rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoicePreviewLine {
    pub source: InvoiceSource,
    pub selected: bool,
    pub project_id: Uuid,
    pub currency: String,
    pub description: String,
    pub minutes: Option<i32>,
    pub rate_cents: Option<i64>,
    pub amount_cents: i64,
    pub fee_balance: Option<InvoiceFeeBalance>,
    /// Absent until conflicting invoice defaults have been resolved.
    pub net_before_tax_cents: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceFeeSelection {
    pub source: InvoiceSource,
    pub description: String,
    pub amount_cents: i64,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceSourceExcess {
    pub source: InvoiceSource,
    pub excess_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvoiceGenerationRequest {
    pub request_id: Uuid,
    pub review: InvoicePreparation,
    pub confirmed_excess: Vec<InvoiceSourceExcess>,
    /// The invoice this one duplicates: the new draft takes its subject and
    /// notes. It must belong to the same client. Not serialized when absent,
    /// for the same replay reason as `awaiting_approval_minutes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<Uuid>,
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

impl InvoicePreparation {
    pub fn fee_selection(&self) -> Vec<InvoiceFeeSelection> {
        self.lines
            .iter()
            .filter(|line| matches!(line.source, InvoiceSource::Fee { .. }))
            .map(|line| InvoiceFeeSelection {
                source: line.source.clone(),
                description: line.description.clone(),
                amount_cents: line.amount_cents,
                selected: line.selected,
            })
            .collect()
    }
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
    /// Not serialized when empty, so a stored edit from before subjects
    /// existed still replays as the same request.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subject: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    // A request stored by a tab before these fields existed must serialize
    // the same way now, or its replay is refused as a different request.
    #[test]
    fn fields_added_for_step_one_are_absent_from_the_wire_until_set() {
        let edit = InvoiceDraftEdit {
            revision: 1,
            defaults: InvoiceDefaults::default(),
            fees: Vec::new(),
            subject: String::new(),
        };
        let json = serde_json::to_value(&edit).unwrap();
        assert!(json.get("subject").is_none());
        let parsed: InvoiceDraftEdit = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, edit);

        let request = InvoiceGenerationRequest {
            request_id: Uuid::nil(),
            review: InvoicePreparation {
                issued_on: NaiveDate::MIN,
                due_on: None,
                currency: "EUR".into(),
                projects: Vec::new(),
                defaults: None,
                lines: Vec::new(),
                subtotal_cents: 0,
                amounts: None,
                excess: Vec::new(),
                awaiting_approval_minutes: 0,
            },
            confirmed_excess: Vec::new(),
            duplicate_of: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert!(json.get("duplicate_of").is_none());
        assert!(json["review"].get("awaiting_approval_minutes").is_none());
        let parsed: InvoiceGenerationRequest = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, request);
    }

    #[test]
    fn the_next_invoice_starts_the_day_after_the_last_one_covered() {
        let last = LastInvoice {
            id: Uuid::nil(),
            number: "INV-1".into(),
            issued_on: NaiveDate::from_ymd_opt(2026, 11, 3).unwrap(),
            covered_to: NaiveDate::from_ymd_opt(2026, 10, 31).unwrap(),
        };
        assert_eq!(last.next_start(), NaiveDate::from_ymd_opt(2026, 11, 1));
    }
}
