use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use std::time::Duration;
use uuid::Uuid;

async fn administrator(pool: &PgPool, org_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, 'Test User', $4)",
        id,
        org_id,
        format!("{id}@test.com"),
        OrgRole::Admin as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_generation_and_user_revocation_commit_in_gate_order(pool: PgPool) {
    for invoice_first in [true, false] {
        let ids = seed(&pool, OrgRole::Manager).await;
        let administrator = administrator(&pool, ids.org_id).await;
        let entry = time_entry(&pool, &ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE projects SET rate_cents=10000 WHERE id=$1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let day = "2026-09-07".parse().unwrap();
        let review = preview::prepare(
            &pool,
            (ids.org_id, ids.user_id),
            ids.client_id,
            (day, day),
            Some(&[ids.project_id]),
            None,
        )
        .await
        .unwrap();
        let request = InvoiceGenerationRequest {
            request_id: Uuid::now_v7(),
            review,
            confirmed_excess: vec![],
        };
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *barrier)
            .await
            .unwrap();
        if invoice_first {
            sqlx::query!("SELECT id FROM time_entries WHERE id=$1 FOR UPDATE", entry)
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
        } else {
            sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE", ids.user_id)
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
        }
        let mut operations = tokio::task::JoinSet::new();
        for invoice in [invoice_first, !invoice_first] {
            let db = pool.clone();
            let request = request.clone();
            operations.spawn(async move {
                let result = if invoice {
                    generate_invoice_with_request(
                        &db,
                        ids.org_id,
                        ids.client_id,
                        (day, day),
                        Some(&[ids.project_id]),
                        None,
                        Some((&request, ids.user_id)),
                    )
                    .await
                    .map(|(invoice, _)| Some(invoice.invoice.id))
                } else {
                    crate::server_fns::users::change_user_role(
                        &db,
                        ids.org_id,
                        administrator,
                        ids.user_id,
                        OrgRole::Member,
                    )
                    .await
                    .map(|_| None)
                };
                (invoice, result)
            });
            if invoice == invoice_first {
                wait_for_blocked(&pool, blocker).await;
            }
        }
        let first_pid = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            blocker,
        ).fetch_one(&pool).await.unwrap();
        wait_for_blocked(&pool, first_pid).await;
        barrier.commit().await.unwrap();
        let outcomes = tokio::time::timeout(Duration::from_secs(10), async {
            let mut outcomes = Vec::new();
            while let Some(outcome) = operations.join_next().await {
                outcomes.push(outcome.unwrap());
            }
            outcomes
        })
        .await
        .expect("invoice and revocation must finish without a lock cycle");
        for (invoice, result) in outcomes {
            if invoice && !invoice_first {
                assert!(
                    matches!(
                        result,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{result:?}"
                );
            } else {
                result.unwrap();
            }
        }
        let state = sqlx::query_scalar!(
            r#"SELECT state AS "state: EntryState" FROM time_entries WHERE id=$1"#,
            entry,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            state,
            if invoice_first {
                EntryState::Invoiced
            } else {
                EntryState::Open
            }
        );
        let saved = sqlx::query!(
            "SELECT (SELECT count(*) FROM invoices WHERE org_id=$1) AS invoices,
             (SELECT count(*) FROM invoice_generation_requests WHERE org_id=$1) AS requests",
            ids.org_id,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let expected = Some(i64::from(invoice_first));
        assert_eq!((saved.invoices, saved.requests), (expected, expected));
        assert!(matches!(
            generate_invoice_with_request(
                &pool,
                ids.org_id,
                ids.client_id,
                (day, day),
                Some(&[ids.project_id]),
                None,
                Some((&request, ids.user_id))
            )
            .await,
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_edit_and_transition_observe_user_revocation_order(pool: PgPool) {
    for edit_invoice in [true, false] {
        for invoice_first in [true, false] {
            let ids = super::super::fee_tests::single_fee(&pool).await;
            let administrator = administrator(&pool, ids.org_id).await;
            let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
            let invoice =
                generate_invoice_with_options(&pool, ids.org_id, ids.client_id, period, None, None)
                    .await
                    .unwrap();
            let invoice_id = invoice.invoice.id;
            let mut editor = editing::load(&pool, ids.org_id, ids.user_id, invoice_id)
                .await
                .unwrap();
            editor.edit.defaults.po_number = "AUTHORIZED-PO".into();
            let review = editing::review(&pool, ids.org_id, ids.user_id, invoice_id, &editor.edit)
                .await
                .unwrap();
            let request = InvoiceDraftSave {
                request_id: Uuid::now_v7(),
                edit: editor.edit,
                review,
                confirmed_excess: vec![],
            };
            let mut barrier = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *barrier)
                .await
                .unwrap();
            if invoice_first {
                sqlx::query!("SELECT id FROM invoices WHERE id=$1 FOR UPDATE", invoice_id)
                    .fetch_one(&mut *barrier)
                    .await
                    .unwrap();
            } else {
                sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE", ids.user_id)
                    .fetch_one(&mut *barrier)
                    .await
                    .unwrap();
            }
            let mut operations = tokio::task::JoinSet::new();
            for write_invoice in [invoice_first, !invoice_first] {
                let db = pool.clone();
                let request = request.clone();
                operations.spawn(async move {
                    let result = if !write_invoice {
                        crate::server_fns::users::change_user_role(
                            &db,
                            ids.org_id,
                            administrator,
                            ids.user_id,
                            OrgRole::Member,
                        )
                        .await
                        .map(|_| ())
                    } else if edit_invoice {
                        editing::save(&db, ids.org_id, ids.user_id, invoice_id, &request)
                            .await
                            .map(|_| ())
                    } else {
                        transition_invoice(
                            &db,
                            ids.org_id,
                            invoice_id,
                            InvoiceStatus::Sent,
                            ids.user_id,
                        )
                        .await
                        .map(|_| ())
                    };
                    (write_invoice, result)
                });
                if write_invoice == invoice_first {
                    wait_for_blocked(&pool, blocker).await;
                }
            }
            let first_pid = sqlx::query_scalar!(
                "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
                blocker,
            ).fetch_one(&pool).await.unwrap();
            wait_for_blocked(&pool, first_pid).await;
            barrier.commit().await.unwrap();
            let outcomes = tokio::time::timeout(Duration::from_secs(10), async {
                let mut outcomes = Vec::new();
                while let Some(outcome) = operations.join_next().await {
                    outcomes.push(outcome.unwrap());
                }
                outcomes
            })
            .await
            .expect("invoice write and revocation must finish without a lock cycle");
            for (write_invoice, result) in outcomes {
                if write_invoice && !invoice_first {
                    assert!(
                        matches!(
                            result,
                            Err(ServerFnError::ServerError {
                                code: FORBIDDEN,
                                ..
                            })
                        ),
                        "{result:?}"
                    );
                } else {
                    result.unwrap();
                }
            }
            let saved = sqlx::query!(
                r#"SELECT po_number, status AS "status: InvoiceStatus", total_cents,
                   (SELECT count(*) FROM invoice_edit_requests WHERE invoice_id=$1) AS requests
                   FROM invoices WHERE id=$1"#,
                invoice_id,
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(
                saved.po_number,
                if invoice_first && edit_invoice {
                    "AUTHORIZED-PO"
                } else {
                    ""
                }
            );
            assert_eq!(
                saved.status,
                if invoice_first && !edit_invoice {
                    InvoiceStatus::Sent
                } else {
                    InvoiceStatus::Draft
                }
            );
            assert_eq!(saved.total_cents, invoice.invoice.total_cents);
            assert_eq!(
                saved.requests,
                Some(i64::from(invoice_first && edit_invoice))
            );
            let replay = if edit_invoice {
                editing::save(&pool, ids.org_id, ids.user_id, invoice_id, &request)
                    .await
                    .map(|_| ())
            } else {
                transition_invoice(
                    &pool,
                    ids.org_id,
                    invoice_id,
                    InvoiceStatus::Sent,
                    ids.user_id,
                )
                .await
                .map(|_| ())
            };
            assert!(
                matches!(
                    replay,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{replay:?}"
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn invoice_writers_override_readonly_defaults_without_changing_the_session(pool: PgPool) {
    let ids = super::super::fee_tests::single_fee(&pool).await;
    let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
    let review = preview::prepare(
        &pool,
        (ids.org_id, ids.user_id),
        ids.client_id,
        period,
        None,
        None,
    )
    .await
    .unwrap();
    let generation = InvoiceGenerationRequest {
        request_id: Uuid::now_v7(),
        review,
        confirmed_excess: vec![],
    };
    let restricted = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only = on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation = 'repeatable read'")
                    .execute(&mut *connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let (invoice, _) = generate_invoice_with_request(
        &restricted,
        ids.org_id,
        ids.client_id,
        period,
        None,
        None,
        Some((&generation, ids.user_id)),
    )
    .await
    .unwrap();
    let id = invoice.invoice.id;
    let mut editor = editing::load(&pool, ids.org_id, ids.user_id, id)
        .await
        .unwrap();
    editor.edit.defaults.po_number = "LOCAL-TRANSACTION".into();
    let review = editing::review(&pool, ids.org_id, ids.user_id, id, &editor.edit)
        .await
        .unwrap();
    let edit = InvoiceDraftSave {
        request_id: Uuid::now_v7(),
        edit: editor.edit,
        review,
        confirmed_excess: vec![],
    };
    let saved = editing::save(&restricted, ids.org_id, ids.user_id, id, &edit)
        .await
        .unwrap();
    assert_eq!(saved.invoice.po_number, "LOCAL-TRANSACTION");
    let sent = transition_invoice(
        &restricted,
        ids.org_id,
        id,
        InvoiceStatus::Sent,
        ids.user_id,
    )
    .await
    .unwrap();
    assert_eq!(sent.status, InvoiceStatus::Sent);
    assert_eq!(sent.total_cents, invoice.invoice.total_cents);
    let defaults = sqlx::query!("SELECT current_setting('default_transaction_isolation') AS isolation, current_setting('default_transaction_read_only') AS readonly")
        .fetch_one(&restricted).await.unwrap();
    assert_eq!(defaults.isolation.as_deref(), Some("repeatable read"));
    assert_eq!(defaults.readonly.as_deref(), Some("on"));
    restricted.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn cancelled_invoice_writers_roll_back_and_release_single_connection(pool: PgPool) {
    for command in ["generate", "edit", "transition"] {
        let ids = super::super::fee_tests::single_fee(&pool).await;
        let administrator = administrator(&pool, ids.org_id).await;
        let period = ("2026-09-01".parse().unwrap(), "2026-09-30".parse().unwrap());
        let review = preview::prepare(
            &pool,
            (ids.org_id, ids.user_id),
            ids.client_id,
            period,
            None,
            None,
        )
        .await
        .unwrap();
        let generation = InvoiceGenerationRequest {
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
            None,
            Some((&generation, ids.user_id)),
        )
        .await
        .unwrap();
        let id = invoice.invoice.id;
        let editor = editing::load(&pool, ids.org_id, ids.user_id, id)
            .await
            .unwrap();
        let edit = InvoiceDraftSave {
            request_id: Uuid::now_v7(),
            edit: editor.edit,
            review: editor.review,
            confirmed_excess: vec![],
        };
        let mut barrier = pool.begin().await.unwrap();
        let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *barrier)
            .await
            .unwrap();
        balances::lock_invoices(&mut barrier, ids.org_id)
            .await
            .unwrap();
        let writer_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(pool.connect_options().as_ref().clone())
            .await
            .unwrap();
        let db = writer_pool.clone();
        let mut writers = tokio::task::JoinSet::new();
        writers.spawn(async move {
            match command {
                "generate" => generate_invoice_with_request(
                    &db,
                    ids.org_id,
                    ids.client_id,
                    period,
                    None,
                    None,
                    Some((&generation, ids.user_id)),
                )
                .await
                .map(|_| ()),
                "edit" => editing::save(&db, ids.org_id, ids.user_id, id, &edit)
                    .await
                    .map(|_| ()),
                "transition" => {
                    transition_invoice(&db, ids.org_id, id, InvoiceStatus::Sent, ids.user_id)
                        .await
                        .map(|_| ())
                }
                _ => unreachable!(),
            }
        });
        wait_for_blocked(&pool, blocker).await;
        let writer_pid = sqlx::query_scalar!(
            "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
            blocker,
        ).fetch_one(&pool).await.unwrap();
        let db = pool.clone();
        let mut revocation = tokio::task::JoinSet::new();
        revocation.spawn(async move {
            crate::server_fns::users::change_user_role(
                &db,
                ids.org_id,
                administrator,
                ids.user_id,
                OrgRole::Member,
            )
            .await
        });
        wait_for_blocked(&pool, writer_pid).await;
        writers.abort_all();
        let cancelled = writers.join_next().await.unwrap().unwrap_err();
        assert!(cancelled.is_cancelled());
        // SQLx queues rollback behind the in-flight statement; dropping its
        // future is not a PostgreSQL CancelRequest.
        barrier.rollback().await.unwrap();
        tokio::time::timeout(Duration::from_secs(10), revocation.join_next())
            .await
            .expect("revocation must finish after the cancelled writer rolls back")
            .unwrap()
            .unwrap()
            .unwrap();
        let saved = tokio::time::timeout(
            Duration::from_secs(5),
            sqlx::query!(
                r#"SELECT status AS "status: InvoiceStatus", total_cents,
                (SELECT count(*) FROM invoice_edit_requests WHERE org_id=$1) AS edits,
                (SELECT count(*) FROM invoice_generation_requests WHERE org_id=$1) AS generations,
                (SELECT count(*) FROM invoices WHERE org_id=$1) AS invoices
                FROM invoices WHERE id=$2"#,
                ids.org_id,
                id,
            )
            .fetch_one(&writer_pool),
        )
        .await
        .expect("cancelled writer must release its connection")
        .unwrap();
        assert_eq!(saved.status, InvoiceStatus::Draft);
        assert_eq!(saved.total_cents, invoice.invoice.total_cents);
        assert_eq!(
            (saved.edits, saved.generations, saved.invoices),
            (Some(0), Some(1), Some(1))
        );
        writer_pool.close().await;
    }
}
