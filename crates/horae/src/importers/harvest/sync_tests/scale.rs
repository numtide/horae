//! Explicit release-mode scale checks. Run one test per process so Linux's
//! process high-water RSS is not inherited from another benchmark scenario.
//! Includes loopback HTTP, production request pacing, joins and SQL. The
//! fixture generates one page at a time and retains only 16 request headers.

use std::time::Instant;

use super::super::api_source::http::{
    ApiHttp,
    test_server::{Response, Server},
};
use super::*;
use std::sync::atomic::Ordering;

const RECORDS: u64 = 100_000;
const INVALID: u64 = 100;

fn require_release() {
    #[cfg(debug_assertions)]
    panic!("scale measurements require --release");
}

fn config() -> HarvestConfig {
    HarvestConfig {
        client_id: "test".into(),
        client_secret: "test".into(),
        redirect_url: "http://localhost/auth/harvest/callback".into(),
        encryption_key_hex: KEY.into(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn http_pages_preview_commit_and_reimport(pool: PgPool) {
    let org = setup(&pool).await;
    let server = scale_server(205);
    for mode in [ImportMode::DryRun, ImportMode::Commit, ImportMode::Commit] {
        let before = server.count.load(Ordering::Acquire);
        let report = run_api_import_with_http(
            &pool,
            org,
            "USD",
            &config(),
            mode,
            SyncScope::Full,
            ApiHttp::local(server.base.clone()),
        )
        .await
        .unwrap();
        assert!(report.reconciles());
        assert_eq!(report.summary.time_entries.processed(), 205);
        assert_eq!(report.summary.time_entries.errored, 1);
        assert_eq!(server.count.load(Ordering::Acquire) - before, 7);
        if before < 14 {
            assert_eq!(report.summary.time_entries.created, 204);
        } else {
            assert_eq!(report.summary.time_entries.skipped, 204);
        }
        assert_stored_rows(&pool, org, if mode == ImportMode::DryRun { 0 } else { 204 }).await;
    }
}

fn memory_status() -> String {
    std::fs::read_to_string("/proc/self/status")
        .map(|status| {
            status
                .lines()
                .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmHWM:"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|_| "RSS unavailable on this platform".into())
}

fn scale_server(records: u64) -> Server {
    let start = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    Server::start(move |url| {
        let collection = url.path().rsplit('/').next().unwrap();
        let page: u64 = url
            .query_pairs()
            .find(|(key, _)| key == "cursor" || key == "page")
            .unwrap()
            .1
            .parse()
            .unwrap();
        let offset = (page - 1) * 100;
        let items = match collection {
            "clients" => json!([{"id":1,"name":"Client","currency":"USD"}]),
            "projects" => json!([{"id":2,"name":"Project","client":{"id":1}}]),
            "tasks" => json!([{"id":3,"name":"Task"}]),
            "users" => {
                json!([{"id":4,"email":"known@example.com"},{"id":5,"email":"missing@example.com"}])
            }
            "time_entries" => json!(
                (offset..(offset + 100).min(records))
                    .map(|n| {
                        json!({
                            "id": 1_000_000 + n,
                            "spent_date": start + chrono::Duration::days((n % 365) as i64),
                            "hours": 1,
                            "notes": format!("scale entry {n}"),
                            "project": {"id": 2},
                            "task": {"id": 3},
                            "user": {"id": if n % 1000 == 0 { 5 } else { 4 }},
                            "updated_at": day(3)
                        })
                    })
                    .collect::<Vec<_>>()
            ),
            _ => panic!("unexpected collection {collection}"),
        };
        let next = if collection == "time_entries" && offset + 100 < records {
            let mut next = url.clone();
            next.query_pairs_mut()
                .clear()
                .append_pair("cursor", &(page + 1).to_string())
                .append_pair("per_page", "100");
            Some(next.to_string())
        } else {
            None
        };
        Response::json(json!({collection:items, "next_page":null, "links":{"next":next}}))
    })
}

async fn assert_stored_rows(pool: &PgPool, org: Uuid, expected: i64) {
    let stored = sqlx::query!(
        "SELECT count(*) AS entries, COALESCE(sum(minutes), 0)::bigint AS minutes FROM time_entries WHERE org_id = $1",
        org
    ).fetch_one(pool).await.unwrap();
    assert_eq!(stored.entries, Some(expected));
    assert_eq!(stored.minutes, Some(expected * 60));
    let provenance = sqlx::query_scalar!(
        "SELECT count(*) FROM harvest_import_map WHERE org_id = $1 AND harvest_entity_type = 'time_entry'::harvest_entity_type",
        org
    ).fetch_one(pool).await.unwrap();
    assert_eq!(provenance, Some(expected));
    assert_eq!(watermark(pool, org).await, json!({}));
}

async fn measured_apply(
    pool: &PgPool,
    org: Uuid,
    server: &Server,
    mode: ImportMode,
) -> ImportReport {
    let started = Instant::now();
    eprintln!("apply_start mode={mode:?} {}", memory_status());
    let cfg = config();
    let requests_before = server.count.load(Ordering::Acquire);
    let report = run_api_import_with_http(
        pool,
        org,
        "USD",
        &cfg,
        mode,
        SyncScope::Full,
        ApiHttp::local(server.base.clone()),
    )
    .await
    .unwrap();
    assert_eq!(
        server.count.load(Ordering::Acquire) - requests_before,
        RECORDS as usize / 100 + 4
    );
    eprintln!(
        "apply_complete mode={mode:?} elapsed_s={:.3} {}",
        started.elapsed().as_secs_f64(),
        memory_status()
    );
    assert!(report.reconciles());
    assert_eq!(report.summary.time_entries.processed(), RECORDS);
    assert_eq!(report.summary.time_entries.errored, INVALID);
    assert_eq!(report.error_count(), INVALID as usize);
    assert!(
        report
            .row_errors
            .iter()
            .all(|e| e.reason.contains("no Horae user matches email"))
    );
    report
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "100,000-entry scale measurement; run explicitly with --release --nocapture"]
async fn api_100k_commit_and_reimport(pool: PgPool) {
    require_release();
    let org = setup(&pool).await;
    eprintln!("baseline {}", memory_status());
    let server = scale_server(RECORDS);
    let first = measured_apply(&pool, org, &server, ImportMode::Commit).await;
    assert_eq!(first.summary.time_entries.created, RECORDS - INVALID);
    assert_stored_rows(&pool, org, (RECORDS - INVALID) as i64).await;
    let repeated = measured_apply(&pool, org, &server, ImportMode::Commit).await;
    assert_eq!(repeated.summary.time_entries.created, 0);
    assert_eq!(repeated.summary.time_entries.skipped, RECORDS - INVALID);
    assert_stored_rows(&pool, org, (RECORDS - INVALID) as i64).await;
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "100,000-entry scale measurement; run explicitly with --release --nocapture"]
async fn api_100k_dry_run(pool: PgPool) {
    require_release();
    let org = setup(&pool).await;
    eprintln!("baseline {}", memory_status());
    let server = scale_server(RECORDS);
    let preview = measured_apply(&pool, org, &server, ImportMode::DryRun).await;
    assert_eq!(preview.summary.time_entries.created, RECORDS - INVALID);
    assert_stored_rows(&pool, org, 0).await;
    let parents = sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(parents, Some(0));
}
