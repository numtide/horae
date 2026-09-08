use std::{
    convert::Infallible,
    io::{self, Cursor, Seek, SeekFrom, Write},
    sync::{Arc, LazyLock},
    time::Duration,
};

use axum::{body::Body, http::StatusCode};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_EXPORTS: usize = 2;
const RENDER_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;

static EXPORTS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(MAX_EXPORTS)));

/// Reject writes before growing the destination past its budget. ZIP output
/// needs seeking to patch headers, so counting written bytes alone is insufficient.
struct LimitedBuffer {
    cursor: Cursor<Vec<u8>>,
    limit: u64,
    exceeded: bool,
}

impl LimitedBuffer {
    fn new(limit: usize) -> Self {
        Self {
            cursor: Cursor::new(Vec::new()),
            limit: limit as u64,
            exceeded: false,
        }
    }

    fn too_large(&mut self) -> io::Error {
        self.exceeded = true;
        io::Error::other("Export output exceeds the size limit")
    }
}

impl Write for LimitedBuffer {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self
            .cursor
            .position()
            .checked_add(data.len() as u64)
            .is_none_or(|end| end > self.limit)
        {
            return Err(self.too_large());
        }
        self.cursor.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for LimitedBuffer {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let previous = self.cursor.position();
        let position = self.cursor.seek(from)?;
        if position > self.limit {
            self.cursor.set_position(previous);
            return Err(self.too_large());
        }
        Ok(position)
    }
}

