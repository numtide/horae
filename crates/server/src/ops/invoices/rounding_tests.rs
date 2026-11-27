//! Per-entry amounts round to the currency's precision everywhere they are
//! computed (spec 026, User Story 3): invoice previews and lines, the time
//! report, project figures and amount budgets agree, and totals are sums of
//! the rounded lines. An invoice's discount and taxes round to the same
//! precision by the method it stores, which a new invoice takes from the
//! organization's setting.

use chrono::NaiveDate;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::{EntryState, RoundingMethod};
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

async fn begin_write(pool: &PgPool, ids: &SeedIds) -> crate::ops::WriteTx {
    begin_gated(
        pool,
        Caller::session(ids.org_id, ids.user_id),
        OrganizationLock::Shared,
        WriteIsolation::ReadCommitted,
    )
    .await
    .unwrap()
}

async fn set_org_rounding(pool: &PgPool, ids: &SeedIds, method: RoundingMethod) {
    let mut write = begin_write(pool, ids).await;
    let set =
        crate::ops::organization::set_invoice_rounding(&mut write.tx, &write.actor, method).await;
    write.commit(set, &Recorder::default()).await.unwrap();
}

async fn generate_invoice(pool: &PgPool, ids: &SeedIds) -> crate::models::InvoiceWithLines {
    let mut write = begin_write(pool, ids).await;
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
    write.commit(generated, &Recorder::default()).await.unwrap()
}

