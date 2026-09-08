//! Count executed SQL statements around the real row pipeline, without global
//! logging or timing assertions. Each future owns its tracing subscriber.

use std::sync::{Arc, Mutex};

use tracing::instrument::WithSubscriber;
use tracing_subscriber::prelude::*;

use super::*;

#[derive(Debug, Default, Clone)]
struct QueryCounts {
    total: usize,
    users: usize,
    links: usize,
    releases: usize,
    rollbacks: usize,
}

#[derive(Clone, Default)]
struct CountQueries(Arc<Mutex<QueryCounts>>);

impl CountQueries {
    fn dispatches(&self) -> (tracing::Dispatch, tracing::Dispatch) {
        // With only one live dispatcher, tracing may register a new callsite
        // against another thread's empty default and cache Interest::never.
        // Retain an uninstalled registry beside the scoped collector so cold
        // callsites consider both dispatchers. Neither changes global logging.
        let registration = tracing::Dispatch::new(tracing_subscriber::registry());
        let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(self.clone()));
        (registration, collector)
    }
}

#[derive(Default)]
struct Statement {
    sql: String,
    summary: String,
}

impl tracing::field::Visit for Statement {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "db.statement" => self.sql = value.trim().into(),
            "summary" => self.summary = value.into(),
            _ => {}
        }
    }

    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for CountQueries {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        if event.metadata().target() != "sqlx::query" {
            return;
        }
        let mut statement = Statement::default();
        event.record(&mut statement);
        let sql = if statement.sql.is_empty() {
            &statement.summary
        } else {
            &statement.sql
        };
        let mut counts = self.0.lock().unwrap();
        counts.total += 1;
        counts.users += usize::from(sql.starts_with("SELECT id FROM users"));
        counts.links += usize::from(sql.starts_with("INSERT INTO project_tasks"));
        counts.releases += usize::from(sql.starts_with("RELEASE SAVEPOINT"));
        counts.rollbacks += usize::from(sql.starts_with("ROLLBACK TO SAVEPOINT"));
    }
}

async fn measured_import(
    pool: &PgPool,
    org: Uuid,
    source: SourceKind,
    mode: ImportMode,
    rows: Vec<SourceRow>,
) -> (super::super::report::ImportReport, QueryCounts) {
    let queries = CountQueries::default();
    let (_registration, subscriber) = queries.dispatches();
    let report = run_import(pool, org, "USD", source, mode, VecSource::new(rows))
        .with_subscriber(subscriber)
        .await
        .unwrap();
    let counts = queries.0.lock().unwrap().clone();
    (report, counts)
}

