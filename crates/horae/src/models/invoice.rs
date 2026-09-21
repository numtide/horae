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
