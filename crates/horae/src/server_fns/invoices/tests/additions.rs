//! Overdue, unapproved time, the Uninvoiced view, Subject and Duplicate.

use super::*;
use crate::models::invoice::{InvoiceDuplicate, UninvoicedClient};
use crate::testing::{SeedIds, add_person};
use uuid::Uuid;

async fn uninvoiced(pool: &PgPool, ids: &SeedIds) -> Result<Vec<UninvoicedClient>, ServerFnError> {
    let mut read = begin_read(pool, ids.org_id, ids.user_id).await?;
    let result = crate::ops::invoices::uninvoiced::list(&mut read.tx, &read.actor).await;
    read.finish(result).await.map_err(op_error)
}

async fn duplicate(
    pool: &PgPool,
    ids: &SeedIds,
    invoice_id: Uuid,
) -> Result<InvoiceDuplicate, ServerFnError> {
    let mut read = begin_read(pool, ids.org_id, ids.user_id).await?;
    let result = crate::ops::invoices::duplicate(&mut read.tx, &read.actor, invoice_id).await;
    read.finish(result).await.map_err(op_error)
}

/// A priced project with one open hour on 2026-09-07.
async fn billable(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, BuiltInProfile::ExecutiveManager).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10000 WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    time_entry(pool, &ids, EntryState::Open).await;
    ids
}

