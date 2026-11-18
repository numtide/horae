//! Bounded public reports and append-only, complete error details.

use anyhow::Context;
use horae_core::importers::harvest::types::ImportReport;
use sqlx::PgConnection;
use uuid::Uuid;

use super::{JobLease, lease::Interrupted};

pub(crate) const REPORT_BYTES: usize = 16 * 1024;
pub const CHUNK_BYTES: usize = 64 * 1024;

mod summary;
pub(crate) use summary::summarize;

#[cfg(test)]
mod tests;

impl JobLease {
    /// Call within the checkpoint/completion transaction. The caller must roll
    /// back both chunks and report metadata if its final ownership fence fails.
    pub async fn archive_report(
        &self,
        connection: &mut PgConnection,
        report: &mut ImportReport,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(report.reconciles(), "import report does not reconcile");
        // PostgreSQL adds whitespace when rendering JSONB. Reserving half the
        // database budget bounds both encodings without another round trip.
        if serde_json::to_vec(&report)?.len() <= REPORT_BYTES / 2 {
            return Ok(());
        }
        // Only row errors are archived; a report without them (people and id
        // notices only) stays whole if it fits the database budget.
        if report.row_errors.is_empty() {
            anyhow::ensure!(
                serde_json::to_vec(report)?.len() <= REPORT_BYTES,
                "report metadata exceeds its budget"
            );
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
                    append_report_chunk(connection, self.id, self.org_id, sequence, &chunk).await?;
                    sequence = sequence
                        .checked_add(1)
                        .context("report archive is too large")?;
                    chunk.clear();
                }
            }
        }
        if !chunk.is_empty() {
            append_report_chunk(connection, self.id, self.org_id, sequence, &chunk).await?;
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
}

async fn append_report_chunk(
    connection: &mut PgConnection,
    job_id: Uuid,
    org_id: Uuid,
    sequence: i64,
    body: &[u8],
) -> anyhow::Result<()> {
    sqlx::query!(
        r#"INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body)
               VALUES ($1, $2, $3, $4, $5)"#,
        Uuid::now_v7(),
        job_id,
        org_id,
        sequence,
        body,
    )
    .execute(connection)
    .await?;
    Ok(())
}

/// A fixed-size read at a captured report boundary, never the archive's live end.
pub async fn chunks<'e, E: sqlx::PgExecutor<'e>>(
    exec: E,
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
    .fetch_all(exec)
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
