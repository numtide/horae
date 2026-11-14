//! Invoices: reading, preparing, generating, editing drafts and moving them
//! between statuses.
//!
//! Invoices are read with `InvoiceReadAll` and generated, edited and moved
//! between statuses with `InvoiceWriteAll`. The managed grants
//! (`InvoiceReadManaged`, `InvoiceDraftWriteManaged`, `InvoiceWriteManaged`)
//! authorize nothing yet: which invoices a person manages is still open (T006,
//! `specs/015-scoped-permissions/contracts/invoice-scope-evidence.md`), and a
//! guessed rule could expose a whole invoice through one of its projects.
//!
//! Preparing, generating and editing a draft also require
//! `BillableRateReadAll`: they price uninvoiced work from the current billable
//! rates, or show each project's invoice defaults, agreed fees and what other
//! invoices billed against them. An issued invoice carries the rates and
//! amounts it recorded, which its invoice grants alone disclose. No invoice
//! payload carries a cost rate.
//!
//! Reads run on [`super::begin_read_snapshot`] with
//! [`super::SnapshotLimits::LockWait`], so every statement sees the snapshot
//! the actor's authority was taken in. Writes run on [`super::begin_gated`]
//! with the `Shared` organization gate and then serialize on the
//! organization's invoice lock ([`balances::lock_invoices`]).

use horae_core::permissions::catalog::Permission;
use horae_core::types::InvoiceStatus;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::invoice::{InvoiceDetailView, InvoiceListItem};
use crate::models::{Invoice, InvoiceWithLines};
use crate::plugin::AppEvent;
use crate::plugin::event::invoice_payload;

pub(crate) mod balances;
pub(crate) mod defaults;
pub(crate) mod editing;
mod entries;
mod fees;
mod generate;
pub(crate) mod preview;

pub(crate) use generate::{Generation, generate};

/// Reading issued invoices, including their exports.
pub(crate) const READ: &[Permission] = &[Permission::InvoiceReadAll];
/// Changing an invoice's status.
pub(crate) const WRITE: &[Permission] = &[Permission::InvoiceWriteAll];
/// Previewing and generating an invoice from uninvoiced work, and editing a
/// draft's values and fee lines.
pub(crate) const PREPARE: &[Permission] =
    &[Permission::InvoiceWriteAll, Permission::BillableRateReadAll];

const ACCESS_REQUIRED: &str = "Current access is required";
const WRITE_REQUIRED: &str = "Permission to manage all invoices is required";
const PREPARE_REQUIRED: &str = "Preparing or editing an invoice requires permission to manage all invoices and view everyone's billable rates";

/// Whether the actor holds every `required` grant.
pub(crate) fn holds(actor: &Actor, required: &[Permission]) -> bool {
    required.iter().all(|&permission| actor.has(permission))
}

/// Refuses a read unless the actor holds every `required` grant: [`READ`]
/// for issued invoices, [`PREPARE`] for previews and the draft editor.
pub(crate) fn authorize_read(actor: &Actor, required: &[Permission]) -> Result<(), OpError> {
    if holds(actor, required) {
        Ok(())
    } else {
        Err(OpError::forbidden(ACCESS_REQUIRED))
    }
}

/// Refuses an actor who may not manage every invoice.
pub(crate) fn authorize_write(actor: &Actor) -> Result<(), OpError> {
    if holds(actor, WRITE) {
        Ok(())
    } else {
        Err(OpError::forbidden(WRITE_REQUIRED))
    }
}

/// Refuses an actor who may not price work into a new invoice or a draft.
/// The write grant is checked first, so its refusal names what is missing.
pub(crate) fn authorize_prepare(actor: &Actor) -> Result<(), OpError> {
    authorize_write(actor)?;
    if holds(actor, PREPARE) {
        Ok(())
    } else {
        Err(OpError::forbidden(PREPARE_REQUIRED))
    }
}

/// Invalid input that no single field is to blame for.
fn bad_request(message: impl std::fmt::Display) -> OpError {
    OpError::Invalid {
        field: None,
        message: message.to_string(),
    }
}

