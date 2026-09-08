//! DB-backed engine tests (`#[sqlx::test]`, throwaway database per test).
//!
//! These cover the story-level guarantees that need a real Postgres: FK-safe
//! creation with exact minutes/cents and provenance (US1), idempotent and
//! edit-robust re-runs (US2), dry-run-writes-nothing (US3), per-record resilience
//! and reconciliation (US4), and the CSV natural-key path (US5). The engine is
//! reached through the crate-internal API, so these live in the bin crate rather
//! than `tests/` (an integration test cannot import a binary's modules).

use chrono::{NaiveDate, TimeZone, Utc};
use horae_core::importers::harvest::types::{EntityType, ImportMode, SourceKind, SourceRow};
use sqlx::PgPool;
use uuid::Uuid;

use super::{RowSource, VecSource, run_import};

struct PausedSource {
    before_pause: Option<SourceRow>,
    rows: VecSource,
    pause: Option<(
        tokio::sync::oneshot::Sender<()>,
        tokio::sync::oneshot::Receiver<()>,
    )>,
}

impl RowSource for PausedSource {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        if let Some(row) = self.before_pause.take() {
            return Ok(Some(row));
        }
        if let Some((started, release)) = self.pause.take() {
            let _ = started.send(());
            release.await?;
        }
        self.rows.next_row().await
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_imports_reject_overlap_without_collapsing_repeated_entries(pool: PgPool) {
    for mode in [ImportMode::Commit, ImportMode::DryRun] {
        let org = seed_org(&pool).await;
        let email = format!("dev-{org}@acme.com");
        seed_user(&pool, org, &email).await;
        let row = nk_row(
            "Acme",
            "Website",
            "Design",
            &email,
            (2026, 1, 15),
            "1",
            None,
        );
        let rows = vec![row.clone(), row];
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = tokio::sync::oneshot::channel();
        let source = PausedSource {
            before_pause: None,
            rows: VecSource::new(rows.clone()),
            pause: Some((started, wait)),
        };
        let first_pool = pool.clone();
        let first = tokio::spawn(async move {
            run_import(&first_pool, org, "USD", SourceKind::Csv, mode, source).await
        });
        ready.await.unwrap();
        let second = run_import(
            &pool,
            org,
            "USD",
            SourceKind::Csv,
            ImportMode::Commit,
            VecSource::new(rows.clone()),
        )
        .await;
        release.send(()).unwrap();
        let first = first.await.unwrap().unwrap();
        assert_eq!(
            second.unwrap_err().to_string(),
            "Another Harvest import is already running for this organization; retry when it finishes"
        );
        assert_eq!(first.summary.time_entries.created, 2);
        let count_before_retry =
            sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            count_before_retry,
            Some(if mode == ImportMode::Commit { 2 } else { 0 })
        );
        let retry = commit_csv(&pool, org, rows).await;
        assert_eq!(
            (
                retry.summary.time_entries.created,
                retry.summary.time_entries.skipped
            ),
            if mode == ImportMode::Commit {
                (0, 2)
            } else {
                (2, 0)
            }
        );
    }
}

