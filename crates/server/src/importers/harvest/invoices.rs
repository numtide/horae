//! The invoice phase of an API import (spec 025): after the catalog, before
//! time entries. Each Harvest invoice is created, updated, skipped or reported,
//! and the run cache remembers what became of it so the entries that follow
//! can be linked.
//!
//! Every check runs before an invoice's first write, so a reported invoice
//! leaves nothing behind without a savepoint of its own. A savepoint per
//! invoice made every later lookup of the same transaction slower: with 2,000
//! of them, a 15,000-entry import took two thirds longer (SC-006). A database
//! error still fails the run, as the transaction cannot continue after it.
//!
//! Imported invoices raise no plugin events (FR-023): they were sent and paid
//! in Harvest.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::importers::harvest::convert::{self, MoneyError};
use horae_core::importers::harvest::invoice::{self, Amounts, AmountsError, Mapped};
use horae_core::importers::harvest::types::{EntityType, InvoiceErrorKind, RowOutcome};
use horae_core::types::InvoiceStatus;
use sqlx::{PgConnection, Postgres, Transaction};
use uuid::Uuid;

use super::api_source::{ApiInvoice, HarvestData, decimal};
use super::report::ImportReport;
use super::resolve::{InvoiceSlot, RunCache};

enum Failure {
    /// The invoice is reported and nothing of it written.
    Invoice {
        kind: Option<InvoiceErrorKind>,
        reason: String,
    },
    /// The transaction is aborted: the run fails.
    Database(sqlx::Error),
}

impl Failure {
    fn new(kind: InvoiceErrorKind, reason: impl Into<String>) -> Self {
        Self::Invoice {
            kind: Some(kind),
            reason: reason.into(),
        }
    }
}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// The longest purchase order and subject an invoice keeps (the columns'
/// checks), refused before anything is written.
const MAX_TEXT: usize = 200;

impl From<MoneyError> for Failure {
    fn from(error: MoneyError) -> Self {
        let kind = match error {
            MoneyError::UnknownCurrency(_) => InvoiceErrorKind::UnknownCurrency,
            MoneyError::Precision { .. } | MoneyError::Invalid(_) => {
                InvoiceErrorKind::AmountPrecision
            }
        };
        Self::new(kind, error.to_string())
    }
}

impl From<AmountsError> for Failure {
    fn from(error: AmountsError) -> Self {
        let kind = match error {
            AmountsError::NegativeTotal(_) => InvoiceErrorKind::NegativeTotal,
            AmountsError::Mismatch { .. } | AmountsError::OutOfRange(_) => {
                InvoiceErrorKind::AmountsMismatch
            }
        };
        Self::new(kind, error.to_string())
    }
}

