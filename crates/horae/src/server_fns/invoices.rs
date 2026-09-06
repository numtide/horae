//! Invoice server functions.

use super::*;

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
                  currency, total_cents, notes,
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
) -> Result<InvoiceWithLines, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let from = parse_date(&period_from, "period_from")?;
    let to = parse_date(&period_to, "period_to")?;

    // Verify client belongs to this org and get its currency.
    let client = sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE id = $1 AND org_id = $2"#,
        client_id,
        manager.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;

    // Everything from selecting the entries to flipping them to 'invoiced'
    // runs in one transaction so two concurrent generate calls cannot bill the
    // same time twice or mint the same invoice number.
    let mut tx = state.db.begin().await.map_err(server_err)?;

    // Serialize invoice creation per org while this transaction runs. The
    // invoice number is derived from a COUNT over existing invoices, which two
    // concurrent transactions would otherwise compute identically and then
    // trip the UNIQUE (org_id, number) constraint. The lock is released
    // automatically at commit or rollback.
    sqlx::query_scalar!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended($1, 0)) as "lock!: ()""#,
        manager.org_id.to_string(),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;

    // Fetch billable, un-invoiced entries for this client in the period,
    // with rate candidates from all cascade levels. Open and approved time is
    // invoiceable (spec 001: billable, un-invoiced time is directly
    // invoiceable); submitted time is locked pending an approval decision.
    // FOR UPDATE locks the entry rows: a competing transaction blocks here
    // until this one commits, then re-evaluates its WHERE and skips rows that
    // were just invoiced.
    struct EntryWithRates {
        entry_id: uuid::Uuid,
        minutes: i32,
        project_name: String,
        task_name: String,
        notes: Option<String>,
        spent_date: chrono::NaiveDate,
        task_rate_cents: Option<i64>,
        assignment_rate_cents: Option<i64>,
        user_rate_cents: Option<i64>,
    }

    let entries = sqlx::query_as!(
        EntryWithRates,
        r#"SELECT
             te.id as entry_id,
             te.minutes,
             p.name as project_name,
             t.name as task_name,
             te.notes,
             te.spent_date as "spent_date: chrono::NaiveDate",
             pt.rate_cents as task_rate_cents,
             a.rate_cents as assignment_rate_cents,
             u.billable_rate_cents as user_rate_cents
           FROM time_entries te
           JOIN projects p ON p.id = te.project_id
           JOIN tasks t ON t.id = te.task_id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           JOIN users u ON u.id = te.user_id
           WHERE te.org_id = $1
             AND p.client_id = $2
             AND te.billable = true
             AND te.invoice_id IS NULL
             AND te.state IN ('open', 'approved')
             AND te.spent_date >= $3
             AND te.spent_date <= $4
           ORDER BY te.spent_date, te.id
           FOR UPDATE OF te"#,
        manager.org_id,
        client_id,
        from as chrono::NaiveDate,
        to as chrono::NaiveDate,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;

    if entries.is_empty() {
        return Err(not_found(
            "No billable, un-invoiced time found for this client and period.",
        ));
    }

    // Generate invoice number: INV-YYYYMM-NNN
    let now = chrono::Utc::now();
    let year_month = now.format("%Y%m").to_string();
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM invoices
           WHERE org_id = $1 AND number LIKE $2"#,
        manager.org_id,
        format!("INV-{year_month}-%"),
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;
    let invoice_number = format!("INV-{year_month}-{:03}", count + 1);

    let invoice_id = uuid::Uuid::now_v7();
    let issued_on = now.date_naive();
    // Default due date: 30 days from issue
    let due_on = issued_on + chrono::Duration::days(30);

    // Build line items and compute total.
    let mut lines = Vec::with_capacity(entries.len());
    let mut total_cents: i64 = 0;

    for e in &entries {
        let rate = horae_core::invoice::resolve_rate(
            e.task_rate_cents,
            e.assignment_rate_cents,
            e.user_rate_cents,
        )
        .unwrap_or(0);

        let amount = horae_core::invoice::line_amount_cents(rate, e.minutes);
        total_cents += amount;

        let description = if let Some(notes) = &e.notes {
            format!(
                "{} — {} ({}): {}",
                e.spent_date, e.project_name, e.task_name, notes
            )
        } else {
            format!("{} — {} ({})", e.spent_date, e.project_name, e.task_name)
        };

        lines.push(InvoiceLine {
            id: uuid::Uuid::now_v7(),
            invoice_id,
            time_entry_id: e.entry_id,
            description,
            minutes: e.minutes,
            rate_cents: rate,
            amount_cents: amount,
        });
    }

    // Insert invoice.
    sqlx::query!(
        r#"INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
           VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8)"#,
        invoice_id,
        manager.org_id,
        client_id,
        invoice_number,
        issued_on as chrono::NaiveDate,
        due_on as chrono::NaiveDate,
        client.currency.trim(),
        total_cents,
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
    let line_entry_ids: Vec<uuid::Uuid> = lines.iter().map(|l| l.time_entry_id).collect();
    let line_descriptions: Vec<String> = lines.iter().map(|l| l.description.clone()).collect();
    let line_minutes: Vec<i32> = lines.iter().map(|l| l.minutes).collect();
    let line_rates: Vec<i64> = lines.iter().map(|l| l.rate_cents).collect();
    let line_amounts: Vec<i64> = lines.iter().map(|l| l.amount_cents).collect();

    sqlx::query!(
        r#"INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
           SELECT * FROM unnest($1::uuid[], $2::uuid[], $3::uuid[], $4::text[], $5::int4[], $6::int8[], $7::int8[])"#,
        &line_ids,
        &line_invoice_ids,
        &line_entry_ids,
        &line_descriptions,
        &line_minutes,
        &line_rates,
        &line_amounts,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    // Mark entries as invoiced. The row locks taken above already exclude
    // concurrent writers; re-checking state and invoice_id here is the final
    // guarantee that exactly the entries behind the line items get flipped.
    let entry_ids: Vec<uuid::Uuid> = entries.iter().map(|e| e.entry_id).collect();
    let flipped = sqlx::query!(
        r#"UPDATE time_entries
           SET invoice_id = $1,
               state = 'invoiced'
           WHERE id = ANY($2)
             AND invoice_id IS NULL
             AND state IN ('open', 'approved')"#,
        invoice_id,
        &entry_ids,
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

    tx.commit().await.map_err(server_err)?;

    let invoice = Invoice {
        id: invoice_id,
        org_id: manager.org_id,
        client_id,
        number: invoice_number,
        status: InvoiceStatus::Draft,
        issued_on,
        due_on,
        currency: client.currency.trim().to_string(),
        total_cents,
        notes: None,
        created_at: now,
    };

    // Dispatch invoice_created event (FR-019).
    let state = crate::state::global_state().await;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::InvoiceCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            invoice: invoice_payload(&invoice),
        });

    Ok(InvoiceWithLines { invoice, lines })
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

    let current_status: InvoiceStatus = sqlx::query_scalar!(
        r#"SELECT status as "status: InvoiceStatus"
           FROM invoices WHERE id = $1 AND org_id = $2"#,
        id,
        manager.org_id,
    )
    .fetch_optional(&state.db)
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

    // Voiding both un-invoices the covered entries and flips the invoice status,
    // so run them in one transaction: entries must never be released without the
    // invoice actually reaching 'void'.
    let mut tx = state.db.begin().await.map_err(server_err)?;

    // On void: restore entries to open, un-invoiced state.
    if target == InvoiceStatus::Void {
        sqlx::query!(
            r#"UPDATE time_entries
               SET invoice_id = NULL, state = 'open'
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
                     currency, total_cents, notes,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        target as InvoiceStatus,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;

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
