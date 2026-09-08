use super::*;
use crate::server_fns::test_seed::{seed, time_entry};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn billing_cascade_agrees_across_all_four_levels_including_zero(pool: PgPool) {
    for (task, assignment, project, user, expected) in [
        (Some(5000), Some(4000), Some(3500), Some(3000), 5000),
        (None, Some(4000), Some(3500), Some(3000), 4000),
        (None, None, Some(3500), Some(3000), 3500),
        (None, None, None, Some(3000), 3000),
        (None, None, None, None, 0),
        (Some(0), Some(4000), Some(3500), Some(3000), 0),
        (None, Some(0), Some(3500), Some(3000), 0),
        (None, None, Some(0), Some(3000), 0),
    ] {
        let ids = seed(&pool, OrgRole::Manager).await;
        time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = $2 WHERE id = $1",
            ids.user_id,
            user
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE projects SET rate_cents = $2 WHERE id = $1",
            ids.project_id,
            project
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, $3)", ids.project_id, ids.task_id, task)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', $4)", uuid::Uuid::now_v7(), ids.project_id, ids.user_id, assignment)
            .execute(&pool).await.unwrap();
        let day = "2026-09-07".parse().unwrap();
        let report = crate::server_fns::reports::fetch_report(
            &pool,
            ids.org_id,
            (day, day),
            "project",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
            .await
            .unwrap();
        let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
            .await
            .unwrap();
        assert_eq!(
            (
                report[0].billable_cents,
                spend[0].spent_cents,
                invoice.invoice.total_cents,
                invoice.lines[0].rate_cents
            ),
            (expected, expected, expected, expected),
            "rates: {task:?}, {assignment:?}, {project:?}, {user:?}",
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn project_rate_is_used_by_invoices_reports_and_spend(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 3000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents = 6000 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(invoice.lines[0].rate_cents, 6000);
    assert_eq!(invoice.invoice.total_cents, 6000);
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(report[0].billable_cents, 6000);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
        .await
        .unwrap();
    assert_eq!(spend[0].spent_cents, 6000);
}

#[sqlx::test(migrations = "./migrations")]
async fn invoiced_amounts_survive_rate_changes_and_void_uses_current_rates(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 3000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents = 6000 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(report[0].billable_cents, invoice.invoice.total_cents);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
        .await
        .unwrap();
    assert_eq!(spend[0].spent_cents, invoice.invoice.total_cents);
    transition_invoice(&pool, ids.org_id, invoice.invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();
    let replacement = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(replacement.invoice.total_cents, 6000);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
        .await
        .unwrap();
    assert_eq!(
        spend[0].spent_cents, 6000,
        "the old void invoice must not duplicate spend"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn non_billable_context_produces_no_unbilled_amount_on_any_report(pool: PgPool) {
    for project_billable in [false, true] {
        let ids = seed(&pool, OrgRole::Manager).await;
        time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE users SET billable_rate_cents = 6000 WHERE id = $1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("UPDATE projects SET project_type = CASE WHEN $2 THEN 'time_and_materials'::project_type ELSE 'non_billable'::project_type END WHERE id = $1", ids.project_id, project_billable).execute(&pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, $3)",
            ids.project_id,
            ids.task_id,
            !project_billable
        )
        .execute(&pool)
        .await
        .unwrap();
        let day = "2026-09-07".parse().unwrap();
        let report = crate::server_fns::reports::fetch_report(
            &pool,
            ids.org_id,
            (day, day),
            "project",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            (
                report[0].total_minutes,
                report[0].billable_minutes,
                report[0].billable_cents
            ),
            (60, 0, 0)
        );
        let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
            .await
            .unwrap();
        assert_eq!((spend[0].spent_minutes, spend[0].spent_cents), (60, 0));
        let detail = crate::reports::fetch_entries(&pool, ids.org_id, day, day, None, None, None)
            .await
            .unwrap();
        assert!(!detail[0].billable);
        assert!(
            generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
                .await
                .is_err()
        );
    }
    assert_no_partial_invoice(&pool).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn invoiced_billability_survives_later_project_changes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 6000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE projects SET project_type = 'non_billable', active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(report[0].billable_cents, invoice.invoice.total_cents);
    assert_eq!(report[0].billable_minutes, 60);
}

#[sqlx::test(migrations = "./migrations")]
async fn invoice_total_overflow_leaves_all_time_unbilled(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    for _ in 0..3 {
        time_entry(&pool, &ids, EntryState::Open).await;
    }
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = $2 WHERE id = $1",
        ids.user_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day).await;

    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert_no_partial_invoice(&pool).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn invoice_line_overflow_leaves_time_unbilled(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = $2 WHERE id = $1",
        ids.user_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE time_entries SET minutes = 61 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day).await;

    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert_no_partial_invoice(&pool).await;
}

async fn assert_no_partial_invoice(pool: &PgPool) {
    let row = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM invoices) as "invoices!",
                  (SELECT count(*) FROM invoice_line_items) as "lines!",
                  EXISTS(SELECT 1 FROM time_entries WHERE state <> 'open'
                    OR invoice_id IS NOT NULL OR rounded_minutes IS NOT NULL) as "changed!""#,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!((row.invoices, row.lines, row.changed), (0, 0, false));
}

#[sqlx::test(migrations = "./migrations")]
async fn large_invoice_and_reports_agree_without_intermediate_overflow(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = $2 WHERE id = $1",
        ids.user_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
        .await
        .unwrap();
    assert_eq!(
        (
            invoice.lines[0].amount_cents,
            invoice.invoice.total_cents,
            report[0].billable_cents,
            spend[0].spent_cents
        ),
        (i64::MAX, i64::MAX, i64::MAX, i64::MAX)
    );

    time_entry(&pool, &ids, EntryState::Open).await;
    let report_error = crate::server_fns::reports::fetch_report(
        &pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap_err();
    let spend_error = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id)
        .await
        .unwrap_err();
    for error in [report_error, spend_error] {
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some("22003")
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn invoicing_uses_and_freezes_effective_minutes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let open = time_entry(&pool, &ids, EntryState::Open).await;
    let approved = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15, round_dir = 'nearest' WHERE id = $1",
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 6000, cost_rate_cents = 6000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET minutes = 8 WHERE org_id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET rounded_minutes = 10 WHERE id = $1",
        approved
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    assert_reporting_minutes(&pool, &ids, 25, 2500).await;

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();

    let amounts: Vec<_> = result
        .lines
        .iter()
        .map(|line| (line.time_entry_id, line.minutes, line.amount_cents))
        .collect();
    assert_eq!(amounts, vec![(open, 15, 1500), (approved, 10, 1000)]);
    assert_eq!(result.invoice.total_cents, 2500);
    let frozen = sqlx::query!(
        "SELECT id, minutes, rounded_minutes FROM time_entries WHERE org_id = $1 ORDER BY id",
        ids.org_id,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        frozen
            .iter()
            .map(|row| (row.minutes, row.rounded_minutes))
            .collect::<Vec<_>>(),
        vec![(8, Some(15)), (8, Some(10))]
    );

    sqlx::query!(
        "UPDATE organizations SET round_minutes = 60, round_dir = 'up' WHERE id = $1",
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_reporting_minutes(&pool, &ids, 25, 2500).await;
}

async fn assert_reporting_minutes(
    pool: &PgPool,
    ids: &crate::server_fns::test_seed::SeedIds,
    minutes: i64,
    cents: i64,
) {
    let day = "2026-09-07".parse().unwrap();
    let report = crate::server_fns::reports::fetch_report(
        pool,
        ids.org_id,
        (day, day),
        "project",
        None,
        None,
        None,
    )
    .await
    .unwrap();
    assert_eq!(report.len(), 1);
    assert_eq!(
        (
            report[0].rounded_minutes,
            report[0].billable_minutes,
            report[0].billable_cents
        ),
        (minutes, minutes, cents)
    );
    assert_eq!(
        report[0].total_minutes, 16,
        "worked minutes are not overwritten"
    );
    assert_eq!(report[0].cost_cents, 1600, "labor cost uses worked minutes");
    let detail = crate::reports::fetch_entries(pool, ids.org_id, day, day, None, None, None)
        .await
        .unwrap();
    assert_eq!(
        detail
            .iter()
            .map(|row| i64::from(row.rounded_minutes.unwrap()))
            .sum::<i64>(),
        minutes
    );
    let csv = crate::reports::entries_csv(&detail).unwrap();
    let records = csv::Reader::from_reader(csv.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        records.iter().map(|row| &row[5]).collect::<Vec<_>>(),
        vec!["0.25", "0.17"]
    );
    let spend = crate::server_fns::projects::fetch_project_spend(pool, ids.org_id)
        .await
        .unwrap();
    assert_eq!(spend.len(), 1);
    assert_eq!((spend[0].spent_minutes, spend[0].spent_cents), (16, cents));
}

#[sqlx::test(migrations = "./migrations")]
async fn effective_minutes_sql_matches_domain_rounding(pool: PgPool) {
    let rows = sqlx::query!(
        r#"SELECT m as "minutes!", inc as "increment!", dir as "dir!: horae_core::types::RoundDir",
                  frozen as "frozen?", effective_minutes(m, frozen, inc, dir) as "effective!"
           FROM generate_series(0, 1440) m
           CROSS JOIN unnest(ARRAY[0, 1, 2, 5, 15, 30, 32767]::smallint[]) inc
           CROSS JOIN unnest(enum_range(NULL::round_dir)) dir
           CROSS JOIN unnest(ARRAY[NULL, 0, 10]::integer[]) frozen"#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for row in rows {
        assert_eq!(
            row.effective as u32,
            horae_core::rounding::effective_minutes(
                row.minutes as u32,
                row.frozen.map(|value| value as u32),
                row.increment as u32,
                row.dir
            ),
            "minutes={}, frozen={:?}, increment={}, direction={:?}",
            row.minutes,
            row.frozen,
            row.increment,
            row.dir
        );
    }
}

#[sqlx::test(migrations = false)]
async fn effective_minutes_migration_preserves_historical_invoice_quantities(pool: PgPool) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|migration| migration.version < 17)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Manager).await;
    let legacy = time_entry(&pool, &ids, EntryState::Open).await;
    let frozen = time_entry(&pool, &ids, EntryState::Open).await;
    let open = time_entry(&pool, &ids, EntryState::Open).await;
    let invoice_id = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, 'HISTORICAL', 'sent', '2026-09-07', '2026-10-07', 'EUR', 1600)",
        invoice_id, ids.org_id, ids.client_id,
    ).execute(&pool).await.unwrap();
    for entry in [legacy, frozen] {
        sqlx::query!(
            "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
             VALUES ($1, $2, $3, 'Historical time', 8, 6000, 800)",
            uuid::Uuid::now_v7(), invoice_id, entry,
        ).execute(&pool).await.unwrap();
        sqlx::query!(
            "UPDATE time_entries SET invoice_id = $2, state = 'invoiced' WHERE id = $1",
            entry,
            invoice_id,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query!(
        "UPDATE time_entries SET rounded_minutes = 10 WHERE id = $1",
        frozen
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let rows = sqlx::query!(
        "SELECT id, minutes, rounded_minutes FROM time_entries WHERE org_id = $1 ORDER BY id",
        ids.org_id,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.id, row.minutes, row.rounded_minutes))
            .collect::<Vec<_>>(),
        vec![
            (legacy, 60, Some(8)),
            (frozen, 60, Some(10)),
            (open, 60, None)
        ]
    );
    let lines = sqlx::query!(
        "SELECT minutes, amount_cents FROM invoice_line_items WHERE invoice_id = $1",
        invoice_id,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        lines
            .iter()
            .all(|line| line.minutes == 8 && line.amount_cents == 800)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invoicing_leaves_running_time_open(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let running = time_entry(&pool, &ids, EntryState::Open).await;
    let stopped = time_entry(&pool, &ids, EntryState::Open).await;
    let approved = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now(), minutes = 0 WHERE id = $1",
        running,
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();

    let billed: Vec<_> = result.lines.iter().map(|line| line.time_entry_id).collect();
    assert_eq!(billed, vec![stopped, approved]);
    let timer = sqlx::query!(
        r#"SELECT state as "state: EntryState", invoice_id, is_running FROM time_entries WHERE id = $1"#,
        running,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (timer.state, timer.invoice_id, timer.is_running),
        (EntryState::Open, None, true)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn only_running_time_does_not_create_an_invoice(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let running = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now() WHERE id = $1",
        running,
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();

    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day).await;

    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM invoices")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn void_waits_for_a_concurrent_payment_and_then_refuses(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Approved).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Sent)
        .await
        .unwrap();

    // Hold an uncommitted payment so the competing request must wait. A plain
    // SELECT still sees 'sent', which is precisely the stale-read window.
    let mut payment = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE invoices SET status = 'paid' WHERE id = $1",
        invoice.id
    )
    .execute(&mut *payment)
    .await
    .unwrap();
    let pid = sqlx::query_scalar!(r#"SELECT pg_backend_pid() as "pid!""#)
        .fetch_one(&mut *payment)
        .await
        .unwrap();

    let mut tasks = tokio::task::JoinSet::new();
    let competing_pool = pool.clone();
    tasks.spawn(async move {
        transition_invoice(&competing_pool, ids.org_id, invoice.id, InvoiceStatus::Void).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let waiting = sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                   WHERE $1 = ANY(pg_blocking_pids(pid))) as "waiting!""#,
                pid,
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
        }
    })
    .await
    .expect("the competing transition must reach the locked invoice");

    payment.commit().await.unwrap();
    let result = tasks.join_next().await.unwrap().unwrap();
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let row = sqlx::query!(
        r#"SELECT i.status as "status: InvoiceStatus", te.invoice_id,
                  te.state as "state: EntryState"
           FROM invoices i JOIN time_entries te ON te.id = $2 WHERE i.id = $1"#,
        invoice.id,
        entry,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (row.status, row.invoice_id, row.state),
        (InvoiceStatus::Paid, Some(invoice.id), EntryState::Invoiced)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn void_reopens_time_without_stale_rounding(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!(
        "UPDATE time_entries SET rounded_minutes = 75 WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;

    let result = transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Void)
        .await
        .unwrap();

    assert_eq!(result.status, InvoiceStatus::Void);
    let row = sqlx::query!(
        r#"SELECT state as "state: EntryState", invoice_id, rounded_minutes, minutes
           FROM time_entries WHERE id = $1"#,
        entry,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (row.state, row.invoice_id, row.rounded_minutes, row.minutes),
        (EntryState::Open, None, None, 60)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn invoice_transitions_reject_invalid_states_and_foreign_organizations(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;

    assert!(matches!(
        transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Paid).await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert!(matches!(
        transition_invoice(&pool, uuid::Uuid::now_v7(), invoice.id, InvoiceStatus::Sent).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Sent)
        .await
        .unwrap();
    transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Paid)
        .await
        .unwrap();
    assert!(matches!(
        transition_invoice(&pool, ids.org_id, invoice.id, InvoiceStatus::Void).await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
}
