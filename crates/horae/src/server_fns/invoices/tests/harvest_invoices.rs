//! Invoices imported from Harvest, as the invoice operations see them
//! (spec 025): read only apart from their status, and the time they billed
//! never offered for billing again.

use super::*;
use uuid::Uuid;

/// An invoice imported from Harvest for the seeded client: a total of 100.00
/// with 40.00 still due, and one line.
async fn imported_invoice(pool: &PgPool, ids: &crate::testing::SeedIds, number: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency,
                               total_cents, source)
         VALUES ($1, $2, $3, $4, 'sent', '2026-09-30', '2026-10-30', 'EUR', 10000, 'harvest')",
        id,
        ids.org_id,
        ids.client_id,
        number,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO harvest_invoices (invoice_id, org_id, harvest_invoice_id, due_cents,
                                       tax1_rate, harvest_updated_at)
         VALUES ($1, $2, $3, 4000, 8.875, now())",
        id,
        ids.org_id,
        i64::from(id.as_u128() as u32),
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO harvest_invoice_lines (id, org_id, invoice_id, position, harvest_line_id, kind,
                                            description, quantity, unit_price_cents, amount_cents,
                                            taxed, taxed2)
         VALUES ($1, $2, $3, 0, 1, 'Service', 'Design', 2.5, 4000, 10000, true, false)",
        Uuid::now_v7(),
        ids.org_id,
        id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// A billable entry on `day`, invoiced on `invoice` or, with none, invoiced
