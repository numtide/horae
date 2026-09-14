//! Bridge the async request body to the blocking CSV parser with a one-row queue.

use std::future::poll_fn;
use std::io::{self, Read};
use std::sync::Arc;

use anyhow::Context;
use axum::body::{Body, Bytes, HttpBody};
use horae_core::importers::harvest::types::{EntityType, ImportMode, RowOutcome, SourceKind};
use sqlx::Acquire;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use super::super::{
    apply, finish_import, lock_import, release_import,
    report::ImportReport,
    resolve::{OrgDefaults, RunCache},
};
use super::{CsvError, Cursor, Record, read_csv_from};

const BATCH_ROWS: u64 = 500;

#[derive(serde::Serialize, serde::Deserialize)]
struct Checkpoint {
    version: u8,
    default_currency: String,
    cursor: Cursor,
    report: ImportReport,
    cache: RunCache,
}

#[derive(Debug, thiserror::Error)]
#[error("CSV upload ended before completion")]
struct IncompleteUpload;

struct BodyReader<'a> {
    body: Body,
    remaining: Bytes,
    runtime: tokio::runtime::Handle,
    rows: &'a mpsc::Sender<Record>,
}

impl Read for BodyReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        if self.rows.is_closed() {
            return Err(io::Error::other("CSV import cancelled"));
        }
        while self.remaining.is_empty() {
            // A disconnected consumer must wake a parser waiting for a slow
            // upload as well as one waiting to send its next parsed record.
            let frame = self.runtime.block_on(async {
                tokio::select! {
                    _ = self.rows.closed() => Err(io::Error::other("CSV import cancelled")),
                    frame = poll_fn(|cx| std::pin::Pin::new(&mut self.body).poll_frame(cx)) => Ok(frame),
                }
            })?;
            let Some(frame) = frame else { return Ok(0) };
            if let Ok(bytes) = frame.map_err(io::Error::other)?.into_data() {
                self.remaining = bytes;
            }
        }
        let count = output.len().min(self.remaining.len());
        output[..count].copy_from_slice(&self.remaining.split_to(count));
        Ok(count)
    }
}

/// Import an unbuffered HTTP body. Only normal EOF permits a commit; a broken
/// upload or cancelled parser rolls back the whole run, including parent rows.
pub async fn import_body(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    default_currency: &str,
    body: Body,
    mode: ImportMode,
) -> Result<ImportReport, CsvError> {
    import_body_with_lease(pool, org_id, default_currency, body, mode, None).await
}

/// Durable commits publish batches and their cursor atomically. Inline imports
/// and previews retain their whole-run transaction; a preview never commits data.
pub(crate) async fn import_body_with_lease(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    default_currency: &str,
    body: Body,
    mode: ImportMode,
    lease: Option<&crate::jobs::JobLease>,
) -> Result<ImportReport, CsvError> {
    if let Some(lease) = lease {
        lease.check_organization(org_id)?;
    }
    let mut connection = lock_import(pool, org_id)
        .await
        .map_err(anyhow::Error::from)?;
    let durable = lease.filter(|_| mode == ImportMode::Commit);
    let stored = match durable {
        Some(lease) => lease.load_checkpoint(&mut connection).await?,
        None => None,
    };
    let (mut checkpoint, resume) = match stored {
        Some(value) => {
            let checkpoint: Checkpoint =
                serde_json::from_value(value).map_err(anyhow::Error::from)?;
            if checkpoint.version != 1
                || checkpoint.report.mode != mode
                || checkpoint.report.source != SourceKind::Csv
            {
                return Err(anyhow::anyhow!("unsupported CSV checkpoint").into());
            }
            let cursor = checkpoint.cursor.clone();
            (checkpoint, Some(cursor))
        }
        None => (
            Checkpoint {
                version: 1,
                default_currency: default_currency.to_owned(),
                cursor: Cursor::default(),
                report: ImportReport::new(SourceKind::Csv, mode),
                cache: RunCache::default(),
            },
            None,
        ),
    };
    let session = Arc::new(Mutex::new(connection));
    let worker_session = session.clone();
    let runtime = tokio::runtime::Handle::current();
    let (send, mut receive) = mpsc::channel(1);
    let worker = tokio::task::spawn_blocking(move || {
        // Cancellation cannot release the organization lock while its parser
        // still owns the request body. No second pool connection is needed.
        let _keep_session = worker_session;
        let input = BodyReader {
            body,
            remaining: Bytes::new(),
            runtime,
            rows: &send,
        };
        read_csv_from(input, resume.as_ref(), |record| {
            send.blocking_send(record).context("CSV import cancelled")
        })?;
        Ok::<_, CsvError>(())
    });
    let work = async {
        // Validate headers and reject header-only input before opening the TX.
        let Some(Record::Headers(headers)) = receive.recv().await else {
            return Err(IncompleteUpload.into());
        };
        checkpoint.cursor.headers = headers;
        let first = receive.recv().await.ok_or(IncompleteUpload)?;
        let mut connection = session.lock().await;
        let mut tx = connection.begin().await?;
        let org = OrgDefaults {
            org_id,
            default_currency: &checkpoint.default_currency,
        };
        let mut next = first;
        while let Record::Row(record, position) = next {
            match *record {
                Ok(row) => {
                    let result = apply::apply_row(
                        &mut tx,
                        &mut checkpoint.cache,
                        org,
                        &row,
                        SourceKind::Csv,
                    )
                    .await;
                    for (entity, outcome) in &result.outcomes {
                        checkpoint.report.record(*entity, outcome);
                    }
                }
                Err(error) => checkpoint.report.record(
                    EntityType::TimeEntry,
                    &RowOutcome::Errored {
                        source_location: error.source_location,
                        reason: error.reason,
                    },
                ),
            }
            checkpoint.cursor.position = position;
            if let Some(lease) = durable
                && position.record.is_multiple_of(BATCH_ROWS)
            {
                let (_, processed) = super::super::job_report(&checkpoint.report)?;
                lease
                    .save_checkpoint(
                        &mut tx,
                        &serde_json::to_value(&checkpoint)?,
                        "time_entries",
                        processed,
                    )
                    .await?;
                tx.commit().await?;
                tx = connection.begin().await?;
            }
            next = receive.recv().await.ok_or(IncompleteUpload)?;
        }
        anyhow::ensure!(matches!(next, Record::Complete), "unexpected CSV headers");
        debug_assert!(checkpoint.report.reconciles());
        finish_import(tx, &checkpoint.report, lease).await?;
        Ok::<_, anyhow::Error>(checkpoint.report)
    };
    let result = match lease {
        Some(lease) => lease.run(work).await,
        None => work.await,
    };
    drop(receive);
    let parsed = worker.await.context("CSV parser task failed");
    let connection = Arc::try_unwrap(session)
        .map_err(|_| anyhow::anyhow!("CSV session is still in use"))?
        .into_inner();
    release_import(connection).await?;
    match (result, parsed) {
        (Ok(report), Ok(Ok(()))) => Ok(report),
        (Err(error), Ok(Err(parse_error))) if error.is::<IncompleteUpload>() => Err(parse_error),
        (Err(error), Err(join_error)) if error.is::<IncompleteUpload>() => Err(join_error.into()),
        (Err(error), _) => Err(error.into()),
        (_, Ok(Err(error))) => Err(error),
        (_, Err(error)) => Err(error.into()),
    }
}