fn disconnected_config() -> crate::config::HarvestConfig {
    crate::config::HarvestConfig {
        client_id: "test-client".into(),
        client_secret: "test-secret".into(),
        redirect_url: "http://localhost/auth/harvest/callback".into(),
        encryption_key_hex: "11".repeat(32),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelled_http_waiter_keeps_the_lock_until_its_worker_exits(pool: PgPool) {
    let org = seed_org(&pool).await;
    let one_connection = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let guard = super::lock_import(&one_connection, org).await.unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let pending = tokio::spawn(super::blocking_import_call(guard, move || {
        let _ = started.send(());
        wait.recv().unwrap();
    }));
    ready.await.unwrap();
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    let competing = super::lock_import(&pool, org).await;
    // Release the worker even when the assertion fails; runtime teardown waits
    // for blocking tasks, so a failing regression must not hang the test suite.
    release.send(()).unwrap();
    assert!(matches!(competing, Err(super::ApiImportError::Busy)));
    let retry = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        super::lock_import(&one_connection, org),
    )
    .await
    .unwrap()
    .unwrap();
    retry.close().await.unwrap();
    one_connection.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn api_and_csv_share_the_lock_but_other_organizations_can_import(pool: PgPool) {
    let org = seed_org(&pool).await;
    let other_org = seed_org(&pool).await;
    let guard = super::lock_import(&pool, org).await.unwrap();
    let api = super::run_api_import(
        &pool,
        org,
        "USD",
        &disconnected_config(),
        ImportMode::Commit,
        super::SyncScope::Full,
    )
    .await;
    assert!(matches!(api, Err(super::ApiImportError::Busy)), "{api:?}");
    let csv =
        b"Date,Client,Project,Task,Hours,Email\n2026-01-15,Acme,Website,Design,1,dev@acme.com\n";
    let result = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::DryRun)
        .await
        .unwrap_err();
    assert!(result.to_string().contains("already running"), "{result}");
    let other = run_import(
        &pool,
        other_org,
        "USD",
        SourceKind::Csv,
        ImportMode::Commit,
        VecSource::new(vec![]),
    )
    .await;
    assert!(other.is_ok(), "{other:?}");
    guard.close().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn cancellation_rolls_back_rows_and_releases_the_import_session(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let one_connection = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let row = api_row(
        (1, 10, 100, 5000, 1000),
        "Acme",
        "Website",
        "Design",
        "dev@acme.com",
        (2026, 1, 15),
        "1",
        None,
    );
    let (started, ready) = tokio::sync::oneshot::channel();
    let (_release, wait) = tokio::sync::oneshot::channel();
    let source = PausedSource {
        before_pause: Some(row.clone()),
        rows: VecSource::new(vec![]),
        pause: Some((started, wait)),
    };
    let first_pool = one_connection.clone();
    let first = tokio::spawn(async move {
        run_import(
            &first_pool,
            org,
            "USD",
            SourceKind::HarvestApi,
            ImportMode::Commit,
            source,
        )
        .await
    });
    ready.await.unwrap();
    assert!(matches!(
        super::lock_import(&pool, org).await,
        Err(super::ApiImportError::Busy)
    ));
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    // The one-connection pool cannot lend a replacement before closing the
    // cancelled session, which also rolls back its already-applied first row.
    let retry = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        run_import(
            &one_connection,
            org,
            "USD",
            SourceKind::HarvestApi,
            ImportMode::Commit,
            VecSource::new(vec![row]),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(retry.summary.time_entries.created, 1);
    assert_eq!(count(&pool, "time_entries").await, 1);
    one_connection.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_source_releases_the_session_and_rolls_back_partial_rows(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let row = nk_row(
        "Acme",
        "Website",
        "Design",
        "dev@acme.com",
        (2026, 1, 15),
        "1",
        None,
    );
    let (started, _ready) = tokio::sync::oneshot::channel();
    let (release, wait) = tokio::sync::oneshot::channel();
    drop(release);
    let source = PausedSource {
        before_pause: Some(row),
        rows: VecSource::new(vec![]),
        pause: Some((started, wait)),
    };
    assert!(
        run_import(
            &pool,
            org,
            "USD",
            SourceKind::Csv,
            ImportMode::Commit,
            source
        )
        .await
        .is_err()
    );
    assert_eq!(count(&pool, "time_entries").await, 0);
    let guard = super::lock_import(&pool, org).await.unwrap();
    guard.close().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn api_connection_checks_work_with_a_single_connection_pool(pool: PgPool) {
    let org = seed_org(&pool).await;
    let one_connection = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        super::run_api_import(
            &one_connection,
            org,
            "USD",
            &disconnected_config(),
            ImportMode::Commit,
            super::SyncScope::Full,
        ),
    )
    .await
    .unwrap();
    assert!(
        matches!(result, Err(super::ApiImportError::NotConnected)),
        "{result:?}"
    );
    one_connection.close().await;
}

// ── Fixtures ──────────────────────────────────────────────────────────────────

async fn seed_org(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name, default_currency) VALUES ($1, 'Test Org', 'USD')",
        id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn seed_user(pool: &PgPool, org_id: Uuid, email: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Dana Dev')",
        id,
        org_id,
        email,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// A source row with the given Harvest ids and fields; other fields take sane
/// defaults so tests only set what they care about.
#[allow(clippy::too_many_arguments)]
fn api_row(
    hids: (i64, i64, i64, i64, i64),
    client: &str,
    project: &str,
    task: &str,
    email: &str,
    date: (i32, u32, u32),
    hours: &str,
    notes: Option<&str>,
) -> SourceRow {
    let (c, p, t, te, u) = hids;
    SourceRow {
        harvest_client_id: Some(c),
        harvest_project_id: Some(p),
        harvest_task_id: Some(t),
        harvest_time_entry_id: Some(te),
        harvest_user_id: Some(u),
        client_name: client.to_string(),
        client_address: None,
        client_active: true,
        project_name: project.to_string(),
        project_code: None,
        project_active: true,
        project_starts_on: None,
        project_ends_on: None,
        task_name: task.to_string(),
        task_billable_default: true,
        user_email: Some(email.to_string()),
        user_name: None,
        spent_date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
        hours: hours.to_string(),
        notes: notes.map(str::to_string),
        billable: true,
        invoiced: false,
        billable_rate: Some("150".to_string()),
        billable_amount: None,
        cost_rate: None,
        cost_amount: None,
        currency: Some("USD".to_string()),
        harvest_updated_at: Some(Utc.with_ymd_and_hms(2026, 1, 16, 0, 0, 0).unwrap()),
        source_location: format!("time_entry {te}"),
    }
}

/// A natural-key (CSV-style) row: no Harvest ids, so matching falls back to the
/// composite natural key and exercises `harvest_norm` (FIX #1).
fn nk_row(
    client: &str,
    project: &str,
    task: &str,
    email: &str,
    date: (i32, u32, u32),
    hours: &str,
    notes: Option<&str>,
) -> SourceRow {
    SourceRow {
        harvest_client_id: None,
        harvest_project_id: None,
        harvest_task_id: None,
        harvest_time_entry_id: None,
        harvest_user_id: None,
        client_name: client.to_string(),
        client_address: None,
        client_active: true,
        project_name: project.to_string(),
        project_code: None,
        project_active: true,
        project_starts_on: None,
        project_ends_on: None,
        task_name: task.to_string(),
        task_billable_default: true,
        user_email: Some(email.to_string()),
        user_name: None,
        spent_date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
        hours: hours.to_string(),
        notes: notes.map(str::to_string),
        billable: true,
        invoiced: false,
        billable_rate: None,
        billable_amount: None,
        cost_rate: None,
        cost_amount: None,
        currency: Some("USD".to_string()),
        harvest_updated_at: None,
        source_location: "row".to_string(),
    }
}

async fn commit_csv(pool: &PgPool, org: Uuid, rows: Vec<SourceRow>) -> super::report::ImportReport {
    run_import(
        pool,
        org,
        "USD",
        SourceKind::Csv,
        ImportMode::Commit,
        VecSource::new(rows),
    )
    .await
    .unwrap()
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    // `table` is a hard-coded literal from tests, never user input.
    let sql = format!("SELECT COUNT(*) FROM {table}");
    sqlx::query_scalar::<_, i64>(&sql)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn commit(pool: &PgPool, org: Uuid, rows: Vec<SourceRow>) -> super::report::ImportReport {
    run_import(
        pool,
        org,
        "USD",
        SourceKind::HarvestApi,
        ImportMode::Commit,
        VecSource::new(rows),
    )
    .await
    .unwrap()
}

// ── US1: full API import, exact conversions, provenance ───────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn full_import_creates_all_levels_with_exact_minutes(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;

    let rows = vec![
        api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("kickoff"),
        ),
        api_row(
            (1, 10, 100, 5001, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 16),
            "0.25",
            None,
        ),
    ];
    let report = commit(&pool, org, rows).await;

    assert!(report.reconciles());
    assert_eq!(report.summary.clients.created, 1);
    assert_eq!(report.summary.projects.created, 1);
    assert_eq!(report.summary.tasks.created, 1);
    assert_eq!(report.summary.time_entries.created, 2);

    assert_eq!(count(&pool, "clients").await, 1);
    assert_eq!(count(&pool, "projects").await, 1);
    assert_eq!(count(&pool, "tasks").await, 1);
    assert_eq!(count(&pool, "time_entries").await, 2);

    // Exact minutes: 1.5h → 90, 0.25h → 15.
    let minutes: Vec<i32> =
        sqlx::query_scalar!("SELECT minutes FROM time_entries ORDER BY spent_date")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(minutes, vec![90, 15]);

    // A provenance row per created record: 1 client + 1 project + 1 task + 2 entries.
    assert_eq!(count(&pool, "harvest_import_map").await, 5);

    // project_tasks link created (FR-009).
    assert_eq!(count(&pool, "project_tasks").await, 1);
}

// ── FIX #1: natural-key re-import with whitespace/case variance is duplicate-free

#[sqlx::test(migrations = "./migrations")]
async fn natural_key_reimport_normalizes_whitespace_and_ascii_case(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;

    // First import with clean, non-ASCII names via the natural-key (CSV) path.
    let first = commit_csv(
        &pool,
        org,
        vec![nk_row(
            "Café",
            "Wébsite",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("note"),
        )],
    )
    .await;
    assert_eq!(first.summary.clients.created, 1);
    assert_eq!(first.summary.projects.created, 1);
    assert_eq!(first.summary.tasks.created, 1);
    assert_eq!(first.summary.time_entries.created, 1);

    // Re-import the SAME records with a leading tab + NBSP, flipped ASCII case, and
    // the non-ASCII letters unchanged. Rust `normalize` and SQL `harvest_norm` must
    // agree, so every record is matched (Skipped) — zero creations, no duplicates.
    let second = commit_csv(
        &pool,
        org,
        vec![nk_row(
            "\t\u{a0}CAFé",
            "  wébSITE ",
            " design\t",
            "DEV@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("note"),
        )],
    )
    .await;
    assert_eq!(second.summary.clients.created, 0, "client duplicated");
    assert_eq!(second.summary.projects.created, 0, "project duplicated");
    assert_eq!(second.summary.tasks.created, 0, "task duplicated");
    assert_eq!(
        second.summary.time_entries.created, 0,
        "time entry duplicated"
    );
    assert_eq!(second.summary.clients.skipped, 1);
    assert_eq!(second.summary.time_entries.skipped, 1);

    // No duplicate rows exist.
    assert_eq!(count(&pool, "clients").await, 1);
    assert_eq!(count(&pool, "projects").await, 1);
    assert_eq!(count(&pool, "tasks").await, 1);
    assert_eq!(count(&pool, "time_entries").await, 1);
}

// ── US2: idempotent + edit-robust re-sync ─────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn second_identical_run_creates_nothing(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let rows = || {
        vec![api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("kickoff"),
        )]
    };

    commit(&pool, org, rows()).await;
    let second = commit(&pool, org, rows()).await;

    assert_eq!(second.summary.clients.created, 0);
    assert_eq!(second.summary.projects.created, 0);
    assert_eq!(second.summary.tasks.created, 0);
    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 1);
    // No duplicates.
    assert_eq!(count(&pool, "time_entries").await, 1);
    assert_eq!(count(&pool, "clients").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn edited_entry_still_matched_by_provenance(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    commit(
        &pool,
        org,
        vec![api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("kickoff"),
        )],
    )
    .await;

    // Edit the imported entry's notes in Horae (diverging the natural key).
    sqlx::query!("UPDATE time_entries SET notes = 'edited in horae'")
        .execute(&pool)
        .await
        .unwrap();

    // Re-import the same Harvest entry: matched by Harvest id, not duplicated.
    let second = commit(
        &pool,
        org,
        vec![api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("kickoff"),
        )],
    )
    .await;
    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 1);
    assert_eq!(count(&pool, "time_entries").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn watermark_advances_only_on_commit(pool: PgPool) {
    let org = seed_org(&pool).await;
    // Seed a credentials row so watermark advance has somewhere to write.
    super::credentials::store(
        &pool,
        org,
        &"11".repeat(32),
        "acct-1",
        "access",
        "refresh",
        None,
        None,
    )
    .await
    .unwrap();

    let mark = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
    super::credentials::advance_watermark(&pool, org, &[(EntityType::TimeEntry, mark)])
        .await
        .unwrap();
    let conn = super::credentials::load(&pool, org, &"11".repeat(32))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(conn.watermark_for(EntityType::TimeEntry), Some(mark));
}

// ── US3: dry-run writes nothing and previews the commit ───────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn dry_run_writes_nothing_and_matches_commit(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let rows = || {
        vec![
            api_row(
                (1, 10, 100, 5000, 1000),
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 15),
                "1.5",
                Some("kickoff"),
            ),
            api_row(
                (1, 10, 100, 5001, 1000),
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 16),
                "2",
                None,
            ),
        ]
    };

    let dry = run_import(
        &pool,
        org,
        "USD",
        SourceKind::HarvestApi,
        ImportMode::DryRun,
        VecSource::new(rows()),
    )
    .await
    .unwrap();

    // Nothing persisted — no data, no provenance, no watermark.
    assert_eq!(count(&pool, "clients").await, 0);
    assert_eq!(count(&pool, "time_entries").await, 0);
    assert_eq!(count(&pool, "harvest_import_map").await, 0);

    let real = commit(&pool, org, rows()).await;
    // The dry-run preview equals the real run's outcome.
    assert_eq!(dry.summary, real.summary);
    assert_eq!(real.summary.time_entries.created, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

// ── US4: resilience + reconciliation ──────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn bad_records_are_reported_and_run_continues(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;

    let rows = vec![
        // Good.
        api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "1.5",
            Some("ok"),
        ),
        // Unknown user → errored (FR-010).
        api_row(
            (1, 10, 100, 5001, 2000),
            "Acme",
            "Website",
            "Design",
            "ghost@acme.com",
            (2026, 1, 16),
            "1",
            None,
        ),
        // Unparseable hours → errored (FR-005).
        api_row(
            (1, 10, 100, 5002, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 17),
            "not-a-number",
            None,
        ),
    ];
    let report = commit(&pool, org, rows).await;

    // Valid record imported; two errored; totals reconcile.
    assert!(report.reconciles());
    assert_eq!(report.summary.time_entries.created, 1);
    assert_eq!(report.summary.time_entries.errored, 2);
    assert_eq!(report.row_errors.len(), 2);
    assert_eq!(count(&pool, "time_entries").await, 1);
    // No partial fragment: the errored rows left no dangling entry.
    let entries = count(&pool, "time_entries").await;
    assert_eq!(entries, 1);
}