fn day() -> chrono::NaiveDate {
    "2026-09-07".parse().unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_sent_invoice_past_its_due_date_reads_as_overdue_until_paid(pool: PgPool) {
    let ids = billable(&pool).await;
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap()
        .invoice;
    let overdue = |pool: PgPool| async move {
        let list = fetch_list(&pool, ids.org_id, ids.user_id, None)
            .await
            .unwrap();
        let detail = fetch_detail(&pool, ids.org_id, ids.user_id, invoice.id)
            .await
            .unwrap();
        assert_eq!(list[0].overdue, detail.overdue);
        detail.overdue
    };
    sqlx::query!(
        "UPDATE invoices SET issued_on = '2026-01-01', due_on = '2026-01-31' WHERE id = $1",
        invoice.id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(!overdue(pool.clone()).await, "a draft is never overdue");
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
    .await
    .unwrap();
    assert!(overdue(pool.clone()).await);
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Paid,
        ids.user_id,
    )
    .await
    .unwrap();
    assert!(!overdue(pool.clone()).await);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn the_preview_counts_time_awaiting_approval_and_leaves_it_out(pool: PgPool) {
    let ids = billable(&pool).await;
    let submitted = time_entry(&pool, &ids, EntryState::Submitted).await;
    let review = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(review.awaiting_approval_minutes, 60);
    assert_eq!(review.lines.len(), 1, "only the open hour is a line");
    assert!(!review.lines.iter().any(|line| line.source
        == InvoiceSource::Time {
            entry_id: submitted
        }));

    sqlx::query!(
        "UPDATE time_entries SET state = 'approved' WHERE id = $1",
        submitted
    )
    .execute(&pool)
    .await
    .unwrap();
    let review = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(review.awaiting_approval_minutes, 0);
    assert_eq!(review.lines.len(), 2);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn uninvoiced_lists_clients_with_unbilled_time_until_it_is_invoiced(pool: PgPool) {
    let ids = billable(&pool).await;
    let clients = uninvoiced(&pool, &ids).await.unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].client_id, ids.client_id);
    assert_eq!(clients[0].unbilled_minutes, 60);
    assert_eq!(clients[0].time.len(), 1);
    assert_eq!(clients[0].time[0].cents, 10000);
    assert!(clients[0].fees.is_empty());

    generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap();
    assert_eq!(uninvoiced(&pool, &ids).await.unwrap(), Vec::new());

    let member = add_person(&pool, &ids, BuiltInProfile::Member).await;
    assert!(uninvoiced(&pool, &member).await.is_err());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn the_subject_is_saved_on_the_draft_and_printed_on_the_pdf(pool: PgPool) {
    let ids = billable(&pool).await;
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap()
        .invoice;
    assert_eq!(invoice.subject, "");
    let mut editor = editing::load(&pool, ids.org_id, ids.user_id, invoice.id)
        .await
        .unwrap();

    editor.edit.subject = "é".repeat(201);
    assert!(
        editing::review(&pool, ids.org_id, ids.user_id, invoice.id, &editor.edit)
            .await
            .is_err()
    );

    editor.edit.subject = "  Website redesign, September  ".into();
    let review = editing::review(&pool, ids.org_id, ids.user_id, invoice.id, &editor.edit)
        .await
        .unwrap();
    let saved = editing::save(
        &pool,
        ids.org_id,
        ids.user_id,
        invoice.id,
        &InvoiceDraftSave {
            request_id: Uuid::now_v7(),
            edit: editor.edit,
            review,
            confirmed_excess: Vec::new(),
        },
    )
    .await
    .unwrap();
    assert_eq!(saved.invoice.subject, "Website redesign, September");
    let detail = fetch_detail(&pool, ids.org_id, ids.user_id, invoice.id)
        .await
        .unwrap();
    assert_eq!(detail.invoice.subject, "Website redesign, September");

    let branding = crate::ops::organization::fetch_branding_from(&pool, ids.org_id)
        .await
        .unwrap();
    let text = horae_server::render::invoice_text(&detail.invoice, &detail.lines, None, &branding);
    assert!(text.contains("Website redesign, September"), "{text}");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn duplicate_copies_the_values_into_a_new_draft_without_billed_time(pool: PgPool) {
    let ids = billable(&pool).await;
    let values = InvoiceDefaults {
        terms_days: 15,
        po_number: "PO-7".into(),
        discount_bps: 500,
        tax1_bps: 2100,
        tax2_name: Some("Local tax".into()),
        tax2_bps: Some(150),
    };
    let source = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        (day(), day()),
        None,
        Some(&values),
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE invoices SET subject = 'Retainer', notes = 'Thanks' WHERE id = $1",
        source.invoice.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let billed = source.lines[0].time_entry_id.unwrap();

    let template = duplicate(&pool, &ids, source.invoice.id).await.unwrap();
    assert_eq!(template.client_id, ids.client_id);
    assert_eq!(template.project_ids, [ids.project_id]);
    assert_eq!(template.defaults, values);

    // Nothing new to bill yet: the source's hour is not copied, and the
    // preview is empty instead of an error.
    let empty = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        Some(&template.project_ids),
        Some(&template.defaults),
    )
    .await
    .unwrap();
    assert!(empty.lines.is_empty());

    let fresh = time_entry(&pool, &ids, EntryState::Open).await;
    let review = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        Some(&template.project_ids),
        Some(&template.defaults),
    )
    .await
    .unwrap();
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: Vec::new(),
        duplicate_of: Some(source.invoice.id),
    };
    let (copy, _) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        (day(), day()),
        Some(&template.project_ids),
        Some(&template.defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    assert_ne!(copy.invoice.id, source.invoice.id);
    assert_eq!(copy.invoice.status, InvoiceStatus::Draft);
    assert_eq!(copy.invoice.subject, "Retainer");
    assert_eq!(copy.invoice.notes.as_deref(), Some("Thanks"));
    assert_eq!(
        (
            copy.invoice.terms_days,
            copy.invoice.po_number.as_str(),
            copy.invoice.discount_bps,
            copy.invoice.tax1_bps,
            copy.invoice.tax2_name.as_deref(),
            copy.invoice.tax2_bps,
        ),
        (15, "PO-7", 500, 2100, Some("Local tax"), Some(150))
    );
    let copied: Vec<_> = copy.lines.iter().map(|line| line.time_entry_id).collect();
    assert_eq!(copied, [Some(fresh)]);
    assert_eq!(
        sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", billed)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(source.invoice.id)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn duplicate_needs_preparation_rights_and_the_same_client(pool: PgPool) {
    let ids = billable(&pool).await;
    let source = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap()
        .invoice;

    let member = add_person(&pool, &ids, BuiltInProfile::Member).await;
    assert_eq!(
        code(duplicate(&pool, &member, source.id).await),
        Some(FORBIDDEN)
    );
    assert_eq!(
        code(duplicate(&pool, &ids, Uuid::now_v7()).await),
        Some(NOT_FOUND)
    );

    // Another client of the same organization cannot lend its subject.
    let other_client = second_client(&pool, &ids).await;
    let foreign = generate_invoice_for_period(&pool, ids.org_id, other_client, day(), day())
        .await
        .unwrap()
        .invoice;
    time_entry(&pool, &ids, EntryState::Open).await;
    let refused = generate_duplicate(&pool, &ids, foreign.id).await;
    assert_eq!(code(refused), Some(NOT_FOUND));

    // Preparing without reading invoices may not copy one either.
    let source_of_this_client = source.id;
    without_invoice_reads(&pool, &ids).await;
    assert_eq!(
        code(duplicate(&pool, &ids, source_of_this_client).await),
        Some(FORBIDDEN)
    );
    let refused = generate_duplicate(&pool, &ids, source_of_this_client).await;
    assert_eq!(code(refused), Some(FORBIDDEN));
}

const FORBIDDEN: u16 = 403;
const NOT_FOUND: u16 = 404;

/// The status a refused call answers with, or `None` when it succeeded.
fn code<T>(result: Result<T, ServerFnError>) -> Option<u16> {
    match result {
        Ok(_) => None,
        Err(ServerFnError::ServerError { code, .. }) => Some(code),
        Err(other) => panic!("unexpected error {other:?}"),
    }
}

/// A second client of `ids`' organization with a priced project and one open
/// hour on the test day.
async fn second_client(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let client = Uuid::now_v7();
    let project = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Second client', 'EUR')",
        client,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents)
         VALUES ($1, $2, $3, 'Second project', 'EUR', 10000)",
        project,
        ids.org_id,
        client
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', 60, true)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        project,
        ids.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    client
}

/// Generates `ids`' client's open time as a duplicate of `source`.
async fn generate_duplicate(
    pool: &PgPool,
    ids: &SeedIds,
    source: Uuid,
) -> Result<(InvoiceWithLines, bool), ServerFnError> {
    let review = preview::prepare(
        pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        Some(&[ids.project_id]),
        None,
    )
    .await?;
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: Vec::new(),
        duplicate_of: Some(source),
    };
    generate_invoice_with_request(
        pool,
        ids.org_id,
        ids.client_id,
        (day(), day()),
        Some(&[ids.project_id]),
        None,
        Some((&request, ids.user_id)),
    )
    .await
}

