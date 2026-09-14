//! Run alone in release mode: AppState and the process memory peak are global.

use horae_core::importers::harvest::types::{EntityType, RowError, RowOutcome, SourceKind};

use super::*;

fn require_release() {
    #[cfg(debug_assertions)]
    panic!("memory stress requires --release");
}

fn high_water_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "isolated Linux HTTP/memory stress; run with --release --exact --ignored --nocapture"]
async fn large_report_streams_over_http_without_buffering_the_archive(pool: PgPool) {
    require_release();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let owner = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    crate::state::init_state(
        pool.clone(),
        Arc::new(crate::plugin::PluginRegistry::empty()),
        None,
        None,
        Default::default(),
    )
    .await;
    let router = Router::new()
        .route(
            "/api/import/harvest/jobs/{job_id}/errors",
            get(crate::jobs::report::download),
        )
        .route(
            "/test/login/{id}",
            post(|session: Session, Path(id): Path<Uuid>| async move {
                crate::auth::session::set_session_user_id(&session, id)
                    .await
                    .unwrap();
                StatusCode::NO_CONTENT
            }),
        )
        .layer(
            crate::auth::make_session_layer(pool.clone(), false)
                .await
                .unwrap(),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = Api {
        base: format!("http://{}", listener.local_addr().unwrap()),
        client: reqwest::Client::builder()
            .cookie_store(false)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap(),
    };
    let mut server = tokio::task::JoinSet::new();
    server.spawn(async move { axum::serve(listener, router).await.unwrap() });
    let cookie = api.cookie(owner.user_id).await;
    let id = crate::jobs::enqueue(
        &pool,
        owner.org_id,
        &crate::jobs::JobPayload::HarvestCsv {
            mode: ImportMode::Commit,
        },
        "http-report-stress",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, _stop) = crate::jobs::claim_lease_for_test(&pool).await;

    // Keep only one 64-KiB record in the fixture, not the 64-MiB archive.
    const CHUNK: usize = 64 * 1024;
    const RECORDS: usize = 1024;
    let mut error = RowError {
        entity: EntityType::TimeEntry,
        source_location: "stress record".into(),
        reason: String::new(),
    };
    error.reason = "x".repeat(CHUNK - 1 - serde_json::to_vec(&error).unwrap().len());
    let mut expected_line = serde_json::to_vec(&error).unwrap();
    expected_line.push(b'\n');
    assert_eq!(expected_line.len(), CHUNK);
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
    for _ in 0..RECORDS {
        report.record(
            EntityType::TimeEntry,
            &RowOutcome::Errored {
                source_location: error.source_location.clone(),
                reason: error.reason.clone(),
            },
        );
        let mut tx = pool.begin().await.unwrap();
        lease.archive_report(&mut tx, &mut report).await.unwrap();
        tx.commit().await.unwrap();
        assert!(report.row_errors.is_empty());
    }
    assert_eq!(report.archived_error_chunks(), RECORDS as u64);
    let mut tx = pool.begin().await.unwrap();
    assert!(
        lease
            .complete(
                &mut tx,
                &serde_json::to_value(&report).unwrap(),
                RECORDS as i64
            )
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();

    let baseline = high_water_kib();
    let mut response = api.errors(id, Some(&cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut received = 0;
    while let Some(chunk) = response.chunk().await.unwrap() {
        let mut bytes = chunk.as_ref();
        while !bytes.is_empty() {
            let offset = received % CHUNK;
            let take = bytes.len().min(CHUNK - offset);
            assert!(bytes[..take] == expected_line[offset..offset + take]);
            received += take;
            bytes = &bytes[take..];
        }
    }
    assert_eq!(received, CHUNK * RECORDS);
    let peak = high_water_kib();
    eprintln!("archive_bytes={received} baseline_hwm_kib={baseline} download_hwm_kib={peak}");
    assert!(
        peak - baseline < 16 * 1024,
        "download buffered a substantial part of the archive"
    );

    // A disconnected consumer must not prevent another request on a size-one pool.
    let mut abandoned = api.errors(id, Some(&cookie)).await;
    assert!(abandoned.chunk().await.unwrap().is_some());
    drop(abandoned);
    let mut interrupted = api.errors(id, Some(&cookie)).await;
    assert_eq!(interrupted.status(), StatusCode::OK);
    let mut received = interrupted.chunk().await.unwrap().unwrap().len();
    sqlx::query!(
        "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    // Apply the same terminal-job retention query while the HTTP body is live.
    sqlx::query!(
        r#"DELETE FROM horae_jobs
            WHERE status IN ('succeeded', 'failed', 'cancelled')
              AND finished_at < now() - interval '30 days'"#
    )
    .execute(&pool)
    .await
    .unwrap();
    loop {
        match interrupted.chunk().await {
            Ok(Some(chunk)) => received += chunk.len(),
            Ok(None) => panic!("a missing archive must not be a successful HTTP EOF"),
            Err(_) => break,
        }
    }
    assert!(received < CHUNK * RECORDS);
    let peak = high_water_kib();
    eprintln!("interrupted_bytes={received} final_hwm_kib={peak}");
    assert!(peak - baseline < 16 * 1024);
    server.abort_all();
    while server.join_next().await.is_some() {}
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .unwrap();
}