// ── US5: CSV natural-key path ─────────────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn csv_name_only_matches_one_org_user_and_reimports_without_duplicates(pool: PgPool) {
    let org = seed_org(&pool).await;
    let user = seed_user(&pool, org, "dev@acme.com").await;
    let other_org = seed_org(&pool).await;
    seed_user(&pool, other_org, "other@example.com").await;
    let csv = b"Date,Client,Project,Task,Hours,First Name,Last Name\n2026-01-15,Acme,Website,Design,1, dana , DEV \n";
    let first = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(first.summary.time_entries.created, 1, "{first:?}");
    let assigned = sqlx::query_scalar!("SELECT user_id FROM time_entries WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(assigned, user);
    let second = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(second.summary.time_entries.skipped, 1);
    assert_eq!(count(&pool, "users").await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn import_rejects_ambiguous_normalized_email_without_partial_writes(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    seed_user(&pool, org, "DEV@ACME.COM").await;
    let report =
        super::csv_source::import_csv(&pool, org, "USD", CSV.as_bytes(), ImportMode::Commit)
            .await
            .unwrap();
    assert_eq!(report.summary.time_entries.errored, 2);
    assert!(
        report
            .row_errors
            .iter()
            .all(|error| error.reason.contains("ambiguous user email"))
    );
    assert_eq!(count(&pool, "time_entries").await, 0);
    assert_eq!(count(&pool, "clients").await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_name_fallback_requires_a_full_unique_name_and_never_overrides_email(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv = b"Date,Client,Project,Task,Hours,Email,First Name,Last Name\n\
2026-01-15,Wrong email,P,T,1,missing@example.com,Dana,Dev\n\
2026-01-15,Partial name,P,T,1,,Dana,\n\
2026-01-15,Unknown name,P,T,1,,Unknown,User\n\
2026-01-15,Missing identity,P,T,1,,,\n\
2026-01-15,Valid,P,T,1, ,Dana,Dev\n";
    let report = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert!(report.reconciles());
    assert_eq!(report.summary.time_entries.errored, 4);
    assert_eq!(report.summary.time_entries.created, 1);
    assert!(
        report.row_errors[0]
            .reason
            .contains("no Horae user matches email")
    );
    assert!(
        report.row_errors[1]
            .reason
            .contains("no Horae user matches name")
    );
    assert!(
        report.row_errors[3]
            .reason
            .contains("no user email or full name")
    );
    assert_eq!(count(&pool, "clients").await, 1);
    assert_eq!(count(&pool, "projects").await, 1);
    assert_eq!(count(&pool, "tasks").await, 1);
    assert_eq!(count(&pool, "users").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_ambiguous_names_include_inactive_users_but_unique_email_disambiguates(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let inactive = seed_user(&pool, org, "old@example.com").await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", inactive)
        .execute(&pool)
        .await
        .unwrap();
    let csv = b"Date,Client,Project,Task,Hours,Email,First Name,Last Name\n\
2026-01-15,Ambiguous,P,T,1,,Dana,Dev\n\
2026-01-15,Historical,P,T,1,old@example.com,Dana,Dev\n";
    let report = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(report.summary.time_entries.errored, 1);
    assert!(report.row_errors[0].reason.contains("ambiguous user name"));
    assert_eq!(report.summary.time_entries.created, 1);
    let assigned = sqlx::query_scalar!("SELECT user_id FROM time_entries WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(assigned, inactive);
    assert_eq!(count(&pool, "clients").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_name_only_dry_run_and_duplicate_occurrences_preserve_history(pool: PgPool) {
    let org = seed_org(&pool).await;
    let user = seed_user(&pool, org, "dev@acme.com").await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", user)
        .execute(&pool)
        .await
        .unwrap();
    let csv = b"Date,Client,Project,Task,Hours,First Name,Last Name\n\
2026-01-15,A,P,T,1,Dana,Dev\n\
2026-01-15,A,P,T,1,Dana,Dev\n";
    let preview = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::DryRun)
        .await
        .unwrap();
    assert_eq!(preview.summary.time_entries.created, 2);
    assert_eq!(count(&pool, "time_entries").await, 0);
    assert_eq!(count(&pool, "clients").await, 0);
    let committed = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(preview.summary, committed.summary);
    let repeated = super::csv_source::import_csv(&pool, org, "USD", csv, ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(repeated.summary.time_entries.skipped, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

const CSV: &str = "Date,Client,Project,Project Code,Task,Notes,Hours,Billable?,Invoiced?,Email,Currency\n\
2026-01-15,Acme,Website,WEB,Design,kickoff,1.5,Yes,No,dev@acme.com,USD\n\
2026-01-16,Acme,Website,WEB,Design,,0.25,No,No,dev@acme.com,USD\n";

#[sqlx::test(migrations = "./migrations")]
async fn csv_import_populates_and_is_idempotent(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;

    let first =
        super::csv_source::import_csv(&pool, org, "USD", CSV.as_bytes(), ImportMode::Commit)
            .await
            .unwrap();
    assert!(first.reconciles());
    assert_eq!(first.summary.time_entries.created, 2);
    assert_eq!(count(&pool, "clients").await, 1);
    assert_eq!(count(&pool, "time_entries").await, 2);
    // CSV carries no Harvest ids, so no provenance rows are written.
    assert_eq!(count(&pool, "harvest_import_map").await, 0);

    // Re-import the same file: zero duplicates via the composite natural key.
    let second =
        super::csv_source::import_csv(&pool, org, "USD", CSV.as_bytes(), ImportMode::Commit)
            .await
            .unwrap();
    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

// ── Distinct-but-identical entries are never collapsed ────────────────────────

/// Two identical rows in one Detailed export are two real entries (e.g. two
/// 30-minute calls on the same day) — both must be imported.
#[sqlx::test(migrations = "./migrations")]
async fn two_identical_csv_rows_import_as_two_entries(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let row = || {
        nk_row(
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "0.5",
            Some("client call"),
        )
    };

    let report = commit_csv(&pool, org, vec![row(), row()]).await;

    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

/// Re-importing the same file with repeated identical rows stays idempotent:
/// each occurrence matches its own stored entry, leaving exactly two.
#[sqlx::test(migrations = "./migrations")]
async fn reimporting_identical_csv_rows_is_idempotent(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let rows = || {
        vec![
            nk_row(
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 15),
                "0.5",
                Some("client call"),
            ),
            nk_row(
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 15),
                "0.5",
                Some("client call"),
            ),
        ]
    };

    commit_csv(&pool, org, rows()).await;
    let second = commit_csv(&pool, org, rows()).await;

    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

/// Two Harvest entries with different ids but identical fields are distinct
/// records: each gets its own Horae entry and its own provenance mapping, and an
/// incremental re-sync matches each by its id.
#[sqlx::test(migrations = "./migrations")]
async fn distinct_harvest_ids_with_identical_fields_import_as_two(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let rows = || {
        vec![
            api_row(
                (1, 10, 100, 5000, 1000),
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 15),
                "0.5",
                Some("client call"),
            ),
            api_row(
                (1, 10, 100, 5001, 1000),
                "Acme",
                "Website",
                "Design",
                "dev@acme.com",
                (2026, 1, 15),
                "0.5",
                Some("client call"),
            ),
        ]
    };

    let first = commit(&pool, org, rows()).await;
    assert_eq!(first.summary.time_entries.created, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);

    // Each Harvest id maps to its own Horae entry — never two ids on one row.
    let distinct = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT horae_id) FROM harvest_import_map
         WHERE harvest_entity_type = 'time_entry'::harvest_entity_type"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(distinct, 2);

    // An incremental re-sync matches both entries by id.
    let second = commit(&pool, org, rows()).await;
    assert_eq!(second.summary.time_entries.created, 0);
    assert_eq!(second.summary.time_entries.skipped, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
}

/// A first API sync after a CSV import must adopt the id-less entries the CSV
/// created — one Harvest id per stored entry — instead of duplicating them.
#[sqlx::test(migrations = "./migrations")]
async fn api_sync_adopts_entries_from_a_prior_csv_import(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv_row = || {
        nk_row(
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "0.5",
            Some("client call"),
        )
    };
    commit_csv(&pool, org, vec![csv_row(), csv_row()]).await;
    assert_eq!(count(&pool, "time_entries").await, 2);

    // The same two entries arrive from the API, now carrying their Harvest ids.
    let api_rows = vec![
        api_row(
            (1, 10, 100, 5000, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "0.5",
            Some("client call"),
        ),
        api_row(
            (1, 10, 100, 5001, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, 15),
            "0.5",
            Some("client call"),
        ),
    ];
    let sync = commit(&pool, org, api_rows).await;

    assert_eq!(sync.summary.time_entries.created, 0);
    assert_eq!(sync.summary.time_entries.skipped, 2);
    assert_eq!(count(&pool, "time_entries").await, 2);
    let distinct = sqlx::query_scalar!(
        "SELECT COUNT(DISTINCT horae_id) FROM harvest_import_map
         WHERE harvest_entity_type = 'time_entry'::harvest_entity_type"
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(distinct, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_malformed_file_is_rejected_with_no_writes(pool: PgPool) {
    let org = seed_org(&pool).await;
    let result =
        super::csv_source::import_csv(&pool, org, "USD", b"Foo,Bar\n1,2\n", ImportMode::Commit)
            .await;
    assert!(result.is_err());
    assert_eq!(count(&pool, "clients").await, 0);
    assert_eq!(count(&pool, "time_entries").await, 0);
}

// ── Polish: scale (SC-006) and reconciliation (SC-003/SC-007) ─────────────────

#[sqlx::test(migrations = "./migrations")]
async fn large_run_reuses_parents_and_reconciles_totals(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;

    // Many entries sharing one client/project/task: the in-run cache must create
    // each parent once (SC-006), and the stored minutes must sum to the source
    // total with zero drift (SC-003/SC-007).
    let n = 2_000i64;
    let mut rows = Vec::with_capacity(n as usize);
    let mut expected_minutes = 0i64;
    for i in 0..n {
        let day = (i % 27 + 1) as u32;
        // Alternate durations so several distinct natural keys exist per day.
        let hours = if i % 2 == 0 { "1.5" } else { "0.25" };
        expected_minutes += if i % 2 == 0 { 90 } else { 15 };
        rows.push(api_row(
            (1, 10, 100, 6000 + i, 1000),
            "Acme",
            "Website",
            "Design",
            "dev@acme.com",
            (2026, 1, day),
            hours,
            Some(&format!("entry {i}")),
        ));
    }

    let report = commit(&pool, org, rows).await;
    assert!(report.reconciles());

    // Parents created exactly once despite thousands of references.
    assert_eq!(report.summary.clients.created, 1);
    assert_eq!(report.summary.projects.created, 1);
    assert_eq!(report.summary.tasks.created, 1);
    assert_eq!(count(&pool, "clients").await, 1);
    assert_eq!(count(&pool, "projects").await, 1);
    assert_eq!(count(&pool, "tasks").await, 1);
    assert_eq!(report.summary.time_entries.created as i64, n);
    assert_eq!(count(&pool, "time_entries").await, n);

    // Zero-drift reconciliation: summed minutes equal the source total.
    let total: i64 =
        sqlx::query_scalar!("SELECT COALESCE(SUM(minutes), 0)::bigint FROM time_entries")
            .fetch_one(&pool)
            .await
            .unwrap()
            .unwrap_or(0);
    assert_eq!(total, expected_minutes);
}