pub(super) fn workbook_bytes(
    workbook: &mut rust_xlsxwriter::Workbook,
) -> Result<Vec<u8>, StatusCode> {
    let mut output = LimitedBuffer::new(MAX_OUTPUT_BYTES);
    let result = workbook.save_to_writer(&mut output);
    if output.exceeded {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    result.map_err(|error| {
        tracing::error!("XLSX export failed: {error}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(output.cursor.into_inner())
}

/// Covers loading, rendering, and the response body. Admission happens before
/// loading data, so queued clients cannot accumulate full datasets in memory.
pub(super) struct ExportPermit(OwnedSemaphorePermit);

impl ExportPermit {
    pub(super) fn acquire() -> Result<Self, StatusCode> {
        Self::from_slots(&EXPORTS)
    }

    fn from_slots(slots: &Arc<Semaphore>) -> Result<Self, StatusCode> {
        Arc::clone(slots)
            .try_acquire_owned()
            .map(Self)
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    }

    pub(super) async fn render(
        self,
        render: impl FnOnce() -> Result<Vec<u8>, StatusCode> + Send + 'static,
    ) -> Result<Body, StatusCode> {
        self.render_with_timeout(render, RENDER_TIMEOUT).await
    }

    async fn render_with_timeout(
        self,
        render: impl FnOnce() -> Result<Vec<u8>, StatusCode> + Send + 'static,
        timeout: Duration,
    ) -> Result<Body, StatusCode> {
        let mut task = tokio::task::spawn_blocking(move || {
            let data = render()?;
            if data.len() > MAX_OUTPUT_BYTES {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            Ok((data, self.0))
        });
        let (data, permit) = match tokio::time::timeout(timeout, &mut task).await {
            Ok(result) => result.map_err(|error| {
                tracing::error!("Export worker failed: {error}");
                StatusCode::INTERNAL_SERVER_ERROR
            })??,
            Err(_) => {
                // Only queued work can be aborted. A running renderer retains
                // its permit until it actually exits, even after cancellation.
                task.abort();
                return Err(StatusCode::GATEWAY_TIMEOUT);
            }
        };
        Ok(Body::from_stream(futures_util::stream::unfold(
            (Some(data), permit),
            |(data, permit)| async move { data.map(|data| (Ok::<_, Infallible>(data), (None, permit))) },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_budget_checks_writes_and_seeks_without_truncation() {
        let mut output = LimitedBuffer::new(4);
        output.write_all(b"1234").unwrap();
        assert!(output.write_all(b"5").is_err());
        assert_eq!(output.cursor.get_ref(), b"1234");
        output.seek(SeekFrom::Start(1)).unwrap();
        output.write_all(b"x").unwrap();
        assert!(output.seek(SeekFrom::End(1)).is_err());
        assert_eq!(output.cursor.position(), 2);
        assert_eq!(output.cursor.get_ref(), b"1x34");
        assert!(output.exceeded);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn export_rendering_runs_off_the_async_worker() {
        let slots = Arc::new(Semaphore::new(1));
        let runtime_thread = std::thread::current().id();
        let body = ExportPermit::from_slots(&slots)
            .unwrap()
            .render(move || {
                assert_ne!(std::thread::current().id(), runtime_thread);
                Ok(b"document".to_vec())
            })
            .await
            .unwrap();
        assert_eq!(axum::body::to_bytes(body, 1024).await.unwrap(), "document");
    }

    #[tokio::test]
    async fn export_admission_stays_reserved_until_the_body_is_dropped() {
        let slots = Arc::new(Semaphore::new(1));
        let permit = ExportPermit::from_slots(&slots).unwrap();
        assert!(matches!(
            ExportPermit::from_slots(&slots),
            Err(StatusCode::SERVICE_UNAVAILABLE)
        ));
        let body = permit.render(|| Ok(b"document".to_vec())).await.unwrap();
        assert_eq!(slots.available_permits(), 0);
        drop(body);
        assert_eq!(slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn cancelled_export_keeps_its_slot_until_the_renderer_exits() {
        let slots = Arc::new(Semaphore::new(1));
        let permit = ExportPermit::from_slots(&slots).unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let task = tokio::spawn(permit.render(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(Vec::new())
        }));
        started_rx.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(slots.available_permits(), 0);
        release_tx.send(()).unwrap();
        let available = tokio::time::timeout(Duration::from_secs(2), slots.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(available);
        assert_eq!(slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn timed_out_export_keeps_its_slot_until_the_renderer_exits() {
        let slots = Arc::new(Semaphore::new(1));
        let permit = ExportPermit::from_slots(&slots).unwrap();
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let task = tokio::spawn(permit.render_with_timeout(
            move || {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(Vec::new())
            },
            Duration::from_secs(1),
        ));
        started_rx.await.unwrap();
        assert!(matches!(
            task.await.unwrap(),
            Err(StatusCode::GATEWAY_TIMEOUT)
        ));
        assert_eq!(slots.available_permits(), 0);
        release_tx.send(()).unwrap();
        let available = tokio::time::timeout(Duration::from_secs(2), slots.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(available);
    }

    #[tokio::test]
    async fn rendered_output_limit_rejects_oversized_files_and_releases_admission() {
        let slots = Arc::new(Semaphore::new(1));
        let body = ExportPermit::from_slots(&slots)
            .unwrap()
            .render(|| Ok(vec![0; MAX_OUTPUT_BYTES]))
            .await
            .unwrap();
        assert_eq!(
            axum::body::to_bytes(body, MAX_OUTPUT_BYTES)
                .await
                .unwrap()
                .len(),
            MAX_OUTPUT_BYTES
        );
        let oversized = ExportPermit::from_slots(&slots)
            .unwrap()
            .render(|| Ok(vec![0; MAX_OUTPUT_BYTES + 1]))
            .await;
        assert!(matches!(oversized, Err(StatusCode::PAYLOAD_TOO_LARGE)));
        assert_eq!(slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn renderer_errors_and_panics_release_admission() {
        let slots = Arc::new(Semaphore::new(1));
        let failed = ExportPermit::from_slots(&slots)
            .unwrap()
            .render(|| Err(StatusCode::PAYLOAD_TOO_LARGE))
            .await;
        assert!(matches!(failed, Err(StatusCode::PAYLOAD_TOO_LARGE)));
        let panicked = ExportPermit::from_slots(&slots)
            .unwrap()
            .render(|| panic!("renderer probe"))
            .await;
        assert!(matches!(panicked, Err(StatusCode::INTERNAL_SERVER_ERROR)));
        assert_eq!(slots.available_permits(), 1);
    }
}
