//! Invoice server functions.

use super::*;
use crate::models::invoice::{InvoiceDefaults, InvoicePreparation};

#[cfg(feature = "server")]
mod defaults;

#[cfg(feature = "server")]
mod fees;

#[cfg(feature = "server")]
mod entries;

#[cfg(feature = "server")]
mod preview;

#[cfg(feature = "server")]
mod balances;

#[cfg(feature = "server")]
#[derive(Clone, Copy)]
enum SourceRead {
    Preview,
    Generate,
}

#[server]
pub async fn prepare_invoice(
    client_id: String,
    period_from: String,
    period_to: String,
    project_ids: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
) -> Result<InvoicePreparation, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    if project_ids
        .as_ref()
        .is_some_and(|ids| ids.is_empty() || ids.len() > 1000)
    {
        return Err(err(BAD_REQUEST, "Select between 1 and 1000 projects"));
    }
    let selected = project_ids
        .map(|ids| {
            ids.iter()
                .map(|id| parse_uuid(id, "project_id"))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    preview::prepare(
        &state.db,
        manager.org_id,
        parse_uuid(&client_id, "client_id")?,
        (
            parse_date(&period_from, "period_from")?,
            parse_date(&period_to, "period_to")?,
        ),
        selected.as_deref(),
        overrides.as_ref(),
    )
    .await
}

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(all(test, feature = "server"))]
mod fee_tests;

// ── Invoices ──────────────────────────────────────────────────────────────────

#[server]
pub async fn list_invoices(status: Option<String>) -> Result<Vec<Invoice>, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let status_filter: Option<InvoiceStatus> = status
        .as_deref()
        .map(|s| parse_enum(s, "status"))
        .transpose()?;

    let invoices = sqlx::query_as!(
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
           ORDER BY created_at DESC"#,
        manager.org_id,
        status_filter as Option<InvoiceStatus>,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(invoices)
}

#[server]
pub async fn get_invoice(invoice_id: String) -> Result<InvoiceWithLines, ServerFnError> {
    let manager = require_manager().await?;
    let id = parse_uuid(&invoice_id, "invoice_id")?;

    let (invoice, lines) = crate::reports::fetch_invoice_with_lines(id, manager.org_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| not_found("Invoice not found"))?;

    Ok(InvoiceWithLines { invoice, lines })
}

#[server]
pub async fn generate_invoice(
    client_id: String,
    period_from: String,
    period_to: String,
    project_ids: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
    request_id: String,
) -> Result<InvoiceWithLines, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let from = parse_date(&period_from, "period_from")?;
    let to = parse_date(&period_to, "period_to")?;

    if project_ids
        .as_ref()
        .is_some_and(|ids| ids.is_empty() || ids.len() > 1000)
    {
        return Err(err(BAD_REQUEST, "Select between 1 and 1000 projects"));
    }
    let project_ids = project_ids
        .map(|ids| {
            ids.iter()
                .map(|id| parse_uuid(id, "project_id"))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    let request_id = parse_uuid(&request_id, "request_id")?;
    if request_id.get_version_num() != 7 {
        return Err(err(BAD_REQUEST, "Request identity must be a UUID v7"));
    }
    let (result, created) = generate_invoice_with_request(
        &state.db,
        manager.org_id,
        client_id,
        (from, to),
        project_ids.as_deref(),
        overrides.as_ref(),
        Some((request_id, manager.id)),
    )
    .await?;
    if created {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoiceCreated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&result.invoice),
            });
    }
    Ok(result)
}

