//! Importing Harvest invoices and linking the time they billed (spec 025).

use super::*;
use horae_core::importers::harvest::types::{EntryScan, InvoiceErrorKind, LinkCounts};
use horae_core::types::{EntryState, InvoiceStatus};

/// A JSON number keeping its decimal text exactly, as Harvest sends it.
fn num(text: &str) -> serde_json::Value {
    serde_json::Value::Number(text.parse().unwrap())
}

fn invoice(id: i64, number: &str, state: &str, lines: &[&str], total: &str, due: &str) -> serde_json::Value {
    let line_items: Vec<_> = lines
        .iter()
        .enumerate()
        .map(|(n, amount)| {
            json!({"id": id * 10 + n as i64, "kind": "Service", "description": format!("Line {n}"),
                   "quantity": 1, "unit_price": num(amount), "amount": num(amount),
                   "taxed": false, "taxed2": false, "project": {"id": 2}})
        })
        .collect();
    json!({"id": id, "client": {"id": 1}, "number": number, "purchase_order": "PO-7",
           "amount": num(total), "due_amount": num(due), "tax": null, "tax_amount": 0, "tax2": null,
           "tax2_amount": 0, "discount": null, "discount_amount": 0, "subject": "Work",
           "notes": "Thanks", "currency": "USD", "state": state,
           "period_start": "2026-01-01", "period_end": "2026-01-31",
           "issue_date": "2026-02-01", "due_date": "2026-03-01", "paid_date": null,
           "updated_at": day(3), "line_items": line_items})
}

fn entry(id: i64, date: u32, billed_on: Option<i64>) -> serde_json::Value {
    json!({"id": id, "spent_date": format!("2026-01-{date:02}"), "hours": 1.25,
           "project": {"id": 2}, "task": {"id": 3}, "user": {"id": 4},
           "is_billed": billed_on.is_some(),
           "invoice": billed_on.map(|id| json!({"id": id, "number": id.to_string()})),
           "updated_at": day(3)})
}

/// Quickstart §2: invoices of each state, a draft, a number a Horae invoice
/// already has, amounts that do not add up, and entries billed on each, one on
/// an invoice the list does not return, and one not billed.
fn fixture() -> HarvestData {
    let mut data = valid_data();
    let mut partly_paid = invoice(100, "1001", "open", &["1000.00", "200.00", "-100.00"], "1050.00", "450.00");
    partly_paid["discount"] = json!(10);
    partly_paid["discount_amount"] = num("110.00");
    partly_paid["tax"] = num("8.875");
    partly_paid["tax_amount"] = num("50.00");
    partly_paid["tax2"] = json!(2);
    partly_paid["tax2_amount"] = num("10.00");
    partly_paid["line_items"][0]["taxed"] = json!(true);
    partly_paid["line_items"][1]["kind"] = json!("Product");
    let mut paid = invoice(101, "1002", "paid", &["300.00"], "300.00", "0");
    paid["paid_date"] = json!("2026-02-10");
    data.invoices = serde_json::from_value(json!([
        partly_paid,
        paid,
        invoice(102, "1003", "closed", &["50.00"], "50.00", "50.00"),
        invoice(103, "1004", "draft", &["70.00"], "70.00", "70.00"),
        invoice(104, "HORAE-1", "open", &["10.00"], "10.00", "10.00"),
        invoice(105, "1006", "open", &["100.00"], "101.00", "101.00"),
    ]))
    .unwrap();
    data.time_entries = serde_json::from_value(json!([
        entry(10, 10, Some(100)),
        entry(11, 11, Some(101)),
        entry(12, 12, Some(102)),
        entry(13, 13, Some(103)),
        entry(14, 14, Some(104)),
        entry(15, 15, Some(105)),
        entry(16, 16, Some(999)),
        entry(17, 17, None),
    ]))
    .unwrap();
    data
}

/// A Horae-made invoice holding the number Harvest invoice 104 has.
async fn horae_invoice(pool: &PgPool, org: Uuid, number: &str) -> Uuid {
    let client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Horae client', 'USD')",
        client,
        org,
    )
    .execute(pool)
    .await
    .unwrap();
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency)
         VALUES ($1, $2, $3, $4, 'sent', '2026-01-01', '2026-01-31', 'USD')",
        id,
        org,
        client,
        number,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Each entry's state and invoice number, by Harvest entry id.