/// Takes the invoice read grant from `ids`' person, as the permission editor
/// does: the organization's access revision first, then the stored state.
async fn without_invoice_reads(pool: &PgPool, ids: &SeedIds) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision = access_revision + 1 WHERE id = $1",
        ids.org_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants = array_remove(grants, 'invoice_read_all')
         WHERE user_id = $1",
        ids.user_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn overdue_follows_the_organizations_day_not_utc(pool: PgPool) {
    let ids = billable(&pool).await;
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap()
        .invoice;
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
    .await
    .unwrap();
    // UTC+14 is a day ahead of UTC for ten hours of every day, and never behind.
    for zone in ["Pacific/Kiritimati", "Pacific/Pago_Pago"] {
        sqlx::query!(
            "UPDATE organizations SET timezone = $2 WHERE id = $1",
            ids.org_id,
            zone
        )
        .execute(&pool)
        .await
        .unwrap();
        let today = sqlx::query_scalar!(
            r#"SELECT (now() AT TIME ZONE $1)::date AS "today!: chrono::NaiveDate""#,
            zone
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        for (due, overdue) in [(today, false), (today.pred_opt().unwrap(), true)] {
            sqlx::query!(
                "UPDATE invoices SET issued_on = $2::date - 30, due_on = $2 WHERE id = $1",
                invoice.id,
                due as chrono::NaiveDate
            )
            .execute(&pool)
            .await
            .unwrap();
            let detail = fetch_detail(&pool, ids.org_id, ids.user_id, invoice.id)
                .await
                .unwrap();
            assert_eq!(detail.overdue, overdue, "{zone}, due {due}");
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_period_whose_time_all_waits_for_approval_is_an_empty_preview(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    time_entry(&pool, &ids, EntryState::Submitted).await;
    let review = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        (day(), day()),
        None,
        None,
    )
    .await
    .unwrap();
    assert!(review.lines.is_empty());
    assert_eq!(review.awaiting_approval_minutes, 60);
    assert!(!review.currency.is_empty());
    // An empty preview still creates nothing.
    assert!(
        generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
            .await
            .is_err()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn uninvoiced_matches_a_preview_names_the_last_invoice_and_skips_archived_clients(
    pool: PgPool,
) {
    let ids = billable(&pool).await;
    let first = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day(), day())
        .await
        .unwrap()
        .invoice;
    // New time after the invoice, at a rate that rounds: 50 minutes at
    // 100.01 an hour.
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10001 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         VALUES ($1, $2, $3, $4, $5, '2026-09-08', 50, true)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let clients = uninvoiced(&pool, &ids).await.unwrap();
    assert_eq!(clients.len(), 1);
    let last = clients[0].last_invoice.clone().unwrap();
    assert_eq!(last.id, first.id);
    assert_eq!(last.covered_to, day(), "the last day the invoice billed");
    assert_eq!(last.next_start(), day().succ_opt());

    let today = sqlx::query_scalar!(r#"SELECT current_date AS "today!: chrono::NaiveDate""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let preview = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        ("1970-01-01".parse().unwrap(), today),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(clients[0].unbilled_minutes, 50);
    assert_eq!(clients[0].time[0].cents, preview.subtotal_cents);

    let step_two = last_invoice(&pool, &ids).await.unwrap();
    assert_eq!(step_two, Some(last));

    sqlx::query!(
        "UPDATE clients SET active = false WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(uninvoiced(&pool, &ids).await.unwrap(), Vec::new());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn uninvoiced_fees_due_match_a_preview_of_the_same_client(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    // A single fee due since September and a monthly fee from August: the
    // preview finds both, and Uninvoiced must sum the same amounts.
    let monthly = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, project_type, starts_on)
         VALUES ($1, $2, $3, 'Monthly retainer', 'EUR', 'fixed_fee', '2026-08-01')",
        monthly,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET project_type = 'fixed_fee', currency = 'EUR', starts_on = '2026-09-01' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for (project, mode, amount, day) in [
        (ids.project_id, "single", 12500_i64, None),
        (monthly, "monthly", 90000, Some("last")),
    ] {
        sqlx::query!(
            "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, fee_mode, fee_amount_cents, monthly_day)
             VALUES ($1, $2, $1, $3, 'person', $4, $5, $6)",
            project,
            ids.org_id,
            ids.user_id,
            mode,
            amount,
            day
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let today = sqlx::query_scalar!(r#"SELECT current_date AS "today!: chrono::NaiveDate""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let preview = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        ("1970-01-01".parse().unwrap(), today),
        None,
        Some(&InvoiceDefaults::default()),
    )
    .await
    .unwrap();
    let due: i64 = preview
        .lines
        .iter()
        .filter(|line| line.selected)
        .map(|line| line.amount_cents)
        .sum();
    assert!(due > 12500, "the monthly fee adds at least one month");
    let clients = uninvoiced(&pool, &ids).await.unwrap();
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0].fees.len(), 1);
    assert_eq!(clients[0].fees[0].cents, due);

    // Once one fee is on an invoice, it is no longer due in either.
    generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        ("1970-01-01".parse().unwrap(), today),
        Some(&[ids.project_id]),
        Some(&InvoiceDefaults::default()),
    )
    .await
    .unwrap();
    let clients = uninvoiced(&pool, &ids).await.unwrap();
    assert_eq!(clients[0].fees[0].cents, due - 12500);
}

async fn last_invoice(
    pool: &PgPool,
    ids: &SeedIds,
) -> Result<Option<crate::models::invoice::LastInvoice>, ServerFnError> {
    let mut read = begin_read(pool, ids.org_id, ids.user_id).await?;
    let result =
        crate::ops::invoices::uninvoiced::last_invoice(&mut read.tx, &read.actor, ids.client_id)
            .await;
    read.finish(result).await.map_err(op_error)
}
