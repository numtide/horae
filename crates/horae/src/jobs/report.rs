//! Bounded public reports and append-only, complete error details.

use anyhow::Context;
use horae_core::importers::harvest::types::ImportReport;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{JobLease, lease::Interrupted};

pub(crate) const REPORT_BYTES: usize = 16 * 1024;
const CHUNK_BYTES: usize = 64 * 1024;

impl JobLease {
    /// Call within the checkpoint/completion transaction. The caller must roll
    /// back both chunks and report metadata if its final ownership fence fails.
    pub(crate) async fn archive_report(
        &self,
        connection: &mut PgConnection,
        report: &mut ImportReport,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(report.reconciles(), "import report does not reconcile");
        if serde_json::to_vec(&report)?.len() <= REPORT_BYTES {
            return Ok(());
        }
        let live = sqlx::query_scalar!(
            r#"SELECT id FROM horae_jobs
               WHERE id = $1 AND org_id = $2 AND claim_token = $3
                 AND status = 'running' AND NOT cancellation_requested
                 AND lease_until > clock_timestamp()
               FOR UPDATE"#,
            self.id,
            self.org_id,
            self.token,
        )
        .fetch_optional(&mut *connection)
        .await?;
        live.context(Interrupted)?;
        let mut sequence = i64::try_from(report.archived_error_chunks())?;
        let mut chunk = Vec::with_capacity(CHUNK_BYTES);
        for error in &report.row_errors {
            let mut encoded = serde_json::to_vec(error)?;
            encoded.push(b'\n');
            let mut bytes = encoded.as_slice();
            while !bytes.is_empty() {
                let count = (CHUNK_BYTES - chunk.len()).min(bytes.len());
                chunk.extend_from_slice(&bytes[..count]);
                bytes = &bytes[count..];
                if chunk.len() == CHUNK_BYTES {
                    self.append_report_chunk(connection, sequence, &chunk)
                        .await?;
                    sequence = sequence
                        .checked_add(1)
                        .context("report archive is too large")?;
                    chunk.clear();
                }
            }
        }
        if !chunk.is_empty() {
            self.append_report_chunk(connection, sequence, &chunk)
                .await?;
            sequence = sequence
                .checked_add(1)
                .context("report archive is too large")?;
        }
        report
            .archive_errors(sequence.try_into()?)
            .map_err(anyhow::Error::msg)?;
        anyhow::ensure!(
            serde_json::to_vec(report)?.len() <= REPORT_BYTES,
            "report metadata exceeds its budget"
        );
        Ok(())
    }

    async fn append_report_chunk(
        &self,
        connection: &mut PgConnection,
        sequence: i64,
        body: &[u8],
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body)
               VALUES ($1, $2, $3, $4, $5)"#,
            Uuid::now_v7(),
            self.id,
            self.org_id,
            sequence,
            body,
        )
        .execute(connection)
        .await?;
        Ok(())
    }
}

/// A fixed-size read at a captured report boundary, never the archive's live end.
pub(crate) async fn chunks(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    job_id: Uuid,
    next: i64,
    end: i64,
) -> anyhow::Result<Vec<Vec<u8>>> {
    anyhow::ensure!(next >= 0 && end >= next, "invalid report archive range");
    let rows = sqlx::query!(
        r#"SELECT sequence, body FROM horae_job_report_error_chunks
           WHERE job_id = $1 AND org_id = $2 AND sequence >= $3 AND sequence < $4
           ORDER BY sequence LIMIT 16"#,
        job_id,
        org_id,
        next,
        end,
    )
    .fetch_all(pool)
    .await?;
    anyhow::ensure!(
        rows.len() as i64 == (end - next).min(16),
        "report archive is incomplete"
    );
    let mut output = Vec::with_capacity(rows.len());
    for (offset, row) in rows.into_iter().enumerate() {
        anyhow::ensure!(
            row.sequence == next + offset as i64,
            "report archive is incomplete"
        );
        output.push(row.body);
    }
    Ok(output)
}

/// Download a single confirmed report snapshot; later checkpoints are not part
/// of this response. A missing archive fragment fails the body, never silently
/// turns into a successful but incomplete download.
pub(crate) async fn download(
    session: tower_sessions::Session,
    axum::extract::Path(job_id): axum::extract::Path<Uuid>,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    use axum::{
        body::Body,
        http::{StatusCode, header},
        response::IntoResponse,
    };
    use std::collections::VecDeque;

    let user_id = crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let state = crate::state::global_state().await;
    let user = sqlx::query!(
        r#"SELECT org_id, org_role as "org_role: horae_core::types::OrgRole"
           FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;
    if user.org_role != horae_core::types::OrgRole::Admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let job = super::status(&state.db, user.org_id, job_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let report: ImportReport = serde_json::from_value(job.report.ok_or(StatusCode::NOT_FOUND)?)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let end = i64::try_from(report.archived_error_chunks())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut tail = Vec::new();
    for error in report.row_errors {
        serde_json::to_writer(&mut tail, &error).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        tail.push(b'\n');
    }
    let pool = state.db.clone();
    let stream = futures_util::stream::try_unfold(
        (0_i64, VecDeque::<Vec<u8>>::new(), Some(tail)),
        move |(mut next, mut pending, mut tail)| {
            let pool = pool.clone();
            async move {
                if pending.is_empty() && next < end {
                    let page = chunks(&pool, user.org_id, job_id, next, end)
                        .await
                        .map_err(std::io::Error::other)?;
                    next += page.len() as i64;
                    pending.extend(page);
                }
                let chunk = pending
                    .pop_front()
                    .or_else(|| tail.take().filter(|bytes| !bytes.is_empty()));
                Ok::<_, std::io::Error>(chunk.map(|chunk| (chunk, (next, pending, tail))))
            }
        },
    );
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"import-{job_id}-errors.jsonl\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_owned()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}
