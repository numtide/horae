//! Bridge the async request body to the blocking CSV parser with a one-row queue.

use std::future::poll_fn;
use std::io::{self, Read};
use std::sync::Arc;

use anyhow::Context;
use axum::body::{Body, Bytes, HttpBody};
use horae_core::importers::harvest::types::{
    EntityType, ImportMode, RowOutcome, SourceKind, SourceRow,
};
use sqlx::Acquire;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use super::super::{
    apply, lock_import,
    report::ImportReport,
    resolve::{OrgDefaults, RunCache},
};
use super::{CsvError, ParseErr, read_csv};

enum Record {
    Row(Box<Result<SourceRow, ParseErr>>),
    Complete,
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
    let connection = lock_import(pool, org_id)
        .await
        .map_err(anyhow::Error::from)?;
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
        read_csv(input, |row| {
            send.blocking_send(Record::Row(Box::new(row)))
                .context("CSV import cancelled")
        })?;
        send.blocking_send(Record::Complete)
            .context("CSV import cancelled")?;
        Ok::<_, CsvError>(())
    });
    let result = async {
        // Validate headers and reject header-only input before opening the TX.
        let first = receive.recv().await.ok_or(IncompleteUpload)?;
        let mut connection = session.lock().await;
        let mut tx = connection.begin().await?;
        let org = OrgDefaults {
            org_id,
            default_currency,
        };
        let mut cache = RunCache::default();
        let mut report = ImportReport::new(SourceKind::Csv, mode);
        let mut next = first;
        while let Record::Row(record) = next {
            match *record {
                Ok(row) => {
                    let result =
                        apply::apply_row(&mut tx, &mut cache, org, &row, SourceKind::Csv).await;
                    for (entity, outcome) in &result.outcomes {
                        report.record(*entity, outcome);
                    }
                }
                Err(error) => report.record(
                    EntityType::TimeEntry,
                    &RowOutcome::Errored {
                        source_location: error.source_location,
                        reason: error.reason,
                    },
                ),
            }
            next = receive.recv().await.ok_or(IncompleteUpload)?;
        }
        debug_assert!(report.reconciles());
        match mode {
            ImportMode::Commit => tx.commit().await?,
            ImportMode::DryRun => tx.rollback().await?,
        }
        Ok::<_, anyhow::Error>(report)
    }
    .await;
    drop(receive);
    let parsed = worker.await.context("CSV parser task failed");
    let connection = Arc::try_unwrap(session)
        .map_err(|_| anyhow::anyhow!("CSV session is still in use"))?
        .into_inner();
    connection.close().await.map_err(anyhow::Error::from)?;
    match (result, parsed) {
        (Ok(report), Ok(Ok(()))) => Ok(report),
        (Err(error), Ok(Err(parse_error))) if error.is::<IncompleteUpload>() => Err(parse_error),
        (Err(error), Err(join_error)) if error.is::<IncompleteUpload>() => Err(join_error.into()),
        (Err(error), _) => Err(error.into()),
        (_, Ok(Err(error))) => Err(error),
        (_, Err(error)) => Err(error.into()),
    }
}
