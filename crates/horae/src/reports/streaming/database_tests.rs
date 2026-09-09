use horae_core::types::{EntryState, OrgRole};

use crate::server_fns::test_seed::{SeedIds, seed, time_entry};

use super::*;

fn params() -> ExportParams {
    ExportParams {
        from: "2026-09-07".to_owned(),
        to: "2026-09-07".to_owned(),
        client_id: None,
        project_id: None,
        user_id: None,
    }
}

async fn body(response: Response) -> Vec<u8> {
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "text/csv");
    axum::body::to_bytes(response.into_body(), 32 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec()
}

async fn add_entries(pool: &PgPool, ids: &SeedIds, count: usize) {
    let keys: Vec<_> = (0..count).map(|_| Uuid::now_v7()).collect();
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         SELECT id, $2, $3, $4, $5, '2026-09-07', 60, true FROM unnest($1::uuid[]) id",
        &keys, ids.org_id, ids.user_id, ids.project_id, ids.task_id,
    ).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_timesheet_preserves_csv_escaping_frozen_zero_and_all_filters(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &other, EntryState::Open).await;
    let id = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET notes = $2, rounded_minutes = 0 WHERE id = $1",
        id,
        "line one, \"quoted\"\n界"
    )
    .execute(&pool)
    .await
    .unwrap();
    let response = entries(pool.clone(), ids.org_id, params()).await.unwrap();
    assert_eq!(
        response.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"timesheet.csv\""
    );
    assert_eq!(
        String::from_utf8(body(response).await).unwrap(),
        "Date,Project,Task,User,Hours,Rounded Hours,Billable,Notes\n2026-09-07,Widget,Dev,Test User,1.00,0.00,Yes,\"line one, \"\"quoted\"\"\n界\"\n"
    );
    for filtered in [
        ExportParams {
            client_id: Some(other.client_id),
            ..params()
        },
        ExportParams {
            project_id: Some(other.project_id),
            ..params()
        },
        ExportParams {
            user_id: Some(other.user_id),
            ..params()
        },
        ExportParams {
            from: "2026-09-08".to_owned(),
            to: "2026-09-08".to_owned(),
            ..params()
        },
    ] {
        let csv = body(entries(pool.clone(), ids.org_id, filtered).await.unwrap()).await;
        assert_eq!(
            csv::Reader::from_reader(csv.as_slice()).records().count(),
            0
        );
    }
    assert_eq!(
        entries(
            pool,
            ids.org_id,
            ExportParams {
                from: "invalid".to_owned(),
                ..params()
            }
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_timesheet_exceeds_xlsx_row_limit_without_one_large_body_chunk(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 10_001).await;
    let response = entries(pool, ids.org_id, params()).await.unwrap();
    let mut chunks = response.into_body().into_data_stream();
    let first = chunks.next().await.unwrap().unwrap();
    assert!(first.len() < 1024);
    let mut bytes = first.to_vec();
    let mut count = 1;
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.unwrap();
        assert!(chunk.len() < CHUNK_BYTES + 16 * 1024);
        bytes.extend_from_slice(&chunk);
        count += 1;
    }
    assert!(count > 2);
    assert_eq!(
        csv::Reader::from_reader(bytes.as_slice()).records().count(),
        10_001
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_projects_preserve_scope_budget_and_tenant_isolation(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let _other = seed(&pool, OrgRole::Member).await;
    sqlx::query!("UPDATE projects SET code = 'A,B', budget_kind = 'hours', budget_minutes = 90 WHERE id = $1",
        ids.project_id).execute(&pool).await.unwrap();
    for scope in [None, Some("active"), Some("budgeted"), Some("unknown")] {
        let result = projects(
            pool.clone(),
            ids.org_id,
            ProjectsExportParams {
                scope: scope.map(str::to_owned),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            result.headers()[header::CONTENT_DISPOSITION],
            "attachment; filename=\"projects.csv\""
        );
        let bytes = body(result).await;
        let rows: Vec<_> = csv::Reader::from_reader(bytes.as_slice())
            .records()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(&rows[0][0], "Acme");
        assert_eq!(&rows[0][1], "A,B");
        assert_eq!(&rows[0][2], "Widget");
        assert_eq!(&rows[0][4], "EUR");
        assert_eq!(&rows[0][6], "Active");
        let expected = super::super::fetch_projects_export(&pool, ids.org_id, "active")
            .await
            .unwrap();
        assert_eq!(&rows[0][5], &super::super::budget_cell(&expected[0]));
    }
    let bytes = body(
        projects(
            pool,
            ids.org_id,
            ProjectsExportParams {
                scope: Some("archived".to_owned()),
            },
        )
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
async fn streamed_invoice_uses_stored_exact_cents_and_tenant_scoped_metadata(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, 'CSV-1', '2026-09-07', '2026-10-07', 'EUR', $4)",
        invoice_id, ids.org_id, ids.client_id, i64::MAX,
    ).execute(&pool).await.unwrap();
    let line_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
         VALUES ($1, $2, $3, $4, 0, $5, $5)",
        line_id, invoice_id, entry, "Quoted \"界\",\nline", i64::MAX,
    ).execute(&pool).await.unwrap();
    let result = invoice(pool.clone(), ids.org_id, invoice_id).await.unwrap();
    assert_eq!(
        result.headers()[header::CONTENT_DISPOSITION],
        "attachment; filename=\"invoice-CSV-1.csv\""
    );
    let bytes = body(result).await;
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "Description,Hours,Rate,Amount\n\"Quoted \"\"界\"\",\nline\",0.00,92233720368547758.07,92233720368547758.07\nTotal,,,92233720368547758.07\n"
    );
    assert_eq!(
        invoice(pool.clone(), other.org_id, invoice_id)
            .await
            .unwrap_err(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        invoice(pool, ids.org_id, Uuid::now_v7()).await.unwrap_err(),
        StatusCode::NOT_FOUND
    );
}

async fn export_connection(pool: &PgPool) -> (PgPool, i32) {
    let export_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&export_pool)
        .await
        .unwrap();
    (export_pool, pid)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn dropping_streamed_timesheet_closes_its_database_connection(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 10_001).await;
    let (export_pool, pid) = export_connection(&pool).await;
    let response = entries(export_pool.clone(), ids.org_id, params())
        .await
        .unwrap();
    drop(response);
    tokio::time::timeout(Duration::from_secs(10), async {
        while sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid = $1) as \"exists!\"",
            pid
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        {}
    })
    .await
    .expect("Cancelled CSV retained its database connection");
    let replacement = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&export_pool)
        .await
        .unwrap();
    assert_ne!(replacement, pid);
    export_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn a_real_database_error_interrupts_the_csv_body(pool: PgPool) {
    let slots = Arc::new(Semaphore::new(1));
    let result = response(&slots, DOWNLOAD_TIMEOUT, move |sender, filename| async move {
        let mut connection = pool.acquire().await.map_err(database_error)?;
        connection.close_on_drop();
        let _ = filename.send("error.csv".to_owned());
        let rows = sqlx::query_scalar!(
            "SELECT CASE WHEN n = 2 THEN 1 / (n - 2) ELSE n END as \"value!\" FROM generate_series(1, 2) n"
        ).fetch(&mut *connection);
        write_rows(&sender, rows, &["Value"], |writer, row| writer.write_record([row.to_string()])).await
    }).await.unwrap();
    let mut chunks = result.into_body().into_data_stream();
    assert_eq!(chunks.next().await.unwrap().unwrap(), "Value\n1\n");
    assert!(chunks.next().await.unwrap().is_err());
    assert!(chunks.next().await.is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn streamed_invoice_metadata_and_lines_share_one_snapshot(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, 'SNAPSHOT', '2026-09-07', '2026-10-07', 'EUR', 100)",
        invoice_id, ids.org_id, ids.client_id,
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
         VALUES ($1, $2, $1, 'Original', 60, 100, 100)", entry, invoice_id,
    ).execute(&pool).await.unwrap();
    let (export_pool, pid) = export_connection(&pool).await;
    let mut edit = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *edit)
        .await
        .unwrap();
    let task_pool = export_pool.clone();
    let task = tokio::spawn(invoice(task_pool, ids.org_id, invoice_id));
    let locked = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid = $1 AND wait_event_type = 'Lock') as \"locked!\"", pid
            ).fetch_one(&pool).await.unwrap() { break; }
        }
    }).await;
    if locked.is_err() {
        task.abort();
        let _ = task.await;
        panic!("CSV did not reach its line query");
    }
    sqlx::query!(
        "UPDATE invoices SET total_cents = 200 WHERE id = $1",
        invoice_id
    )
    .execute(&mut *edit)
    .await
    .unwrap();
    sqlx::query!("UPDATE invoice_line_items SET description = 'Changed', amount_cents = 200 WHERE invoice_id = $1", invoice_id)
        .execute(&mut *edit).await.unwrap();
    edit.commit().await.unwrap();
    let bytes = body(task.await.unwrap().unwrap()).await;
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "Description,Hours,Rate,Amount\nOriginal,1.00,1.00,1.00\nTotal,,,1.00\n"
    );
    export_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
#[ignore = "Manual release measurement of a 100,000-row CSV download"]
async fn measure_streaming_csv_export(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    add_entries(&pool, &ids, 100_000).await;
    let started = std::time::Instant::now();
    let result = entries(pool, ids.org_id, params()).await.unwrap();
    let first_chunk_time = started.elapsed();
    let mut stream = result.into_body().into_data_stream();
    let mut bytes = 0;
    let mut lines = 0;
    let mut chunks = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        bytes += chunk.len();
        lines += chunk.iter().filter(|&&byte| byte == b'\n').count();
        chunks += 1;
    }
    assert_eq!(lines, 100_001);
    assert!(chunks > 2);
    eprintln!(
        "100000 CSV rows: first chunk {first_chunk_time:?}, total {:?}, {bytes} bytes, {chunks} chunks",
        started.elapsed()
    );
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines().filter(|line| line.starts_with("VmHWM:")) {
            eprintln!("Process high-water RSS (includes fixture setup): {line}");
        }
    }
}