async fn billing(pool: &PgPool, org: Uuid) -> Vec<(i64, EntryState, Option<String>, Option<i32>)> {
    sqlx::query!(
        r#"SELECT m.harvest_import_id, te.state AS "state: EntryState", i.number AS "number?",
                  te.rounded_minutes
           FROM harvest_import_map m
           JOIN time_entries te ON te.id = m.horae_id
           LEFT JOIN invoices i ON i.id = te.invoice_id
           WHERE m.org_id = $1 AND m.harvest_entity_type = 'time_entry'
           ORDER BY 1"#,
        org,
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.harvest_import_id, row.state, row.number, row.rounded_minutes))
    .collect()
}

/// Every row an import writes for invoices and links, to compare runs.
async fn snapshot(pool: &PgPool, org: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
             'invoices', (SELECT jsonb_agg(to_jsonb(i) ORDER BY i.id) FROM invoices i WHERE i.org_id = $1),
             'harvest', (SELECT jsonb_agg(to_jsonb(h) ORDER BY h.invoice_id) FROM harvest_invoices h WHERE h.org_id = $1),
             'lines', (SELECT jsonb_agg(to_jsonb(l) ORDER BY l.invoice_id, l.position) FROM harvest_invoice_lines l WHERE l.org_id = $1),
             'entries', (SELECT jsonb_agg(jsonb_build_array(te.id, te.state, te.invoice_id, te.rounded_minutes, te.updated_at) ORDER BY te.id)
                         FROM time_entries te WHERE te.org_id = $1)
           ) AS "snapshot!""#,
        org,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// The migration loosens Horae's rounding checks for imported invoices only:
/// each still refuses a Horae invoice it refused before, and Closed and
/// rates stay Harvest's.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn horae_invoices_keep_their_checks(pool: PgPool) {
    let org = setup(&pool).await;
    let client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'C', 'USD')",
        client,
        org
    )
    .execute(&pool)
    .await
    .unwrap();
    // A total of 1000 with `adjust` as both discount and tax: with no rates,
    // only an adjustment of 0 adds up under Horae's rounding.
    let insert = |number: &'static str, source: &'static str, status: &'static str, discount_bps: i16, adjust: i64| {
        let pool = pool.clone();
        async move {
            sqlx::query!(
                "INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency,
                                       total_cents, discount_cents, tax1_cents, discount_bps,
                                       status, source)
                 VALUES ($1, $2, $3, $4, '2026-01-01', '2026-01-31', 'USD', 1000, $8, $8, $5,
                         $6::text::invoice_status, $7::text::invoice_source)",
                Uuid::now_v7(),
                org,
                client,
                number,
                discount_bps,
                status,
                source,
                adjust,
            )
            .execute(&pool)
            .await
            .map_err(|error| error.as_database_error().unwrap().constraint().unwrap().to_owned())
        }
    };
    insert("0", "horae", "sent", 0, 0).await.unwrap();
    // Constraints are checked by name, so the first is the one reported.
    assert_eq!(insert("1", "horae", "sent", 0, 100).await.unwrap_err(), "invoice_discount_exact");
    insert("2", "harvest", "sent", 0, 100).await.unwrap();
    assert_eq!(insert("3", "horae", "closed", 0, 0).await.unwrap_err(), "invoices_closed_imported");
    insert("4", "harvest", "closed", 0, 100).await.unwrap();
    assert_eq!(
        insert("5", "harvest", "sent", 1000, 100).await.unwrap_err(),
        "invoices_harvest_no_rates"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_commit_imports_invoices_and_links_the_time_they_billed(pool: PgPool) {
    let org = setup(&pool).await;
    horae_invoice(&pool, org, "HORAE-1").await;
    let source = fixture();
    let before = snapshot(&pool, org).await;

    let preview = apply_api_data(&pool, org, "USD", ImportMode::DryRun, &source, day(4))
        .await
        .unwrap();
    assert_eq!(snapshot(&pool, org).await, before, "a preview writes nothing");
    let commit = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();

    for report in [&preview, &commit] {
        assert_eq!(
            (report.summary.invoices.created, report.summary.invoices.errored),
            (3, 2)
        );
        assert_eq!(report.invoices_skipped_drafts, 1);
        assert_eq!(
            report.summary.links,
            LinkCounts {
                linked: 3,
                unlinked: 0,
                billed_without_invoice: 3,
                conflicts: 0
            }
        );
        let kinds: Vec<_> = report
            .row_errors
            .iter()
            .map(|error| (error.source_location.as_str(), error.invoice_kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("invoice 104 (HORAE-1)", Some(InvoiceErrorKind::NumberTaken)),
                ("invoice 105 (1006)", Some(InvoiceErrorKind::AmountsMismatch)),
            ]
        );
        assert!(report.reconciles());
    }
    assert!(commit.row_errors[1].reason.contains("difference of -100"), "{:?}", commit.row_errors);

    let billed = |number: &str| (EntryState::Invoiced, Some(number.to_owned()), Some(75));
    let without = (EntryState::Invoiced, None, Some(75));
    let open = (EntryState::Open, None, None);
    let states: Vec<_> = billing(&pool, org)
        .await
        .into_iter()
        .map(|(id, state, number, rounded)| (id, (state, number, rounded)))
        .collect();
    assert_eq!(
        states,
        [
            (10, billed("1001")),
            (11, billed("1002")),
            (12, billed("1003")),
            (13, open.clone()),
            (14, without.clone()),
            (15, without.clone()),
            (16, without),
            (17, open),
        ]
    );

    let invoice = sqlx::query!(
        r#"SELECT i.status AS "status: InvoiceStatus", i.total_cents, i.subtotal_cents,
                  i.discount_cents, i.tax1_cents, i.tax2_cents, i.po_number, i.subject, i.notes,
                  h.due_cents, h.discount_rate::text AS "discount_rate?",
                  h.tax1_rate::text AS "tax1_rate?", h.tax2_rate::text AS "tax2_rate?"
           FROM invoices i JOIN harvest_invoices h ON h.invoice_id = i.id
           WHERE i.org_id = $1 AND i.number = '1001'"#,
        org,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(invoice.status, InvoiceStatus::Sent);
    assert_eq!(
        (
            invoice.subtotal_cents,
            invoice.discount_cents,
            invoice.tax1_cents,
            invoice.tax2_cents,
            invoice.total_cents,
            invoice.due_cents
        ),
        (110_000, 11_000, 5000, 1000, 105_000, 45_000)
    );
    assert_eq!(
        (
            invoice.discount_rate.as_deref(),
            invoice.tax1_rate.as_deref(),
            invoice.tax2_rate.as_deref()
        ),
        (Some("10"), Some("8.875"), Some("2"))
    );
    assert_eq!(
        (invoice.po_number.as_str(), invoice.subject.as_str(), invoice.notes.as_deref()),
        ("PO-7", "Work", Some("Thanks"))
    );
    let lines = sqlx::query!(
        r#"SELECT l.kind, l.amount_cents, l.taxed, l.project_id IS NOT NULL AS "project!"
           FROM harvest_invoice_lines l JOIN invoices i ON i.id = l.invoice_id
           WHERE i.org_id = $1 AND i.number = '1001' ORDER BY l.position"#,
        org,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        lines
            .iter()
            .map(|l| (l.kind.as_str(), l.amount_cents, l.taxed, l.project))
            .collect::<Vec<_>>(),
        [
            ("Service", 100_000, true, true),
            ("Product", 20_000, false, true),
            ("Service", -10_000, false, true)
        ]
    );
    let statuses = sqlx::query!(
        r#"SELECT number, status AS "status: InvoiceStatus" FROM invoices
           WHERE org_id = $1 AND source = 'harvest' ORDER BY number"#,
        org,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        statuses
            .iter()
            .map(|row| (row.number.as_str(), row.status))
            .collect::<Vec<_>>(),
        [
            ("1001", InvoiceStatus::Sent),
            ("1002", InvoiceStatus::Paid),
            ("1003", InvoiceStatus::Closed)
        ]
    );
    let events = sqlx::query_scalar!(
        "SELECT count(*) FROM horae_outbox WHERE org_id = $1 AND event_kind ILIKE '%invoice%'",
        org,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, Some(0), "an import raises no invoice events");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unchanged_rerun_skips_every_invoice_and_writes_nothing(pool: PgPool) {
    let org = setup(&pool).await;
    horae_invoice(&pool, org, "HORAE-1").await;
    let source = fixture();
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    let first = snapshot(&pool, org).await;
    let again = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(5))
        .await
        .unwrap();
    assert_eq!(
        (again.summary.invoices.created, again.summary.invoices.updated),
        (0, 0)
    );
    assert_eq!(again.summary.invoices.skipped, 3);
    assert_eq!(again.summary.links, LinkCounts::default());
    assert_eq!(snapshot(&pool, org).await, first);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn entries_imported_before_invoices_are_linked_by_a_full_scan(pool: PgPool) {
    let org = setup(&pool).await;
    // A run from before this feature: entries only, no invoices read.
    let mut earlier = fixture();
    earlier.invoices.clear();
    for entry in &mut earlier.time_entries {
        entry.is_billed = false;
        entry.invoice = None;
    }
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &earlier, day(4))
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE harvest_credentials SET synced_watermark = synced_watermark - 'invoice'
         WHERE org_id = $1",
        org
    )
    .execute(&pool)
    .await
    .unwrap();
    let connection = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    let invoices = fixture().invoices;
    assert_eq!(
        api_source::entry_scan(
            connection.watermark_for(EntityType::TimeEntry),
            connection.watermark_for(EntityType::Invoice),
            &invoices,
        ),
        (EntryScan::Full, None),
        "the first run with invoices reads every entry"
    );

    let mut source = fixture();
    source.invoices.retain(|invoice| invoice.id != 104);
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(5))
        .await
        .unwrap();
    assert_eq!(report.summary.time_entries.skipped, 8);
    assert_eq!(report.summary.links.linked, 3);
    // 104 is no longer listed, 105 does not add up, 999 never was.
    assert_eq!(report.summary.links.billed_without_invoice, 3);

    let connection = credentials::load(&pool, org, KEY).await.unwrap().unwrap();
    let since = connection.watermark_for(EntityType::TimeEntry);
    let mark = connection.watermark_for(EntityType::Invoice);
    assert!(mark.is_none(), "an invoice error keeps the next run in full");
    assert_eq!(api_source::entry_scan(since, Some(day(3)), &invoices).0, EntryScan::Incremental);
    let mut changed = invoices.clone();
    changed[0].updated_at = day(6);
    assert_eq!(api_source::entry_scan(since, Some(day(3)), &changed).0, EntryScan::Full);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reimports_follow_harvest_and_keep_what_horae_billed(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = fixture();
    source.invoices.retain(|invoice| invoice.id < 104);
    source.time_entries.retain(|entry| entry.id <= 13 || entry.id == 17);
    apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();

    // In Harvest: 1001 is paid; a new invoice bills 17; 12 is no longer
    // billed (its invoice was deleted). In Horae: 1002 was voided, which
    // reopened 11, and 13 (on a Harvest draft) went on a Horae invoice.
    let mut next = source.clone();
    next.invoices[0].state = "paid".into();
    next.invoices[0].due_amount = "0".parse().unwrap();
    next.invoices[0].updated_at = day(5);
    next.invoices.retain(|invoice| invoice.id != 102);
    next.invoices
        .push(serde_json::from_value(invoice(106, "1007", "open", &["20.00"], "20.00", "20.00")).unwrap());
    for entry in &mut next.time_entries {
        match entry.id {
            12 => {
                entry.is_billed = false;
                entry.invoice = None;
                entry.updated_at = Some(day(5));
            }
            17 => {
                entry.is_billed = true;
                entry.invoice = serde_json::from_value(json!({"id": 106})).unwrap();
                entry.updated_at = Some(day(5));
            }
            13 => {
                entry.is_billed = true;
                entry.invoice = serde_json::from_value(json!({"id": 100})).unwrap();
                entry.updated_at = Some(day(5));
            }
            _ => {}
        }
    }
    let voided = sqlx::query_scalar!("SELECT id FROM invoices WHERE org_id = $1 AND number = '1002'", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE invoices SET status = 'void' WHERE id = $1",
        voided
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE time_entries SET invoice_id = NULL, state = 'open', rounded_minutes = NULL WHERE invoice_id = $1", voided)
        .execute(&pool)
        .await
        .unwrap();
    let horae = horae_invoice(&pool, org, "HORAE-9").await;
    sqlx::query!(
        "UPDATE time_entries te SET invoice_id = $2, state = 'invoiced'
         FROM harvest_import_map m
         WHERE m.horae_id = te.id AND m.harvest_import_id = 13 AND m.org_id = $1",
        org,
        horae,
    )
    .execute(&pool)
    .await
    .unwrap();

    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &next, day(6))
        .await
        .unwrap();
    assert_eq!(
        (
            report.summary.invoices.created,
            report.summary.invoices.updated,
            report.summary.invoices.skipped
        ),
        (1, 1, 1)
    );
    assert_eq!(report.invoices_no_longer_listed, 1);
    assert_eq!(
        report.summary.links,
        LinkCounts {
            linked: 1,
            unlinked: 1,
            billed_without_invoice: 0,
            conflicts: 1
        }
    );
    assert_eq!(report.link_conflicts.len(), 1);
    assert_eq!(report.link_conflicts[0].harvest_time_entry_id, Some(13));
    assert_eq!(report.link_conflicts[0].horae_invoice_number.as_deref(), Some("HORAE-9"));

    let states: Vec<_> = billing(&pool, org)
        .await
        .into_iter()
        .map(|(id, state, number, _)| (id, state, number))
        .collect();
    assert_eq!(
        states,
        [
            (10, EntryState::Invoiced, Some("1001".to_owned())),
            // Harvest has not changed 1002, so its void in Horae stands.
            (11, EntryState::Open, None),
            (12, EntryState::Open, None),
            (13, EntryState::Invoiced, Some("HORAE-9".to_owned())),
            (17, EntryState::Invoiced, Some("1007".to_owned())),
        ]
    );
    let statuses = sqlx::query!(
        r#"SELECT i.number, i.status AS "status: InvoiceStatus", h.due_cents
           FROM invoices i JOIN harvest_invoices h ON h.invoice_id = i.id
           WHERE i.org_id = $1 ORDER BY i.number"#,
        org,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        statuses
            .iter()
            .map(|row| (row.number.as_str(), row.status, row.due_cents))
            .collect::<Vec<_>>(),
        [
            ("1001", InvoiceStatus::Paid, 0),
            ("1002", InvoiceStatus::Void, 0),
            ("1003", InvoiceStatus::Closed, 5000),
            ("1007", InvoiceStatus::Sent, 2000),
        ]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invoice_errors_name_the_reason_and_leave_their_time_billed(pool: PgPool) {
    let org = setup(&pool).await;
    let mut source = valid_data();
    let mut precise = invoice(201, "2001", "open", &["10.005"], "10.005", "0");
    precise["line_items"][0]["unit_price"] = num("10.00");
    let mut currency = invoice(202, "2002", "open", &["10.00"], "10.00", "0");
    currency["currency"] = json!("usd");
    let mut client = invoice(203, "2003", "open", &["10.00"], "10.00", "0");
    client["client"] = json!({"id": 77});
    source.invoices = serde_json::from_value(json!([
        precise,
        currency,
        client,
        invoice(204, "2004", "open", &["-10.00"], "-10.00", "0"),
        invoice(205, "2005", "sent", &["10.00"], "10.00", "0"),
    ]))
    .unwrap();
    source.time_entries =
        serde_json::from_value(json!([entry(10, 10, Some(201)), entry(11, 11, Some(204))])).unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(report.summary.invoices.errored, 5);
    assert_eq!(
        report
            .row_errors
            .iter()
            .map(|error| error.invoice_kind)
            .collect::<Vec<_>>(),
        [
            Some(InvoiceErrorKind::AmountPrecision),
            Some(InvoiceErrorKind::UnknownCurrency),
            Some(InvoiceErrorKind::ClientMissing),
            Some(InvoiceErrorKind::NegativeTotal),
            Some(InvoiceErrorKind::UnknownState),
        ]
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM invoices WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    assert!(
        billing(&pool, org)
            .await
            .iter()
            .all(|(_, state, number, _)| *state == EntryState::Invoiced && number.is_none())
    );
}
