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
    let text = horae_server::render::invoice_text(&detail.invoice, &detail.lines, &branding);
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

    // Nothing new to bill yet: the source's hour is not copied.
    assert!(
        preview::prepare(
            &pool,
            (ids.org_id, ids.user_id),
            ids.client_id,
            (day(), day()),
            Some(&template.project_ids),
            Some(&template.defaults),
        )
        .await
        .is_err()
    );

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
    assert!(duplicate(&pool, &member, source.id).await.is_err());
    assert!(duplicate(&pool, &ids, Uuid::now_v7()).await.is_err());

    // An invoice of another client cannot lend its subject to this one.
    let other = billable(&pool).await;
    let foreign = generate_invoice_for_period(&pool, other.org_id, other.client_id, day(), day())
        .await
        .unwrap()
        .invoice;
    time_entry(&pool, &ids, EntryState::Open).await;
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
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: Vec::new(),
        duplicate_of: Some(foreign.id),
    };
    assert!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            (day(), day()),
            None,
            None,
            Some((&request, ids.user_id)),
        )
        .await
        .is_err()
    );
}
