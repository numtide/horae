use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use sqlx::PgPool;
use uuid::Uuid;

async fn single_fee(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE projects SET project_type = 'fixed_fee', starts_on = '2026-09-01' WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,fee_mode,fee_amount_cents) VALUES ($1,$2,$3,$4,'person','single',12500)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
        .execute(pool).await.unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn discounted_fee_leaves_a_balance_and_void_releases_only_its_contribution(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let discounted = InvoiceDefaults {
        discount_bps: 1000,
        ..Default::default()
    };
    let first = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&discounted),
    )
    .await
    .unwrap();
    let remaining = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .expect("the discounted portion must remain invoiceable");
    assert_eq!(remaining.subtotal_cents, 1250);
    let second = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, period.0, period.1)
        .await
        .unwrap();
    assert_eq!(second.invoice.total_cents, 1250);
    assert_eq!(
        first.lines[0].fee_occurrence_id,
        second.lines[0].fee_occurrence_id
    );
    let error = update_invoice_defaults_in_db(
        &pool,
        ids.org_id,
        first.invoice.id,
        &InvoiceDefaults::default(),
    )
    .await
    .expect_err("removing the discount would overbill the fee");
    assert!(error.to_string().contains("balance"), "{error}");
    transition_invoice(&pool, ids.org_id, second.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    update_invoice_defaults_in_db(
        &pool,
        ids.org_id,
        first.invoice.id,
        &InvoiceDefaults::default(),
    )
    .await
    .unwrap();
    assert!(
        preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
            .await
            .is_err()
    );
    transition_invoice(&pool, ids.org_id, first.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    let restored = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(restored.subtotal_cents, 12500);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn concurrent_discounted_invoice_retries_return_one_invoice(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults {
        discount_bps: 1000,
        ..Default::default()
    };
    let request = Some((Uuid::now_v7(), ids.user_id));
    let (a, b) = tokio::join!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            request
        ),
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            request
        ),
    );
    let (a, created_a) = a.unwrap();
    let (b, created_b) = b.unwrap();
    assert_eq!(a.invoice.id, b.invoice.id);
    assert_ne!(created_a, created_b, "only one call may dispatch creation");
    assert_eq!(a.lines, b.lines);
    let remaining = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(remaining.subtotal_cents, 1250);
    let changed = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        None,
        request,
    )
    .await;
    assert!(matches!(
        changed,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    transition_invoice(&pool, ids.org_id, a.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    let (replayed, created) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        request,
    )
    .await
    .unwrap();
    assert_eq!(replayed.invoice.status, InvoiceStatus::Void);
    assert!(
        !created,
        "retrying a void invoice must not create another draft"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn mixed_time_and_fee_discount_conserves_cents_without_reopening_time(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents=2 WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let time_project = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents) VALUES ($1,$2,$3,'Hourly','EUR',60)", time_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET project_id=$2,minutes=1 WHERE id=$1",
        entry,
        time_project
    )
    .execute(&pool)
    .await
    .unwrap();
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let mut defaults = InvoiceDefaults {
        discount_bps: 5000,
        ..Default::default()
    };
    let invoice = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let rows = sqlx::query!("SELECT time_entry_id, amount_cents, net_before_tax_cents FROM invoice_line_items WHERE invoice_id=$1 ORDER BY time_entry_id NULLS LAST", invoice.invoice.id).fetch_all(&pool).await.unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.amount_cents, row.net_before_tax_cents))
            .collect::<Vec<_>>(),
        vec![(1, Some(0)), (2, Some(1))]
    );
    defaults.tax1_bps = 10000;
    let taxed = update_invoice_defaults_in_db(&pool, ids.org_id, invoice.invoice.id, &defaults)
        .await
        .unwrap();
    assert_eq!(
        (
            taxed.subtotal_cents,
            taxed.discount_cents,
            taxed.tax1_cents,
            taxed.total_cents
        ),
        (3, 2, 1, 2)
    );
    let remaining = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(remaining.lines.len(), 1);
    assert_eq!(remaining.subtotal_cents, 1);
    assert!(remaining.lines[0].minutes.is_none());
    assert_eq!(
        sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id=$1", entry)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(invoice.invoice.id)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn removing_discount_and_billing_the_remainder_cannot_both_succeed(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let discounted = InvoiceDefaults {
        discount_bps: 1000,
        ..Default::default()
    };
    let original = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&discounted),
    )
    .await
    .unwrap();
    let defaults = InvoiceDefaults::default();
    let (edit, generation) = tokio::join!(
        update_invoice_defaults_in_db(&pool, ids.org_id, original.invoice.id, &defaults),
        generate_invoice_for_period(&pool, ids.org_id, ids.client_id, period.0, period.1),
    );
    assert_eq!(
        usize::from(edit.is_ok()) + usize::from(generation.is_ok()),
        1
    );
    assert_eq!(sqlx::query_scalar!(
        "SELECT sum(l.net_before_tax_cents)::bigint FROM invoice_line_items l JOIN invoices i ON i.id=l.invoice_id WHERE i.org_id=$1 AND i.status <> 'void'",
        ids.org_id,
    ).fetch_one(&pool).await.unwrap(), Some(12500));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn zero_fee_is_invoiced_once_until_voided(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents=0 WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, period.0, period.1)
        .await
        .unwrap();
    assert_eq!(invoice.invoice.total_cents, 0);
    assert!(
        preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
            .await
            .is_err()
    );
    transition_invoice(&pool, ids.org_id, invoice.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    assert_eq!(
        preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
            .await
            .unwrap()
            .subtotal_cents,
        0
    );
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn fee_balance_migration_allocates_discount_without_rewriting_invoice_snapshots(
    pool: PgPool,
) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|m| m.version < 40)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let ids = single_fee(&pool).await;
    let invoice = Uuid::now_v7();
    sqlx::query!("INSERT INTO invoices (id,org_id,client_id,number,status,issued_on,due_on,currency,total_cents,discount_bps,discount_cents) VALUES ($1,$2,$3,'BEFORE-BALANCES','draft','2026-09-01','2026-10-01','EUR',1,5000,2)", invoice, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    let line_ids = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
    for (key, line) in ["a", "b", "c"].into_iter().zip(line_ids.iter().rev()) {
        let fee = Uuid::now_v7();
        sqlx::query!("INSERT INTO project_fee_occurrences (id,org_id,project_id,period_key,due_on,description,amount_cents,currency) VALUES ($1,$2,$3,$4,'2026-09-01','Fee',1,'EUR')", fee, ids.org_id, ids.project_id, key).execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,fee_occurrence_id,description,amount_cents) VALUES ($1,$2,$3,'Fee',1)", line, invoice, fee).execute(&pool).await.unwrap();
    }
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let net = sqlx::query_scalar!(
        "SELECT l.net_before_tax_cents FROM invoice_line_items l JOIN project_fee_occurrences f ON f.id=l.fee_occurrence_id WHERE l.id = ANY($1) ORDER BY f.period_key COLLATE \"C\"",
        &line_ids
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(net, vec![Some(0), Some(0), Some(1)]);
    let header = crate::reports::fetch_invoice_metadata(&pool, invoice, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            header.subtotal_cents,
            header.discount_cents,
            header.total_cents
        ),
        (3, 2, 1)
    );
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM invoice_line_items WHERE invoice_id=$1 AND amount_cents=1 AND description='Fee'", invoice).fetch_one(&pool).await.unwrap(), Some(3));
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM information_schema.role_table_grants WHERE table_name='invoice_generation_requests' AND grantee='PUBLIC'").fetch_one(&pool).await.unwrap(), Some(0));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_preview_does_not_materialize_fees_and_preserves_released_snapshots(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let first = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(first.lines.len(), 1);
    assert_eq!(
        (
            first.lines[0].amount_cents,
            first.lines[0].minutes,
            first.lines[0].rate_cents
        ),
        (12500, None, None)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_fee_occurrences WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    let generated =
        generate_invoice_for_period(&pool, ids.org_id, ids.client_id, period.0, period.1)
            .await
            .unwrap();
    assert!(
        preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
            .await
            .is_err()
    );
    transition_invoice(&pool, ids.org_id, generated.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents = 99999 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let released = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(released.lines, first.lines);
    assert_eq!(released.amounts.unwrap().total_cents, 12500);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_apply_to_selected_fees_without_claiming_other_projects(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let other = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,project_type,starts_on) VALUES ($1,$2,$3,'Other fee','USD','fixed_fee','2026-09-01')", other, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,fee_mode,fee_amount_cents,terms_days) VALUES ($1,$2,$3,$4,'person','single',90000,90)", Uuid::now_v7(), ids.org_id, other, ids.user_id).execute(&pool).await.unwrap();
    let other_fee = Uuid::now_v7();
    sqlx::query!("INSERT INTO project_fee_occurrences (id,org_id,project_id,period_key,due_on,description,amount_cents,currency) VALUES ($1,$2,$3,'single','2026-09-01','Other fee',90000,'USD')", other_fee, ids.org_id, other).execute(&pool).await.unwrap();
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let overrides = InvoiceDefaults {
        terms_days: 14,
        discount_bps: 1000,
        tax1_bps: 2100,
        ..Default::default()
    };
    assert!(
        preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
            .await
            .is_err()
    );
    let estimate = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        Some(&[ids.project_id]),
        Some(&overrides),
    )
    .await
    .unwrap();
    assert_eq!(estimate.lines.len(), 1);
    assert_eq!(estimate.amounts.unwrap().total_cents, 13613);
    let result = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        Some(&[ids.project_id]),
        Some(&overrides),
    )
    .await
    .unwrap();
    assert_eq!(result.lines.len(), 1);
    assert!(result.lines[0].fee_occurrence_id.is_some());
    assert_eq!(result.lines[0].time_entry_id, None);
    assert_eq!(
        (
            result.invoice.subtotal_cents,
            result.invoice.discount_cents,
            result.invoice.tax1_cents,
            result.invoice.total_cents
        ),
        (12500, 1250, 2363, 13613)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM invoice_line_items WHERE fee_occurrence_id = $1",
            other_fee
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    transition_invoice(&pool, ids.org_id, result.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    let stored = crate::reports::fetch_invoice_metadata(&pool, result.invoice.id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.total_cents, 13613);
    assert_eq!(stored.tax1_cents, 2363);
}

#[sqlx::test(migrations = "./migrations")]
async fn single_fee_can_be_invoiced_without_time(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let from = "2026-09-01".parse().unwrap();
    let to = "2026-09-30".parse().unwrap();
    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    assert_eq!((result.lines.len(), result.invoice.total_cents), (1, 12500));
    assert_eq!(
        (
            result.lines[0].time_entry_id,
            result.lines[0].minutes,
            result.lines[0].rate_cents
        ),
        (None, None, None)
    );
    assert!(result.lines[0].fee_occurrence_id.is_some());
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM time_entries WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_invoices_claim_a_single_fee_once(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let from = "2026-09-01".parse().unwrap();
    let to = "2026-09-30".parse().unwrap();
    let (first, second) = tokio::join!(
        generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to),
        generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to),
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM invoices WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_fee_occurrences WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn void_releases_fee_identity_without_rewriting_the_original_line(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let from = "2026-09-01".parse().unwrap();
    let to = "2026-09-30".parse().unwrap();
    let first = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    transition_invoice(&pool, ids.org_id, first.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents = 99999 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let second = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    assert_eq!(
        first.lines[0].fee_occurrence_id,
        second.lines[0].fee_occurrence_id
    );
    assert_eq!(second.invoice.total_cents, 12500);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT amount_cents FROM invoice_line_items WHERE id = $1",
            first.lines[0].id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        12500
    );
    assert_ne!(first.invoice.id, second.invoice.id);
}

#[sqlx::test(migrations = "./migrations")]
async fn milestones_include_unbilled_overdue_fees_but_not_future_fees(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'milestones', fee_amount_cents = NULL WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    for (position, date, amount) in [
        (0_i16, "2026-08-01", 1000_i64),
        (1, "2026-09-15", 2000),
        (2, "2026-10-01", 3000),
    ] {
        sqlx::query!("INSERT INTO project_fee_milestones (id,org_id,project_id,name,due_on,amount_cents,position) VALUES ($1,$2,$3,$4,$5,$6,$7)", Uuid::now_v7(), ids.org_id, ids.project_id, format!("Milestone {position}"), date.parse::<chrono::NaiveDate>().unwrap() as chrono::NaiveDate, amount, position)
            .execute(&pool).await.unwrap();
    }
    let estimate = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(estimate.lines.len(), 2);
    assert_eq!(estimate.amounts.unwrap().total_cents, 3000);
    let invoice = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        (invoice.lines.len(), invoice.invoice.total_cents),
        (2, 3000)
    );
    assert!(
        invoice
            .lines
            .iter()
            .all(|line| line.minutes.is_none() && line.rate_cents.is_none())
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn monthly_fees_use_calendar_dates_and_skip_already_claimed_months(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'monthly', monthly_day = 'last' WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let estimate = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        ("2028-02-16".parse().unwrap(), "2028-03-31".parse().unwrap()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(estimate.lines.len(), 2);
    assert_eq!(estimate.amounts.unwrap().total_cents, 25000);
    let first = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2028-02-16".parse().unwrap(),
        "2028-03-31".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!((first.lines.len(), first.invoice.total_cents), (2, 25000));
    let dates = sqlx::query_scalar!(r#"SELECT due_on as "due_on: chrono::NaiveDate" FROM project_fee_occurrences WHERE project_id = $1 ORDER BY due_on"#, ids.project_id).fetch_all(&pool).await.unwrap();
    assert_eq!(
        dates.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["2028-02-29", "2028-03-31"]
    );
    let estimate = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        ("2028-02-01".parse().unwrap(), "2028-04-30".parse().unwrap()),
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(estimate.lines.len(), 1);
    assert_eq!(estimate.amounts.unwrap().total_cents, 12500);
    assert!(estimate.lines[0].description.contains("2028-04"));
    let next = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2028-02-01".parse().unwrap(),
        "2028-04-30".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!((next.lines.len(), next.invoice.total_cents), (1, 12500));
    assert!(next.lines[0].description.contains("2028-04"));
}

#[sqlx::test(migrations = "./migrations")]
async fn fee_total_overflow_rolls_back_occurrences_and_invoice(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'monthly', monthly_day = 'first', fee_amount_cents = $2 WHERE project_id = $1", ids.project_id, i64::MAX).execute(&pool).await.unwrap();
    let result = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-10-31".parse().unwrap(),
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("exceeds the supported range")
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_fee_occurrences WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM invoices WHERE org_id = $1",
            ids.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn fee_invoice_does_not_claim_hours_or_another_clients_fee(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let other = single_fee(&pool).await;
    let entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    let invoice = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(invoice.invoice.total_cents, 12500);
    assert!(
        sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", entry)
            .fetch_one(&pool)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_fee_occurrences WHERE org_id = $1",
            other.org_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invoice_line_sources_reject_mixed_sources_and_invented_fee_quantities(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    let invoice = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    let fee = invoice.lines[0].fee_occurrence_id;
    for (time, fee, minutes, rate) in [
        (None, None, None, None),
        (Some(entry), fee, Some(60), Some(100_i64)),
        (None, fee, Some(0), None),
        (None, fee, None, Some(0)),
        (Some(entry), None, None, Some(100)),
    ] {
        let error = sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,fee_occurrence_id,description,minutes,rate_cents,amount_cents) VALUES ($1,$2,$3,$4,'Invalid source',$5,$6,0)", Uuid::now_v7(), invoice.invoice.id, time, fee, minutes, rate)
            .execute(&pool).await.unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("23514")
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn fee_occurrence_rejects_cross_organization_project(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let other = seed(&pool, OrgRole::Manager).await;
    let error = sqlx::query!("INSERT INTO project_fee_occurrences (id,org_id,project_id,period_key,due_on,description,amount_cents,currency) VALUES ($1,$2,$3,'single','2026-09-01','Foreign',100,'EUR')", Uuid::now_v7(), other.org_id, ids.project_id)
        .execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn monthly_first_and_fifteenth_respect_partial_period_and_project_end(pool: PgPool) {
    for (day, expected) in [("first", "2028-02-01"), ("fifteenth", "2028-01-15")] {
        let ids = single_fee(&pool).await;
        sqlx::query!("UPDATE project_settings SET fee_mode = 'monthly', monthly_day = $2 WHERE project_id = $1", ids.project_id, day).execute(&pool).await.unwrap();
        sqlx::query!(
            "UPDATE projects SET ends_on = '2028-02-14' WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let invoice = generate_invoice_for_period(
            &pool,
            ids.org_id,
            ids.client_id,
            "2028-01-15".parse().unwrap(),
            "2028-03-31".parse().unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(
            (invoice.lines.len(), invoice.invoice.total_cents),
            (1, 12500)
        );
        let date = sqlx::query_scalar!(r#"SELECT due_on as "due_on: chrono::NaiveDate" FROM project_fee_occurrences WHERE project_id = $1"#, ids.project_id).fetch_one(&pool).await.unwrap();
        assert_eq!(date.to_string(), expected);
    }
}
