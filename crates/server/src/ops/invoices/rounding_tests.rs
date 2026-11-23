//! Per-entry amounts round to the currency's precision everywhere they are
//! computed (spec 026, User Story 3): invoice previews and lines, the time
//! report, project figures and amount budgets agree, and totals are sums of
//! the rounded lines.

use chrono::NaiveDate;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::OrganizationLock;
use crate::models::OrgBranding;
use crate::ops::reports::ReportFilters;
use crate::ops::testing::Recorder;
use crate::ops::{Caller, WriteIsolation, begin_gated, begin_read, begin_read_snapshot};
use crate::testing::{SeedIds, seed, time_entry};

const DAY: &str = "2026-09-07";

fn day() -> NaiveDate {
    DAY.parse().unwrap()
}

/// A time-and-materials project in `currency`, billed by task rate, with an
/// amount budget and two entries: 80.50 × 90 min and 5.00 × 30 min.
async fn project_in(pool: &PgPool, currency: &str) -> SeedIds {
    let ids = seed(pool, BuiltInProfile::Administrator).await;
    sqlx::query!(
        "UPDATE clients SET currency = $2 WHERE id = $1",
        ids.client_id,
        currency
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET currency = $2, budget_kind = 'amount', budget_amount_cents = 10000000
         WHERE id = $1",
        ids.project_id,
        currency
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode)
         VALUES ($1, $2, $3, $4, 'task')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let cheap_task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Review')",
        cheap_task,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    for (task_id, rate, minutes) in [(ids.task_id, 8050_i64, 90), (cheap_task, 500, 30)] {
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
             VALUES ($1, $2, true, $3)",
            ids.project_id,
            task_id,
            rate
        )
        .execute(pool)
        .await
        .unwrap();
        let entry = time_entry(pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE time_entries SET task_id = $2, minutes = $3 WHERE id = $1",
            entry,
            task_id,
            minutes
        )
        .execute(pool)
        .await
        .unwrap();
    }
    ids
}

/// The billable amount of the time report, the project's tracked amount and
/// its budget consumption.
async fn figures(pool: &PgPool, ids: &SeedIds) -> (Option<i64>, Option<i64>, i64) {
    let caller = Caller::session(ids.org_id, ids.user_id);
    let mut read = begin_read(pool, caller).await.unwrap();
    let report = crate::ops::reports::time(
        &mut read.tx,
        &read.actor,
        (day(), day()),
        "project",
        ReportFilters::default(),
    )
    .await;
    let report = read.finish(report).await.unwrap();
    let mut read = begin_read(pool, caller).await.unwrap();
    let spend = crate::ops::projects::spend(&mut read.tx, &read.actor, None).await;
    let spend = read.finish(spend).await.unwrap();
    let budget = crate::notifications::budgets::configured_progress(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        ids.project_id,
        day(),
    )
    .await
    .unwrap();
    (
        report[0].billable_cents,
        spend[0].spent_cents,
        budget[0].consumed,
    )
}

