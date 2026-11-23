//! Harvest v2 JSON shapes: the fields the importer consumes here, and in
//! [`compat`] the shapes Horae's Harvest-compatible API writes.

pub mod compat;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiClient {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRef {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiProject {
    pub id: i64,
    pub name: String,
    pub code: Option<String>,
    #[serde(default = "yes")]
    pub is_active: bool,
    pub client: ApiRef,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTask {
    pub id: i64,
    pub name: String,
    #[serde(default = "yes")]
    pub billable_by_default: bool,
    #[serde(default = "yes")]
    pub is_active: bool,
    /// Harvest's common task flag.
    #[serde(default)]
    pub is_default: bool,
    pub default_hourly_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiUser {
    pub id: i64,
    /// Missing on some records; such a person is reported, never guessed.
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default = "yes")]
    pub is_active: bool,
}

impl ApiUser {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiTimeEntry {
    pub id: i64,
    pub spent_date: NaiveDate,
    pub hours: serde_json::Number,
    pub notes: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub is_billed: bool,
    #[serde(default)]
    pub is_running: bool,
    pub client: Option<ApiRef>,
    pub project: ApiRef,
    pub task: ApiRef,
    pub user: ApiRef,
    pub billable_rate: Option<serde_json::Number>,
    pub cost_rate: Option<serde_json::Number>,
    pub updated_at: Option<DateTime<Utc>>,
    /// The invoice that billed the entry, when it is billed.
    #[serde(default)]
    pub invoice: Option<ApiInvoiceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInvoiceRef {
    pub id: i64,
    #[serde(default)]
    pub number: Option<String>,
}

/// A Harvest invoice with its line items. Amounts stay exact decimals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInvoice {
    pub id: i64,
    pub client: ApiRef,
    #[serde(default)]
    pub number: String,
    pub purchase_order: Option<String>,
    pub amount: serde_json::Number,
    pub due_amount: serde_json::Number,
    /// Percentages, as written.
    pub tax: Option<serde_json::Number>,
    pub tax_amount: Option<serde_json::Number>,
    pub tax2: Option<serde_json::Number>,
    pub tax2_amount: Option<serde_json::Number>,
    pub discount: Option<serde_json::Number>,
    pub discount_amount: Option<serde_json::Number>,
    pub subject: Option<String>,
    pub notes: Option<String>,
    pub currency: String,
    /// draft, open, paid or closed.
    pub state: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub issue_date: NaiveDate,
    pub due_date: Option<NaiveDate>,
    pub paid_date: Option<NaiveDate>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub line_items: Vec<ApiInvoiceLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInvoiceLine {
    pub id: i64,
    pub kind: String,
    pub description: Option<String>,
    pub quantity: serde_json::Number,
    pub unit_price: serde_json::Number,
    pub amount: serde_json::Number,
    #[serde(default)]
    pub taxed: bool,
    #[serde(default)]
    pub taxed2: bool,
    pub project: Option<ApiRef>,
}

/// serde default for Harvest's `is_active` / `billable_by_default` flags, which
/// are `true` unless the record says otherwise.
fn yes() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_flags_default_to_harvests_values() {
        let task: ApiTask = serde_json::from_str(r#"{"id":1,"name":"Design"}"#).unwrap();
        assert!(task.billable_by_default && task.is_active);
        let user: ApiUser = serde_json::from_str(r#"{"id":2,"first_name":"Dana"}"#).unwrap();
        assert!(user.is_active);
        assert_eq!(user.email, None);
        assert_eq!(user.full_name(), "Dana");
        let entry: ApiTimeEntry = serde_json::from_str(
            r#"{"id":3,"spent_date":"2026-01-15","hours":1.5,"notes":null,
                "project":{"id":10},"task":{"id":100},"user":{"id":1000}}"#,
        )
        .unwrap();
        assert!(!entry.billable && !entry.is_billed && !entry.is_running);
        assert_eq!(entry.hours.to_string(), "1.5");
        assert!(entry.invoice.is_none());
    }

    #[test]
    fn billed_entries_name_their_invoice() {
        let entry: ApiTimeEntry = serde_json::from_str(
            r#"{"id":3,"spent_date":"2026-01-15","hours":1,"notes":null,"is_billed":true,
                "invoice":{"id":77,"number":"1042"},
                "project":{"id":10},"task":{"id":100},"user":{"id":1000}}"#,
        )
        .unwrap();
        let invoice = entry.invoice.unwrap();
        assert_eq!((invoice.id, invoice.number.as_deref()), (77, Some("1042")));
    }

    #[test]
    fn invoices_keep_their_decimals_and_lines() {
        let invoice: ApiInvoice = serde_json::from_str(
            r#"{"id":77,"client":{"id":5,"name":"Acme"},"number":"1042",
                "purchase_order":"","amount":1089.00,"due_amount":489.00,
                "tax":8.875,"tax_amount":89.00,"tax2":null,"tax2_amount":0.0,
                "discount":null,"discount_amount":0.0,"subject":"Work","notes":null,
                "currency":"USD","state":"open","period_start":"2026-09-01",
                "period_end":"2026-09-28","issue_date":"2026-09-30","due_date":"2026-10-30",
                "paid_date":null,"updated_at":"2026-10-01T10:00:00Z",
                "line_items":[{"id":1,"kind":"Service","description":"Design",
                  "quantity":10.5,"unit_price":95.2381,"amount":1000.0,"taxed":true,
                  "taxed2":false,"project":{"id":10,"name":"Site"}}]}"#,
        )
        .unwrap();
        assert_eq!(invoice.tax.unwrap().to_string(), "8.875");
        assert_eq!(invoice.amount.to_string(), "1089.00");
        assert_eq!(invoice.line_items[0].unit_price.to_string(), "95.2381");
        assert_eq!(invoice.line_items[0].project.as_ref().unwrap().id, 10);
    }
}
