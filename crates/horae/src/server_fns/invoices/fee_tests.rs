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
async fn project_fee_context_matches_invoice_balances_and_current_authority(pool: PgPool) {
    use crate::server_fns::projects::fetch_project_fee_balances;
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let initial =
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
            .await
            .unwrap();
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].period_key, "single");
    assert_eq!(initial[0].balance.remaining_cents, 12500);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_fee_occurrences")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    let defaults = InvoiceDefaults {
        discount_bps: 1000,
        tax1_bps: 2100,
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
    let partial =
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
            .await
            .unwrap();
    assert_eq!(partial[0].balance.agreed_cents, 12500);
    assert_eq!(partial[0].balance.invoiced_cents, 11250);
    assert_eq!(partial[0].balance.remaining_cents, 1250);
    let mut edit = edit_request(&pool, &ids, invoice.invoice.id, 15000).await;
    edit.confirmed_excess = vec![crate::models::invoice::InvoiceExcessConfirmation {
        line_id: edit.edit.fees[0].line_id,
        excess_cents: 1000,
    }];
    editing::save(&pool, ids.org_id, ids.user_id, invoice.invoice.id, &edit)
        .await
        .unwrap();
    let over = fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
        .await
        .unwrap();
    assert_eq!(over[0].balance.invoiced_cents, 13500);
    assert_eq!(over[0].balance.remaining_cents, -1000);
    let preview = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(Some(over[0].balance), preview.lines[0].fee_balance);
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    let restored =
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
            .await
            .unwrap();
    assert_eq!(restored[0].balance.remaining_cents, 12500);
    let foreign = single_fee(&pool).await;
    assert!(
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, foreign.project_id, period)
            .await
            .is_err()
    );
    assert!(
        fetch_project_fee_balances(
            &pool,
            foreign.org_id,
            ids.user_id,
            foreign.project_id,
            period
        )
        .await
        .is_err()
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
            .await
            .is_err(),
        "even a project lead cannot read billing amounts after demotion"
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'admin', active = false WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_fee_balances(&pool, ids.org_id, ids.user_id, ids.project_id, period)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_fee_context_keeps_monthly_balances_separate(pool: PgPool) {
    use crate::server_fns::projects::fetch_project_fee_balances;
    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'monthly', monthly_day = 'last' WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let september = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    generate_invoice_for_period(&pool, ids.org_id, ids.client_id, september.0, september.1)
        .await
        .unwrap();
    let both = fetch_project_fee_balances(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        (september.0, "2026-10-31".parse().unwrap()),
    )
    .await
    .unwrap();
    assert_eq!(both.len(), 2);
    assert_eq!(
        (&*both[0].period_key, both[0].balance.remaining_cents),
        ("month:2026-09", 0)
    );
    assert_eq!(
        (&*both[1].period_key, both[1].balance.remaining_cents),
        ("month:2026-10", 12500)
    );
    let october = fetch_project_fee_balances(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ("2026-10-01".parse().unwrap(), "2026-10-31".parse().unwrap()),
    )
    .await
    .unwrap();
    assert_eq!(october, both[1..]);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_fee_occurrences WHERE project_id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
    assert!(
        fetch_project_fee_balances(
            &pool,
            ids.org_id,
            ids.user_id,
            ids.project_id,
            (september.1, september.0)
        )
        .await
        .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_status_rechecks_authority_after_waiting_for_the_invoice_lock(pool: PgPool) {
    for deactivate in [false, true] {
        let ids = single_fee(&pool).await;
        let invoice = generate_invoice_for_period(
            &pool,
            ids.org_id,
            ids.client_id,
            "2026-09-01".parse().unwrap(),
            "2026-09-30".parse().unwrap(),
        )
        .await
        .unwrap();
        let mut blocker = pool.begin().await.unwrap();
        balances::lock_invoices(&mut blocker, ids.org_id)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        let task_pool = pool.clone();
        let id = invoice.invoice.id;
        let transition = tokio::spawn(async move {
            transition_invoice(&task_pool, ids.org_id, id, InvoiceStatus::Void, ids.user_id).await
        });
        crate::server_fns::test_seed::wait_for_blocked(&pool, pid).await;
        if deactivate {
            sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "UPDATE users SET org_role = 'member' WHERE id = $1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        blocker.commit().await.unwrap();
        let error = transition
            .await
            .unwrap()
            .expect_err("a revoked actor must not release reserved fee balances");
        assert!(
            matches!(
                error,
                ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                }
            ),
            "{error}"
        );
        let stored = crate::reports::fetch_invoice_metadata(&pool, id, ids.org_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.status, InvoiceStatus::Draft);
        assert_eq!(stored.total_cents, invoice.invoice.total_cents);
        let preview = preview::prepare(
            &pool,
            ids.org_id,
            ids.client_id,
            ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap()),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(preview.lines[0].fee_balance.unwrap().remaining_cents, 0);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn settled_fee_sources_remain_reviewable_without_automatic_charges(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults::default();
    generate_invoice_for_period(&pool, ids.org_id, ids.client_id, period.0, period.1)
        .await
        .unwrap();
    let settled = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .expect("settled sources must remain available for explicit overbilling");
    assert_eq!(settled.lines.len(), 1);
    assert!(!settled.lines[0].selected);
    assert_eq!(settled.lines[0].amount_cents, 0);
    assert_eq!(settled.subtotal_cents, 0);
    let mut edits = settled.fee_selection();
    edits[0].selected = true;
    edits[0].amount_cents = 100;
    let review = preview::prepare_with_edits(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some(&edits),
    )
    .await
    .unwrap();
    assert_eq!(review.excess[0].excess_cents, 100);
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        confirmed_excess: review.excess.clone(),
        review,
    };
    generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    let overdrawn = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    assert!(!overdrawn.lines[0].selected);
    assert_eq!(
        overdrawn.lines[0].fee_balance.unwrap().remaining_cents,
        -100
    );
    assert_eq!(
        overdrawn.lines[0].fee_balance.unwrap().invoiced_cents,
        12600
    );
    assert_eq!(overdrawn.lines[0].fee_balance.unwrap().agreed_cents, 12500);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reviewed_generation_bills_partial_fees_and_rejects_stale_balances(pool: PgPool) {
    use crate::models::invoice::{InvoiceFeeSelection, InvoiceGenerationRequest};
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults::default();
    let initial = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let edits = vec![InvoiceFeeSelection {
        source: initial.lines[0].source.clone(),
        description: "First installment".into(),
        amount_cents: 6000,
        selected: true,
    }];
    let review = preview::prepare_with_edits(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some(&edits),
    )
    .await
    .unwrap();
    assert_eq!(review.subtotal_cents, 6000);
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    let (invoice, created) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    assert!(created);
    assert_eq!(invoice.lines[0].amount_cents, 6000);
    assert_eq!(invoice.lines[0].description, "First installment");
    let (replay, created) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    assert!(!created);
    assert_eq!(replay.invoice.id, invoice.invoice.id);
    let stale = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        ..request
    };
    assert!(matches!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            Some((&stale, ids.user_id))
        )
        .await,
        Err(ServerFnError::ServerError { code: CONFLICT, .. })
    ));
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
            "SELECT amount_cents FROM project_fee_occurrences WHERE id = $1",
            invoice.lines[0].fee_occurrence_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        12500
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reviewed_generation_requires_exact_excess_and_current_authority(pool: PgPool) {
    use crate::models::invoice::{InvoiceFeeSelection, InvoiceSourceExcess};
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults {
        discount_bps: 1000,
        tax1_bps: 2100,
        ..Default::default()
    };
    let initial = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let edits = vec![InvoiceFeeSelection {
        source: initial.lines[0].source.clone(),
        description: "Additional scope".into(),
        amount_cents: 14000,
        selected: true,
    }];
    let review = preview::prepare_with_edits(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some(&edits),
    )
    .await
    .unwrap();
    let mut request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    let error = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("exact amount"), "{error}");
    request.confirmed_excess = vec![InvoiceSourceExcess {
        source: edits[0].source.clone(),
        excess_cents: 99,
    }];
    assert!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            Some((&request, ids.user_id))
        )
        .await
        .is_err()
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
    request.confirmed_excess[0].excess_cents = 100;
    let foreign = seed(&pool, OrgRole::Manager).await;
    assert!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            Some((&request, foreign.user_id))
        )
        .await
        .is_err()
    );
    let (invoice, _) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    assert_eq!(
        invoice.invoice.subtotal_cents - invoice.invoice.discount_cents,
        12600
    );
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            Some((&request, ids.user_id))
        )
        .await
        .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reviewed_generation_does_not_materialize_unselected_months(pool: PgPool) {
    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'monthly', monthly_day = 'last' WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let period = ("2028-02-16".parse().unwrap(), "2028-03-31".parse().unwrap());
    let defaults = InvoiceDefaults::default();
    let initial = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let mut edits = initial.fee_selection();
    edits[1].selected = false;
    let review = preview::prepare_with_edits(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some(&edits),
    )
    .await
    .unwrap();
    assert_eq!(review.subtotal_cents, 12500);
    assert_eq!(review.lines[1].net_before_tax_cents, Some(0));
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    let (invoice, _) = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap();
    assert_eq!(invoice.lines.len(), 1);
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
    let remaining = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let selected: Vec<_> = remaining
        .lines
        .iter()
        .filter(|line| line.selected)
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].source, edits[1].source);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reviewed_generation_rejects_changed_time_before_claiming_it(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = crate::server_fns::test_seed::time_entry(&pool, &ids, EntryState::Open).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults::default();
    let review = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET minutes=minutes+60 WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    let error = generate_invoice_with_request(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some((&request, ids.user_id)),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error, ServerFnError::ServerError { code: CONFLICT, .. }),
        "{error}"
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id=$1", entry)
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
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn prepared_fee_selection_rejects_invalid_values_and_empty_generation(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults::default();
    let initial = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    for case in 0..7 {
        let mut edits = initial.fee_selection();
        match case {
            0 => edits[0].amount_cents = -1,
            1 => edits[0].amount_cents = 0,
            2 => edits[0].description = " ".into(),
            3 => edits[0].description = "x\0y".into(),
            4 => edits.push(edits[0].clone()),
            5 => edits.clear(),
            _ => {
                edits[0].source = InvoiceSource::Time {
                    entry_id: Uuid::now_v7(),
                }
            }
        }
        assert!(
            preview::prepare_with_edits(
                &pool,
                ids.org_id,
                ids.client_id,
                period,
                None,
                Some(&defaults),
                Some(&edits)
            )
            .await
            .is_err(),
            "case {case}"
        );
    }
    let mut edits = initial.fee_selection();
    edits[0].selected = false;
    edits[0].amount_cents = 0;
    let review = preview::prepare_with_edits(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
        Some(&edits),
    )
    .await
    .unwrap();
    assert_eq!(review.amounts.unwrap().total_cents, 0);
    let request = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    assert!(
        generate_invoice_with_request(
            &pool,
            ids.org_id,
            ids.client_id,
            period,
            None,
            Some(&defaults),
            Some((&request, ids.user_id))
        )
        .await
        .is_err()
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
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn fee_preview_keeps_source_identity_and_reports_discounted_draft_balances(pool: PgPool) {
    use crate::models::invoice::{InvoiceFeeBalance, InvoiceSource};

    let ids = single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults {
        discount_bps: 1000,
        tax1_bps: 2100,
        ..Default::default()
    };
    let initial = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let source = InvoiceSource::Fee {
        project_id: ids.project_id,
        period_key: "single".into(),
    };
    assert_eq!(initial.lines[0].source, source);
    assert_eq!(initial.lines[0].net_before_tax_cents, Some(11250));
    assert_eq!(
        initial.lines[0].fee_balance,
        Some(InvoiceFeeBalance {
            agreed_cents: 12500,
            invoiced_cents: 0,
            remaining_cents: 12500,
        })
    );
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
    let remaining = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    assert_eq!(remaining.lines[0].source, source);
    assert_eq!(remaining.lines[0].amount_cents, 1250);
    assert_eq!(remaining.lines[0].net_before_tax_cents, Some(1125));
    assert_eq!(
        remaining.lines[0].fee_balance,
        Some(InvoiceFeeBalance {
            agreed_cents: 12500,
            invoiced_cents: 11250,
            remaining_cents: 1250,
        })
    );
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    let restored = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    assert_eq!(restored.lines, initial.lines);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn fee_preview_discount_ties_follow_source_keys_not_display_dates(pool: PgPool) {
    use crate::models::invoice::InvoiceSource;

    let ids = single_fee(&pool).await;
    sqlx::query!("UPDATE project_settings SET fee_mode = 'milestones', fee_amount_cents = NULL WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let mut milestones = [Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7()];
    milestones.sort();
    for (position, date) in [(0_i16, "2026-09-30"), (1, "2026-09-15"), (2, "2026-09-01")] {
        sqlx::query!("INSERT INTO project_fee_milestones (id,org_id,project_id,name,due_on,amount_cents,position) VALUES ($1,$2,$3,$4,$5,$6,$7)", milestones[position as usize], ids.org_id, ids.project_id, format!("Milestone {position}"), date.parse::<chrono::NaiveDate>().unwrap() as chrono::NaiveDate, 1_i64, position)
            .execute(&pool).await.unwrap();
    }
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let defaults = InvoiceDefaults {
        discount_bps: 5000,
        ..Default::default()
    };
    let preview = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    assert_eq!(
        preview
            .lines
            .iter()
            .map(|line| (&line.source, line.net_before_tax_cents))
            .collect::<Vec<_>>(),
        vec![
            (
                &InvoiceSource::Fee {
                    project_id: ids.project_id,
                    period_key: format!("milestone:{}", milestones[2])
                },
                Some(1)
            ),
            (
                &InvoiceSource::Fee {
                    project_id: ids.project_id,
                    period_key: format!("milestone:{}", milestones[1])
                },
                Some(0)
            ),
            (
                &InvoiceSource::Fee {
                    project_id: ids.project_id,
                    period_key: format!("milestone:{}", milestones[0])
                },
                Some(0)
            ),
        ]
    );
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
    let persisted = sqlx::query!(
        "SELECT description, net_before_tax_cents FROM invoice_line_items WHERE invoice_id = $1",
        invoice.invoice.id
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for line in &preview.lines {
        let saved = persisted
            .iter()
            .find(|saved| saved.description == line.description)
            .unwrap();
        assert_eq!(saved.net_before_tax_cents, line.net_before_tax_cents);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn draft_fee_edit_replaces_amount_and_preserves_source_and_retry_identity(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let from = "2026-09-01".parse().unwrap();
    let to = "2026-09-30".parse().unwrap();
    let invoice = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    let mut editor = editing::load(&pool, ids.org_id, ids.user_id, invoice.invoice.id)
        .await
        .unwrap();
    editor.edit.fees[0].amount_cents = 6000;
    editor.edit.fees[0].description = "First installment".into();
    let review = editing::review(
        &pool,
        ids.org_id,
        ids.user_id,
        invoice.invoice.id,
        &editor.edit,
    )
    .await
    .unwrap();
    assert_eq!(
        (review.amounts.total_cents, review.fees[0].remaining_cents),
        (6000, 6500)
    );
    let request = InvoiceDraftSave {
        request_id: Uuid::now_v7(),
        edit: editor.edit,
        review,
        confirmed_excess: Vec::new(),
    };
    let saved = editing::save(&pool, ids.org_id, ids.user_id, invoice.invoice.id, &request)
        .await
        .unwrap();
    let retry = editing::save(&pool, ids.org_id, ids.user_id, invoice.invoice.id, &request)
        .await
        .unwrap();
    assert_eq!(saved, retry);
    assert_eq!(saved.lines[0].id, invoice.lines[0].id);
    assert_eq!(saved.lines[0].description, "First installment");
    let remaining = preview::prepare(&pool, ids.org_id, ids.client_id, (from, to), None, None)
        .await
        .unwrap();
    assert_eq!(remaining.subtotal_cents, 6500);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT amount_cents FROM project_fee_occurrences WHERE id = $1",
            saved.lines[0].fee_occurrence_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        12500
    );
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
    transition_invoice(
        &pool,
        ids.org_id,
        second.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    let settled = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert!(settled.lines.iter().all(|line| !line.selected));
    assert_eq!(settled.subtotal_cents, 0);
    transition_invoice(
        &pool,
        ids.org_id,
        first.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    let restored = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert_eq!(restored.subtotal_cents, 12500);
}

async fn edit_request(
    pool: &PgPool,
    ids: &SeedIds,
    invoice_id: Uuid,
    amount: i64,
) -> InvoiceDraftSave {
    let mut editor = editing::load(pool, ids.org_id, ids.user_id, invoice_id)
        .await
        .unwrap();
    editor.edit.fees[0].amount_cents = amount;
    let review = editing::review(pool, ids.org_id, ids.user_id, invoice_id, &editor.edit)
        .await
        .unwrap();
    InvoiceDraftSave {
        request_id: Uuid::now_v7(),
        edit: editor.edit,
        review,
        confirmed_excess: Vec::new(),
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn draft_fee_edit_requires_current_balance_and_exact_excess_confirmation(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let from = "2026-09-01".parse().unwrap();
    let to = "2026-09-30".parse().unwrap();
    let first = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    let first_id = first.invoice.id;
    let partial = edit_request(&pool, &ids, first_id, 6000).await;
    editing::save(&pool, ids.org_id, ids.user_id, first_id, &partial)
        .await
        .unwrap();
    let mut stale = edit_request(&pool, &ids, first_id, 7000).await;
    let second = generate_invoice_for_period(&pool, ids.org_id, ids.client_id, from, to)
        .await
        .unwrap();
    assert_eq!(second.invoice.total_cents, 6500);
    let error = editing::save(&pool, ids.org_id, ids.user_id, first_id, &stale)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("balances changed"), "{error}");
    stale.review = editing::review(&pool, ids.org_id, ids.user_id, first_id, &stale.edit)
        .await
        .unwrap();
    assert_eq!(
        (
            stale.review.fees[0].remaining_cents,
            stale.review.fees[0].excess_cents
        ),
        (-1000, 1000)
    );
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, first_id, &stale)
            .await
            .is_err()
    );
    stale
        .confirmed_excess
        .push(crate::models::invoice::InvoiceExcessConfirmation {
            line_id: stale.edit.fees[0].line_id,
            excess_cents: 999,
        });
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, first_id, &stale)
            .await
            .is_err()
    );
    let unchanged = editing::load(&pool, ids.org_id, ids.user_id, first_id)
        .await
        .unwrap();
    assert_eq!(
        (unchanged.edit.revision, unchanged.edit.fees[0].amount_cents),
        (stale.edit.revision, 6000)
    );
    stale.confirmed_excess[0].excess_cents = 1000;
    let saved = editing::save(&pool, ids.org_id, ids.user_id, first_id, &stale)
        .await
        .unwrap();
    assert_eq!(saved.invoice.total_cents, 7000);
    assert_eq!(
        editing::load(&pool, ids.org_id, ids.user_id, first_id)
            .await
            .unwrap()
            .review
            .fees[0]
            .remaining_cents,
        -1000
    );
    transition_invoice(
        &pool,
        ids.org_id,
        first_id,
        InvoiceStatus::Void,
        ids.user_id,
    )
    .await
    .unwrap();
    assert_eq!(
        preview::prepare(&pool, ids.org_id, ids.client_id, (from, to), None, None)
            .await
            .unwrap()
            .subtotal_cents,
        6000
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn draft_fee_edit_serializes_retries_and_rejects_stale_revisions(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let invoice = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    let id = invoice.invoice.id;
    let request = edit_request(&pool, &ids, id, 6000).await;
    let (a, b) = tokio::join!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &request),
        editing::save(&pool, ids.org_id, ids.user_id, id, &request)
    );
    assert_eq!(a.unwrap(), b.unwrap());
    let mut changed = request.clone();
    changed.edit.fees[0].description = "Different payload".into();
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &changed)
            .await
            .is_err()
    );
    changed.request_id = Uuid::now_v7();
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &changed)
            .await
            .is_err()
    );
    let newer = edit_request(&pool, &ids, id, 7000).await;
    editing::save(&pool, ids.org_id, ids.user_id, id, &newer)
        .await
        .unwrap();
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &request)
            .await
            .is_err()
    );
    assert_eq!(
        editing::load(&pool, ids.org_id, ids.user_id, id)
            .await
            .unwrap()
            .edit
            .fees[0]
            .amount_cents,
        7000
    );
    transition_invoice(&pool, ids.org_id, id, InvoiceStatus::Sent, ids.user_id)
        .await
        .unwrap();
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &newer)
            .await
            .is_err()
    );
    assert!(
        editing::load(&pool, ids.org_id, ids.user_id, id)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn draft_fee_edit_rejects_invalid_lines_and_current_unauthorized_actors(pool: PgPool) {
    let ids = single_fee(&pool).await;
    let invoice = generate_invoice_for_period(
        &pool,
        ids.org_id,
        ids.client_id,
        "2026-09-01".parse().unwrap(),
        "2026-09-30".parse().unwrap(),
    )
    .await
    .unwrap();
    let id = invoice.invoice.id;
    let request = edit_request(&pool, &ids, id, 6000).await;
    let mut invalid = Vec::new();
    for amount in [-1, 0] {
        let mut edit = request.edit.clone();
        edit.fees[0].amount_cents = amount;
        invalid.push(edit);
    }
    for description in [" ".into(), "x".repeat(1001), "a\0b".into()] {
        let mut edit = request.edit.clone();
        edit.fees[0].description = description;
        invalid.push(edit);
    }
    let mut duplicate = request.edit.clone();
    duplicate.fees.push(duplicate.fees[0].clone());
    invalid.push(duplicate);
    let mut unknown = request.edit.clone();
    unknown.fees[0].line_id = Uuid::now_v7();
    invalid.push(unknown);
    for edit in invalid {
        assert!(
            editing::review(&pool, ids.org_id, ids.user_id, id, &edit)
                .await
                .is_err()
        );
    }
    let outsider = seed(&pool, OrgRole::Manager).await;
    assert!(
        editing::load(&pool, outsider.org_id, outsider.user_id, id)
            .await
            .is_err()
    );
    assert!(
        editing::save(&pool, outsider.org_id, outsider.user_id, id, &request)
            .await
            .is_err()
    );
    assert!(
        editing::save(&pool, ids.org_id, outsider.user_id, id, &request)
            .await
            .is_err()
    );
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        editing::load(&pool, ids.org_id, ids.user_id, id)
            .await
            .is_err()
    );
    assert!(
        editing::save(&pool, ids.org_id, ids.user_id, id, &request)
            .await
            .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT amount_cents FROM invoice_line_items WHERE id=$1",
            invoice.lines[0].id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        12500
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM invoice_edit_requests")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
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
    let review = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    let mutation = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: Vec::new(),
    };
    let request = Some((&mutation, ids.user_id));
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
    transition_invoice(
        &pool,
        ids.org_id,
        a.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    let preview = preview::prepare(
        &pool,
        ids.org_id,
        ids.client_id,
        period,
        None,
        Some(&defaults),
    )
    .await
    .unwrap();
    assert_eq!(
        preview.lines[0].source,
        crate::models::invoice::InvoiceSource::Time { entry_id: entry }
    );
    assert_eq!(preview.lines[0].fee_balance, None);
    assert_eq!(
        preview
            .lines
            .iter()
            .map(|line| line.net_before_tax_cents)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );
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
    let settled = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert!(settled.lines.iter().all(|line| !line.selected));
    assert_eq!(settled.subtotal_cents, 0);
    transition_invoice(
        &pool,
        ids.org_id,
        invoice.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    let settled = preview::prepare(&pool, ids.org_id, ids.client_id, period, None, None)
        .await
        .unwrap();
    assert!(settled.lines.iter().all(|line| !line.selected));
    assert_eq!(settled.subtotal_cents, 0);
    transition_invoice(
        &pool,
        ids.org_id,
        generated.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    transition_invoice(
        &pool,
        ids.org_id,
        result.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    transition_invoice(
        &pool,
        ids.org_id,
        first.invoice.id,
        InvoiceStatus::Void,
        ids.user_id,
    )
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
    assert_eq!(
        estimate
            .lines
            .iter()
            .map(|line| line.source.clone())
            .collect::<Vec<_>>(),
        vec![
            crate::models::invoice::InvoiceSource::Fee {
                project_id: ids.project_id,
                period_key: "month:2028-02".into()
            },
            crate::models::invoice::InvoiceSource::Fee {
                project_id: ids.project_id,
                period_key: "month:2028-03".into()
            },
        ]
    );
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
    assert_eq!(estimate.lines.len(), 3);
    assert_eq!(estimate.amounts.unwrap().total_cents, 12500);
    let selected: Vec<_> = estimate.lines.iter().filter(|line| line.selected).collect();
    assert_eq!(selected.len(), 1);
    assert!(selected[0].description.contains("2028-04"));
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