#[test]
fn cold_query_callsite_on_untraced_thread_is_still_captured() {
    fn query_event() {
        tracing::debug!(target: "sqlx::query", summary = "SELECT id FROM users", db.statement = "SELECT id FROM users");
    }
    let queries = CountQueries::default();
    let (_registration, subscriber) = queries.dispatches();
    std::thread::spawn(query_event).join().unwrap();
    tracing::dispatcher::with_default(&subscriber, query_event);
    assert_eq!(queries.0.lock().unwrap().users, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn repeated_rows_resolve_each_user_and_project_task_once(pool: PgPool) {
    let mut measurements = Vec::new();
    for (source, name_only) in [
        (SourceKind::Csv, false),
        (SourceKind::Csv, true),
        (SourceKind::HarvestApi, false),
    ] {
        let org = seed_org(&pool).await;
        let email = format!("dev-{org}@example.com");
        seed_user(&pool, org, &email).await;
        let rows: Vec<_> = (0..100)
            .map(|i| {
                let mut row = if source == SourceKind::Csv {
                    nk_row("A", "P", "T", &email, (2026, 1, 15), "1", None)
                } else {
                    api_row(
                        (1, 2, 3, 1000 + i, 4),
                        "A",
                        "P",
                        "T",
                        &email,
                        (2026, 1, 15),
                        "1",
                        None,
                    )
                };
                row.notes = Some(format!("entry {i}"));
                if name_only {
                    row.user_email = None;
                    row.user_name = Some(if i % 2 == 0 { " Dana DEV " } else { "dana dev" }.into());
                } else if i % 2 == 0 {
                    row.user_email = Some(format!(" {} ", email.to_uppercase()));
                }
                row
            })
            .collect();
        for (mode, expected_created) in [
            (ImportMode::DryRun, 100),
            (ImportMode::Commit, 100),
            (ImportMode::Commit, 0),
        ] {
            let (report, counts) = measured_import(&pool, org, source, mode, rows.clone()).await;
            assert_eq!(report.summary.time_entries.created, expected_created);
            assert_eq!(report.error_count(), 0);
            eprintln!(
                "{source:?} name_only={name_only} {mode:?} created={expected_created}: {counts:?}"
            );
            measurements.push(counts);
        }
    }
    for counts in measurements {
        assert_eq!((counts.users, counts.links), (1, 1), "{counts:?}");
        assert_eq!((counts.releases, counts.rollbacks), (100, 0));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn rolled_back_row_does_not_cache_user_or_project_task(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@example.com").await;
    let row = nk_row("A", "P", "T", "dev@example.com", (2026, 1, 15), "1", None);
    commit_csv(&pool, org, vec![row.clone()]).await;
    // Keep stable parent IDs, but make the next row create a link that must be
    // rolled back on its invalid duration. The following row must recreate it.
    sqlx::query!("DELETE FROM project_tasks")
        .execute(&pool)
        .await
        .unwrap();
    let mut invalid = row.clone();
    invalid.hours = "invalid".into();
    let (report, counts) = measured_import(
        &pool,
        org,
        SourceKind::Csv,
        ImportMode::Commit,
        vec![invalid, row.clone(), row],
    )
    .await;
    assert_eq!(report.error_count(), 1);
    assert_eq!(report.summary.time_entries.skipped, 1);
    assert_eq!(report.summary.time_entries.created, 1);
    assert_eq!((counts.users, counts.links), (2, 2), "{counts:?}");
    assert_eq!((counts.releases, counts.rollbacks), (2, 1));
    assert_eq!(count(&pool, "project_tasks").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn cached_project_task_still_validates_each_rows_rate(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@example.com").await;
    let row = nk_row("A", "P", "T", "dev@example.com", (2026, 1, 15), "1", None);
    let mut invalid = row.clone();
    invalid.billable_rate = Some("not-money".into());
    let (report, counts) = measured_import(
        &pool,
        org,
        SourceKind::Csv,
        ImportMode::Commit,
        vec![row.clone(), invalid, row],
    )
    .await;
    assert_eq!(report.error_count(), 1);
    assert_eq!(report.summary.tasks.errored, 1);
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!((counts.users, counts.links), (1, 1));
    assert_eq!((counts.releases, counts.rollbacks), (2, 1));
}

#[sqlx::test(migrations = "./migrations")]
async fn cached_email_and_full_name_do_not_share_a_namespace(pool: PgPool) {
    let org = seed_org(&pool).await;
    let email_user = seed_user(&pool, org, "dev@example.com").await;
    let name_user = seed_user(&pool, org, "other@example.com").await;
    sqlx::query!(
        "UPDATE users SET name = $1 WHERE id = $2",
        "dev@example.com",
        name_user
    )
    .execute(&pool)
    .await
    .unwrap();
    let email = nk_row("A", "P", "T", "dev@example.com", (2026, 1, 15), "1", None);
    let mut name = email.clone();
    name.user_email = None;
    name.user_name = Some("DEV@example.com".into());
    let (report, counts) = measured_import(
        &pool,
        org,
        SourceKind::Csv,
        ImportMode::Commit,
        vec![email, name],
    )
    .await;
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!((counts.users, counts.links), (2, 1));
    let assigned = sqlx::query_scalar!("SELECT user_id FROM time_entries WHERE org_id = $1", org)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(assigned.contains(&email_user));
    assert!(assigned.contains(&name_user));
}

#[sqlx::test(migrations = "./migrations")]
async fn new_run_rechecks_previously_cached_user_for_ambiguity(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@example.com").await;
    let row = nk_row("A", "P", "T", "dev@example.com", (2026, 1, 15), "1", None);
    commit_csv(&pool, org, vec![row.clone()]).await;
    seed_user(&pool, org, "DEV@EXAMPLE.COM").await;
    let (report, counts) = measured_import(
        &pool,
        org,
        SourceKind::Csv,
        ImportMode::Commit,
        vec![row.clone(), row],
    )
    .await;
    assert_eq!(report.summary.time_entries.errored, 2);
    assert!(
        report
            .row_errors
            .iter()
            .all(|e| e.reason.contains("ambiguous user email"))
    );
    assert_eq!(counts.users, 2);
    assert_eq!(count(&pool, "time_entries").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn project_task_cache_distinguishes_both_sides_of_the_link(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@example.com").await;
    let rows = [("P1", "T1"), ("P1", "T2"), ("P2", "T1"), ("P1", "T1")]
        .into_iter()
        .map(|(project, task)| {
            nk_row(
                "A",
                project,
                task,
                "dev@example.com",
                (2026, 1, 15),
                "1",
                None,
            )
        })
        .collect();
    let (report, counts) =
        measured_import(&pool, org, SourceKind::Csv, ImportMode::Commit, rows).await;
    assert_eq!(report.summary.time_entries.created, 4);
    assert_eq!((counts.users, counts.links), (1, 3));
    assert_eq!(count(&pool, "project_tasks").await, 3);
}

#[sqlx::test(migrations = false)]
async fn lookup_index_migration_preserves_existing_ambiguous_identities(pool: PgPool) {
    let mut before = sqlx::migrate!("./migrations");
    before.migrations = before
        .iter()
        .filter(|m| m.version < 22)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    before.run(&pool).await.unwrap();
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@example.com").await;
    seed_user(&pool, org, "DEV@EXAMPLE.COM").await;
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let indexes = sqlx::query!(
        "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = 'public' AND tablename = 'users' AND indexname IN ('users_import_email_idx', 'users_import_name_idx') ORDER BY indexname"
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(indexes.len(), 2);
    assert!(
        indexes
            .iter()
            .all(|index| !index.indexdef.as_deref().unwrap().contains("UNIQUE"))
    );
    assert!(
        indexes[0]
            .indexdef
            .as_deref()
            .unwrap()
            .contains("(org_id, harvest_norm(email))")
    );
    assert!(
        indexes[1]
            .indexdef
            .as_deref()
            .unwrap()
            .contains("(org_id, harvest_norm(name))")
    );
    let email = nk_row("A", "P", "T", "dev@example.com", (2026, 1, 15), "1", None);
    let mut name = email.clone();
    name.user_email = None;
    name.user_name = Some("Dana Dev".into());
    let report = commit_csv(&pool, org, vec![email, name]).await;
    assert_eq!(report.summary.time_entries.errored, 2);
    assert_eq!(count(&pool, "users").await, 2);
    assert_eq!(count(&pool, "time_entries").await, 0);
}
