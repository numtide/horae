//! Invoice authority and the one invoice list query.
//!
//! Invoices are read with `InvoiceReadAll` and generated, edited and moved
//! between statuses with `InvoiceWriteAll`. The managed grants
//! (`InvoiceReadManaged`, `InvoiceDraftWriteManaged`, `InvoiceWriteManaged`)
//! authorize nothing yet: which invoices a person manages is still open (T006,
//! `specs/015-scoped-permissions/contracts/invoice-scope-evidence.md`), and a
//! guessed rule could expose a whole invoice through one of its projects.
//!
//! The invoice payloads carry the rates and amounts the invoice itself
//! recorded, never a person's cost rate.

use horae_core::permissions::catalog::Permission;
use horae_core::types::InvoiceStatus;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Actor, OpError};
use crate::models::Invoice;

pub(crate) const READ: Permission = Permission::InvoiceReadAll;
pub(crate) const WRITE: Permission = Permission::InvoiceWriteAll;

const WRITE_REQUIRED: &str = "Permission to manage all invoices is required";

/// Refuses an actor who may not manage every invoice.
pub(crate) fn authorize_write(actor: &Actor) -> Result<(), OpError> {
    if actor.has(WRITE) {
        Ok(())
    } else {
        Err(OpError::forbidden(WRITE_REQUIRED))
    }
}

/// The organization's invoices, newest first, optionally of one status or one
/// client. It does not authorize: callers hold the actor's [`READ`] grant in
/// the same transaction first.
pub(crate) async fn list(
    connection: &mut PgConnection,
    org_id: Uuid,
    status: Option<InvoiceStatus>,
    client_id: Option<Uuid>,
) -> Result<Vec<Invoice>, sqlx::Error> {
    sqlx::query_as!(
        Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes, terms_days, po_number,
                  discount_bps, tax1_bps, tax2_name, tax2_bps,
                  subtotal_cents, discount_cents, tax1_cents, tax2_cents,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE org_id = $1
             AND ($2::invoice_status IS NULL OR status = $2)
             AND ($3::uuid IS NULL OR client_id = $3)
           ORDER BY created_at DESC, id DESC"#,
        org_id,
        status as Option<InvoiceStatus>,
        client_id,
    )
    .fetch_all(connection)
    .await
}
