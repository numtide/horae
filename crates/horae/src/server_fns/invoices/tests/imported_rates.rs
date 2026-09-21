use super::*;
use crate::importers::harvest::csv_source::import_body;
use horae_core::importers::harvest::types::ImportMode;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn imported_legacy_project_keeps_exact_rates_and_totals_on_retry(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 3000, cost_rate_cents = 1200 WHERE id = $1",
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let csv = format!(
        "Date,Client,Project,Task,Email,Hours,Billable?,Billable Rate,Currency\n\
         2026-09-07,Acme,Imported,Design,{}@test.com,1.25,Yes,80,EUR\n\
         2026-09-07,Acme,Imported,Review,{}@test.com,0.5,Yes,0,EUR\n\
         2026-09-07,Acme,Imported,Dev,{}@test.com,1,Yes,,EUR\n",
        ids.user_id, ids.user_id, ids.user_id,
    );
    for (created, skipped) in [(3, 0), (0, 3)] {
        let result = import_body(
            &pool,
            ids.org_id,
            "EUR",
            axum::body::Body::from(csv.clone()),
            ImportMode::Commit,
        )
        .await
        .unwrap();
        assert_eq!(result.error_count(), 0, "{result:?}");
        assert_eq!(
            (
                result.summary.time_entries.created,
                result.summary.time_entries.skipped
            ),
            (created, skipped),
        );
    }
    let project_id = sqlx::query_scalar!(
        "SELECT id FROM projects WHERE org_id = $1 AND name = 'Imported'",
        ids.org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let settings = sqlx::query_scalar!(
        "SELECT count(*) FROM project_settings WHERE project_id = $1",
        project_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(settings, Some(0));
    let day = "2026-09-07".parse().unwrap();
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
    )
    .await
    .unwrap();
    assert_eq!(
        (report[0].billable_cents, report[0].cost_cents),
        (13000, Some(3300))
    );
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    let imported = spend
        .iter()
        .find(|row| row.project_id == project_id)
        .unwrap();
    assert_eq!((imported.spent_minutes, imported.spent_cents), (165, 13000));
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(
        (
            invoice.invoice.currency.as_str(),
            invoice.invoice.total_cents
        ),
        ("EUR", 13000)
    );
    let mut rates: Vec<_> = invoice.lines.iter().map(|line| line.rate_cents).collect();
    rates.sort();
    assert_eq!(rates, vec![Some(0), Some(3000), Some(8000)]);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn imported_task_rates_require_matching_currency_only_for_new_configured_links(pool: PgPool) {
    for (configured, existing_rate, source_rate, currency, expected_errors, expected_rate) in [
        (true, None, "80", "EUR", 1, None),
        (true, None, "0", "EUR", 1, None),
        (true, None, "80", "", 1, None),
        (true, None, "80", " usd ", 0, Some(8000)),
        (true, None, "", "EUR", 0, None),
        (true, Some(0_i64), "80", "EUR", 0, Some(0)),
        (false, None, "80", "EUR", 0, Some(8000)),
    ] {
        let ids = seed(&pool, OrgRole::Manager).await;
        sqlx::query!(
            "UPDATE projects SET currency = 'USD' WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        if configured {
            sqlx::query!(
                "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'task')",
                uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
            ).execute(&pool).await.unwrap();
        }
        if let Some(rate) = existing_rate {
            sqlx::query!(
                "INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,true,$3)",
                ids.project_id, ids.task_id, rate,
            ).execute(&pool).await.unwrap();
        }
        let csv = format!(
            "Date,Client,Project,Task,Email,Hours,Billable?,Billable Rate,Currency\n\
             2026-09-07,Acme,Widget,Dev,{}@test.com,1,Yes,{source_rate},{currency}\n",
            ids.user_id,
        );
        let result = import_body(
            &pool,
            ids.org_id,
            "EUR",
            axum::body::Body::from(csv),
            ImportMode::Commit,
        )
        .await
        .unwrap();
        assert_eq!(
            result.error_count(),
            expected_errors,
            "configured={configured}, existing={existing_rate:?}, source={source_rate:?} {currency:?}: {result:?}"
        );
        let link = sqlx::query!(
            "SELECT rate_cents FROM project_tasks WHERE project_id = $1 AND task_id = $2",
            ids.project_id,
            ids.task_id,
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(
            link.as_ref().and_then(|link| link.rate_cents),
            expected_rate
        );
        let entries = sqlx::query_scalar!(
            "SELECT count(*) FROM time_entries WHERE project_id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if expected_errors == 1 {
            assert!(
                link.is_none(),
                "a rejected rate must not leave a project-task link"
            );
            assert_eq!(entries, Some(0));
            assert!(result.row_errors[0].reason.contains("currency"));
        } else {
            assert!(link.is_some());
            assert_eq!(entries, Some(1));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn rejected_import_rate_does_not_poison_the_next_row_or_its_catalog_rate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE projects SET currency = 'USD' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'task')",
        uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
    ).execute(&pool).await.unwrap();
    let csv = format!(
        "Date,Client,Project,Task,Email,Hours,Billable?,Billable Rate,Currency\n\
         2026-09-07,Acme,Widget,Fresh,{}@test.com,1,Yes,99,EUR\n\
         2026-09-07,Acme,Widget,Fresh,{}@test.com,1,Yes,80,USD\n",
        ids.user_id, ids.user_id,
    );
    let result = import_body(
        &pool,
        ids.org_id,
        "EUR",
        axum::body::Body::from(csv),
        ImportMode::Commit,
    )
    .await
    .unwrap();
    assert_eq!(
        (result.error_count(), result.summary.time_entries.created),
        (1, 1)
    );
    let rates = sqlx::query!(
        "SELECT t.default_rate_cents, pt.rate_cents FROM tasks t
         JOIN project_tasks pt ON pt.task_id = t.id
         WHERE pt.project_id = $1 AND t.name = 'Fresh'",
        ids.project_id,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rates.len(), 1);
    assert_eq!(
        (rates[0].default_rate_cents, rates[0].rate_cents),
        (Some(8000), Some(8000))
    );
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(
        (
            invoice.invoice.currency.as_str(),
            invoice.invoice.total_cents
        ),
        ("USD", 8000)
    );
    assert_eq!(invoice.lines.len(), 1);
}
