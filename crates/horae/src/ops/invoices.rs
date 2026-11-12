//! Invoice authority and the one invoice list query.
//!
//! Invoices are read with `InvoiceReadAll` and generated, edited and moved
//! between statuses with `InvoiceWriteAll`. The managed grants
//! (`InvoiceReadManaged`, `InvoiceDraftWriteManaged`, `InvoiceWriteManaged`)
//! authorize nothing yet: which invoices a person manages is still open (T006,
//! `specs/015-scoped-permissions/contracts/invoice-scope-evidence.md`), and a
//! guessed rule could expose a whole invoice through one of its projects.
//!
//! Preparing and generating an invoice also requires `BillableRateReadAll`:
//! they price uninvoiced work from the current billable rates and show each
//! project's invoice defaults and fee balances. An issued invoice carries the
//! rates and amounts it recorded, which its invoice grants alone disclose.
//! No invoice payload carries a cost rate.

use horae_core::permissions::catalog::Permission;
use horae_core::types::InvoiceStatus;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Actor, OpError};
use crate::models::Invoice;

/// Reading issued invoices, including their exports.
pub(crate) const READ: &[Permission] = &[Permission::InvoiceReadAll];
/// Editing drafts and changing an invoice's status.
pub(crate) const WRITE: &[Permission] = &[Permission::InvoiceWriteAll];
/// Previewing and generating an invoice from uninvoiced work.
pub(crate) const PREPARE: &[Permission] =
    &[Permission::InvoiceWriteAll, Permission::BillableRateReadAll];

const WRITE_REQUIRED: &str = "Permission to manage all invoices is required";
const PREPARE_REQUIRED: &str = "Preparing an invoice requires permission to manage all invoices and view everyone's billable rates";

/// Whether the actor holds every `required` grant.
pub(crate) fn holds(actor: &Actor, required: &[Permission]) -> bool {
    required.iter().all(|&permission| actor.has(permission))
}

/// Refuses an actor who may not manage every invoice.
pub(crate) fn authorize_write(actor: &Actor) -> Result<(), OpError> {
    if holds(actor, WRITE) {
        Ok(())
    } else {
        Err(OpError::forbidden(WRITE_REQUIRED))
    }
}

/// Refuses an actor who may not price uninvoiced work into a new invoice.
pub(crate) fn authorize_prepare(actor: &Actor) -> Result<(), OpError> {
    if holds(actor, PREPARE) {
        Ok(())
    } else {
        Err(OpError::forbidden(PREPARE_REQUIRED))
    }
}

/// The organization's invoices, newest first, optionally of one status or one
/// client. It does not authorize: callers hold the actor's [`READ`] grants in
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
