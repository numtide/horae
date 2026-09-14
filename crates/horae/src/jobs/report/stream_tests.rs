use futures_util::StreamExt;
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, ImportReport, RowError, RowOutcome, SourceKind,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{config::JobPolicy, jobs};

struct ArchivedJob {
    org: Uuid,
    id: Uuid,
    lease: jobs::JobLease,
    _stop: tokio::sync::watch::Sender<bool>,
    report: ImportReport,
    expected: Vec<RowError>,
}

impl ArchivedJob {
    fn body(&self, pool: &PgPool) -> axum::body::Body {
        let mut tail = Vec::new();
        for error in &self.report.row_errors {
            serde_json::to_writer(&mut tail, error).unwrap();
            tail.push(b'\n');
        }
        super::download_body(
            pool.clone(),
            self.org,
            self.id,
            self.report.archived_error_chunks().try_into().unwrap(),
            tail,
        )
    }
}

async fn archived_job(pool: &PgPool) -> ArchivedJob {
    let org = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Jobs test')",
        org
    )
    .execute(pool)
    .await
    .unwrap();
    let id = jobs::enqueue(
        pool,
        org,
        &jobs::JobPayload::HarvestCsv {
            mode: ImportMode::Commit,
        },
        "stream-snapshot",
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(pool).await;
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::Commit);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 1".into(),
            reason: "x".repeat(super::CHUNK_BYTES * 33),
        },
    );
    let mut expected = report.row_errors.clone();
    let mut tx = pool.begin().await.unwrap();
    lease.archive_report(&mut tx, &mut report).await.unwrap();
    assert!(report.archived_error_chunks() > 32);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 2".into(),
            reason: "inline at the captured checkpoint".into(),
        },
    );
    expected.extend(report.row_errors.clone());
    let metadata = serde_json::to_value(&report).unwrap();
    lease
        .save_checkpoint(
            &mut tx,
            &serde_json::json!({ "version": 2, "report": metadata }),
            &metadata,
            "time_entries",
            2,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ArchivedJob {
        org,
        id,
        lease,
        _stop: stop,
        report,
        expected,
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn download_keeps_its_snapshot_when_later_progress_archives_the_inline_tail(pool: PgPool) {
    let mut job = archived_job(&pool).await;
    let captured_end = job.report.archived_error_chunks();
    let mut stream = job.body(&pool).into_data_stream();
    let mut bytes = stream.next().await.unwrap().unwrap().to_vec();

    job.report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV row 3".into(),
            reason: "later error ".repeat(1_000),
        },
    );
    let mut tx = pool.begin().await.unwrap();
    job.lease
        .archive_report(&mut tx, &mut job.report)
        .await
        .unwrap();
    assert!(job.report.row_errors.is_empty());
    assert!(job.report.archived_error_chunks() > captured_end);
    assert!(
        job.lease
            .complete(&mut tx, &serde_json::to_value(&job.report).unwrap(), 3)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();

    while let Some(part) = stream.next().await {
        bytes.extend(part.unwrap());
    }
    let errors: Vec<RowError> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert_eq!(errors, job.expected);
}

#[sqlx::test]
#[serial_test::serial]
async fn download_reports_missing_fragments_after_its_buffered_page(pool: PgPool) {
    let job = archived_job(&pool).await;
    let mut stream = job.body(&pool).into_data_stream();
    assert_eq!(
        stream.next().await.unwrap().unwrap().len(),
        super::CHUNK_BYTES
    );

    let mut tx = pool.begin().await.unwrap();
    assert!(
        job.lease
            .complete(&mut tx, &serde_json::to_value(&job.report).unwrap(), 2)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    sqlx::query!(
        "UPDATE horae_jobs SET finished_at = now() - interval '31 days' WHERE id = $1",
        job.id
    )
    .execute(&pool)
    .await
    .unwrap();
    jobs::cleanup(&pool).await.unwrap();

    for _ in 1..16 {
        assert_eq!(
            stream.next().await.unwrap().unwrap().len(),
            super::CHUNK_BYTES
        );
    }
    let error = stream
        .next()
        .await
        .expect("missing data must not become a successful EOF")
        .unwrap_err();
    assert!(error.to_string().contains("report archive is incomplete"));
}

#[sqlx::test]
#[serial_test::serial]
async fn download_reads_only_when_consumed_and_releases_its_connection(pool: PgPool) {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    let acquisitions = Arc::new(AtomicUsize::new(0));
    let observed = acquisitions.clone();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .before_acquire(move |_, _| {
            let observed = observed.clone();
            Box::pin(async move {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(true)
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let job = archived_job(&pool).await;
    acquisitions.store(0, Ordering::SeqCst);
    let mut stream = job.body(&pool).into_data_stream();
    assert_eq!(acquisitions.load(Ordering::SeqCst), 0);
    for _ in 0..16 {
        assert_eq!(
            stream.next().await.unwrap().unwrap().len(),
            super::CHUNK_BYTES
        );
        assert_eq!(acquisitions.load(Ordering::SeqCst), 1);
    }
    assert_eq!(
        stream.next().await.unwrap().unwrap().len(),
        super::CHUNK_BYTES
    );
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
    drop(stream);
    tokio::time::timeout(Duration::from_secs(5), pool.close())
        .await
        .unwrap();
    assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
}
