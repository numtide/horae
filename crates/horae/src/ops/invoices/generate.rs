//! Generating a draft invoice from a client's uninvoiced time and fees.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::{SourceRead, balances, defaults, entries, fees, preview};
use crate::models::invoice::InvoiceLine;
use crate::models::invoice::{InvoiceDefaults, InvoiceGenerationRequest, InvoiceSource};
use crate::models::{Invoice, InvoiceWithLines};
use crate::ops::{Actor, Done, OpError, Tx};
use crate::plugin::AppEvent;
use crate::plugin::event::invoice_payload;

pub(crate) struct Generation<'a> {
    pub client_id: Uuid,
    pub period: (NaiveDate, NaiveDate),
    pub project_ids: Option<&'a [Uuid]>,
    pub overrides: Option<&'a InvoiceDefaults>,
    /// The reviewed preview the person confirmed, and the identity that makes
    /// a repeated request return the invoice it already created.
    pub request: Option<&'a InvoiceGenerationRequest>,
}

/// Creates a draft invoice from the reviewed preview and marks its time as
/// invoiced. A repeated request returns the invoice it created, without a new
/// `InvoiceCreated` event.
///
/// Everything from selecting the entries to flipping them to 'invoiced' runs in
/// one transaction, serialized per organization by the invoice lock, so two
/// concurrent generate calls cannot bill the same time twice or mint the same
/// invoice number.
pub(crate) async fn generate(
    tx: &mut Tx,
    actor: &Actor,
    input: Generation<'_>,
) -> Result<Done<InvoiceWithLines>, OpError> {
    let Generation {
        client_id,
        period: (from, to),
        project_ids,
        overrides,
        request,
    } = input;
    if request.is_some_and(|request| {
        request.request_id.get_version_num() != 7
            || request.review.lines.len() > 20000
            || request.review.projects.len() > 1000
            || request.review.excess.len() > 10000
            || request.confirmed_excess.len() > 10000
    }) {
        return Err(OpError::bad_input(
            "Invalid invoice request identity or source count",
        ));
    }
    if from > to {
        return Err(OpError::bad_input("Invoice period ends before it starts"));
    }
    if project_ids.is_some_and(|ids| ids.is_empty() || ids.len() > 1000) {
        return Err(OpError::bad_input("Select between 1 and 1000 projects"));
    }
    let selected = project_ids.map(|ids| {
        ids.iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    });
    // Generation prices the work from current billable rates, as its preview
    // did.
    super::authorize_prepare(actor)?;
    let org_id = actor.org_id();
    let actor_id = actor.id();
    balances::lock_invoices(tx, org_id).await?;
    let canonical = request.map(|request| {
        let mut request = request.clone();
        request.review.lines.sort_by(|a, b| a.source.cmp(&b.source));
        request
            .review
            .projects
            .sort_by_key(|project| project.project_id);
        request
            .review
            .excess
            .sort_by(|a, b| a.source.cmp(&b.source));
        request
            .confirmed_excess
            .sort_by(|a, b| a.source.cmp(&b.source));
        request
    });
    let payload = serde_json::json!({
        "client_id": client_id, "from": from, "to": to,
        "projects": selected, "overrides": overrides, "request": canonical,
    });
    if request.is_some() {
        let size = sqlx::query_scalar!(
            r#"SELECT octet_length($1::jsonb::text) AS "size!""#,
            payload
        )
        .fetch_one(&mut **tx)
        .await?;
        if size > 262144 {
            return Err(OpError::bad_input("Invoice request exceeds 256 KiB"));
        }
    }
    // Verify the client belongs to this org before selecting any billable
    // work, and only once the actor's authority is held.
    sqlx::query_scalar!(
        "SELECT id FROM clients WHERE id = $1 AND org_id = $2",
        client_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found("Client not found"))?;
    if let Some(request) = request
        && let Some(previous) = sqlx::query!(
            "SELECT actor_id, invoice_id, payload FROM invoice_generation_requests WHERE id = $1 AND org_id = $2",
            request.request_id, org_id,
        ).fetch_optional(&mut **tx).await? {
            if previous.actor_id != actor_id || previous.payload != payload {
                return Err(OpError::conflict("This request identity was already used with different invoice values"));
            }
            return Ok(Done::new(super::with_lines(tx, org_id, previous.invoice_id).await?));
    }

    if let Some(ids) = &selected {
        let allowed = sqlx::query_scalar!("SELECT id FROM projects WHERE org_id = $1 AND client_id = $2 AND id = ANY($3) ORDER BY id FOR SHARE", org_id, client_id, ids)
            .fetch_all(&mut **tx).await?;
        if allowed.len() != ids.len() {
            return Err(OpError::not_found(
                "Some selected projects are unavailable for this client",
            ));
        }
    }

    // Fetch billable, un-invoiced entries for this client in the period,
    // with the selected hourly rate. Open and approved time is
    // invoiceable (spec 001: billable, un-invoiced time is directly
    // invoiceable); submitted time is locked pending an approval decision.
    // FOR UPDATE locks the entry rows: a competing transaction blocks here
    // until this one commits, then re-evaluates its WHERE and skips rows that
    // were just invoiced.
    let entries = entries::read(
        tx,
        org_id,
        client_id,
        (from, to),
        selected.as_deref(),
        SourceRead::Generate,
    )
    .await?;

    let fee_selection = canonical
        .as_ref()
        .map(|request| request.review.fee_selection());
    let fees = fees::prepare_fees(
        tx,
        org_id,
        client_id,
        from,
        to,
        selected.as_deref(),
        fee_selection.as_deref(),
    )
    .await?;
    let mut prepared = entries
        .iter()
        .map(entries::EntryWithRates::preview)
        .collect::<Result<Vec<_>, _>>()?;
    prepared
        .extend(fees::preview_fees(tx, org_id, client_id, from, to, selected.as_deref()).await?);
    if let Some(request) = &canonical {
        let current_sources: BTreeSet<_> = prepared.iter().map(|line| &line.source).collect();
        let reviewed_sources: BTreeSet<_> = request
            .review
            .lines
            .iter()
            .map(|line| &line.source)
            .collect();
        if current_sources != reviewed_sources
            || reviewed_sources.len() != request.review.lines.len()
        {
            return Err(OpError::conflict(
                "Invoice sources or fee balances changed. Review the invoice again.",
            ));
        }
        preview::apply_fee_selection(&mut prepared, &request.review.fee_selection())?;
    }
    let currency = preview::currency(&prepared)?;
    if !prepared.iter().any(|line| line.selected) {
        return Err(OpError::bad_input("Select at least one invoice charge"));
    }
    let billed_projects: Vec<_> = prepared
        .iter()
        .filter(|line| line.selected)
        .map(|line| line.project_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let settings = defaults::resolve(
        tx,
        org_id,
        selected.as_deref().unwrap_or(&billed_projects),
        overrides,
    )
    .await?;

    // Generate invoice number: INV-YYYYMM-NNN
    let now = chrono::Utc::now();
    let year_month = now.format("%Y%m").to_string();
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM invoices
           WHERE org_id = $1 AND number LIKE $2"#,
        org_id,
        format!("INV-{year_month}-%"),
    )
    .fetch_one(&mut **tx)
    .await?;
    let invoice_number = format!("INV-{year_month}-{:03}", count + 1);

    let invoice_id = Uuid::now_v7();
    let issued_on = now.date_naive();
    let due_on = horae_core::project::payment_due_date(issued_on, settings.terms_days as u16)
        .map_err(OpError::bad_input)?;
    preview::allocate_lines(&mut prepared, settings.discount_bps)?;
    let subtotal_cents = preview::subtotal(&prepared)?;
    let amounts = defaults::amounts(&settings, subtotal_cents)?;
    let excess = preview::excess(&prepared)?;
    if let Some(request) = &canonical {
        let review = &request.review;
        let mut current_lines = prepared.clone();
        current_lines.sort_by(|a, b| a.source.cmp(&b.source));
        if review.lines != current_lines
            || review.defaults.as_ref() != Some(&settings)
            || review.currency != currency
            || review.issued_on != issued_on
            || review.due_on != Some(due_on)
            || review.subtotal_cents != subtotal_cents
            || review.amounts != Some(amounts)
            || review.excess != excess
        {
            return Err(OpError::conflict(
                "Invoice sources or fee balances changed. Review the invoice again.",
            ));
        }
    }
    if canonical
        .as_ref()
        .map(|request| request.confirmed_excess.as_slice())
        .unwrap_or_default()
        != excess
    {
        return Err(OpError::conflict(
            "Confirm the exact amount exceeding the fee balance before generating.",
        ));
    }
    let fee_ids: BTreeMap<_, _> = fees
        .iter()
        .map(|fee| ((fee.project_id, fee.period_key.as_str()), fee.id))
        .collect();
    let mut lines = Vec::with_capacity(prepared.len());
    let mut line_discounts = Vec::with_capacity(prepared.len());
    for line in prepared.iter().filter(|line| line.selected) {
        let (time_entry_id, fee_occurrence_id) = match &line.source {
            InvoiceSource::Time { entry_id } => (Some(*entry_id), None),
            InvoiceSource::Fee {
                project_id,
                period_key,
            } => (
                None,
                Some(
                    *fee_ids
                        .get(&(*project_id, period_key.as_str()))
                        .ok_or_else(|| {
                            OpError::conflict("Fee source changed. Review the invoice again.")
                        })?,
                ),
            ),
        };
        let net = line
            .net_before_tax_cents
            .ok_or_else(|| OpError::conflict("Invoice contributions are unavailable"))?;
        line_discounts.push(line.amount_cents - net);
        lines.push(InvoiceLine {
            id: Uuid::now_v7(),
            invoice_id,
            time_entry_id,
            fee_occurrence_id,
            description: line.description.clone(),
            minutes: line.minutes,
            rate_cents: line.rate_cents,
            amount_cents: line.amount_cents,
        });
    }
    let total_cents = amounts.total_cents;

    // Insert invoice-owned values; later project or settings edits cannot alter
    // this snapshot, including the organization's default notes.
    let notes = sqlx::query_scalar!(
        r#"INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents,
             po_number,discount_bps,tax1_bps,tax2_name,tax2_bps,discount_cents,tax1_cents,tax2_cents,notes)
           VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8, $9,$10,$11,$12,$13,$14,$15,$16,
             (SELECT invoice_notes FROM organizations WHERE id = $2))
           RETURNING notes"#,
        invoice_id,
        org_id,
        client_id,
        invoice_number,
        issued_on as chrono::NaiveDate,
        due_on as chrono::NaiveDate,
        currency,
        total_cents,
        settings.po_number, settings.discount_bps, settings.tax1_bps, settings.tax2_name,
        settings.tax2_bps, amounts.discount_cents, amounts.tax1_cents, amounts.tax2_cents,
    )
    .fetch_one(&mut **tx)
    .await
    // The advisory lock makes a number collision between generate calls
    // impossible, but any other writer racing the UNIQUE (org_id, number)
    // constraint should surface as a retryable conflict, not a 500 carrying
    // raw database text.
    .map_err(|e| {
        if e.as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            OpError::conflict("Invoice number was just taken by another invoice; please retry.")
        } else {
            OpError::from(e)
        }
    })?;

    // Insert the line items in one statement. A statement per line would hold
    // the per-org advisory lock — and the FOR UPDATE locks on every selected
    // entry — for one round trip per line, so a large invoice would block every
    // other invoice for the org for as long as that takes.
    let line_ids: Vec<Uuid> = lines.iter().map(|l| l.id).collect();
    let line_invoice_ids: Vec<Uuid> = lines.iter().map(|l| l.invoice_id).collect();
    let line_entry_ids: Vec<Option<Uuid>> = lines.iter().map(|l| l.time_entry_id).collect();
    let line_fee_ids: Vec<Option<Uuid>> = lines.iter().map(|l| l.fee_occurrence_id).collect();
    let line_descriptions: Vec<String> = lines.iter().map(|l| l.description.clone()).collect();
    let line_minutes: Vec<Option<i32>> = lines.iter().map(|l| l.minutes).collect();
    let line_rates: Vec<Option<i64>> = lines.iter().map(|l| l.rate_cents).collect();
    let line_amounts: Vec<i64> = lines.iter().map(|l| l.amount_cents).collect();

    sqlx::query!(
        r#"INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents, fee_occurrence_id, allocated_discount_cents)
           SELECT * FROM unnest($1::uuid[], $2::uuid[], $3::uuid[], $4::text[], $5::int4[], $6::int8[], $7::int8[], $8::uuid[], $9::bigint[])"#,
        &line_ids,
        &line_invoice_ids,
        &line_entry_ids as &[Option<Uuid>],
        &line_descriptions,
        &line_minutes as &[Option<i32>],
        &line_rates as &[Option<i64>],
        &line_amounts,
        &line_fee_ids as &[Option<Uuid>],
        &line_discounts,
    )
    .execute(&mut **tx)
    .await?;

    // Mark entries as invoiced. The row locks taken above already exclude
    // concurrent writers; re-checking state and invoice_id here is the final
    // guarantee that exactly the entries behind the line items get flipped.
    let entry_ids: Vec<Uuid> = entries.iter().map(|e| e.entry_id).collect();
    let billed_minutes: Vec<i32> = entries.iter().map(|e| e.minutes).collect();
    let flipped = sqlx::query!(
        r#"UPDATE time_entries
           SET invoice_id = $1,
               state = 'invoiced',
               rounded_minutes = billed.minutes
           FROM unnest($2::uuid[], $3::int4[]) AS billed(id, minutes)
           WHERE time_entries.id = billed.id
             AND invoice_id IS NULL
             AND NOT is_running
             AND state IN ('open', 'approved')"#,
        invoice_id,
        &entry_ids,
        &billed_minutes,
    )
    .execute(&mut **tx)
    .await?
    .rows_affected();

    if flipped != entry_ids.len() as u64 {
        return Err(OpError::conflict(
            "Some of the selected time was modified concurrently; no invoice was created. Please retry.",
        ));
    }

    if let Some(request) = request {
        sqlx::query!(
            "INSERT INTO invoice_generation_requests (id,org_id,actor_id,invoice_id,payload) VALUES ($1,$2,$3,$4,$5)",
            request.request_id, org_id, actor_id, invoice_id, payload,
        ).execute(&mut **tx).await?;
    }

    let invoice = Invoice {
        id: invoice_id,
        org_id,
        client_id,
        number: invoice_number,
        status: InvoiceStatus::Draft,
        issued_on,
        due_on,
        currency: currency.to_string(),
        total_cents,
        terms_days: i32::from(settings.terms_days),
        po_number: settings.po_number,
        discount_bps: settings.discount_bps,
        tax1_bps: settings.tax1_bps,
        tax2_name: settings.tax2_name,
        tax2_bps: settings.tax2_bps,
        subtotal_cents: amounts.subtotal_cents,
        discount_cents: amounts.discount_cents,
        tax1_cents: amounts.tax1_cents,
        tax2_cents: amounts.tax2_cents,
        notes,
        created_at: now,
    };
    let event = AppEvent::InvoiceCreated {
        occurred_at: now,
        org_id,
        invoice: invoice_payload(&invoice),
    };
    Ok(Done::new(InvoiceWithLines { invoice, lines }).with(event))
}
