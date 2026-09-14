//! Exercise the actual HTTP-body/parser/SQL bridge, including incomplete uploads.

use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, Bytes};
use tokio::sync::{Notify, mpsc};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::prelude::*;

use super::super::csv_source::import_body;
use super::*;

const HEADER: &str = "Date,Client,Project,Task,Hours,Email,Notes\n";
const ROW: &str = "2026-01-15,Acme,Website,Design,1,dev@acme.com,kickoff\n";

#[derive(Clone, Default)]
struct Applied(Arc<Notify>, bool);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Applied {
    fn on_event(&self, event: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        struct Statement<'a>(bool, &'a str);
        impl tracing::field::Visit for Statement<'_> {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if matches!(field.name(), "summary" | "db.statement")
                    && value.trim().starts_with(self.1)
                {
                    self.0 = true;
                }
            }
            fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
        }
        let mut statement = Statement(
            false,
            if self.1 {
                "COMMIT"
            } else {
                "RELEASE SAVEPOINT"
            },
        );
        event.record(&mut statement);
        if event.metadata().target() == "sqlx::query" && statement.0 {
            self.0.notify_one();
        }
    }
}

fn upload_channel() -> (mpsc::Sender<std::io::Result<Bytes>>, Body) {
    let (send, receive) = mpsc::channel(1);
    let stream = futures_util::stream::unfold(receive, |mut receive| async {
        receive.recv().await.map(|bytes| (bytes, receive))
    });
    (send, Body::from_stream(stream))
}

