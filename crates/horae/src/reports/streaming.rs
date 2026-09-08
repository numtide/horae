use std::{
    future::Future,
    io,
    sync::{Arc, LazyLock},
    time::Duration,
};

use axum::{
    body::Body,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::{Stream, StreamExt, TryStreamExt};
use sqlx::{Connection, PgPool};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot},
    task::JoinHandle,
};
use uuid::Uuid;

use super::{
    ExportParams, ProjectsExportParams,
    limits::{configure_transaction, database_error},
};

const CHUNK_BYTES: usize = 64 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
static EXPORTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));

struct Download {
    receiver: mpsc::Receiver<Vec<u8>>,
    task: JoinHandle<Result<(), StatusCode>>,
    _permit: Arc<OwnedSemaphorePermit>,
}

impl Download {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>, StatusCode> {
        if let Some(chunk) = self.receiver.recv().await {
            return Ok(Some(chunk));
        }
        // Channel closure alone is not success: a panic, query failure or
        // timeout must interrupt the HTTP body rather than silently truncate it.
        (&mut self.task).await.map_err(|error| {
            tracing::error!("CSV worker failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        })??;
        Ok(None)
    }
}

impl Drop for Download {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn response<F>(
    slots: &Arc<Semaphore>,
    timeout: Duration,
    produce: impl FnOnce(mpsc::Sender<Vec<u8>>, oneshot::Sender<String>) -> F + Send + 'static,
) -> Result<Response, StatusCode>
where
    F: Future<Output = Result<(), StatusCode>> + Send + 'static,
{
    let permit = Arc::new(
        Arc::clone(slots)
            .try_acquire_owned()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    );
    let worker_permit = Arc::clone(&permit);
    let (sender, receiver) = mpsc::channel(1);
    let (filename_tx, filename_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _permit = worker_permit;
        tokio::time::timeout(timeout, produce(sender, filename_tx))
            .await
            .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
    });
    let mut download = Download {
        receiver,
        task,
        _permit: permit,
    };
    // The first query result (or an empty, successful query) is checked before
    // sending status 200. The guard also covers cancellation during startup.
    let first = download
        .next_chunk()
        .await?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let filename = filename_rx
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rest = futures_util::stream::try_unfold(download, |mut download| async move {
        download
            .next_chunk()
            .await
            .map(|chunk| chunk.map(|chunk| (chunk, download)))
            .map_err(|status| io::Error::other(format!("CSV download failed: {status}")))
    });
    let body = Body::from_stream(futures_util::stream::once(async { Ok(first) }).chain(rest));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    )
        .into_response())
}

pub(super) async fn entries(
    pool: PgPool,
    org_id: Uuid,
    params: ExportParams,
) -> Result<Response, StatusCode> {
    let from = params.from.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let to = params.to.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    response(
        &EXPORTS,
        DOWNLOAD_TIMEOUT,
        move |sender, filename| async move {
            let mut connection = pool.acquire().await.map_err(database_error)?;
            // Never put a connection with an unfinished stream back in the shared
            // pool. SQLx retains its pool slot while closing it on cancellation.
            connection.close_on_drop();
            let mut tx = connection.begin().await.map_err(database_error)?;
            configure_transaction(&mut tx).await?;
            let _ = filename.send("timesheet.csv".to_owned());
            write_rows(
                &sender,
                super::stream_entries(
                    &mut *tx,
                    org_id,
                    from,
                    to,
                    params.client_id,
                    params.project_id,
                    params.user_id,
                ),
                &super::ENTRY_EXPORT_HEADERS,
                |writer, row| super::write_entry_csv(writer, &row),
            )
            .await?;
            tx.commit().await.map_err(database_error)
        },
    )
    .await
}

pub(super) async fn projects(
    pool: PgPool,
    org_id: Uuid,
    params: ProjectsExportParams,
) -> Result<Response, StatusCode> {
    response(
        &EXPORTS,
        DOWNLOAD_TIMEOUT,
        move |sender, filename| async move {
            let mut connection = pool.acquire().await.map_err(database_error)?;
            connection.close_on_drop();
            let mut tx = connection.begin().await.map_err(database_error)?;
            configure_transaction(&mut tx).await?;
            let _ = filename.send("projects.csv".to_owned());
            write_rows(
                &sender,
                super::stream_projects_export(
                    &mut *tx,
                    org_id,
                    params.scope.as_deref().unwrap_or("active"),
                ),
                &super::PROJECT_EXPORT_HEADERS,
                |writer, row| {
                    writer.write_record([
                        row.client_name.as_str(),
                        row.code.as_deref().unwrap_or(""),
                        &row.name,
                        row.project_type.label(),
                        row.currency.trim(),
                        &super::budget_cell(&row),
                        if row.active { "Active" } else { "Archived" },
                    ])
                },
            )
            .await?;
            tx.commit().await.map_err(database_error)
        },
    )
    .await
}

pub(super) async fn invoice(
    pool: PgPool,
    org_id: Uuid,
    invoice_id: Uuid,
) -> Result<Response, StatusCode> {
    response(
        &EXPORTS,
        DOWNLOAD_TIMEOUT,
        move |sender, filename| async move {
            let mut connection = pool.acquire().await.map_err(database_error)?;
            connection.close_on_drop();
            let mut tx = connection.begin().await.map_err(database_error)?;
            configure_transaction(&mut tx).await?;
            let invoice = super::fetch_invoice_metadata(&mut *tx, invoice_id, org_id)
                .await
                .map_err(database_error)?
                .ok_or(StatusCode::NOT_FOUND)?;
            let _ = filename.send(format!("invoice-{}.csv", invoice.number));
            write_rows(
                &sender,
                super::stream_invoice_lines(&mut *tx, invoice_id),
                &["Description", "Hours", "Rate", "Amount"],
                |writer, line| {
                    writer.write_record([
                        line.description.as_str(),
                        &super::format_hours2(line.minutes.into()),
                        &super::format_cents_plain(line.rate_cents),
                        &super::format_cents_plain(line.amount_cents),
                    ])
                },
            )
            .await?;
            let mut total = csv::Writer::from_writer(Vec::new());
            total
                .write_record([
                    "Total",
                    "",
                    "",
                    &super::format_cents_plain(invoice.total_cents),
                ])
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            flush(&sender, total).await?;
            tx.commit().await.map_err(database_error)
        },
    )
    .await
}

async fn flush(
    sender: &mpsc::Sender<Vec<u8>>,
    writer: csv::Writer<Vec<u8>>,
) -> Result<(), StatusCode> {
    let data = writer
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !data.is_empty() {
        sender
            .send(data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(())
}

async fn write_rows<T>(
    sender: &mpsc::Sender<Vec<u8>>,
    rows: impl Stream<Item = Result<T, sqlx::Error>>,
    headers: &[&str],
    write: impl Fn(&mut csv::Writer<Vec<u8>>, T) -> Result<(), csv::Error>,
) -> Result<(), StatusCode> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record(headers)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    futures_util::pin_mut!(rows);
    let mut first = true;
    while let Some(row) = rows.try_next().await.map_err(database_error)? {
        write(&mut writer, row).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if first || writer.get_ref().len() >= CHUNK_BYTES {
            flush(sender, writer).await?;
            writer = csv::Writer::from_writer(Vec::new());
            first = false;
        }
    }
    flush(sender, writer).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use futures_util::StreamExt;

    use super::*;

    struct Finished(Option<oneshot::Sender<()>>);

    impl Drop for Finished {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[tokio::test]
    async fn csv_emits_a_chunk_before_the_source_finishes() {
        let rows = futures_util::stream::once(async { Ok("first") })
            .chain(futures_util::stream::pending());
        let (sender, mut receiver) = mpsc::channel(1);
        let task = tokio::spawn(async move {
            write_rows(&sender, rows, &["Value"], |writer, row| {
                writer.write_record([row])
            })
            .await
        });
        let first = tokio::time::timeout(Duration::from_secs(1), receiver.recv()).await;
        task.abort();
        let _ = task.await;
        assert_eq!(
            first.expect("CSV waited for all rows").unwrap(),
            b"Value\nfirst\n"
        );
    }

    #[tokio::test]
    async fn csv_does_not_truncate_a_quoted_unicode_record_larger_than_a_chunk() {
        let value = "界,\"quoted\"\n".repeat(10_000);
        let original = value.clone();
        let slots = Arc::new(Semaphore::new(1));
        let result = response(
            &slots,
            DOWNLOAD_TIMEOUT,
            move |sender, filename| async move {
                let _ = filename.send("large.csv".to_owned());
                write_rows(
                    &sender,
                    futures_util::stream::iter([Ok(value)]),
                    &["Value"],
                    |writer, row| writer.write_record([row]),
                )
                .await
            },
        )
        .await
        .unwrap();
        let bytes = axum::body::to_bytes(result.into_body(), 1024 * 1024)
            .await
            .unwrap();
        assert!(bytes.len() > CHUNK_BYTES);
        let records = csv::Reader::from_reader(bytes.as_ref())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(&records[0][0], original);
    }

    #[tokio::test]
    async fn csv_startup_errors_are_http_errors_and_release_admission() {
        let slots = Arc::new(Semaphore::new(1));
        for status in [
            StatusCode::NOT_FOUND,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            let result = response(
                &slots,
                DOWNLOAD_TIMEOUT,
                move |_, _| async move { Err(status) },
            )
            .await;
            assert_eq!(result.unwrap_err(), status);
            assert_eq!(slots.available_permits(), 1);
        }
    }

    #[tokio::test]
    async fn csv_failure_or_panic_after_a_chunk_is_not_successful_eof() {
        for panic in [false, true] {
            let slots = Arc::new(Semaphore::new(1));
            let result = response(
                &slots,
                DOWNLOAD_TIMEOUT,
                move |sender, filename| async move {
                    let _ = filename.send("test.csv".to_owned());
                    sender.send(b"partial\n".to_vec()).await.unwrap();
                    assert!(!panic, "simulated producer panic");
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                },
            )
            .await
            .unwrap();
            let mut body = result.into_body().into_data_stream();
            assert_eq!(body.next().await.unwrap().unwrap(), "partial\n");
            assert!(body.next().await.unwrap().is_err());
            assert!(body.next().await.is_none());
            assert_eq!(slots.available_permits(), 1);
        }
    }

    #[tokio::test]
    async fn csv_timeout_interrupts_an_already_started_body() {
        let slots = Arc::new(Semaphore::new(1));
        let result = response(
            &slots,
            Duration::from_secs(1),
            |sender, filename| async move {
                let _ = filename.send("test.csv".to_owned());
                sender.send(b"partial\n".to_vec()).await.unwrap();
                std::future::pending::<Result<(), StatusCode>>().await
            },
        )
        .await
        .unwrap();
        let mut body = result.into_body().into_data_stream();
        assert_eq!(body.next().await.unwrap().unwrap(), "partial\n");
        let error = body.next().await.unwrap().unwrap_err();
        assert!(error.to_string().contains("504"), "{error}");
        assert_eq!(slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn dropping_csv_during_startup_cancels_the_producer() {
        let slots = Arc::new(Semaphore::new(1));
        let worker_slots = Arc::clone(&slots);
        let (started_tx, started_rx) = oneshot::channel();
        let (finished_tx, finished_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            response(&worker_slots, DOWNLOAD_TIMEOUT, |_, _| async move {
                let _finished = Finished(Some(finished_tx));
                started_tx.send(()).unwrap();
                std::future::pending::<Result<(), StatusCode>>().await
            })
            .await
        });
        started_rx.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        tokio::time::timeout(Duration::from_secs(1), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn csv_backpressure_and_body_drop_bound_and_cancel_source_reads() {
        let slots = Arc::new(Semaphore::new(1));
        let read = Arc::new(AtomicUsize::new(0));
        let worker_read = Arc::clone(&read);
        let (third_tx, third_rx) = oneshot::channel();
        let (finished_tx, finished_rx) = oneshot::channel();
        let result = response(
            &slots,
            DOWNLOAD_TIMEOUT,
            move |sender, filename| async move {
                let _finished = Finished(Some(finished_tx));
                let _ = filename.send("test.csv".to_owned());
                let mut third_tx = Some(third_tx);
                let rows = futures_util::stream::iter(0..100).map(move |_| {
                    if worker_read.fetch_add(1, Ordering::SeqCst) == 2 {
                        third_tx.take().unwrap().send(()).unwrap();
                    }
                    Ok("x".repeat(CHUNK_BYTES))
                });
                write_rows(&sender, rows, &["Value"], |writer, row| {
                    writer.write_record([row])
                })
                .await
            },
        )
        .await
        .unwrap();
        third_rx.await.unwrap();
        // One first chunk in the body, one queued, and one blocked send.
        assert_eq!(read.load(Ordering::SeqCst), 3);
        assert_eq!(
            response(&slots, DOWNLOAD_TIMEOUT, |_, _| async { Ok(()) })
                .await
                .unwrap_err(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        drop(result);
        tokio::time::timeout(Duration::from_secs(1), finished_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(slots.available_permits(), 1);
        assert_eq!(read.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn completed_csv_keeps_admission_until_body_drop() {
        let slots = Arc::new(Semaphore::new(1));
        let (finished_tx, finished_rx) = oneshot::channel();
        let result = response(&slots, DOWNLOAD_TIMEOUT, |sender, filename| async move {
            let _finished = Finished(Some(finished_tx));
            let _ = filename.send("test.csv".to_owned());
            sender.send(b"complete\n".to_vec()).await.unwrap();
            Ok(())
        })
        .await
        .unwrap();
        finished_rx.await.unwrap();
        assert_eq!(slots.available_permits(), 0);
        drop(result);
        assert_eq!(slots.available_permits(), 1);
    }
}

#[cfg(test)]
mod database_tests;