#[cfg(all(test, feature = "server"))]
pub(super) async fn generate_invoice_for_period(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<InvoiceWithLines, ServerFnError> {
    generate_invoice_with_options(pool, org_id, client_id, (from, to), None, None).await
}

#[cfg(all(test, feature = "server"))]
async fn generate_invoice_with_options(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    (from, to): (chrono::NaiveDate, chrono::NaiveDate),
    project_ids: Option<&[uuid::Uuid]>,
    overrides: Option<&InvoiceDefaults>,
) -> Result<InvoiceWithLines, ServerFnError> {
    generate_invoice_with_request(
        pool,
        org_id,
        client_id,
        (from, to),
        project_ids,
        overrides,
        None,
    )
    .await
    .map(|(invoice, _)| invoice)
}

#[cfg(feature = "server")]
async fn generate_invoice_with_request(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    (from, to): (chrono::NaiveDate, chrono::NaiveDate),
    project_ids: Option<&[uuid::Uuid]>,
    overrides: Option<&InvoiceDefaults>,
    request: Option<(uuid::Uuid, uuid::Uuid)>,
) -> Result<(InvoiceWithLines, bool), ServerFnError> {
    if from > to {
        return Err(err(BAD_REQUEST, "Invoice period ends before it starts"));
    }
    if project_ids.is_some_and(|ids| ids.is_empty() || ids.len() > 1000) {
        return Err(err(BAD_REQUEST, "Select between 1 and 1000 projects"));
    }
    let selected = project_ids.map(|ids| {
        ids.iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    });
    // Verify client belongs to this org before selecting any billable work.
    sqlx::query_scalar!(
        "SELECT id FROM clients WHERE id = $1 AND org_id = $2",
        client_id,
        org_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;

    // Everything from selecting the entries to flipping them to 'invoiced'
    // runs in one transaction so two concurrent generate calls cannot bill the
    // same time twice or mint the same invoice number.
    let mut tx = pool.begin().await.map_err(server_err)?;

    balances::lock_invoices(&mut tx, org_id).await?;
    let payload = serde_json::json!({
        "client_id": client_id, "from": from, "to": to,
        "projects": selected, "overrides": overrides,
    });
    if let Some((request_id, actor_id)) = request
        && let Some(previous) = sqlx::query!(
            "SELECT actor_id, invoice_id, payload FROM invoice_generation_requests WHERE id = $1 AND org_id = $2",
            request_id, org_id,
        ).fetch_optional(&mut *tx).await.map_err(server_err)? {
            if previous.actor_id != actor_id || previous.payload != payload {
                return Err(conflict("This request identity was already used with different invoice values"));
            }
            let (invoice, lines) = crate::reports::fetch_invoice_from(&mut tx, previous.invoice_id, org_id)
                .await.map_err(server_err)?.ok_or_else(|| not_found("Invoice not found"))?;
            tx.commit().await.map_err(server_err)?;
            return Ok((InvoiceWithLines { invoice, lines }, false));
    }

    if let Some(ids) = &selected {
        let allowed = sqlx::query_scalar!("SELECT id FROM projects WHERE org_id = $1 AND client_id = $2 AND id = ANY($3) ORDER BY id FOR SHARE", org_id, client_id, ids)
            .fetch_all(&mut *tx).await.map_err(server_err)?;
        if allowed.len() != ids.len() {
            return Err(not_found(
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
        &mut tx,
        org_id,
        client_id,
        (from, to),
        selected.as_deref(),
        SourceRead::Generate,
    )
    .await?;

    let fees =
        fees::prepare_fees(&mut tx, org_id, client_id, from, to, selected.as_deref()).await?;
    let currency = entries
        .first()
        .map(|entry| entry.currency.trim())
        .or_else(|| fees.first().map(|fee| fee.currency.trim()))
        .ok_or_else(|| {
            not_found("No billable, un-invoiced time or fees found for this client and period.")
        })?;
    if entries
        .iter()
        .any(|entry| entry.currency.trim() != currency)
        || fees.iter().any(|fee| fee.currency.trim() != currency)
    {
        return Err(conflict(
            "Projects with different billing currencies cannot share an invoice.",
        ));
    }

    let billed_projects: Vec<_> = entries
        .iter()
        .map(|entry| entry.project_id)
        .chain(fees.iter().map(|fee| fee.project_id))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let settings = defaults::resolve(
        &mut tx,
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
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    let invoice_number = format!("INV-{year_month}-{:03}", count + 1);

    let invoice_id = uuid::Uuid::now_v7();
    let issued_on = now.date_naive();
    let due_on = horae_core::project::payment_due_date(issued_on, settings.terms_days as u16)
        .map_err(|error| err(BAD_REQUEST, error.to_string()))?;

    // Build line items and compute total.
    let mut lines = Vec::with_capacity(entries.len() + fees.len());
    let mut total_cents: i64 = 0;

    for e in &entries {
        let rate = e.rate_cents.unwrap_or(0);

        let amount = e.amount()?;
        total_cents = total_cents.checked_add(amount).ok_or_else(|| {
            conflict("Invoice total exceeds the supported range; select a shorter period.")
        })?;

        let description = e.description();

        lines.push(InvoiceLine {
            id: uuid::Uuid::now_v7(),
            invoice_id,
            time_entry_id: Some(e.entry_id),
            fee_occurrence_id: None,
            description,
            minutes: Some(e.minutes),
            rate_cents: Some(rate),
            amount_cents: amount,
        });
    }

    for fee in &fees {
        total_cents = total_cents.checked_add(fee.amount_cents).ok_or_else(|| {
            conflict("Invoice total exceeds the supported range; select a shorter period.")
        })?;
        lines.push(InvoiceLine {
            id: uuid::Uuid::now_v7(),
            invoice_id,
            time_entry_id: None,
            fee_occurrence_id: Some(fee.id),
            description: fee.description.clone(),
            minutes: None,
            rate_cents: None,
            amount_cents: fee.amount_cents,
        });
    }

    let amounts = defaults::amounts(&settings, total_cents)?;
    let total_cents = amounts.total_cents;

    // Insert invoice-owned values; later project edits cannot alter this snapshot.
    sqlx::query!(
        r#"INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents,
             po_number,discount_bps,tax1_bps,tax2_name,tax2_bps,discount_cents,tax1_cents,tax2_cents)
           VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8, $9,$10,$11,$12,$13,$14,$15,$16)"#,
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
    .execute(&mut *tx)
    .await
    // The advisory lock makes a number collision between generate calls
    // impossible, but any other writer racing the UNIQUE (org_id, number)
    // constraint should surface as a retryable conflict, not a 500 carrying
    // raw database text.
    .map_err(|e| {
        if e.as_database_error()
            .is_some_and(|db| db.is_unique_violation())
        {
            conflict("Invoice number was just taken by another invoice; please retry.")
        } else {
            server_err(e)
        }
    })?;

    // Insert the line items in one statement. A statement per line would hold
    // the per-org advisory lock — and the FOR UPDATE locks on every selected
    // entry — for one round trip per line, so a large invoice would block every
    // other invoice for the org for as long as that takes.
    let line_ids: Vec<uuid::Uuid> = lines.iter().map(|l| l.id).collect();
    let line_invoice_ids: Vec<uuid::Uuid> = lines.iter().map(|l| l.invoice_id).collect();
    let line_entry_ids: Vec<Option<uuid::Uuid>> = lines.iter().map(|l| l.time_entry_id).collect();
    let line_fee_ids: Vec<Option<uuid::Uuid>> = lines.iter().map(|l| l.fee_occurrence_id).collect();
    let line_descriptions: Vec<String> = lines.iter().map(|l| l.description.clone()).collect();
    let line_minutes: Vec<Option<i32>> = lines.iter().map(|l| l.minutes).collect();
    let line_rates: Vec<Option<i64>> = lines.iter().map(|l| l.rate_cents).collect();
    let line_amounts: Vec<i64> = lines.iter().map(|l| l.amount_cents).collect();

    sqlx::query!(
        r#"INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents, fee_occurrence_id)
           SELECT * FROM unnest($1::uuid[], $2::uuid[], $3::uuid[], $4::text[], $5::int4[], $6::int8[], $7::int8[], $8::uuid[])"#,
        &line_ids,
        &line_invoice_ids,
        &line_entry_ids as &[Option<uuid::Uuid>],
        &line_descriptions,
        &line_minutes as &[Option<i32>],
        &line_rates as &[Option<i64>],
        &line_amounts,
        &line_fee_ids as &[Option<uuid::Uuid>],
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    // Mark entries as invoiced. The row locks taken above already exclude
    // concurrent writers; re-checking state and invoice_id here is the final
    // guarantee that exactly the entries behind the line items get flipped.
    let entry_ids: Vec<uuid::Uuid> = entries.iter().map(|e| e.entry_id).collect();
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
    .execute(&mut *tx)
    .await
    .map_err(server_err)?
    .rows_affected();

    if flipped != entry_ids.len() as u64 {
        return Err(conflict(
            "Some of the selected time was modified concurrently; no invoice was created. Please retry.",
        ));
    }

    balances::replace_contributions(&mut tx, org_id, invoice_id, settings.discount_bps).await?;
    if let Some((request_id, actor_id)) = request {
        sqlx::query!(
            "INSERT INTO invoice_generation_requests (id,org_id,actor_id,invoice_id,payload) VALUES ($1,$2,$3,$4,$5)",
            request_id, org_id, actor_id, invoice_id, payload,
        ).execute(&mut *tx).await.map_err(server_err)?;
    }

    tx.commit().await.map_err(server_err)?;

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
        notes: None,
        created_at: now,
    };

    Ok((InvoiceWithLines { invoice, lines }, true))
}

/// Override only an editable invoice; project defaults and source lines stay intact.
#[server]
pub async fn update_invoice_defaults(
    invoice_id: String,
    overrides: InvoiceDefaults,
) -> Result<Invoice, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    update_invoice_defaults_in_db(&state.db, manager.org_id, id, &overrides).await
}

#[cfg(feature = "server")]
async fn update_invoice_defaults_in_db(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    id: uuid::Uuid,
    overrides: &InvoiceDefaults,
) -> Result<Invoice, ServerFnError> {
    let mut tx = pool.begin().await.map_err(server_err)?;
    balances::lock_invoices(&mut tx, org_id).await?;
    let current = sqlx::query!(
        r#"SELECT status as "status: InvoiceStatus", subtotal_cents,
                  issued_on as "issued_on: chrono::NaiveDate"
           FROM invoices WHERE org_id = $1 AND id = $2 FOR UPDATE"#,
        org_id,
        id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Invoice not found"))?;
    if current.status != InvoiceStatus::Draft {
        return Err(conflict("Only draft invoices can be edited"));
    }
    let amounts = defaults::amounts(overrides, current.subtotal_cents)?;
    balances::replace_contributions(&mut tx, org_id, id, overrides.discount_bps).await?;
    let due_on =
        horae_core::project::payment_due_date(current.issued_on, overrides.terms_days as u16)
            .map_err(|error| err(BAD_REQUEST, error.to_string()))?;
    sqlx::query!(
        "UPDATE invoices SET due_on=$3,po_number=$4,discount_bps=$5,tax1_bps=$6,
            tax2_name=$7,tax2_bps=$8,discount_cents=$9,tax1_cents=$10,tax2_cents=$11,total_cents=$12
         WHERE org_id=$1 AND id=$2",
        org_id,
        id,
        due_on as chrono::NaiveDate,
        overrides.po_number,
        overrides.discount_bps,
        overrides.tax1_bps,
        overrides.tax2_name,
        overrides.tax2_bps,
        amounts.discount_cents,
        amounts.tax1_cents,
        amounts.tax2_cents,
        amounts.total_cents,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;
    let invoice = crate::reports::fetch_invoice_metadata(&mut *tx, id, org_id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| not_found("Invoice not found"))?;
    tx.commit().await.map_err(server_err)?;
    Ok(invoice)
}

#[server]
pub async fn update_invoice_status(
    invoice_id: String,
    new_status: String,
) -> Result<Invoice, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    let target: InvoiceStatus = parse_enum(&new_status, "status")?;

    let invoice = transition_invoice(&state.db, manager.org_id, id, target).await?;

    // Dispatch invoice_sent event when transitioning to Sent (FR-019).
    if target == InvoiceStatus::Sent {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoiceSent {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&invoice),
            });
    }

    if target == InvoiceStatus::Paid {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoicePaid {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&invoice),
            });
    } else if target == InvoiceStatus::Void {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoiceVoided {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&invoice),
            });
    }

    Ok(invoice)
}

#[cfg(feature = "server")]
async fn transition_invoice(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    id: uuid::Uuid,
    target: InvoiceStatus,
) -> Result<Invoice, ServerFnError> {
    let mut tx = pool.begin().await.map_err(server_err)?;
    balances::lock_invoices(&mut tx, org_id).await?;

    // Validate under the same row lock as the transition: payment and void
    // must not both accept a previously observed 'sent' state.
    let current_status: InvoiceStatus = sqlx::query_scalar!(
        r#"SELECT status as "status: InvoiceStatus"
           FROM invoices WHERE id = $1 AND org_id = $2 FOR UPDATE"#,
        id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Invoice not found"))?;

    // Enforce state machine: draft->sent, sent->paid, draft|sent->void
    let valid = matches!(
        (current_status, target),
        (InvoiceStatus::Draft, InvoiceStatus::Sent)
            | (InvoiceStatus::Sent, InvoiceStatus::Paid)
            | (InvoiceStatus::Draft, InvoiceStatus::Void)
            | (InvoiceStatus::Sent, InvoiceStatus::Void)
    );
    if !valid {
        return Err(conflict(format!(
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
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
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
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;

    Ok(invoice)
}