/// Previews read billable sources as they are; generation locks them.
#[derive(Clone, Copy)]
enum SourceRead {
    Preview,
    Generate,
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

/// The invoice list page: every invoice, optionally of one status, with its
/// client's name.
pub(crate) async fn list_items(
    tx: &mut Tx,
    actor: &Actor,
    status: Option<InvoiceStatus>,
) -> Result<Vec<InvoiceListItem>, OpError> {
    authorize_read(actor, READ)?;
    let org_id = actor.org_id();
    let invoices = list(tx, org_id, status, None).await?;
    let client_ids: Vec<_> = invoices.iter().map(|invoice| invoice.client_id).collect();
    let names = client_names(tx, org_id, &client_ids).await?;
    invoices
        .into_iter()
        .map(|invoice| {
            let client_name = names
                .get(&invoice.client_id)
                .cloned()
                .ok_or_else(|| OpError::not_found("Invoice client not found"))?;
            Ok(InvoiceListItem {
                invoice,
                client_name,
            })
        })
        .collect()
}

/// One invoice with its lines and client name.
pub(crate) async fn detail(
    tx: &mut Tx,
    actor: &Actor,
    invoice_id: Uuid,
) -> Result<InvoiceDetailView, OpError> {
    authorize_read(actor, READ)?;
    let org_id = actor.org_id();
    let (invoice, lines) = crate::reports::fetch_invoice_from(tx, invoice_id, org_id)
        .await?
        .ok_or_else(|| OpError::not_found("Invoice not found"))?;
    let client_name = client_names(tx, org_id, &[invoice.client_id])
        .await?
        .remove(&invoice.client_id)
        .ok_or_else(|| OpError::not_found("Invoice client not found"))?;
    Ok(InvoiceDetailView {
        invoice,
        lines,
        client_name,
    })
}

async fn client_names(
    tx: &mut Tx,
    org_id: Uuid,
    client_ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, String>, OpError> {
    // Only identities referenced by the already-authorized invoice read; never
    // the client catalog or its address, tax ID and rate fields.
    let clients = sqlx::query!(
        "SELECT id, name FROM clients WHERE org_id=$1 AND id=ANY($2)",
        org_id,
        client_ids,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(clients
        .into_iter()
        .map(|client| (client.id, client.name))
        .collect())
}

/// Moves an invoice along draft → sent → paid, or voids a draft or sent
/// invoice, reopening its time. Sending, paying and voiding each raise their
/// plugin event.
pub(crate) async fn set_status(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    target: InvoiceStatus,
) -> Result<Done<Invoice>, OpError> {
    authorize_write(actor)?;
    let org_id = actor.org_id();
    balances::lock_invoices(tx, org_id).await?;

    // Validate under the same row lock as the transition: payment and void
    // must not both accept a previously observed 'sent' state.
    let current_status: InvoiceStatus = sqlx::query_scalar!(
        r#"SELECT status as "status: InvoiceStatus"
           FROM invoices WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found("Invoice not found"))?;

    // Enforce state machine: draft->sent, sent->paid, draft|sent->void
    let valid = matches!(
        (current_status, target),
        (InvoiceStatus::Draft, InvoiceStatus::Sent)
            | (InvoiceStatus::Sent, InvoiceStatus::Paid)
            | (InvoiceStatus::Draft, InvoiceStatus::Void)
            | (InvoiceStatus::Sent, InvoiceStatus::Void)
    );
    if !valid {
        return Err(OpError::conflict(format!(
            "Cannot transition invoice from {} to {}",
            current_status, target
        )));
    }

    // Reopened time is editable again, so its previously frozen rounding no
    // longer represents a locked duration. Invoice lines remain unchanged.
    if target == InvoiceStatus::Void {
        sqlx::query!(
            r#"UPDATE time_entries
               SET invoice_id = NULL, state = 'open', rounded_minutes = NULL
               WHERE invoice_id = $1"#,
            id,
        )
        .execute(&mut **tx)
        .await?;
    }

    let invoice = sqlx::query_as!(
        Invoice,
        r#"UPDATE invoices SET status = $3
           WHERE id = $1 AND org_id = $2
           RETURNING id, org_id, client_id, number,
                     status as "status: InvoiceStatus",
                     issued_on as "issued_on: chrono::NaiveDate",
                     due_on as "due_on: chrono::NaiveDate",
                     currency, total_cents, notes, terms_days, po_number,
                     discount_bps, tax1_bps, tax2_name, tax2_bps,
                     subtotal_cents, discount_cents, tax1_cents, tax2_cents,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        org_id,
        target as InvoiceStatus,
    )
    .fetch_one(&mut **tx)
    .await?;

    let occurred_at = chrono::Utc::now();
    let payload = invoice_payload(&invoice);
    let event = match target {
        InvoiceStatus::Sent => Some(AppEvent::InvoiceSent {
            occurred_at,
            org_id,
            invoice: payload,
        }),
        InvoiceStatus::Paid => Some(AppEvent::InvoicePaid {
            occurred_at,
            org_id,
            invoice: payload,
        }),
        InvoiceStatus::Void => Some(AppEvent::InvoiceVoided {
            occurred_at,
            org_id,
            invoice: payload,
        }),
        InvoiceStatus::Draft => None,
    };
    Ok(match event {
        Some(event) => Done::new(invoice).with(event),
        None => Done::new(invoice),
    })
}

/// The invoice and its lines as they are now, after a write.
async fn with_lines(tx: &mut Tx, org_id: Uuid, id: Uuid) -> Result<InvoiceWithLines, OpError> {
    let (invoice, lines) = crate::reports::fetch_invoice_from(tx, id, org_id)
        .await?
        .ok_or_else(|| OpError::not_found("Invoice not found"))?;
    Ok(InvoiceWithLines { invoice, lines })
}