/// with no invoice as an import marks time billed on an invoice it lacks.
async fn billed_entry(
    pool: &PgPool,
    ids: &crate::testing::SeedIds,
    day: &str,
    invoice: Option<Uuid>,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state,
            invoice_id, rounded_minutes)
         VALUES ($1, $2, $3, $4, $5, $6::text::date, 60, true, 'invoiced', $7, 60)",
        id,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id,
        day,
        invoice,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn status(pool: &PgPool, id: Uuid) -> (InvoiceStatus, i64, Option<chrono::NaiveDate>) {
    let row = sqlx::query!(
        r#"SELECT i.status AS "status: InvoiceStatus", h.due_cents,
                  h.paid_on AS "paid_on: chrono::NaiveDate"
           FROM invoices i JOIN harvest_invoices h ON h.invoice_id = i.id WHERE i.id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (row.status, row.due_cents, row.paid_on)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn imported_invoices_are_read_only_apart_from_their_status(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (org, actor) = (ids.org_id, ids.user_id);
    let closed = imported_invoice(&pool, &ids, "H-1").await;
    let paid = imported_invoice(&pool, &ids, "H-2").await;
    let voided = imported_invoice(&pool, &ids, "H-3").await;
    let reopened = billed_entry(&pool, &ids, "2026-09-10", Some(voided)).await;

    let refused = editing::load(&pool, org, actor, closed).await.unwrap_err();
    assert!(refused.to_string().contains("imported from Harvest"), "{refused}");
    let mut read = begin_read(&pool, org, actor).await.unwrap();
    let refused = crate::ops::invoices::duplicate(&mut read.tx, &read.actor, closed)
        .await
        .unwrap_err();
    assert!(refused.to_string().contains("imported from Harvest"), "{refused}");
    drop(read);
    for target in [InvoiceStatus::Draft, InvoiceStatus::Sent] {
        assert!(transition_invoice(&pool, org, closed, target, actor).await.is_err());
    }

    transition_invoice(&pool, org, closed, InvoiceStatus::Closed, actor)
        .await
        .unwrap();
    assert_eq!(status(&pool, closed).await, (InvoiceStatus::Closed, 4000, None));
    assert!(
        transition_invoice(&pool, org, closed, InvoiceStatus::Paid, actor)
            .await
            .is_err()
    );

    transition_invoice(&pool, org, paid, InvoiceStatus::Paid, actor)
        .await
        .unwrap();
    let (paid_status, due, paid_on) = status(&pool, paid).await;
    assert_eq!((paid_status, due), (InvoiceStatus::Paid, 0));
    assert!(paid_on.is_some(), "paying records the organization's today");

    transition_invoice(&pool, org, voided, InvoiceStatus::Void, actor)
        .await
        .unwrap();
    let state = sqlx::query!(
        r#"SELECT state AS "state: EntryState", invoice_id, rounded_minutes
           FROM time_entries WHERE id = $1"#,
        reopened
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (state.state, state.invoice_id, state.rounded_minutes),
        (EntryState::Open, None, None)
    );

    // Closed stays Harvest's: a Horae invoice cannot be written off.
    let day = "2026-09-07".parse().unwrap();
    time_entry(&pool, &ids, EntryState::Open).await;
    let own = generate_invoice_for_period(&pool, org, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;
    transition_invoice(&pool, org, own.id, InvoiceStatus::Sent, actor)
        .await
        .unwrap();
    assert!(
        transition_invoice(&pool, org, own.id, InvoiceStatus::Closed, actor)
            .await
            .is_err()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn time_harvest_billed_is_never_offered_again(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let (org, actor) = (ids.org_id, ids.user_id);
    let imported = imported_invoice(&pool, &ids, "H-1").await;
    let open = time_entry(&pool, &ids, EntryState::Open).await;
    billed_entry(&pool, &ids, "2026-09-08", None).await;
    billed_entry(&pool, &ids, "2026-09-20", Some(imported)).await;

    let mut read = begin_read(&pool, org, actor).await.unwrap();
    let clients = crate::ops::invoices::uninvoiced::list(&mut read.tx, &read.actor)
        .await
        .unwrap();
    let last = crate::ops::invoices::uninvoiced::last_invoice(&mut read.tx, &read.actor, ids.client_id)
        .await
        .unwrap()
        .unwrap();
    drop(read);
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].unbilled_minutes, 60);
    assert_eq!(clients[0].last_invoice.as_ref().unwrap().id, imported);
    // The imported invoice covered its linked time, not just its issue date.
    assert_eq!(last.covered_to, "2026-09-20".parse().unwrap());
    assert_eq!(last.next_start(), "2026-09-21".parse().ok());

    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let preparation = preview::prepare(&pool, (org, actor), ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(preparation.lines.len(), 1);
    let generated = generate_invoice_for_period(&pool, org, ids.client_id, period.0, period.1)
        .await
        .unwrap();
    assert_eq!(
        generated.lines.iter().map(|line| line.time_entry_id).collect::<Vec<_>>(),
        [Some(open)]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn new_numbers_skip_one_an_import_took(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let prefix = format!("INV-{}-", chrono::Utc::now().format("%Y%m"));
    imported_invoice(&pool, &ids, &format!("{prefix}007")).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let day = "2026-09-07".parse().unwrap();
    let generated = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(generated.invoice.number, format!("{prefix}008"));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn imported_invoices_read_with_their_lines_and_need_invoice_grants(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let id = imported_invoice(&pool, &ids, "H-1").await;
    let detail = fetch_detail(&pool, ids.org_id, ids.user_id, id).await.unwrap();
    assert!(detail.lines.is_empty());
    let imported = detail.imported.unwrap();
    assert_eq!((imported.due_cents, imported.tax1_rate.as_deref()), (4000, Some("8.875")));
    assert_eq!(detail.imported_lines.len(), 1);
    assert_eq!(
        (
            detail.imported_lines[0].quantity.as_str(),
            detail.imported_lines[0].amount_cents
        ),
        ("2.5", 10000)
    );
    let list = fetch_list(&pool, ids.org_id, ids.user_id, None).await.unwrap();
    assert_eq!(list[0].due_cents(), 4000);

    let member = crate::testing::add_person(&pool, &ids, BuiltInProfile::Member)
        .await
        .user_id;
    assert!(fetch_detail(&pool, ids.org_id, member, id).await.is_err());
    assert!(fetch_list(&pool, ids.org_id, member, None).await.is_err());
}