/// Apply the catalog's invoices at positions `range`, in Harvest's order.
pub(super) async fn apply_batch(
    tx: &mut Transaction<'_, Postgres>,
    cache: &mut RunCache,
    org_id: Uuid,
    data: &HarvestData,
    report: &mut ImportReport,
    range: std::ops::Range<usize>,
) -> anyhow::Result<()> {
    for source in data.invoices.iter().skip(range.start).take(range.len()) {
        let location = format!("invoice {} ({})", source.id, source.number);
        let status = match invoice::map_state(&source.state) {
            Ok(Mapped::Skip) => {
                cache.set_invoice(source.id, InvoiceSlot::Draft);
                match existing(tx, org_id, source.id).await? {
                    Some((id, status, version)) => {
                        if status == InvoiceStatus::Void {
                            report.invoices_skipped_drafts += 1;
                        } else {
                            report.summary.links.unlinked +=
                                reopen_time(tx, org_id, id, Submitted::Keep).await?;
                            report.invoices_voided_drafts += 1;
                        }
                        if status != InvoiceStatus::Void || version != source.updated_at {
                            void_as_draft(tx, org_id, id, source.updated_at).await?;
                        }
                    }
                    None => report.invoices_skipped_drafts += 1,
                }
                continue;
            }
            Ok(Mapped::Import(status)) => status,
            Err(error) => {
                cache.set_invoice(source.id, InvoiceSlot::Failed);
                report.record_invoice_error(
                    location,
                    error.to_string(),
                    Some(InvoiceErrorKind::UnknownState),
                );
                continue;
            }
        };
        let existing = existing(tx, org_id, source.id).await?;
        match apply_one(tx, cache, org_id, data, source, status, existing).await {
            Ok((outcome, slot, freed)) => {
                let was_void =
                    existing.is_some_and(|(_, current, _)| current == InvoiceStatus::Void);
                if was_void && matches!(slot, InvoiceSlot::Imported { voided: false, .. }) {
                    forget_invoice_mark(tx, org_id).await?;
                }
                cache.set_invoice(source.id, slot);
                report.record(EntityType::Invoice, &outcome);
                if let Some(reopened) = freed {
                    report.invoices_removed.push(source.number.clone());
                    report.summary.links.unlinked += reopened;
                }
            }
            Err(Failure::Database(error)) => return Err(error.into()),
            Err(Failure::Invoice { kind, reason }) => {
                // An invoice imported by an earlier run keeps its links.
                cache.set_invoice(
                    source.id,
                    existing.map_or(InvoiceSlot::Failed, |(id, status, _)| {
                        InvoiceSlot::Imported {
                            id,
                            voided: status == InvoiceStatus::Void,
                        }
                    }),
                );
                report.record_invoice_error(location, reason, kind);
            }
        }
    }
    if range.end >= data.invoices.len() {
        report.invoices_no_longer_listed = no_longer_listed(tx, org_id, data).await?;
    }
    Ok(())
}

/// Voids an imported invoice that Harvest turned back into a draft (Q14) and
/// keeps the draft's version, so the next run changes nothing more while the
/// draft stands, and the invoice comes back as changed once Harvest issues it
/// again (`voided_versions` then has the run read every entry).
async fn void_as_draft(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    invoice_id: Uuid,
    version: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE invoices SET status = 'void'
          WHERE id = $1 AND org_id = $2 AND source = 'harvest' AND status <> 'void'",
        invoice_id,
        org_id,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "UPDATE harvest_invoices SET harvest_updated_at = $3
          WHERE invoice_id = $1 AND org_id = $2",
        invoice_id,
        org_id,
        version as DateTime<Utc>,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Removes an imported invoice that Harvest deleted, once a new Harvest
/// invoice takes its number (Q13): Harvest suggests the highest number plus
/// one, so deleting the latest invoice hands its number to the next. Its time
/// reopens first; Harvest unbilled it too, and the entry phase links it again
/// if the new invoice bills it. Returns the entries reopened.
async fn remove_deleted(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    invoice_id: Uuid,
) -> sqlx::Result<u64> {
    let reopened = reopen_time(tx, org_id, invoice_id, Submitted::Detach).await?;
    sqlx::query!(
        "DELETE FROM harvest_import_map
          WHERE org_id = $1 AND harvest_entity_type = 'invoice'::harvest_entity_type
            AND horae_id = $2",
        org_id,
        invoice_id,
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM invoices WHERE id = $1 AND org_id = $2 AND source = 'harvest'",
        invoice_id,
        org_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(reopened)
}

/// Reopens the time of an imported invoice that Harvest turned back into a
/// draft or deleted. A draft's entries' own versions probably do not move in
/// Harvest, so an incremental run would never read them; Horae knows them by
/// the invoice. Submitted entries keep their state, and their link too unless
/// the invoice is going away. Provenance takes the new version of an entry not
/// edited in Horae, as `apply::write_links` does. Returns the entries reopened.
async fn reopen_time(
    tx: &mut Transaction<'_, Postgres>,
    org_id: Uuid,
    invoice_id: Uuid,
    submitted: Submitted,
) -> sqlx::Result<u64> {
    let reopened = sqlx::query_scalar!(
        r#"WITH before AS (
             SELECT id, updated_at, created_at, state = 'submitted' AS submitted
               FROM time_entries
              WHERE org_id = $1 AND invoice_id = $2 AND (state <> 'submitted' OR $3)),
           written AS (
             UPDATE time_entries AS te
                SET invoice_id = NULL,
                    state = CASE WHEN before.submitted THEN te.state ELSE 'open' END,
                    rounded_minutes = CASE WHEN before.submitted THEN te.rounded_minutes END
               FROM before WHERE te.id = before.id
             RETURNING te.id, te.updated_at),
           refreshed AS (
             UPDATE harvest_import_map AS m SET horae_updated_at = written.updated_at
               FROM written JOIN before ON before.id = written.id
              WHERE m.org_id = $1 AND m.harvest_entity_type = 'time_entry'::harvest_entity_type
                AND m.horae_id = written.id
                AND m.horae_updated_at IS DISTINCT FROM written.updated_at
                AND before.updated_at IS NOT DISTINCT FROM COALESCE(m.horae_updated_at, before.created_at))
           SELECT count(*) FILTER (WHERE NOT before.submitted) AS "count!"
             FROM written JOIN before ON before.id = written.id"#,
        org_id,
        invoice_id,
        submitted == Submitted::Detach,
    )
    .fetch_one(&mut **tx)
    .await?;
    Ok(reopened.unsigned_abs())
}

/// What `reopen_time` does with a submitted entry on the invoice.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Submitted {
    /// Leave it, link included: the invoice stays.
    Keep,
    /// Unlink it and keep its state: the invoice is being removed.
    Detach,
}

