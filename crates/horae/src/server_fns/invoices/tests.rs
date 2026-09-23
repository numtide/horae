use super::*;
use crate::server_fns::test_seed::{seed, time_entry};
use sqlx::PgPool;

mod imported_rates;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_preview_keeps_time_rates_and_defaults_in_one_snapshot(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10001 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut edit = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *edit)
        .await
        .unwrap();
    sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *edit)
        .await
        .unwrap();
    let task_pool = pool.clone();
    let day = "2026-09-07".parse().unwrap();
    let task = tokio::spawn(async move {
        preview::prepare(
            &task_pool,
            ids.org_id,
            ids.client_id,
            (day, day),
            None,
            None,
        )
        .await
    });
    crate::server_fns::test_seed::wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 20000 WHERE id = $1",
        ids.project_id
    )
    .execute(&mut *edit)
    .await
    .unwrap();
    sqlx::query!("UPDATE time_entries SET minutes = 120 WHERE id = $1", entry)
        .execute(&mut *edit)
        .await
        .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,terms_days) VALUES ($1,$2,$3,$4,'person',14)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&mut *edit).await.unwrap();
    edit.commit().await.unwrap();
    let preview = task.await.unwrap().unwrap();
    assert_eq!(preview.subtotal_cents, 10001);
    assert_eq!(preview.lines[0].minutes, Some(60));
    assert_eq!(preview.defaults, Some(InvoiceDefaults::default()));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_preview_rejects_large_results_instead_of_truncating_them(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entries: Vec<_> = (0..10000).map(|_| uuid::Uuid::now_v7()).collect();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state)
        SELECT id,$2,$3,$4,$5,'2026-09-07',60,true,'open' FROM unnest($1::uuid[]) id",
        &entries, ids.org_id, ids.user_id, ids.project_id, ids.task_id).execute(&pool).await.unwrap();
    let day = "2026-09-07".parse().unwrap();
    assert_eq!(
        preview::prepare(&pool, ids.org_id, ids.client_id, (day, day), None, None)
            .await
            .unwrap()
            .lines
            .len(),
        10000
    );
    time_entry(&pool, &ids, EntryState::Open).await;
    let error = preview::prepare(&pool, ids.org_id, ids.client_id, (day, day), None, None)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_preview_requires_explicit_resolution_and_scopes_selected_projects(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let other = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other','EUR')",
        other,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,terms_days) VALUES ($1,$2,$3,$4,'person',14)", uuid::Uuid::now_v7(), ids.org_id, other, ids.user_id).execute(&pool).await.unwrap();
    let foreign = seed(&pool, OrgRole::Manager).await;
    let day = "2026-09-07".parse().unwrap();
    let selected = [ids.project_id, other, ids.project_id];
    let preview = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        Some(&selected),
        None,
    )
    .await
    .unwrap();
    assert_eq!(preview.projects.len(), 2);
    assert_eq!(preview.lines.len(), 1);
    assert_eq!(preview.lines[0].net_before_tax_cents, None);
    assert_eq!(
        (preview.defaults, preview.amounts, preview.due_on),
        (None, None, None)
    );
    let overrides = InvoiceDefaults {
        terms_days: 0,
        discount_bps: 10000,
        ..Default::default()
    };
    let resolved = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        Some(&selected),
        Some(&overrides),
    )
    .await
    .unwrap();
    assert_eq!(resolved.due_on, Some(resolved.issued_on));
    assert_eq!(resolved.amounts.unwrap().total_cents, 0);
    assert_eq!(resolved.lines[0].net_before_tax_cents, Some(0));
    let inherited = preview::prepare(&pool, ids.org_id, ids.client_id, (day, day), None, None)
        .await
        .unwrap();
    assert_eq!(inherited.defaults, Some(InvoiceDefaults::default()));
    assert_eq!(inherited.projects.len(), 1);
    for selected in [
        vec![],
        vec![ids.project_id; 1001],
        vec![foreign.project_id],
        vec![ids.project_id, foreign.project_id],
    ] {
        let error = preview::prepare(
            &pool,
            ids.org_id,
            ids.client_id,
            (day, day),
            Some(&selected),
            None,
        )
        .await
        .unwrap_err();
        let expected = if selected.is_empty() || selected.len() > 1000 {
            BAD_REQUEST
        } else {
            NOT_FOUND
        };
        assert!(matches!(error, ServerFnError::ServerError { code, .. } if code == expected));
    }
    assert!(
        preview::prepare(&pool, foreign.org_id, ids.client_id, (day, day), None, None)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_preview_matches_generation_without_claiming_time(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10001 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let settings = InvoiceDefaults {
        terms_days: 21,
        po_number: "Preview PO".into(),
        discount_bps: 1250,
        tax1_bps: 2100,
        tax2_name: Some("Local tax".into()),
        tax2_bps: Some(150),
    };
    let preview = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        Some(&[ids.project_id]),
        Some(&settings),
    )
    .await
    .unwrap();
    assert_eq!(preview.defaults, Some(settings.clone()));
    assert_eq!(preview.projects.len(), 1);
    assert_eq!(preview.projects[0].project_id, ids.project_id);
    assert_eq!(preview.lines.len(), 1);
    assert_eq!(
        sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", entry)
            .fetch_one(&pool)
            .await
            .unwrap(),
        None
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
    let generated = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        Some(&[ids.project_id]),
        Some(&settings),
    )
    .await
    .unwrap();
    let amounts = preview.amounts.unwrap();
    assert_eq!(
        (
            amounts.subtotal_cents,
            amounts.discount_cents,
            amounts.tax1_cents,
            amounts.tax2_cents,
            amounts.total_cents
        ),
        (10001, 1250, 1838, 131, 10720)
    );
    assert_eq!(amounts.total_cents, generated.invoice.total_cents);
    assert_eq!(preview.due_on, Some(generated.invoice.due_on));
    assert_eq!(preview.lines[0].description, generated.lines[0].description);
    assert_eq!(
        preview.lines[0].amount_cents,
        generated.lines[0].amount_cents
    );
    assert!(
        preview::prepare(&pool, ids.org_id, ids.client_id, (day, day), None, None)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_apply_exact_adjustments_and_payment_terms(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10001 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,terms_days,po_number,discount_bps,tax1_bps,tax2_name,tax2_bps)
        VALUES ($1,$2,$3,$4,'project',21,'PO-123',1250,2100,'Local tax',150)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
        .execute(&pool).await.unwrap();
    let day = "2026-09-07".parse().unwrap();
    let result = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(
        (result.invoice.due_on - result.invoice.issued_on).num_days(),
        21
    );
    assert_eq!(result.invoice.total_cents, 10720);
    assert_eq!(
        result
            .lines
            .iter()
            .map(|line| line.amount_cents)
            .sum::<i64>(),
        10001
    );
    assert_eq!(
        (
            result.invoice.subtotal_cents,
            result.invoice.discount_cents,
            result.invoice.tax1_cents,
            result.invoice.tax2_cents
        ),
        (10001, 1250, 1838, 131)
    );
    assert_eq!(result.invoice.po_number, "PO-123");
    assert_eq!(result.invoice.tax2_name.as_deref(), Some("Local tax"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_conflict_rolls_back_all_claims(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let first_entry = time_entry(&pool, &ids, EntryState::Open).await;
    let other = crate::server_fns::test_seed::SeedIds {
        project_id: uuid::Uuid::now_v7(),
        ..ids
    };
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents) VALUES ($1,$2,$3,'Other','EUR',10000)", other.project_id, ids.org_id, ids.client_id)
        .execute(&pool).await.unwrap();
    time_entry(&pool, &other, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,terms_days)
        VALUES ($1,$2,$3,$4,'project',14)",
        uuid::Uuid::now_v7(),
        ids.org_id,
        other.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let day = "2026-09-07".parse().unwrap();
    let error = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError { code: CONFLICT, .. }
    ));
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
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT invoice_id FROM time_entries WHERE id = $1",
            first_entry
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );
    let explicit = InvoiceDefaults {
        terms_days: 0,
        po_number: "Resolved".into(),
        ..Default::default()
    };
    let result = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        None,
        Some(&explicit),
    )
    .await
    .unwrap();
    assert_eq!(result.lines.len(), 2);
    assert_eq!(result.invoice.due_on, result.invoice.issued_on);
    assert_eq!(result.invoice.po_number, "Resolved");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_are_invoice_owned_and_only_drafts_are_editable(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents = 10000 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,terms_days,tax1_bps)
        VALUES ($1,$2,$3,$4,'project',14,2100)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    let day = "2026-09-07".parse().unwrap();
    let created = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET terms_days = 90,tax1_bps = 0 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let stored = crate::reports::fetch_invoice_metadata(&pool, created.invoice.id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (stored.terms_days, stored.tax1_bps, stored.total_cents),
        (14, 2100, 12100)
    );
    let overrides = InvoiceDefaults {
        terms_days: 21,
        po_number: "Invoice only".into(),
        discount_bps: 1000,
        tax1_bps: 1000,
        ..Default::default()
    };
    let edited = update_invoice_defaults_in_db(&pool, ids.org_id, stored.id, &overrides)
        .await
        .unwrap();
    assert_eq!(
        (
            edited.terms_days,
            edited.subtotal_cents,
            edited.discount_cents,
            edited.tax1_cents,
            edited.total_cents
        ),
        (21, 10000, 1000, 900, 9900)
    );
    let settings = sqlx::query!(
        "SELECT terms_days,tax1_bps,po_number FROM project_settings WHERE project_id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            settings.terms_days,
            settings.tax1_bps,
            settings.po_number.as_str()
        ),
        (90, 0, "")
    );
    let foreign = seed(&pool, OrgRole::Manager).await;
    assert!(matches!(
        update_invoice_defaults_in_db(&pool, foreign.org_id, stored.id, &overrides).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    transition_invoice(
        &pool,
        ids.org_id,
        stored.id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
    .await
    .unwrap();
    assert!(matches!(
        update_invoice_defaults_in_db(&pool, ids.org_id, stored.id, &InvoiceDefaults::default())
            .await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    transition_invoice(
        &pool,
        ids.org_id,
        stored.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    let voided = crate::reports::fetch_invoice_metadata(&pool, stored.id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (voided.total_cents, voided.po_number.as_str()),
        (9900, "Invoice only")
    );
    assert!(
        update_invoice_defaults_in_db(&pool, ids.org_id, stored.id, &overrides)
            .await
            .is_err()
    );
    let regenerated = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(
        (
            regenerated.invoice.terms_days,
            regenerated.invoice.total_cents
        ),
        (90, 10000)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_selection_rejects_unavailable_projects_before_claiming(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let unselected = crate::server_fns::test_seed::SeedIds {
        project_id: uuid::Uuid::now_v7(),
        ..ids
    };
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents) VALUES ($1,$2,$3,'Unselected','EUR',10000)", unselected.project_id, ids.org_id, ids.client_id)
        .execute(&pool).await.unwrap();
    let unselected_entry = time_entry(&pool, &unselected, EntryState::Open).await;
    let foreign = seed(&pool, OrgRole::Manager).await;
    let day = "2026-09-07".parse().unwrap();
    for selected in [
        vec![],
        vec![foreign.project_id],
        vec![ids.project_id, foreign.project_id],
    ] {
        assert!(
            generate_invoice_with_options(
                &pool,
                ids.org_id,
                ids.client_id,
                (day, day),
                Some(&selected),
                None
            )
            .await
            .is_err()
        );
    }
    let result = generate_invoice_with_options(
        &pool,
        ids.org_id,
        ids.client_id,
        (day, day),
        Some(&[ids.project_id, ids.project_id]),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.lines[0].time_entry_id, Some(entry));
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT invoice_id FROM time_entries WHERE id = $1",
            unselected_entry
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_defaults_edit_rechecks_status_after_waiting_for_a_concurrent_send(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;
    let mut sending = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *sending)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "SELECT id FROM invoices WHERE id = $1 FOR UPDATE",
        invoice.id
    )
    .fetch_one(&mut *sending)
    .await
    .unwrap();
    let edit_pool = pool.clone();
    let edit = tokio::spawn(async move {
        update_invoice_defaults_in_db(
            &edit_pool,
            ids.org_id,
            invoice.id,
            &InvoiceDefaults {
                po_number: "Must not save".into(),
                ..Default::default()
            },
        )
        .await
    });
    crate::server_fns::test_seed::wait_for_blocked(&pool, blocker).await;
    sqlx::query!(
        "UPDATE invoices SET status = 'sent' WHERE id = $1",
        invoice.id
    )
    .execute(&mut *sending)
    .await
    .unwrap();
    sending.commit().await.unwrap();
    assert!(matches!(
        edit.await.unwrap(),
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    let stored = crate::reports::fetch_invoice_metadata(&pool, invoice.id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.po_number, "");
}

#[sqlx::test(migrations = false)]
#[serial_test::serial]
async fn invoice_defaults_legacy_insert_preserves_dates_and_exact_total(pool: PgPool) {
    let mut previous = sqlx::migrate!("./migrations");
    previous.migrations = std::borrow::Cow::Owned(
        previous
            .iter()
            .filter(|migration| migration.version < 37)
            .cloned()
            .collect(),
    );
    previous.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Manager).await;
    let id = uuid::Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'Legacy','2026-09-01','2026-09-08','EUR',9223372036854775807)",
        id,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let legacy = crate::reports::fetch_invoice_metadata(&pool, id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (legacy.terms_days, legacy.total_cents, legacy.subtotal_cents),
        (7, i64::MAX, i64::MAX)
    );
    let overflow = InvoiceDefaults {
        tax1_bps: 10000,
        ..Default::default()
    };
    assert!(
        update_invoice_defaults_in_db(&pool, ids.org_id, id, &overflow)
            .await
            .is_err()
    );
    let unchanged = crate::reports::fetch_invoice_metadata(&pool, id, ids.org_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(legacy, unchanged);
}

#[test]
fn invoice_defaults_validate_all_override_fields() {
    for invalid in [
        InvoiceDefaults {
            terms_days: -1,
            ..Default::default()
        },
        InvoiceDefaults {
            terms_days: 366,
            ..Default::default()
        },
        InvoiceDefaults {
            discount_bps: -1,
            ..Default::default()
        },
        InvoiceDefaults {
            tax1_bps: 10001,
            ..Default::default()
        },
        InvoiceDefaults {
            tax2_name: Some("Local".into()),
            tax2_bps: Some(-1),
            ..Default::default()
        },
        InvoiceDefaults {
            tax2_bps: Some(0),
            ..Default::default()
        },
        InvoiceDefaults {
            tax2_name: Some(" ".into()),
            tax2_bps: Some(0),
            ..Default::default()
        },
        InvoiceDefaults {
            po_number: "x".repeat(201),
            ..Default::default()
        },
        InvoiceDefaults {
            po_number: "bad\0value".into(),
            ..Default::default()
        },
    ] {
        assert!(
            matches!(
                defaults::amounts(&invalid, 10000),
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST,
                    ..
                })
            ),
            "{invalid:?}"
        );
    }
    let free = InvoiceDefaults {
        discount_bps: 10000,
        tax1_bps: 10000,
        ..Default::default()
    };
    assert_eq!(defaults::amounts(&free, 1).unwrap().total_cents, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn configured_project_currency_and_cost_override_keep_their_denominations(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        uuid::Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 3000, cost_rate_cents = 1000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET currency = 'USD', rate_cents = 6000 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'project')", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
        .execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,1500)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
        .execute(&pool).await.unwrap();
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
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
        (
            invoice.invoice.currency.as_str(),
            invoice.invoice.total_cents
        ),
        ("USD", 6000)
    );
    assert_eq!(
        (
            report[0].currency.as_str(),
            report[0].billable_cents,
            report[0].cost_currency.as_str(),
            report[0].cost_cents
        ),
        ("USD", 6000, "EUR", Some(1500))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn configured_fixed_fee_hours_are_not_invoiced_but_legacy_fees_are_unchanged(pool: PgPool) {
    for configured in [false, true] {
        let ids = seed(&pool, OrgRole::Manager).await;
        let entry = time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE projects SET project_type = 'fixed_fee', rate_cents = 6000, starts_on = '2026-10-01' WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        if configured {
            sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,fee_mode,fee_amount_cents) VALUES ($1,$2,$3,$4,'person','single',10000)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
                .execute(&pool).await.unwrap();
        }
        let day = "2026-09-07".parse().unwrap();
        let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day).await;
        let spend =
            crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
                .await
                .unwrap();
        let report = crate::server_fns::reports::fetch_report(
            &pool,
            ids.user_id,
            (day, day),
            "project",
            crate::reports::ReportFilters::default(),
        )
        .await
        .unwrap();
        if configured {
            assert!(invoice.unwrap_err().to_string().contains("No billable"));
            assert_eq!((spend[0].spent_cents, report[0].billable_cents), (0, 0));
            assert!(
                sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", entry)
                    .fetch_one(&pool)
                    .await
                    .unwrap()
                    .is_none()
            );
        } else {
            assert_eq!(invoice.unwrap().invoice.total_cents, 6000);
            assert_eq!(
                (spend[0].spent_cents, report[0].billable_cents),
                (6000, 6000)
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn mixed_project_currencies_cannot_partially_create_an_invoice(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let second_project = uuid::Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,rate_cents) VALUES ($1,$2,$3,'USD project','USD',6000)", second_project, ids.org_id, ids.client_id)
        .execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'project')", uuid::Uuid::now_v7(), ids.org_id, second_project, ids.user_id)
        .execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state) VALUES ($1,$2,$3,$4,$5,'2026-09-07',60,true,'open')", uuid::Uuid::now_v7(), ids.org_id, ids.user_id, second_project, ids.task_id)
        .execute(&pool).await.unwrap();
    let day = "2026-09-07".parse().unwrap();
    let error = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("different billing currencies"));
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
    assert_eq!(sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1 AND invoice_id IS NULL AND state = 'open'", ids.org_id).fetch_one(&pool).await.unwrap(), Some(2));
}

#[sqlx::test(migrations = "./migrations")]
async fn selected_project_rate_modes_agree_across_billing_consumers(pool: PgPool) {
    for (mode, task, assignment, project, user, client, expected) in [
        (
            "person",
            Some(5000),
            Some(4000),
            Some(3500),
            Some(3000),
            Some(2000),
            4000,
        ),
        (
            "person",
            Some(5000),
            None,
            Some(3500),
            Some(3000),
            Some(2000),
            3000,
        ),
        (
            "person",
            Some(5000),
            None,
            Some(3500),
            None,
            Some(2000),
            2000,
        ),
        (
            "person",
            Some(5000),
            Some(0),
            Some(3500),
            Some(3000),
            Some(2000),
            0,
        ),
        (
            "task",
            Some(5000),
            Some(4000),
            Some(3500),
            Some(3000),
            Some(2000),
            5000,
        ),
        (
            "task",
            None,
            Some(4000),
            Some(3500),
            Some(3000),
            Some(2000),
            2000,
        ),
        (
            "task",
            Some(0),
            Some(4000),
            Some(3500),
            Some(3000),
            Some(2000),
            0,
        ),
        (
            "project",
            Some(5000),
            Some(4000),
            Some(3500),
            Some(3000),
            Some(2000),
            3500,
        ),
        (
            "project",
            Some(5000),
            Some(4000),
            Some(0),
            Some(3000),
            Some(2000),
            0,
        ),
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
        sqlx::query!(
            "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
            ids.client_id,
            client
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, $3)", ids.project_id, ids.task_id, task)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO assignments (id, project_id, user_id, role, rate_cents) VALUES ($1, $2, $3, 'lead', $4)", uuid::Uuid::now_v7(), ids.project_id, ids.user_id, assignment)
            .execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1,$2,$3,$4,$5)", uuid::Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode)
            .execute(&pool).await.unwrap();
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
        let spend =
            crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
            (expected, expected, expected, Some(expected)),
            "selected mode: {mode}; task={task:?}, assignment={assignment:?}, project={project:?}",
        );
    }
}

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
            ids.user_id,
            (day, day),
            "project",
            crate::reports::ReportFilters::default(),
        )
        .await
        .unwrap();
        let spend =
            crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
            (expected, expected, expected, Some(expected)),
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
    assert_eq!(invoice.lines[0].rate_cents, Some(6000));
    assert_eq!(invoice.invoice.total_cents, 6000);
    let report = crate::server_fns::reports::fetch_report(
        &pool,
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
    )
    .await
    .unwrap();
    assert_eq!(report[0].billable_cents, 6000);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
    )
    .await
    .unwrap();
    assert_eq!(report[0].billable_cents, invoice.invoice.total_cents);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(spend[0].spent_cents, invoice.invoice.total_cents);
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    let replacement = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap();
    assert_eq!(replacement.invoice.total_cents, 6000);
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
            ids.user_id,
            (day, day),
            "project",
            crate::reports::ReportFilters::default(),
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
        let spend =
            crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
                .await
                .unwrap();
        assert_eq!((spend[0].spent_minutes, spend[0].spent_cents), (60, 0));
        let detail = crate::reports::fetch_entries(
            &pool,
            ids.org_id,
            (day, day),
            crate::reports::ReportFilters::default(),
        )
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
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
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
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
    )
    .await
    .unwrap();
    let spend = crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
    )
    .await
    .unwrap_err();
    let spend_error =
        crate::server_fns::projects::fetch_project_spend(&pool, ids.org_id, ids.user_id)
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
    assert_eq!(
        amounts,
        vec![
            (Some(open), Some(15), 1500),
            (Some(approved), Some(10), 1000)
        ]
    );
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
        ids.user_id,
        (day, day),
        "project",
        crate::reports::ReportFilters::default(),
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
    assert_eq!(
        report[0].cost_cents,
        Some(1600),
        "labor cost uses worked minutes"
    );
    let detail = crate::reports::fetch_entries(
        pool,
        ids.org_id,
        (day, day),
        crate::reports::ReportFilters::default(),
    )
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
    let spend = crate::server_fns::projects::fetch_project_spend(pool, ids.org_id, ids.user_id)
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
            .all(|line| line.minutes == Some(8) && line.amount_cents == 800)
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
    assert_eq!(billed, vec![Some(stopped), Some(approved)]);
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
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
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
        transition_invoice(
            &competing_pool,
            ids.org_id,
            invoice.id,
            InvoiceStatus::Void,
            ids.user_id,
        )
        .await
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

    let result = transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    let foreign = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let day = "2026-09-07".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, day, day)
        .await
        .unwrap()
        .invoice;

    assert!(matches!(
        transition_invoice(
            &pool,
            ids.org_id,
            invoice.id,
            InvoiceStatus::Paid,
            ids.user_id
        )
        .await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
    assert!(matches!(
        transition_invoice(
            &pool,
            foreign.org_id,
            invoice.id,
            InvoiceStatus::Sent,
            foreign.user_id
        )
        .await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    assert!(matches!(
        transition_invoice(
            &pool,
            ids.org_id,
            invoice.id,
            InvoiceStatus::Sent,
            foreign.user_id
        )
        .await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
    .await
    .unwrap();
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.id,
        InvoiceStatus::Paid,
        ids.user_id,
    )
    .await
    .unwrap();
    assert!(matches!(
        transition_invoice(
            &pool,
            ids.org_id,
            invoice.id,
            InvoiceStatus::Void,
            ids.user_id
        )
        .await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
}
