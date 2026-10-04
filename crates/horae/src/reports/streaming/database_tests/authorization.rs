use super::*;
use crate::server_fns::test_seed::wait_for_blocked;

#[derive(Clone, Copy)]
enum Surface {
    Entries,
    Projects,
    Invoice,
}

impl Surface {
    async fn export(
        self,
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        invoice_id: Uuid,
    ) -> Result<Response, StatusCode> {
        match self {
            Self::Entries => entries(pool.clone(), org, actor, params()).await,
            Self::Projects => {
                projects(
                    pool.clone(),
                    org,
                    actor,
                    ProjectsExportParams { scope: None },
                )
                .await
            }
            Self::Invoice => invoice(pool.clone(), org, actor, invoice_id).await,
        }
    }
}

async fn empty_invoice(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id,org_id,client_id,number,issued_on,due_on,currency,total_cents)
        VALUES ($1,$2,$3,'EMPTY','2026-09-07','2026-10-07','EUR',0)",
        id,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_exports_require_current_tenant_bound_actors_even_when_empty(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let invoice_id = empty_invoice(&pool, &ids).await;
    for surface in [Surface::Entries, Surface::Projects, Surface::Invoice] {
        for (org, actor) in [
            (ids.org_id, foreign.user_id),
            (foreign.org_id, ids.user_id),
            (ids.org_id, Uuid::now_v7()),
            (Uuid::now_v7(), ids.user_id),
        ] {
            assert_eq!(
                surface
                    .export(&pool, org, actor, invoice_id)
                    .await
                    .unwrap_err(),
                StatusCode::FORBIDDEN
            );
        }
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            surface
                .export(&pool, ids.org_id, ids.user_id, invoice_id)
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        let bytes = body(
            surface
                .export(&pool, ids.org_id, ids.user_id, invoice_id)
                .await
                .unwrap(),
        )
        .await;
        assert!(!bytes.is_empty());
    }
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for surface in [Surface::Entries, Surface::Invoice] {
        assert_eq!(
            surface
                .export(&pool, ids.org_id, ids.user_id, invoice_id)
                .await
                .unwrap_err(),
            StatusCode::FORBIDDEN
        );
    }
    let bytes = body(
        Surface::Projects
            .export(&pool, ids.org_id, ids.user_id, invoice_id)
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(bytes.as_slice()).records().count(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_exports_refresh_after_organization_or_actor_wait(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let invoice_id = empty_invoice(&pool, &ids).await;
    for surface in [Surface::Entries, Surface::Projects, Surface::Invoice] {
        for organization_wait in [true, false] {
            let mut writer = pool.begin().await.unwrap();
            if organization_wait {
                crate::db::lock_organization(
                    &mut writer,
                    ids.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
            } else {
                sqlx::query!("SELECT id FROM users WHERE id=$1 FOR UPDATE", ids.user_id)
                    .fetch_one(&mut *writer)
                    .await
                    .unwrap();
            }
            let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *writer)
                .await
                .unwrap();
            let task_pool = pool.clone();
            let pending = tokio::spawn(async move {
                surface
                    .export(&task_pool, ids.org_id, ids.user_id, invoice_id)
                    .await
            });
            wait_for_blocked(&pool, pid).await;
            sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                .execute(&mut *writer)
                .await
                .unwrap();
            writer.commit().await.unwrap();
            assert_eq!(pending.await.unwrap().unwrap_err(), StatusCode::FORBIDDEN);
            sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_projects_capture_scope_gained_during_initial_wait(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let mut writer = pool.begin().await.unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *writer)
        .await
        .unwrap();
    let pending = tokio::spawn(projects(
        pool.clone(),
        ids.org_id,
        ids.user_id,
        ProjectsExportParams { scope: None },
    ));
    wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let bytes = body(pending.await.unwrap().unwrap()).await;
    let rows = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(&rows[0][2], "Widget");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_timesheet_rechecks_after_backpressure_without_retaining_locks(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 1000).await;
    let result = entries(pool.clone(), ids.org_id, ids.user_id, params())
        .await
        .unwrap();
    // The first record has passed authorization; at most one additional block
    // may already be queued. No consumer polls the body before revocation.
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!("SET LOCAL lock_timeout='1s'")
        .execute(&mut *writer)
        .await
        .unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let mut stream = result.into_body().into_data_stream();
    let mut bytes = Vec::new();
    let mut failed = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => bytes.extend_from_slice(&chunk),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "revocation must not become a successful EOF");
    assert!(csv::Reader::from_reader(bytes.as_slice()).records().count() <= 1 + CHUNK_ROWS);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_source_stays_frozen_across_batches_while_authority_is_fresh(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 400).await;
    let result = entries(pool.clone(), ids.org_id, ids.user_id, params())
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET notes='Changed after capture',minutes=120 WHERE org_id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let bytes = body(result).await;
    let rows = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 400);
    assert!(
        rows.iter()
            .all(|row| &row[4] == "1.00" && row[7].is_empty())
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_pending_project_block_checks_all_captured_ids_after_capacity(pool: PgPool) {
    for revoke_previous_block in [false, true] {
        let ids = seed(&pool, OrgRole::Member).await;
        let project_ids = [ids.project_id, Uuid::now_v7(), Uuid::now_v7()];
        for id in &project_ids[1..] {
            sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Later','EUR')",id,ids.org_id,ids.client_id).execute(&pool).await.unwrap();
        }
        for id in project_ids {
            sqlx::query!(
                "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
                Uuid::now_v7(),
                id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let (sender, mut receiver) = mpsc::channel(1);
        let (ready, waiting) = oneshot::channel();
        let worker_pool = pool.clone();
        let task = tokio::spawn(async move {
            let mut connection = worker_pool.acquire().await.unwrap();
            connection.close_on_drop();
            let mut tx = connection.begin().await.unwrap();
            let authority = Authority {
                org_id: ids.org_id,
                actor_id: ids.user_id,
                purpose: Purpose::Projects,
            };
            authority.begin(&mut tx).await.unwrap();
            cursor::declare_projects(&mut tx, ids.org_id, ids.user_id, "active")
                .await
                .unwrap();
            let rows = cursor::projects(&mut tx, 128).await.unwrap();
            assert_eq!(rows.len(), 3);
            let mut output = CsvBuffer::new(&["Project"])?;
            for id in project_ids {
                let row = rows.iter().find(|row| row.id == id).unwrap();
                output.writer.write_record([&row.name]).unwrap();
                output
                    .record(&sender, &mut tx, &authority, Some(id))
                    .await?;
            }
            // First row is queued; the remaining two distinct IDs share a block.
            let _ = ready.send(());
            output.flush(&sender, &mut tx, &authority).await
        });
        waiting.await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("SET LOCAL lock_timeout='1s'")
            .execute(&mut *writer)
            .await
            .unwrap();
        let revoked = if revoke_previous_block {
            project_ids[0]
        } else {
            project_ids[2]
        };
        sqlx::query!(
            "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
            revoked,
            ids.user_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        assert_eq!(receiver.recv().await.unwrap(), b"Project\nWidget\n");
        let result = task.await.unwrap();
        if revoke_previous_block {
            result.unwrap();
            assert_eq!(receiver.recv().await.unwrap(), b"Later\nLater\n");
        } else {
            assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
        }
        assert!(receiver.recv().await.is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_native_batches_keep_crossing_rows_and_reject_invalid_limits(pool: PgPool) {
    for limit in [None, Some(0), Some(129)] {
        let mut tx = pool.begin().await.unwrap();
        let error = sqlx::query!("SELECT value FROM fetch_csv_export_rows($1) AS source(value bigint,export_bytes bigint)", limit)
            .fetch_all(&mut *tx).await.unwrap_err();
        assert!(error.to_string().contains("Invalid CSV batch size"));
        tx.rollback().await.unwrap();
    }
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!("DECLARE horae_csv NO SCROLL CURSOR FOR SELECT n::bigint AS value,
        CASE WHEN n=2 THEN 65536::bigint ELSE 1::bigint END AS export_bytes FROM generate_series(1,260) n")
        .execute(&mut *tx).await.unwrap();
    let mut values = Vec::new();
    for (limit, count) in [(1, 1), (128, 1), (128, 128), (128, 128), (128, 2), (128, 0)] {
        let rows = sqlx::query_scalar!("SELECT value AS \"value!\" FROM fetch_csv_export_rows($1) AS source(value bigint,export_bytes bigint)", limit)
            .fetch_all(&mut *tx).await.unwrap();
        assert_eq!(rows.len(), count);
        values.extend(rows);
    }
    assert_eq!(values, (1_i64..=260).collect::<Vec<_>>());
    tx.rollback().await.unwrap();
    for weight in [None, Some(-1_i64)] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query!(
            "DECLARE horae_csv NO SCROLL CURSOR FOR SELECT 1::bigint value,$1::bigint export_bytes",
            weight
        )
        .execute(&mut *tx)
        .await
        .unwrap();
        let error = sqlx::query!(
            "SELECT value FROM fetch_csv_export_rows(1) AS source(value bigint,export_bytes bigint)"
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap_err();
        assert!(error.to_string().contains("Invalid CSV payload weight"));
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_compressible_unicode_payload_bounds_native_prefetch(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 4).await;
    let note = "界,\"quoted\"\n".repeat(CHUNK_BYTES);
    sqlx::query!(
        "UPDATE time_entries SET notes=$2 WHERE org_id=$1",
        ids.org_id,
        note
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let mut tx = connection.begin().await.unwrap();
    let authority = Authority {
        org_id: ids.org_id,
        actor_id: ids.user_id,
        purpose: Purpose::Manager,
    };
    authority.begin(&mut tx).await.unwrap();
    cursor::declare_entries(
        &mut tx,
        ids.org_id,
        ("2026-09-07".parse().unwrap(), "2026-09-07".parse().unwrap()),
        params().filters(),
    )
    .await
    .unwrap();
    for _ in 0..4 {
        let rows = cursor::entries(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].notes.as_deref(), Some(note.as_str()));
    }
    assert!(cursor::entries(&mut tx, 128).await.unwrap().is_empty());
    tx.rollback().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_invoice_keeps_original_metadata_lines_and_totals_between_batches(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 400).await;
    let invoice_id = empty_invoice(&pool, &ids).await;
    sqlx::query!(
        "UPDATE invoices SET total_cents=40000 WHERE id=$1",
        invoice_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO invoice_line_items (id,invoice_id,time_entry_id,description,minutes,rate_cents,amount_cents)
        SELECT id,$1,id,'Original',60,100,100 FROM time_entries WHERE org_id=$2",invoice_id,ids.org_id).execute(&pool).await.unwrap();
    let result = invoice(pool.clone(), ids.org_id, ids.user_id, invoice_id)
        .await
        .unwrap();
    let mut edit = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE invoices SET number='Changed',currency='USD',total_cents=80000 WHERE id=$1",
        invoice_id
    )
    .execute(&mut *edit)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE invoice_line_items SET description='Changed',amount_cents=200 WHERE invoice_id=$1",
        invoice_id
    )
    .execute(&mut *edit)
    .await
    .unwrap();
    edit.commit().await.unwrap();
    assert_eq!(
        result.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"invoice-EMPTY.csv\""
    );
    let bytes = body(result).await;
    let rows = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 402);
    assert!(
        rows[..400]
            .iter()
            .all(|row| &row[0] == "Original" && &row[3] == "1.00" && &row[4] == "EUR")
    );
    assert_eq!((&rows[400][0], &rows[400][3]), ("Subtotal", "400.00"));
    assert_eq!((&rows[401][0], &rows[401][3]), ("Total", "400.00"));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_header_and_total_blocks_recheck_manager_after_capacity(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    for tail in [false, true] {
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send(b"queued".to_vec()).await.unwrap();
        let (ready, waiting) = oneshot::channel();
        let worker_pool = pool.clone();
        let task = tokio::spawn(async move {
            let mut connection = worker_pool.acquire().await.unwrap();
            connection.close_on_drop();
            let mut tx = connection.begin().await.unwrap();
            let authority = Authority {
                org_id: ids.org_id,
                actor_id: ids.user_id,
                purpose: Purpose::Manager,
            };
            authority.begin(&mut tx).await.unwrap();
            let mut output = CsvBuffer::new(&["Amount"])?;
            if tail {
                output.writer.write_record(["123.45"]).unwrap();
            }
            let _ = ready.send(());
            output.flush(&sender, &mut tx, &authority).await
        });
        waiting.await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("SET LOCAL lock_timeout='1s'")
            .execute(&mut *writer)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role='member' WHERE id=$1",
            ids.user_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        writer.commit().await.unwrap();
        assert_eq!(receiver.recv().await.unwrap(), b"queued");
        assert_eq!(task.await.unwrap().unwrap_err(), StatusCode::FORBIDDEN);
        assert!(receiver.recv().await.is_none());
        sqlx::query!(
            "UPDATE users SET org_role='manager' WHERE id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_project_release_refreshes_scope_after_parent_wait(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM projects WHERE id=$1 FOR NO KEY UPDATE",
        ids.project_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *writer)
        .await
        .unwrap();
    let worker_pool = pool.clone();
    let pending = tokio::spawn(async move {
        let mut connection = worker_pool.acquire().await.unwrap();
        connection.close_on_drop();
        let mut tx = connection.begin().await.unwrap();
        let authority = Authority {
            org_id: ids.org_id,
            actor_id: ids.user_id,
            purpose: Purpose::Projects,
        };
        authority.begin(&mut tx).await?;
        authority.check(&mut tx, &[ids.project_id]).await
    });
    wait_for_blocked(&pool, pid).await;
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id=$1 AND user_id=$2",
        ids.project_id,
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    assert_eq!(pending.await.unwrap().unwrap_err(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_authority_releases_successful_locks_and_overrides_inherited_settings(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let mut connection = pool.acquire().await.unwrap();
    sqlx::query!("SET default_transaction_isolation='serializable'")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query!("SET default_transaction_read_only=on")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query!("SET lock_timeout='50ms'")
        .execute(&mut *connection)
        .await
        .unwrap();
    let mut tx = connection.begin().await.unwrap();
    let authority = Authority {
        org_id: ids.org_id,
        actor_id: ids.user_id,
        purpose: Purpose::Manager,
    };
    authority.begin(&mut tx).await.unwrap();
    for _ in 0..10 {
        authority.check(&mut tx, &[]).await.unwrap();
    }
    let settings=sqlx::query!("SELECT current_setting('transaction_isolation') isolation,current_setting('transaction_read_only') read_only,
        current_setting('lock_timeout') lock_timeout,current_setting('statement_timeout') statement_timeout,current_setting('idle_in_transaction_session_timeout') idle_timeout")
        .fetch_one(&mut *tx).await.unwrap();
    assert_eq!(settings.isolation.as_deref(), Some("read committed"));
    assert_eq!(settings.read_only.as_deref(), Some("off"));
    assert_eq!(settings.lock_timeout.as_deref(), Some("50ms"));
    assert_eq!(settings.statement_timeout.as_deref(), Some("5s"));
    assert_eq!(settings.idle_timeout.as_deref(), Some("10s"));
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
        ids.user_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    writer.rollback().await.unwrap();
    tx.rollback().await.unwrap();
    // The export connection is deliberately closed, not returned with settings.
    connection.close_on_drop();
}

async fn closed_backend(pool: &PgPool, pid: i32) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if !sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1) AS \"exists!\"",
                pid
            )
            .fetch_one(pool)
            .await
            .unwrap()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("cancelled export must close its connection");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_cancelled_authorization_reclaims_its_single_connection(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let (export_pool, pid) = export_connection(&pool).await;
    let mut writer = pool.begin().await.unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *writer)
        .await
        .unwrap();
    let task = tokio::spawn(entries(
        export_pool.clone(),
        ids.org_id,
        ids.user_id,
        params(),
    ));
    wait_for_blocked(&pool, blocker).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    closed_backend(&pool, pid).await;
    writer.rollback().await.unwrap();
    let bytes = body(
        entries(export_pool.clone(), ids.org_id, ids.user_id, params())
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(bytes.as_slice()).records().count(),
        0
    );
    export_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_cancelled_fetch_reclaims_connection_without_authority_locks(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 1).await;
    sqlx::query!("CREATE SCHEMA csv_fetch_test")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!(
        "CREATE FUNCTION csv_fetch_test.pause() RETURNS boolean LANGUAGE plpgsql AS $$
        BEGIN PERFORM pg_advisory_xact_lock(715047); RETURN true; END $$"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("CREATE VIEW csv_fetch_test.time_entries AS SELECT * FROM public.time_entries WHERE csv_fetch_test.pause()")
        .execute(&pool).await.unwrap();
    let export_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET search_path=csv_fetch_test,public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&export_pool)
        .await
        .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT pg_advisory_xact_lock(715047) AS \"lock!: ()\"")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let blocker_pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let task = tokio::spawn(entries(
        export_pool.clone(),
        ids.org_id,
        ids.user_id,
        params(),
    ));
    wait_for_blocked(&pool, blocker_pid).await;
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *blocker)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
        ids.user_id
    )
    .fetch_one(&mut *blocker)
    .await
    .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    closed_backend(&pool, pid).await;
    blocker.rollback().await.unwrap();
    export_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_native_projection_mismatch_fails_instead_of_coercing(pool: PgPool) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!("DECLARE horae_csv NO SCROLL CURSOR FOR SELECT 'not a bigint'::text value,10::bigint export_bytes")
        .execute(&mut *tx).await.unwrap();
    let error = sqlx::query!(
        "SELECT value FROM fetch_csv_export_rows(1) AS source(value bigint,export_bytes bigint)"
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("42804")
    );
    tx.rollback().await.unwrap();
}
