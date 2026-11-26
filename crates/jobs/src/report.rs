//! Append-only storage of a job's report detail, in fixed-size chunks.

use sqlx::PgConnection;
use uuid::Uuid;

pub const CHUNK_BYTES: usize = 64 * 1024;

/// Call within the transaction that saves the report pointing at the chunk.
pub async fn append_chunk(
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