/// Has every later run read entries in full, until one finishes without
/// errors and stores the mark again. A voided invoice that Harvest issued
/// again needs its time linked by a full read, and on the job path the batch
/// that brings it back commits before the entries: a job that stops after it
/// would leave the next one incremental (review 3, M1).
async fn forget_invoice_mark(tx: &mut Transaction<'_, Postgres>, org_id: Uuid) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE harvest_credentials SET synced_watermark = synced_watermark - 'invoice'
          WHERE org_id = $1",
        org_id,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The Harvest versions of imported invoices voided in Horae. A void reopens
/// the time and stands until Harvest changes the invoice; when it does, the
/// invoice comes back but its entries, whose own versions did not move, are
/// read only by a full read (`api_source::entry_scan`).
pub(super) async fn voided_versions(
    conn: &mut PgConnection,
    org_id: Uuid,
) -> sqlx::Result<Vec<(i64, DateTime<Utc>)>> {
    let rows = sqlx::query!(
        r#"SELECT h.harvest_invoice_id,
                  h.harvest_updated_at AS "harvest_updated_at: DateTime<Utc>"
           FROM harvest_invoices h JOIN invoices i ON i.id = h.invoice_id AND i.org_id = h.org_id
           WHERE h.org_id = $1 AND i.status = 'void'"#,
        org_id,
    )
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.harvest_invoice_id, row.harvest_updated_at))
        .collect())
}

type Existing = Option<(Uuid, InvoiceStatus, DateTime<Utc>)>;

/// The invoice an earlier run imported for this Harvest id, its status in
/// Horae and the Harvest version it was written from.
async fn existing(
    conn: &mut PgConnection,
    org_id: Uuid,
    harvest_id: i64,
) -> sqlx::Result<Existing> {
    let row = sqlx::query!(
        r#"SELECT i.id, i.status AS "status: InvoiceStatus",
                  h.harvest_updated_at AS "harvest_updated_at: DateTime<Utc>"
           FROM harvest_invoices h JOIN invoices i ON i.id = h.invoice_id AND i.org_id = h.org_id
           WHERE h.org_id = $1 AND h.harvest_invoice_id = $2"#,
        org_id,
        harvest_id,
    )
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|row| (row.id, row.status, row.harvest_updated_at)))
}