/// (discount, tax1, total) and the stored method of an invoice.
async fn stored(pool: &PgPool, id: Uuid) -> ((i64, i64, i64), RoundingMethod) {
    let row = sqlx::query!(
        r#"SELECT discount_cents, tax1_cents, total_cents, rounding AS "rounding: RoundingMethod"
           FROM invoices WHERE id = $1"#,
        id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    (
        (row.discount_cents, row.tax1_cents, row.total_cents),
        row.rounding,
    )
}

/// A JPY project billing JPY 124 (121 + 3) with a 10% discount and 8% tax:
/// a discount of JPY 12.40, then tax on JPY 112 (8.96) or JPY 111 (8.88).
async fn discounted_jpy_project(pool: &PgPool) -> SeedIds {
    let ids = project_in(pool, "JPY").await;
    sqlx::query!(
        "UPDATE project_settings SET discount_bps = 1000, tax1_bps = 800 WHERE project_id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_new_invoice_rounds_by_the_organization_method_in_whole_yen(pool: PgPool) {
    for (method, expected) in [
        (RoundingMethod::HalfUp, (1200, 900, 12100)),
        (RoundingMethod::Down, (1200, 800, 12000)),
        (RoundingMethod::Up, (1300, 900, 12000)),
    ] {
        let ids = discounted_jpy_project(&pool).await;
        set_org_rounding(&pool, &ids, method).await;

        let caller = Caller::session(ids.org_id, ids.user_id);
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
        let amounts = read.finish(preview).await.unwrap().amounts.unwrap();
        assert_eq!(
            (
                amounts.discount_cents,
                amounts.tax1_cents,
                amounts.total_cents
            ),
            expected,
            "{method:?} preview"
        );

        let invoice = generate_invoice(&pool, &ids).await;
        assert_eq!(
            stored(&pool, invoice.invoice.id).await,
            (expected, method),
            "{method:?} invoice"
        );
        let mut contributions: Vec<i64> = sqlx::query_scalar!(
            r#"SELECT net_before_tax_cents AS "net!" FROM invoice_line_items WHERE invoice_id = $1"#,
            invoice.invoice.id
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        contributions.sort_unstable();
        assert_eq!(
            contributions.iter().sum::<i64>(),
            12400 - expected.0,
            "{method:?} line discounts"
        );
        // The discount moves whole yen: JPY 3 and JPY 121 share JPY 12 or 13.
        assert!(
            contributions.iter().all(|net| net % 100 == 0),
            "{method:?} {contributions:?}"
        );
    }
}

/// The seeded project billed as one fixed fee of `fee_cents`, starting on
/// 2026-09-01, with its client and itself in `currency`.
async fn fee_project_in(pool: &PgPool, currency: &str, fee_cents: i64) -> SeedIds {
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
        "UPDATE projects SET currency = $2, project_type = 'fixed_fee', starts_on = '2026-09-01'
         WHERE id = $1",
        ids.project_id,
        currency
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode, fee_mode, fee_amount_cents)
         VALUES ($1, $2, $3, $4, 'person', 'single', $5)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
        fee_cents
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

fn constraint(error: sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|error| error.constraint().map(str::to_owned))
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_database_keeps_fees_on_the_currency_step(pool: PgPool) {
    const STEP: Option<&str> = Some("project_fee_amount_currency_step");
    // Hundredths are fine where the currency has them.
    for currency in ["EUR", "BHD"] {
        fee_project_in(&pool, currency, 10_050).await;
    }
    let ids = fee_project_in(&pool, "JPY", 10_000).await;
    let settings = sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents = 10050 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(constraint(settings).as_deref(), STEP);
    let milestone = sqlx::query!(
        "INSERT INTO project_fee_milestones (id, org_id, project_id, name, due_on, amount_cents, position)
         VALUES ($1, $2, $3, 'Delivery', '2026-09-01', 10050, 0)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(constraint(milestone).as_deref(), STEP);
    let occurrence = sqlx::query!(
        "INSERT INTO project_fee_occurrences (id, org_id, project_id, period_key, due_on, description, amount_cents, currency)
         VALUES ($1, $2, $3, 'single', '2026-09-01', 'Fixed fee', 10050, 'JPY')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(occurrence).as_deref(),
        Some("project_fee_occurrences_amount_currency_step")
    );

    // Switching a project to a zero-decimal currency rechecks its fees at the
    // commit, so a data migration may write the currency before the new fee.
    let ids = fee_project_in(&pool, "EUR", 10_050).await;
    let switched = sqlx::query!(
        "UPDATE projects SET currency = 'JPY' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(constraint(switched).as_deref(), STEP);
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE projects SET currency = 'JPY' WHERE id = $1",
        ids.project_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET fee_amount_cents = 10000 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_draft_editor_keeps_fee_lines_on_the_currency_step(pool: PgPool) {
    for (currency, edited, refused) in [
        ("JPY", 1_190_000, true),
        ("EUR", 1_190_050, false),
        ("BHD", 1_190_050, false),
    ] {
        let ids = fee_project_in(&pool, currency, 1_200_000).await;
        let mut write = begin_write(&pool, &ids).await;
        let generated = super::generate(
            &mut write.tx,
            &write.actor,
            super::Generation {
                client_id: ids.client_id,
                period: ("2026-09-01".parse().unwrap(), day()),
                project_ids: None,
                overrides: None,
                request: None,
            },
        )
        .await;
        let draft = write
            .commit(generated, &Recorder::default())
            .await
            .unwrap()
            .invoice
            .id;
        let caller = Caller::session(ids.org_id, ids.user_id);
        let mut read = begin_read(&pool, caller).await.unwrap();
        let editor = super::editing::load(&mut read.tx, &read.actor, draft).await;
        let mut edit = read.finish(editor).await.unwrap().edit;
        let line = edit.fees[0].line_id;
        let review = |edit: crate::models::invoice::InvoiceDraftEdit| {
            let pool = pool.clone();
            async move {
                let mut read = begin_read(&pool, caller).await.unwrap();
                let review = super::editing::review(&mut read.tx, &read.actor, draft, &edit).await;
                read.finish(review).await
            }
        };
        // Whole units are fine everywhere; hundredths only where the currency has them.
        edit.fees[0].amount_cents = edited;
        assert!(review(edit.clone()).await.is_ok(), "{currency} {edited}");
        edit.fees[0].amount_cents = 1_190_050;
        let result = review(edit.clone()).await;
        if refused {
            let error = result.unwrap_err();
            assert!(
                matches!(error.kind(), crate::ops::OpError::Invalid { .. }),
                "{error:?}"
            );
            assert_eq!(
                error.detail().and_then(|detail| detail.field.clone()),
                Some(serde_json::json!({ "fee_line": line }))
            );
            assert_eq!(
                error.to_string(),
                "Fee amount: JPY amounts have no decimal places"
            );
        } else {
            assert!(result.is_ok(), "{currency}");
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_invoice_keeps_its_method_when_the_setting_changes(pool: PgPool) {
    let ids = discounted_jpy_project(&pool).await;
    set_org_rounding(&pool, &ids, RoundingMethod::Down).await;
    let draft = generate_invoice(&pool, &ids).await.invoice.id;
    set_org_rounding(&pool, &ids, RoundingMethod::Up).await;

    // Editing the draft recomputes with the method it stores, not the setting.
    let caller = Caller::session(ids.org_id, ids.user_id);
    let mut read = begin_read(&pool, caller).await.unwrap();
    let editor = super::editing::load(&mut read.tx, &read.actor, draft).await;
    let editor = read.finish(editor).await.unwrap();
    assert_eq!(editor.review.amounts.total_cents, 12000);
    let mut edit = editor.edit;
    // Without the discount the tax is JPY 9.92: 9 down, where up gives 10.
    edit.defaults.discount_bps = 0;
    let mut read = begin_read(&pool, caller).await.unwrap();
    let review = super::editing::review(&mut read.tx, &read.actor, draft, &edit).await;
    let review = read.finish(review).await.unwrap();
    assert_eq!(
        (review.amounts.discount_cents, review.amounts.tax1_cents),
        (0, 900)
    );
    let mut write = begin_write(&pool, &ids).await;
    let request = crate::models::invoice::InvoiceDraftSave {
        request_id: Uuid::now_v7(),
        edit,
        review,
        confirmed_excess: Vec::new(),
    };
    let saved = super::editing::save(&mut write.tx, &write.actor, draft, &request).await;
    write.commit(saved, &Recorder::default()).await.unwrap();
    assert_eq!(
        stored(&pool, draft).await,
        ((0, 900, 13300), RoundingMethod::Down)
    );

    // An issued invoice is never recomputed: changing the setting again leaves it.
    let mut write = begin_write(&pool, &ids).await;
    let sent = super::set_status(
        &mut write.tx,
        &write.actor,
        draft,
        horae_core::types::InvoiceStatus::Sent,
    )
    .await;
    write.commit(sent, &Recorder::default()).await.unwrap();
    set_org_rounding(&pool, &ids, RoundingMethod::HalfUp).await;
    assert_eq!(
        stored(&pool, draft).await,
        ((0, 900, 13300), RoundingMethod::Down)
    );
    let mut read = begin_read(&pool, caller).await.unwrap();
    let detail = super::detail(&mut read.tx, &read.actor, draft).await;
    let detail = read.finish(detail).await.unwrap();
    let text = crate::render::invoice_text(&detail.invoice, &detail.lines, None, &branding());
    for amount in ["124.00", "9.00", "133.00"] {
        assert!(text.contains(amount), "missing {amount}: {text}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_database_refuses_amounts_its_method_would_not_give(pool: PgPool) {
    let ids = discounted_jpy_project(&pool).await;
    let invoice = generate_invoice(&pool, &ids).await.invoice.id;
    // The fractional tax two-decimal rounding gave before: JPY 8.96.
    let error = sqlx::query!(
        "UPDATE invoices SET tax1_cents = 896, total_cents = total_cents - 900 + 896 WHERE id = $1",
        invoice
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some("invoice_tax1_exact")
    );
    // Half up's JPY 9 under another method is refused too.
    let error = sqlx::query!(
        "UPDATE invoices SET rounding = 'down' WHERE id = $1",
        invoice
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some("invoice_tax1_exact")
    );
}

/// `invoice_portion_cents`, which the CHECKs use, and `invoice_amounts` give
/// the same discount and taxes for every method in every kind of currency.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn invoice_portion_sql_matches_rust(pool: PgPool) {
    use horae_core::invoice::{Rounding, invoice_amounts};
    use horae_core::project::Percentage;
    let subtotals = [
        0_i64,
        1,
        49,
        50,
        51,
        99,
        100,
        150,
        9_999,
        12_345,
        1_234_500,
        1_234_550,
        i64::MAX / 3,
    ];
    let rates = [0_u16, 1, 800, 1_000, 2_500, 5_000, 9_999, 10_000];
    for currency in ["JPY", "EUR", "BHD", " JPY", "jpy", "XYZ"] {
        for method in [
            RoundingMethod::HalfUp,
            RoundingMethod::Down,
            RoundingMethod::Up,
        ] {
            for subtotal in subtotals {
                for (discount, tax) in rates.iter().zip(rates.iter().rev()) {
                    let rust = invoice_amounts(
                        subtotal,
                        Percentage::try_from(*discount).unwrap(),
                        Percentage::try_from(*tax).unwrap(),
                        Percentage::ZERO,
                        Rounding::new(currency, method),
                    )
                    .unwrap();
                    let sql = sqlx::query!(
                        r#"SELECT d::bigint AS "discount!",
                                  invoice_portion_cents($1::bigint - d, $3, $4, $5)::bigint AS "tax!"
                           FROM (SELECT LEAST($1::bigint::numeric,
                                   invoice_portion_cents($1::bigint, $2, $4, $5)) AS d) discount"#,
                        subtotal,
                        i32::from(*discount),
                        i32::from(*tax),
                        currency,
                        method as RoundingMethod,
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                    assert_eq!(
                        (sql.discount, sql.tax),
                        (rust.discount_cents, rust.tax1_cents),
                        "{currency:?} {method:?} {subtotal} {discount} {tax}"
                    );
                    let negated: i64 = sqlx::query_scalar!(
                        r#"SELECT invoice_portion_cents(-$1::bigint, $2, $3, $4)::bigint AS "p!""#,
                        subtotal,
                        i32::from(*tax),
                        currency,
                        method as RoundingMethod,
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                    let positive: i64 = sqlx::query_scalar!(
                        r#"SELECT invoice_portion_cents($1::bigint, $2, $3, $4)::bigint AS "p!""#,
                        subtotal,
                        i32::from(*tax),
                        currency,
                        method as RoundingMethod,
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                    assert_eq!(
                        negated, -positive,
                        "{currency:?} {method:?} {subtotal} {tax}"
                    );
                }
            }
        }
    }
}
