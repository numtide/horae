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