async fn no_longer_listed(
    conn: &mut PgConnection,
    org_id: Uuid,
    data: &HarvestData,
) -> sqlx::Result<u64> {
    let listed: Vec<i64> = data.invoices.iter().map(|invoice| invoice.id).collect();
    let count = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM harvest_invoices
           WHERE org_id = $1 AND NOT (harvest_invoice_id = ANY($2))"#,
        org_id,
        &listed,
    )
    .fetch_one(conn)
    .await?;
    Ok(u64::try_from(count).unwrap_or(0))
}

struct Line {
    position: i32,
    harvest_id: i64,
    kind: String,
    description: String,
    quantity: String,
    /// Exact decimal text, display only (FR-004): it may have more decimals
    /// than the currency, and nothing sums it.
    unit_price: String,
    amount_cents: i64,
    taxed: bool,
    taxed2: bool,
    project_id: Option<Uuid>,
}

/// What the columns would refuse, refused first: an over-long purchase order
/// or subject, a NUL character, or a quantity or rate that is no plain decimal.
fn check_texts(source: &ApiInvoice, lines: &[Line]) -> Result<(), Failure> {
    let refuse = |reason: String| Err(Failure::Invoice { kind: None, reason });
    let po_number = source.purchase_order.as_deref().unwrap_or_default();
    let subject = source.subject.as_deref().unwrap_or_default();
    for (what, text) in [("purchase order", po_number), ("subject", subject)] {
        if text.chars().count() > MAX_TEXT {
            return refuse(format!("the {what} is longer than {MAX_TEXT} characters"));
        }
    }
    let texts = [Some(source.number.as_str()), Some(po_number), Some(subject)]
        .into_iter()
        .chain([source.notes.as_deref()])
        .flatten()
        .chain(
            lines
                .iter()
                .flat_map(|l| [l.kind.as_str(), l.description.as_str()]),
        );
    if texts.into_iter().any(|text| text.contains('\0')) {
        return refuse("a text contains a NUL character".into());
    }
    let plain = |text: &str| {
        let digits = text.strip_prefix('-').unwrap_or(text);
        !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            && digits.bytes().filter(|&b| b == b'.').count() <= 1
    };
    let rates = [&source.discount, &source.tax, &source.tax2]
        .into_iter()
        .flatten()
        .map(decimal);
    let quantities = lines
        .iter()
        .flat_map(|line| [line.quantity.clone(), line.unit_price.clone()]);
    if let Some(bad) = rates.chain(quantities).find(|text| !plain(text)) {
        return refuse(format!("{bad} is not a decimal number"));
    }
    Ok(())
}

/// What `apply_one` did: the outcome, the run cache's slot, and, when it
/// removed a deleted invoice to free the number, the entries that reopened.
type Applied = (RowOutcome, InvoiceSlot, Option<u64>);

