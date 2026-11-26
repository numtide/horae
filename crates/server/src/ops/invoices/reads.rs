//! Invoices for `/api/v1` (spec 028): [`page`] walks the invoices of the
//! Invoices page in `id` order, [`get`] reads one as its detail page does, and
//! [`require`] refuses what [`get`] would refuse, for the PDF. Each needs the
//! same whole-invoice read grant ([`READ`]) as the web app.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::invoice::is_overdue;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::super::{Actor, OpError, Tx};
use super::{READ, authorize_read, client_names, detail, fetch_invoice_metadata, imported};
use crate::models::Invoice;
use crate::models::invoice::{InvoiceDetailView, InvoiceListItem};

const NOT_FOUND: &str = "Invoice not found";

/// One page of [`page`].
pub struct InvoiceQuery {
    pub status: Option<InvoiceStatus>,
    pub client_id: Option<Uuid>,
    /// Only invoices whose `updated_at` is at or after it.
    pub updated_since: Option<DateTime<Utc>>,
    /// The last id of the previous page.
    pub after: Option<Uuid>,
    pub limit: u32,
}

/// An invoice of the list, with when it last changed.
#[derive(Debug)]
pub struct InvoiceRecord {
    pub item: InvoiceListItem,
    pub updated_at: DateTime<Utc>,
}

/// An invoice with its lines, as its detail page shows it, with when it last
/// changed.
#[derive(Debug)]
pub struct InvoiceDetailRecord {
    pub detail: InvoiceDetailView,
    pub updated_at: DateTime<Utc>,
}

/// The stored invoice and its update time, which `Invoice` does not carry.
struct Row {
    id: Uuid,
    org_id: Uuid,
    client_id: Uuid,
    number: String,
    status: InvoiceStatus,
    issued_on: NaiveDate,
    due_on: NaiveDate,
    currency: String,
    total_cents: i64,
    notes: Option<String>,
    terms_days: i32,
    po_number: String,
    discount_bps: i16,
    tax1_bps: i16,
    tax2_name: Option<String>,
    tax2_bps: Option<i16>,
    subtotal_cents: i64,
    discount_cents: i64,
    tax1_cents: i64,
    tax2_cents: i64,
    subject: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Row {
    fn split(self) -> (Invoice, DateTime<Utc>) {
        let invoice = Invoice {
            id: self.id,
            org_id: self.org_id,
            client_id: self.client_id,
            number: self.number,
            status: self.status,
            issued_on: self.issued_on,
            due_on: self.due_on,
            currency: self.currency,
            total_cents: self.total_cents,
            terms_days: self.terms_days,
            po_number: self.po_number,
            discount_bps: self.discount_bps,
            tax1_bps: self.tax1_bps,
            tax2_name: self.tax2_name,
            tax2_bps: self.tax2_bps,
            subtotal_cents: self.subtotal_cents,
            discount_cents: self.discount_cents,
            tax1_cents: self.tax1_cents,
            tax2_cents: self.tax2_cents,
            notes: self.notes,
            subject: self.subject,
            created_at: self.created_at,
        };
        (invoice, self.updated_at)
    }
}

/// The invoices of the Invoices page in `id` order, optionally of one status
/// or one client: up to `limit + 1` rows, the extra one saying that another
/// page follows.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    query: &InvoiceQuery,
) -> Result<Vec<InvoiceRecord>, OpError> {
    authorize_read(actor, READ)?;
    let org_id = actor.org_id();
    let rows = sqlx::query_as!(
        Row,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes, terms_days, po_number,
                  discount_bps, tax1_bps, tax2_name, tax2_bps,
                  subtotal_cents, discount_cents, tax1_cents, tax2_cents, subject,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>",
                  updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE org_id = $1
             AND ($2::invoice_status IS NULL OR status = $2)
             AND ($3::uuid IS NULL OR client_id = $3)
             AND ($4::timestamptz IS NULL OR updated_at >= $4)
             AND id > $5
           ORDER BY id LIMIT $6"#,
        org_id,
        query.status as Option<InvoiceStatus>,
        query.client_id,
        query.updated_since as Option<DateTime<Utc>>,
        query.after.unwrap_or(Uuid::nil()),
        i64::from(query.limit) + 1,
    )
    .fetch_all(&mut **tx)
    .await?;
    let client_ids: Vec<_> = rows.iter().map(|row| row.client_id).collect();
    let names = client_names(tx, org_id, &client_ids).await?;
    let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let mut imported = imported(tx, org_id, &ids).await?;
    let today = crate::timezone::today_for_org(&mut **tx, org_id).await?;
    rows.into_iter()
        .map(|row| {
            let (invoice, updated_at) = row.split();
            let client_name = names
                .get(&invoice.client_id)
                .cloned()
                .ok_or_else(|| OpError::not_found("Invoice client not found"))?;
            Ok(InvoiceRecord {
                item: InvoiceListItem {
                    overdue: is_overdue(invoice.status, invoice.due_on, today),
                    imported: imported.remove(&invoice.id),
                    invoice,
                    client_name,
                },
                updated_at,
            })
        })
        .collect()
}

/// One invoice of [`page`], with its lines, through the detail page's read.
pub async fn get(
    tx: &mut Tx,
    actor: &Actor,
    invoice_id: Uuid,
) -> Result<InvoiceDetailRecord, OpError> {
    let detail = detail(tx, actor, invoice_id).await?;
    let updated_at = sqlx::query_scalar!(
        r#"SELECT updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
           FROM invoices WHERE id = $1 AND org_id = $2"#,
        invoice_id,
        actor.org_id(),
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    Ok(InvoiceDetailRecord { detail, updated_at })
}

/// Refuses an invoice [`get`] would refuse: without the read grant, or not one
/// of the actor's organization.
pub async fn require(tx: &mut Tx, actor: &Actor, invoice_id: Uuid) -> Result<(), OpError> {
    authorize_read(actor, READ)?;
    fetch_invoice_metadata(&mut **tx, invoice_id, actor.org_id())
        .await?
        .map(|_| ())
        .ok_or_else(|| OpError::not_found(NOT_FOUND))
}

#[cfg(test)]
mod tests;
