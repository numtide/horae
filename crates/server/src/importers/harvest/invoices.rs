//! The invoice phase of an API import (spec 025): after the catalog, before
//! time entries. Each Harvest invoice is created, updated, skipped or reported
//! in its own savepoint, and the run cache remembers what became of it so the
//! entries that follow can be linked.
//!
//! Imported invoices raise no plugin events (FR-023): they were sent and paid
//! in Harvest.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::importers::harvest::convert::{self, MoneyError};
use horae_core::importers::harvest::invoice::{self, Amounts, AmountsError, Mapped};
use horae_core::importers::harvest::types::{EntityType, InvoiceErrorKind, RowOutcome};
use horae_core::types::InvoiceStatus;
use sqlx::{Acquire, PgConnection, Postgres, Transaction};
use uuid::Uuid;

use super::api_source::{ApiInvoice, HarvestData, decimal};
use super::report::ImportReport;
use super::resolve::{InvoiceSlot, RunCache};

struct Failure {
    kind: Option<InvoiceErrorKind>,
    reason: String,
}

impl Failure {
    fn new(kind: InvoiceErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind: Some(kind),
            reason: reason.into(),
        }
    }
}

impl From<sqlx::Error> for Failure {
    fn from(error: sqlx::Error) -> Self {
        Self {
            kind: None,
            reason: format!("database error: {error}"),
        }
    }
}

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
                report.invoices_skipped_drafts += 1;
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
        let mut sp = tx.begin().await?;
        match apply_one(&mut sp, cache, org_id, source, status, existing).await {
            Ok((outcome, slot)) => {
                sp.commit().await?;
                cache.set_invoice(source.id, slot);
                report.record(EntityType::Invoice, &outcome);
            }
            Err(failure) => {
                sp.rollback().await?;
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
                report.record_invoice_error(location, failure.reason, failure.kind);
            }
        }
    }
    if range.end >= data.invoices.len() {
        report.invoices_no_longer_listed = no_longer_listed(tx, org_id, data).await?;
    }
    Ok(())
}

type Existing = Option<(Uuid, InvoiceStatus, DateTime<Utc>)>;

/// The invoice an earlier run imported for this Harvest id, its status in
/// Horae and the Harvest version it was written from.
async fn existing(conn: &mut PgConnection, org_id: Uuid, harvest_id: i64) -> sqlx::Result<Existing> {
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
    unit_price_cents: i64,
    amount_cents: i64,
    taxed: bool,
    taxed2: bool,
    project_id: Option<Uuid>,
}

async fn apply_one(
    sp: &mut Transaction<'_, Postgres>,
    cache: &RunCache,
    org_id: Uuid,
    source: &ApiInvoice,
    status: InvoiceStatus,
    existing: Existing,
) -> Result<(RowOutcome, InvoiceSlot), Failure> {
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
        ));
    }
    let client_id = cache.client_by_harvest_id(source.client.id).ok_or_else(|| {
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
            position: i32::try_from(position).map_err(|_| {
                Failure::new(InvoiceErrorKind::AmountsMismatch, "too many lines")
            })?,
            harvest_id: line.id,
            kind: if kind.is_empty() { "-" } else { kind }
                .chars()
                .take(100)
                .collect(),
            description: line.description.clone().unwrap_or_default(),
            quantity: decimal(&line.quantity),
            unit_price_cents: money(Some(&line.unit_price))?,
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

    let id = existing.map_or_else(Uuid::now_v7, |(id, _, _)| id);
    let taken = sqlx::query_scalar!(
        "SELECT number FROM invoices WHERE org_id = $1 AND number = $2 AND id <> $3",
        org_id,
        source.number,
        id,
    )
    .fetch_optional(&mut **sp)
    .await?;
    if taken.is_some() {
        return Err(Failure::new(
            InvoiceErrorKind::NumberTaken,
            format!("A Horae invoice already uses number {}", source.number),
        ));
    }

    let due_on = source.due_date.unwrap_or(source.issue_date);
    let po_number = source.purchase_order.clone().unwrap_or_default();
    let subject = source.subject.clone().unwrap_or_default();
    let notes = source.notes.clone().filter(|notes| !notes.trim().is_empty());
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
        .execute(&mut **sp)
        .await?;
        sqlx::query!(
            "DELETE FROM harvest_invoice_lines WHERE invoice_id = $1",
            id
        )
        .execute(&mut **sp)
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
        .execute(&mut **sp)
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
    .execute(&mut **sp)
    .await?;
    let ids: Vec<Uuid> = lines.iter().map(|_| Uuid::now_v7()).collect();
    let positions: Vec<i32> = lines.iter().map(|line| line.position).collect();
    let harvest_ids: Vec<i64> = lines.iter().map(|line| line.harvest_id).collect();
    let kinds: Vec<String> = lines.iter().map(|line| line.kind.clone()).collect();
    let descriptions: Vec<String> = lines.iter().map(|l| l.description.clone()).collect();
    let quantities: Vec<String> = lines.iter().map(|line| line.quantity.clone()).collect();
    let prices: Vec<i64> = lines.iter().map(|line| line.unit_price_cents).collect();
    let taxed: Vec<bool> = lines.iter().map(|line| line.taxed).collect();
    let taxed2: Vec<bool> = lines.iter().map(|line| line.taxed2).collect();
    let projects: Vec<Option<Uuid>> = lines.iter().map(|line| line.project_id).collect();
    sqlx::query!(
        "INSERT INTO harvest_invoice_lines
           (id, org_id, invoice_id, position, harvest_line_id, kind, description, quantity,
            unit_price_cents, amount_cents, taxed, taxed2, project_id)
         SELECT l.id, $2, $3, l.position, l.harvest_id, l.kind, l.description,
                l.quantity::numeric, l.price, l.amount, l.taxed, l.taxed2, l.project
         FROM unnest($1::uuid[], $4::int4[], $5::int8[], $6::text[], $7::text[], $8::text[],
                     $9::int8[], $10::int8[], $11::bool[], $12::bool[], $13::uuid[])
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
    .execute(&mut **sp)
    .await?;
    super::provenance::upsert(
        &mut **sp,
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
    ))
}