async fn apply_one(
    tx: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org_id: Uuid,
    data: &HarvestData,
    source: &ApiInvoice,
    status: InvoiceStatus,
    existing: Existing,
) -> Result<Applied, Failure> {
    // Harvest's own version unchanged: a status set in Horae since stands.
    if let Some((id, current, version)) = existing
        && version == source.updated_at
    {
        return Ok((
            RowOutcome::Skipped,
            InvoiceSlot::Imported {
                id,
                voided: current == InvoiceStatus::Void,
            },
            None,
        ));
    }
    let client_id = cache
        .client_by_harvest_id(source.client.id)
        .ok_or_else(|| {
            Failure::new(
                InvoiceErrorKind::ClientMissing,
                format!("client {} was not imported", source.client.id),
            )
        })?;
    let currency = source.currency.trim();
    let money = |amount: Option<&serde_json::Number>| -> Result<i64, Failure> {
        amount.map_or(Ok(0), |amount| {
            Ok(convert::money_exact(&decimal(amount), currency)?)
        })
    };
    let amounts = Amounts {
        discount: money(source.discount_amount.as_ref())?,
        tax1: money(source.tax_amount.as_ref())?,
        tax2: money(source.tax2_amount.as_ref())?,
        total: money(Some(&source.amount))?,
        due: money(Some(&source.due_amount))?,
    };
    let mut lines = Vec::with_capacity(source.line_items.len());
    for (position, line) in source.line_items.iter().enumerate() {
        let kind = line.kind.trim();
        lines.push(Line {
            position: i32::try_from(position)
                .map_err(|_| Failure::new(InvoiceErrorKind::AmountsMismatch, "too many lines"))?,
            harvest_id: line.id,
            kind: if kind.is_empty() { "-" } else { kind }
                .chars()
                .take(100)
                .collect(),
            description: line.description.clone().unwrap_or_default(),
            quantity: decimal(&line.quantity),
            unit_price: decimal(&line.unit_price),
            amount_cents: money(Some(&line.amount))?,
            taxed: line.taxed,
            taxed2: line.taxed2,
            project_id: line
                .project
                .as_ref()
                .and_then(|project| cache.project_by_harvest_id(project.id)),
        });
    }
    let line_amounts: Vec<i64> = lines.iter().map(|line| line.amount_cents).collect();
    invoice::check_amounts(&line_amounts, amounts)?;
    check_texts(source, &lines)?;

    let id = existing.map_or_else(Uuid::now_v7, |(id, _, _)| id);
    let taken = sqlx::query!(
        r#"SELECT i.id, h.harvest_invoice_id AS "harvest_id?"
           FROM invoices i LEFT JOIN harvest_invoices h ON h.invoice_id = i.id
           WHERE i.org_id = $1 AND i.number = $2 AND i.id <> $3"#,
        org_id,
        source.number,
        id,
    )
    .fetch_optional(&mut **tx)
    .await?;
    let mut freed = None;
    if let Some(holder) = taken {
        let listed = |harvest_id| data.invoices.iter().any(|other| other.id == harvest_id);
        match holder.harvest_id {
            // Every run lists every invoice, so one missing was deleted in Harvest.
            Some(harvest_id) if !listed(harvest_id) => {
                freed = Some(remove_deleted(tx, org_id, holder.id).await?);
            }
            imported => {
                let holder = if imported.is_some() {
                    "Another invoice imported from Harvest"
                } else {
                    "An invoice made in Horae"
                };
                return Err(Failure::new(
                    InvoiceErrorKind::NumberTaken,
                    format!("{holder} already uses number {}", source.number),
                ));
            }
        }
    }

    let due_on = source.due_date.unwrap_or(source.issue_date);
    let po_number = source.purchase_order.clone().unwrap_or_default();
    let subject = source.subject.clone().unwrap_or_default();
    let notes = source
        .notes
        .clone()
        .filter(|notes| !notes.trim().is_empty());
    let rate = |rate: &Option<serde_json::Number>| rate.as_ref().map(decimal);
    if existing.is_some() {
        sqlx::query!(
            "UPDATE invoices
                SET client_id = $3, number = $4, status = $5, issued_on = $6, due_on = $7,
                    currency = $8, total_cents = $9, notes = $10, po_number = $11,
                    discount_cents = $12, tax1_cents = $13, tax2_cents = $14, subject = $15
              WHERE id = $1 AND org_id = $2 AND source = 'harvest'",
            id,
            org_id,
            client_id,
            source.number,
            status as InvoiceStatus,
            source.issue_date as NaiveDate,
            due_on as NaiveDate,
            currency,
            amounts.total,
            notes,
            po_number,
            amounts.discount,
            amounts.tax1,
            amounts.tax2,
            subject,
        )
        .execute(&mut **tx)
        .await?;
        sqlx::query!(
            "DELETE FROM harvest_invoice_lines WHERE invoice_id = $1",
            id
        )
        .execute(&mut **tx)
        .await?;
    } else {
        sqlx::query!(
            "INSERT INTO invoices
               (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents,
                notes, po_number, discount_cents, tax1_cents, tax2_cents, subject, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 'harvest')",
            id,
            org_id,
            client_id,
            source.number,
            status as InvoiceStatus,
            source.issue_date as NaiveDate,
            due_on as NaiveDate,
            currency,
            amounts.total,
            notes,
            po_number,
            amounts.discount,
            amounts.tax1,
            amounts.tax2,
            subject,
        )
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query!(
        "INSERT INTO harvest_invoices
           (invoice_id, org_id, harvest_invoice_id, due_cents, paid_on, discount_rate,
            tax1_rate, tax2_rate, period_start, period_end, harvest_updated_at)
         VALUES ($1, $2, $3, $4, $5, $6::text::numeric, $7::text::numeric, $8::text::numeric,
                 $9, $10, $11)
         ON CONFLICT (invoice_id) DO UPDATE SET
           due_cents = EXCLUDED.due_cents, paid_on = EXCLUDED.paid_on,
           discount_rate = EXCLUDED.discount_rate, tax1_rate = EXCLUDED.tax1_rate,
           tax2_rate = EXCLUDED.tax2_rate, period_start = EXCLUDED.period_start,
           period_end = EXCLUDED.period_end, harvest_updated_at = EXCLUDED.harvest_updated_at",
        id,
        org_id,
        source.id,
        amounts.due,
        source.paid_date as Option<NaiveDate>,
        rate(&source.discount),
        rate(&source.tax),
        rate(&source.tax2),
        source.period_start as Option<NaiveDate>,
        source.period_end as Option<NaiveDate>,
        source.updated_at as DateTime<Utc>,
    )
    .execute(&mut **tx)
    .await?;
    let ids: Vec<Uuid> = lines.iter().map(|_| Uuid::now_v7()).collect();
    let positions: Vec<i32> = lines.iter().map(|line| line.position).collect();
    let harvest_ids: Vec<i64> = lines.iter().map(|line| line.harvest_id).collect();
    let kinds: Vec<String> = lines.iter().map(|line| line.kind.clone()).collect();
    let descriptions: Vec<String> = lines.iter().map(|l| l.description.clone()).collect();
    let quantities: Vec<String> = lines.iter().map(|line| line.quantity.clone()).collect();
    let prices: Vec<String> = lines.iter().map(|line| line.unit_price.clone()).collect();
    let taxed: Vec<bool> = lines.iter().map(|line| line.taxed).collect();
    let taxed2: Vec<bool> = lines.iter().map(|line| line.taxed2).collect();
    let projects: Vec<Option<Uuid>> = lines.iter().map(|line| line.project_id).collect();
    sqlx::query!(
        "INSERT INTO harvest_invoice_lines
           (id, org_id, invoice_id, position, harvest_line_id, kind, description, quantity,
            unit_price, amount_cents, taxed, taxed2, project_id)
         SELECT l.id, $2, $3, l.position, l.harvest_id, l.kind, l.description,
                l.quantity::numeric, l.price::numeric, l.amount, l.taxed, l.taxed2, l.project
         FROM unnest($1::uuid[], $4::int4[], $5::int8[], $6::text[], $7::text[], $8::text[],
                     $9::text[], $10::int8[], $11::bool[], $12::bool[], $13::uuid[])
              AS l(id, position, harvest_id, kind, description, quantity, price, amount,
                   taxed, taxed2, project)",
        &ids,
        org_id,
        id,
        &positions,
        &harvest_ids,
        &kinds,
        &descriptions,
        &quantities,
        &prices,
        &line_amounts,
        &taxed,
        &taxed2,
        &projects as &[Option<Uuid>],
    )
    .execute(&mut **tx)
    .await?;
    super::provenance::upsert(
        &mut **tx,
        org_id,
        EntityType::Invoice,
        source.id,
        id,
        Some(source.updated_at),
    )
    .await?;
    Ok((
        if existing.is_some() {
            RowOutcome::Updated
        } else {
            RowOutcome::Created
        },
        InvoiceSlot::Imported {
            id,
            voided: status == InvoiceStatus::Void,
        },
        freed,
    ))
}