fn branding() -> OrgBranding {
    OrgBranding {
        provider_name: None,
        provider_address: None,
        provider_tax_id: None,
        provider_email: None,
        provider_phone: None,
        bank_name: None,
        bank_iban: None,
        bank_bic: None,
        bank_routing: None,
        bank_account: None,
        invoice_notes: None,
        invoice_payment_terms: None,
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_consumer_rounds_lines_to_the_currency_and_sums_them(pool: PgPool) {
    for (currency, expected) in [
        ("JPY", [12100, 300]),
        ("EUR", [12075, 250]),
        ("KWD", [12075, 250]),
    ] {
        let ids = project_in(&pool, currency).await;
        let caller = Caller::session(ids.org_id, ids.user_id);
        let total: i64 = expected.iter().sum();

        let mut read = begin_read_snapshot(&pool, caller, crate::ops::SnapshotLimits::LockWait)
            .await
            .unwrap();
        let preview = super::preview::prepare(
            &mut read.tx,
            &read.actor,
            ids.client_id,
            (day(), day()),
            None,
            None,
            None,
        )
        .await;
        let preview = read.finish(preview).await.unwrap();
        let mut lines: Vec<i64> = preview.lines.iter().map(|line| line.amount_cents).collect();
        lines.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(lines, expected, "{currency} preview lines");
        assert_eq!(preview.subtotal_cents, total, "{currency} preview subtotal");

        assert_eq!(
            figures(&pool, &ids).await,
            (Some(total), Some(total), total),
            "{currency} report, project figures and budget"
        );

        let mut read = begin_read(&pool, caller).await.unwrap();
        let uninvoiced = super::uninvoiced::list(&mut read.tx, &read.actor).await;
        let uninvoiced = read.finish(uninvoiced).await.unwrap();
        let time: Vec<(&str, i64)> = uninvoiced
            .iter()
            .filter(|client| client.client_id == ids.client_id)
            .flat_map(|client| &client.time)
            .map(|amount| (amount.currency.as_str(), amount.cents))
            .collect();
        assert_eq!(time, [(currency, total)], "{currency} work to invoice");

        let mut write = begin_gated(
            &pool,
            caller,
            OrganizationLock::Shared,
            WriteIsolation::ReadCommitted,
        )
        .await
        .unwrap();
        let generated = super::generate(
            &mut write.tx,
            &write.actor,
            super::Generation {
                client_id: ids.client_id,
                period: (day(), day()),
                project_ids: None,
                overrides: None,
                request: None,
            },
        )
        .await;
        let invoice = write.commit(generated, &Recorder::default()).await.unwrap();
        let mut lines: Vec<i64> = invoice.lines.iter().map(|line| line.amount_cents).collect();
        lines.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(lines, expected, "{currency} invoice lines");
        assert_eq!(invoice.invoice.subtotal_cents, total, "{currency} subtotal");

        if currency == "JPY" {
            // The display keeps two decimals for every currency (spec 026 Q3).
            let text =
                crate::render::invoice_text(&invoice.invoice, &invoice.lines, None, &branding());
            for amount in ["121.00", "3.00", "124.00"] {
                assert!(text.contains(amount), "missing {amount}: {text}");
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_invoice_stored_before_the_change_keeps_its_amounts(pool: PgPool) {
    let ids = project_in(&pool, "JPY").await;
    let mut write = begin_gated(
        &pool,
        Caller::session(ids.org_id, ids.user_id),
        OrganizationLock::Shared,
        WriteIsolation::ReadCommitted,
    )
    .await
    .unwrap();
    let generated = super::generate(
        &mut write.tx,
        &write.actor,
        super::Generation {
            client_id: ids.client_id,
            period: (day(), day()),
            project_ids: None,
            overrides: None,
            request: None,
        },
    )
    .await;
    let invoice = write.commit(generated, &Recorder::default()).await.unwrap();
    // What the two-decimal rule stored for 80.50 × 90 min.
    sqlx::query!(
        "UPDATE invoice_line_items SET amount_cents = 12075 WHERE invoice_id = $1 AND amount_cents = 12100",
        invoice.invoice.id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        figures(&pool, &ids).await,
        (Some(12375), Some(12375), 12375)
    );
}

/// An imported JPY invoice on an EUR project: Harvest billed JPY 130 for the
/// 80.50 × 90 min entry, where Horae's own rule gives JPY 121.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_imported_invoice_keeps_its_amounts_and_its_time_rounds_in_its_currency(pool: PgPool) {
    let ids = project_in(&pool, "EUR").await;
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency,
                               total_cents, discount_cents, tax1_cents, tax2_cents, subject, source)
         VALUES ($1, $2, $3, 'H-1', 'sent', $4, $4, 'JPY', 13000, 0, 0, 0, '', 'harvest')",
        invoice_id,
        ids.org_id,
        ids.client_id,
        day() as NaiveDate,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO harvest_invoices (invoice_id, org_id, harvest_invoice_id, due_cents, harvest_updated_at)
         VALUES ($1, $2, 1, 13000, now())",
        invoice_id,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO harvest_invoice_lines (id, org_id, invoice_id, position, harvest_line_id, kind,
                                            description, quantity, unit_price, amount_cents, taxed, taxed2)
         VALUES ($1, $2, $3, 0, 1, 'Service', 'Design', 1, 130, 13000, false, false)",
        Uuid::now_v7(),
        ids.org_id,
        invoice_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    // The import links the time it billed, without a line per entry.
    sqlx::query!(
        "UPDATE time_entries SET invoice_id = $1, state = 'invoiced'
         WHERE project_id = $2 AND minutes = 90",
        invoice_id,
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let caller = Caller::session(ids.org_id, ids.user_id);
    let mut read = begin_read(&pool, caller).await.unwrap();
    let detail = super::detail(&mut read.tx, &read.actor, invoice_id).await;
    let detail = read.finish(detail).await.unwrap();
    assert_eq!(detail.invoice.total_cents, 13000);
    assert_eq!(
        detail.imported.map(|imported| imported.due_cents),
        Some(13000)
    );
    let amounts: Vec<i64> = detail
        .imported_lines
        .iter()
        .map(|line| line.amount_cents)
        .collect();
    assert_eq!(amounts, [13000]);

    // The linked time has no stored line, so it is priced from its rate and
    // rounded in the invoice's currency everywhere: JPY 121, not EUR 120.75.
    let mut read = begin_read(&pool, caller).await.unwrap();
    let report = crate::ops::reports::time(
        &mut read.tx,
        &read.actor,
        (day(), day()),
        "project",
        ReportFilters::default(),
    )
    .await;
    let report: Vec<(String, Option<i64>)> = read
        .finish(report)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.currency, row.billable_cents))
        .collect();
    assert_eq!(
        report,
        [
            ("EUR".to_owned(), Some(250)),
            ("JPY".to_owned(), Some(12100))
        ]
    );
    let mut read = begin_read(&pool, caller).await.unwrap();
    let spend = crate::ops::projects::spend(&mut read.tx, &read.actor, None).await;
    let spend = read.finish(spend).await.unwrap();
    assert_eq!(spend[0].spent_cents, Some(12350));
    let budget = crate::notifications::budgets::configured_progress(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        ids.project_id,
        day(),
    )
    .await
    .unwrap();
    assert_eq!(budget[0].consumed, 12350);
}