async fn entry_count(pool: &PgPool, org: Uuid) -> i64 {
    sqlx::query_scalar!("SELECT count(*) FROM time_entries WHERE org_id = $1", org)
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_resumes_committed_batches_without_recounting_duplicate_rows(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Crash, ImportMode::Commit).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_cancel_preserves_its_completed_batches_for_manual_retry(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Cancel, ImportMode::Commit).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_reclaimed_lease_cannot_commit_the_next_batch(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Reclaim, ImportMode::Commit).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_preview_resumes_after_a_crash_without_recounting_rows(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Crash, ImportMode::DryRun).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_preview_cancel_preserves_simulation_for_manual_retry(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Cancel, ImportMode::DryRun).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_preview_rejects_a_stale_next_batch(pool: PgPool) {
    interrupted_csv_batches(pool, BatchInterruption::Reclaim, ImportMode::DryRun).await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_preview_matches_existing_and_new_rows_with_one_connection(pool: PgPool) {
    use super::super::csv_source::import_body_with_lease;
    use crate::jobs;

    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    import_body(
        &pool,
        org,
        "USD",
        Body::from(format!("{HEADER}{}", ROW.repeat(600))),
        ImportMode::Commit,
    )
    .await
    .unwrap();
    let new_row = ROW.replace("Acme,Website,Design", "New client,New project,New task");
    let csv = format!(
        "{HEADER}{}{}{}bad-date,Acme,Website,Design,1,dev@acme.com,bad\n",
        ROW.repeat(450),
        new_row.repeat(650),
        ROW.repeat(551)
    );
    let expected = import_body(
        &pool,
        org,
        "USD",
        Body::from(csv.clone()),
        ImportMode::DryRun,
    )
    .await
    .unwrap();
    assert_eq!(expected.summary.time_entries.skipped, 600);
    assert_eq!(expected.summary.time_entries.created, 1051);
    assert_eq!(expected.summary.time_entries.errored, 1);
    let id = jobs::enqueue_csv(
        &pool,
        org,
        ImportMode::DryRun,
        csv.clone().into_bytes(),
        "preview-existing",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, _stop) = jobs::claim_lease_for_test(&pool).await;
    let single = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let actual = tokio::time::timeout(
        Duration::from_secs(15),
        import_body_with_lease(
            &single,
            org,
            "USD",
            Body::from(csv.clone()),
            ImportMode::DryRun,
            Some(&lease),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(entry_count(&pool, org).await, 600);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
    let committed = import_body(&pool, org, "USD", Body::from(csv), ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(committed.summary, expected.summary);
    assert_eq!(entry_count(&pool, org).await, 1651);
    single.close().await;
}

#[derive(Clone, Copy)]
enum BatchInterruption {
    Crash,
    Cancel,
    Reclaim,
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_preview_crosses_batch_boundaries_without_committing_domain_data(pool: PgPool) {
    use super::super::csv_source::import_body_with_lease;
    use crate::jobs;

    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv = format!("{HEADER}{}", ROW.repeat(501));
    let id = jobs::enqueue_csv(
        &pool,
        org,
        ImportMode::DryRun,
        csv.clone().into_bytes(),
        "preview-batches",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    jobs::run_claimed(&pool, &lease, stop, async {
        let report = import_body_with_lease(
            &pool,
            org,
            "USD",
            Body::from(csv),
            ImportMode::DryRun,
            Some(&lease),
        )
        .await?;
        assert_eq!(report.summary.time_entries.created, 501);
        assert_eq!(entry_count(&pool, org).await, 0);
        assert_eq!(
            jobs::status(&pool, org, id).await?.unwrap().status,
            "succeeded"
        );
        super::super::job_report(&report)
    })
    .await
    .unwrap();
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
    assert_eq!(entry_count(&pool, org).await, 0);
}

async fn interrupted_csv_batches(pool: PgPool, interruption: BatchInterruption, mode: ImportMode) {
    use super::super::csv_source::import_body_with_lease;
    use crate::jobs;

    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let first_batch = format!(
        "{HEADER}{}bad-date,Acme,Website,Design,1,dev@acme.com,broken\n",
        ROW.repeat(499)
    );
    let csv = format!("{first_batch}{}", ROW.repeat(501));
    // A recovered parser must skip the checkpointed byte prefix, not parse it
    // again. Poison just that prefix while keeping its exact length and suffix.
    let resumed_csv = format!("{}{}", "x".repeat(first_batch.len()), ROW.repeat(501));
    let id = jobs::enqueue_csv(
        &pool,
        org,
        mode,
        csv.clone().into_bytes(),
        "resume-csv",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    let (send, body) = upload_channel();
    let committed = Applied(Arc::new(Notify::new()), true);
    let _registration = tracing::Dispatch::new(tracing_subscriber::registry());
    let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(committed.clone()));
    let run_pool = pool.clone();
    let run = tokio::spawn(
        async move {
            let work = async {
                let report =
                    import_body_with_lease(&run_pool, org, "USD", body, mode, Some(&lease)).await?;
                super::super::job_report(&report)
            };
            match interruption {
                BatchInterruption::Cancel => jobs::run_claimed(&run_pool, &lease, stop, work).await,
                _ => {
                    let _keep_stop = stop;
                    work.await.map(|_| ())
                }
            }
        }
        .with_subscriber(collector),
    );
    send.send(Ok(Bytes::from(first_batch))).await.unwrap();
    let batch = tokio::time::timeout(Duration::from_secs(10), committed.0.notified()).await;
    if batch.is_err() {
        run.abort();
    }
    assert!(
        batch.is_ok(),
        "a durable batch must commit before source EOF"
    );
    let retained = if mode == ImportMode::Commit { 499 } else { 0 };
    assert_eq!(entry_count(&pool, org).await, retained);
    if mode == ImportMode::DryRun {
        assert_eq!(
            sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
                .fetch_one(&pool)
                .await
                .unwrap(),
            Some(0),
            "checkpointing a preview must not publish simulated parents"
        );
    }
    let progress = jobs::status(&pool, org, id).await.unwrap().unwrap();
    assert_eq!(progress.status, "running");
    assert_eq!(progress.processed_count, 503);

    let (lease, _stop) = match interruption {
        BatchInterruption::Crash => {
            run.abort();
            assert!(run.await.unwrap_err().is_cancelled());
            tokio::time::timeout(Duration::from_secs(5), send.closed())
                .await
                .unwrap();
            sqlx::query!(
                "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
                id
            )
            .execute(&pool)
            .await
            .unwrap();
            jobs::claim_lease_for_test(&pool).await
        }
        BatchInterruption::Cancel => {
            assert!(jobs::cancel(&pool, org, id).await.unwrap());
            tokio::time::timeout(Duration::from_secs(10), run)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            tokio::time::timeout(Duration::from_secs(5), send.closed())
                .await
                .unwrap();
            assert_eq!(
                jobs::status(&pool, org, id).await.unwrap().unwrap().status,
                "cancelled"
            );
            assert!(jobs::retry(&pool, org, id).await.unwrap());
            jobs::claim_lease_for_test(&pool).await
        }
        BatchInterruption::Reclaim => {
            sqlx::query!(
                "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
                id
            )
            .execute(&pool)
            .await
            .unwrap();
            let next = jobs::claim_lease_for_test(&pool).await;
            send.send(Ok(Bytes::from(ROW.repeat(500)))).await.unwrap();
            drop(send);
            let error = tokio::time::timeout(Duration::from_secs(10), run)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err();
            assert!(error.to_string().contains("lease lost"), "{error}");
            next
        }
    };
    assert_eq!(
        entry_count(&pool, org).await,
        retained,
        "only the committed batch must survive"
    );
    assert_eq!(
        jobs::status(&pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .processed_count,
        503
    );
    let report = import_body_with_lease(
        &pool,
        org,
        "USD",
        Body::from(resumed_csv),
        mode,
        Some(&lease),
    )
    .await
    .unwrap();
    assert_eq!(
        entry_count(&pool, org).await,
        if mode == ImportMode::Commit { 1000 } else { 0 }
    );
    assert_eq!(report.summary.time_entries.created, 1000);
    assert_eq!(report.summary.time_entries.errored, 1);
    assert_eq!(report.row_errors.len(), 1);
    assert_eq!(report.row_errors[0].source_location, "CSV line 501");
    assert_eq!(report.summary.time_entries.skipped, 0);
    assert_eq!(report.summary.clients.created, 1);
    assert_eq!(report.summary.clients.skipped, 0);
    assert_eq!(
        jobs::status(&pool, org, id)
            .await
            .unwrap()
            .unwrap()
            .total_count,
        Some(1004)
    );
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_applies_before_eof_but_transport_failure_rolls_back_everything(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let (send, body) = upload_channel();
    let applied = Applied::default();
    let _registration = tracing::Dispatch::new(tracing_subscriber::registry());
    let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(applied.clone()));
    let run_pool = pool.clone();
    let run = tokio::spawn(
        async move { import_body(&run_pool, org, "USD", body, ImportMode::Commit).await }
            .with_subscriber(collector),
    );
    send.send(Ok(Bytes::from(format!("{HEADER}{ROW}"))))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), applied.0.notified())
        .await
        .expect("SQL must apply a row before the upload ends");
    assert_eq!(entry_count(&pool, org).await, 0, "uncommitted row leaked");
    send.send(Err(std::io::Error::other("upload connection lost")))
        .await
        .unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(
        error.to_string().contains("upload connection lost"),
        "{error}"
    );
    assert_eq!(entry_count(&pool, org).await, 0);
    let clients = sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(clients, Some(0));
    let retry = import_body(
        &pool,
        org,
        "USD",
        Body::from(format!("{HEADER}{ROW}")),
        ImportMode::Commit,
    )
    .await
    .unwrap();
    assert_eq!(retry.summary.time_entries.created, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_preview_errors_and_repeated_rows_match_commit_and_reimport(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv = format!("{HEADER}{ROW}bad-date,A,P,T,1,dev@acme.com,\n{ROW}");
    let preview = import_body(
        &pool,
        org,
        "USD",
        Body::from(csv.clone()),
        ImportMode::DryRun,
    )
    .await
    .unwrap();
    assert_eq!(entry_count(&pool, org).await, 0);
    let commit = import_body(
        &pool,
        org,
        "USD",
        Body::from(csv.clone()),
        ImportMode::Commit,
    )
    .await
    .unwrap();
    assert_eq!(preview.summary, commit.summary);
    assert_eq!(preview.row_errors, commit.row_errors);
    assert_eq!(commit.summary.time_entries.created, 2);
    assert_eq!(commit.row_errors.len(), 1);
    assert_eq!(commit.row_errors[0].source_location, "CSV line 3");
    let retry = import_body(&pool, org, "USD", Body::from(csv), ImportMode::Commit)
        .await
        .unwrap();
    assert_eq!(retry.summary.time_entries.created, 0);
    assert_eq!(retry.summary.time_entries.skipped, 2);
    assert_eq!(entry_count(&pool, org).await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn cancelling_a_csv_waiting_for_more_bytes_releases_its_session(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let (send, body) = upload_channel();
    let applied = Applied::default();
    let _registration = tracing::Dispatch::new(tracing_subscriber::registry());
    let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(applied.clone()));
    let single = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let run_pool = single.clone();
    let run = tokio::spawn(
        async move { import_body(&run_pool, org, "USD", body, ImportMode::Commit).await }
            .with_subscriber(collector),
    );
    send.send(Ok(Bytes::from(format!("{HEADER}{ROW}"))))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), applied.0.notified())
        .await
        .unwrap();
    let overlap = import_body(&pool, org, "USD", Body::empty(), ImportMode::Commit)
        .await
        .unwrap_err();
    assert!(overlap.to_string().contains("already running"), "{overlap}");
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    tokio::time::timeout(Duration::from_secs(5), send.closed())
        .await
        .expect("cancelled parser retained the upload body");
    let retry = tokio::time::timeout(
        Duration::from_secs(5),
        import_body(
            &single,
            org,
            "USD",
            Body::from(format!("{HEADER}{ROW}")),
            ImportMode::Commit,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(retry.summary.time_entries.created, 1);
    assert_eq!(entry_count(&pool, org).await, 1);
    single.close().await;
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_cancel_joins_parser_and_preserves_committed_rows(pool: PgPool) {
    use super::super::csv_source::import_body_with_lease;
    use crate::jobs;

    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    import_body(
        &pool,
        org,
        "USD",
        Body::from(format!("{HEADER}{ROW}")),
        ImportMode::Commit,
    )
    .await
    .unwrap();
    let csv = format!("{HEADER}{}", ROW.replace("2026-01-15", "2026-01-16"));
    let id = jobs::enqueue_csv(
        &pool,
        org,
        ImportMode::Commit,
        csv.clone().into_bytes(),
        "cancel-live",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    let (send, body) = upload_channel();
    let applied = Applied::default();
    let _registration = tracing::Dispatch::new(tracing_subscriber::registry());
    let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(applied.clone()));
    let run_pool = pool.clone();
    let run = tokio::spawn(
        async move {
            jobs::run_claimed(&run_pool, &lease, stop, async {
                let report = import_body_with_lease(
                    &run_pool,
                    org,
                    "USD",
                    body,
                    ImportMode::Commit,
                    Some(&lease),
                )
                .await?;
                super::super::job_report(&report)
            })
            .await
        }
        .with_subscriber(collector),
    );
    send.send(Ok(Bytes::from(csv.clone()))).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), applied.0.notified())
        .await
        .unwrap();
    assert!(jobs::cancel(&pool, org, id).await.unwrap());
    tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        send.send(Ok(Bytes::from_static(ROW.as_bytes())))
            .await
            .is_err()
    );
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "cancelled"
    );
    assert_eq!(entry_count(&pool, org).await, 1);

    assert!(jobs::retry(&pool, org, id).await.unwrap());
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    jobs::run_claimed(&pool, &lease, stop, async {
        let report = import_body_with_lease(
            &pool,
            org,
            "USD",
            Body::from(csv),
            ImportMode::Commit,
            Some(&lease),
        )
        .await?;
        super::super::job_report(&report)
    })
    .await
    .unwrap();
    assert_eq!(entry_count(&pool, org).await, 2);
    let completed = jobs::status(&pool, org, id).await.unwrap().unwrap();
    assert_eq!(completed.status, "succeeded");
    assert!(completed.report.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn durable_csv_stale_commit_is_fenced_without_a_heartbeat(pool: PgPool) {
    use super::super::csv_source::import_body_with_lease;
    use crate::jobs;

    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv = format!("{HEADER}{ROW}");
    let id = jobs::enqueue_csv(
        &pool,
        org,
        ImportMode::Commit,
        csv.clone().into_bytes(),
        "stale-csv",
        Default::default(),
    )
    .await
    .unwrap();
    let (old, _stop) = jobs::claim_lease_for_test(&pool).await;
    let (send, body) = upload_channel();
    let applied = Applied::default();
    let _registration = tracing::Dispatch::new(tracing_subscriber::registry());
    let collector = tracing::Dispatch::new(tracing_subscriber::registry().with(applied.clone()));
    let run_pool = pool.clone();
    // Deliberately omit the monitor: the transaction itself must reject a
    // stale commit even when no process has noticed the lost lease yet.
    let run = tokio::spawn(
        async move {
            import_body_with_lease(&run_pool, org, "USD", body, ImportMode::Commit, Some(&old))
                .await
        }
        .with_subscriber(collector),
    );
    send.send(Ok(Bytes::from(csv.clone()))).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), applied.0.notified())
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET lease_until = clock_timestamp() WHERE id = $1",
        id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (current, _stop_current) = jobs::claim_lease_for_test(&pool).await;
    drop(send);
    let error = tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("lease lost"), "{error}");
    assert_eq!(entry_count(&pool, org).await, 0);
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
    let pending = jobs::status(&pool, org, id).await.unwrap().unwrap();
    assert_eq!(pending.status, "running");
    assert!(pending.report.is_none());
    import_body_with_lease(
        &pool,
        org,
        "USD",
        Body::from(csv),
        ImportMode::Commit,
        Some(&current),
    )
    .await
    .unwrap();
    assert_eq!(entry_count(&pool, org).await, 1);
    assert_eq!(
        jobs::status(&pool, org, id).await.unwrap().unwrap().status,
        "succeeded"
    );
}

fn scale_body() -> Body {
    let start = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    Body::from_stream(futures_util::stream::iter((0..1_000).map(move |page| {
        use std::fmt::Write;
        let mut chunk = if page == 0 {
            HEADER.to_owned()
        } else {
            String::new()
        };
        for index in page * 100..(page + 1) * 100 {
            let date = start + chrono::Duration::days(index % 365);
            let date = if index % 1_000 == 0 {
                "bad-date".to_owned()
            } else {
                date.to_string()
            };
            writeln!(
                chunk,
                "{date},Acme,Website,Design,1,dev@acme.com,entry {index}"
            )
            .unwrap();
        }
        Ok::<_, std::io::Error>(Bytes::from(chunk))
    })))
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "100,000-row CSV measurement; run explicitly with --release --nocapture"]
async fn measure_csv_streaming_100k(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    for mode in [ImportMode::DryRun, ImportMode::Commit, ImportMode::Commit] {
        let before = entry_count(&pool, org).await;
        let started = std::time::Instant::now();
        let report = import_body(&pool, org, "USD", scale_body(), mode)
            .await
            .unwrap();
        let elapsed = started.elapsed();
        let memory = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        eprintln!(
            "CSV mode={mode:?} existing={before} elapsed={elapsed:?} {}",
            memory
                .lines()
                .filter(|line| line.starts_with("VmRSS:") || line.starts_with("VmHWM:"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert_eq!(report.summary.time_entries.processed(), 100_000);
        assert_eq!(report.summary.time_entries.errored, 100);
        assert!(report.reconciles());
        if before == 0 {
            assert_eq!(report.summary.time_entries.created, 99_900);
        } else {
            assert_eq!(report.summary.time_entries.skipped, 99_900);
        }
        assert_eq!(
            entry_count(&pool, org).await,
            if mode == ImportMode::DryRun {
                0
            } else {
                99_900
            }
        );
        let minutes = sqlx::query_scalar!(
            "SELECT sum(minutes)::bigint FROM time_entries WHERE org_id = $1",
            org
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            minutes,
            if mode == ImportMode::DryRun {
                None
            } else {
                Some(5_994_000)
            }
        );
        let mappings = sqlx::query_scalar!(
            "SELECT count(*) FROM harvest_import_map WHERE org_id = $1",
            org
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(mappings, Some(0));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn csv_sql_backpressure_stops_reading_later_input_rows(pool: PgPool) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let mut barrier = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE time_entries IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let read = Arc::new(AtomicUsize::new(0));
    let progress = Arc::new(Notify::new());
    let (observed, notice) = (read.clone(), progress.clone());
    let body = Body::from_stream(futures_util::stream::iter((0..100).map(move |index| {
        observed.fetch_add(1, Ordering::Release);
        notice.notify_one();
        Ok::<_, std::io::Error>(Bytes::from(if index == 0 {
            format!("{HEADER}{ROW}")
        } else {
            ROW.to_owned()
        }))
    })));
    let run_pool = pool.clone();
    let run =
        tokio::spawn(
            async move { import_body(&run_pool, org, "USD", body, ImportMode::Commit).await },
        );
    tokio::time::timeout(Duration::from_secs(5), async {
        while read.load(Ordering::Acquire) < 3 {
            progress.notified().await;
        }
    })
    .await
    .unwrap();
    let fourth = tokio::time::timeout(Duration::from_secs(1), async {
        while read.load(Ordering::Acquire) < 4 {
            progress.notified().await;
        }
    })
    .await;
    assert!(
        fourth.is_err(),
        "parser consumed input despite SQL backpressure"
    );
    // Cancellation must also wake the parser blocked on its full row queue.
    run.abort();
    assert!(run.await.unwrap_err().is_cancelled());
    barrier.rollback().await.unwrap();
    assert_eq!(entry_count(&pool, org).await, 0);
}
