//! Upgrade persisted reports before exposing them to workers or status readers.

use anyhow::Context;
use horae_core::importers::harvest::types::ImportReport;
use serde_json::json;
use sqlx::PgPool;

use super::{CHUNK_BYTES, REPORT_BYTES, append_report_chunk};

/// One locked job per transaction, using one connection even with a size-one
/// pool. No full legacy error array or checkpoint crosses the database boundary.
pub(crate) async fn upgrade_legacy_reports(pool: &PgPool) -> anyhow::Result<()> {
    loop {
        let mut tx = pool.begin().await?;
        let Some(job) = sqlx::query!(
            r#"SELECT id, org_id FROM horae_jobs
               WHERE octet_length(report::text) > $1
                  OR octet_length((checkpoint->'report')::text) > $1
               ORDER BY id LIMIT 1 FOR UPDATE"#,
            REPORT_BYTES as i32,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            tx.commit().await?;
            return Ok(());
        };
        let header = sqlx::query!(
            r#"WITH current AS (
                 SELECT COALESCE(checkpoint->'report', report) AS report,
                        checkpoint IS NULL OR checkpoint->>'version' IN ('1', '2') AS compatible
                 FROM horae_jobs WHERE id = $1 AND org_id = $2
               )
               SELECT CASE WHEN octet_length((report - 'row_errors')::text) <= $3
                           THEN report - 'row_errors' END AS metadata,
                      jsonb_array_length(report->'row_errors') AS errors, compatible
               FROM current"#,
            job.id,
            job.org_id,
            REPORT_BYTES as i32,
        )
        .fetch_one(&mut *tx)
        .await?;
        anyhow::ensure!(
            header.compatible == Some(true),
            "unsupported legacy checkpoint"
        );
        let mut metadata = header
            .metadata
            .context("legacy report header exceeds its budget")?;
        anyhow::ensure!(metadata.is_object(), "legacy report must be an object");
        let count = header.errors.context("legacy report has no error array")?;
        let version = match metadata.get("version") {
            None => 1,
            Some(value) => value.as_u64().context("invalid legacy report version")?,
        };
        anyhow::ensure!(
            matches!(version, 1 | 2),
            "unsupported legacy report version"
        );
        let archive = metadata
            .get("error_archive")
            .filter(|value| !value.is_null());
        let (archived, mut sequence) = match archive {
            Some(archive) => {
                anyhow::ensure!(version == 2, "legacy archive requires report version 2");
                let count = archive["count"]
                    .as_u64()
                    .context("invalid archived error count")?;
                let chunks = archive["chunks"]
                    .as_u64()
                    .context("invalid archived chunk count")?;
                anyhow::ensure!(count > 0 && chunks > 0, "invalid legacy archive");
                (count, i64::try_from(chunks)?)
            }
            None => (0, 0),
        };
        metadata["version"] = json!(version);
        metadata["row_errors"] = json!([]);
        if count > 0 {
            metadata["version"] = json!(2);
            metadata["error_archive"] = json!({
                "count": archived.checked_add(count as u64).context("report count overflow")?,
                "chunks": sequence.checked_add(1).context("report archive is too large")?,
            });
        }
        // Validate independent summary totals before publishing any replacement.
        let _: ImportReport = serde_json::from_value(metadata.clone())?;
        let mut record = 0_i32;
        let mut offset = 1_i32;
        let mut chunk = Vec::with_capacity(CHUNK_BYTES);
        while record < count {
            let page = sqlx::query!(
                r#"WITH source AS MATERIALIZED (
                     SELECT COALESCE(checkpoint->'report', report)->'row_errors' AS errors
                     FROM horae_jobs WHERE id = $1 AND org_id = $2
                   ), records AS MATERIALIZED (
                     SELECT position, errors->position AS error
                     FROM source, LATERAL generate_series(
                         $3, LEAST($3 + 15, jsonb_array_length(errors) - 1)
                     ) AS position
                   ), encoded AS MATERIALIZED (
                     SELECT position,
                            convert_to(error::text, 'UTF8') || decode('0a', 'hex') AS body,
                            COALESCE(jsonb_typeof(error) = 'object'
                                AND jsonb_typeof(error->'source_location') = 'string'
                                AND jsonb_typeof(error->'reason') = 'string'
                                AND error->>'entity' IN ('client', 'project', 'task', 'time_entry'),
                                false) AS valid
                     FROM records
                   )
                   SELECT position AS "position!", start AS "start!",
                          substring(body FROM start FOR 65536) AS "body!",
                          octet_length(body) AS "size!", valid AS "valid!"
                   FROM encoded, LATERAL generate_series(
                       CASE WHEN position = $3 THEN $4 ELSE 1 END,
                       octet_length(body), 65536
                   ) AS start
                   ORDER BY position, start LIMIT 16"#,
                job.id,
                job.org_id,
                record,
                offset,
            )
            .fetch_all(&mut *tx)
            .await?;
            anyhow::ensure!(!page.is_empty(), "legacy report archive is incomplete");
            for part in page {
                anyhow::ensure!(part.valid, "invalid legacy row error");
                anyhow::ensure!(
                    part.position == record && part.start == offset,
                    "legacy report archive is out of order"
                );
                let mut bytes = part.body.as_slice();
                while !bytes.is_empty() {
                    let take = (CHUNK_BYTES - chunk.len()).min(bytes.len());
                    chunk.extend_from_slice(&bytes[..take]);
                    bytes = &bytes[take..];
                    if chunk.len() == CHUNK_BYTES {
                        append_report_chunk(&mut tx, job.id, job.org_id, sequence, &chunk).await?;
                        sequence = sequence
                            .checked_add(1)
                            .context("report archive is too large")?;
                        chunk.clear();
                    }
                }
                offset = offset
                    .checked_add(i32::try_from(part.body.len())?)
                    .context("legacy row error is too large")?;
                if offset > part.size {
                    record += 1;
                    offset = 1;
                }
            }
        }
        if !chunk.is_empty() {
            append_report_chunk(&mut tx, job.id, job.org_id, sequence, &chunk).await?;
            sequence = sequence
                .checked_add(1)
                .context("report archive is too large")?;
        }
        if count > 0 {
            metadata["error_archive"]["chunks"] = json!(sequence);
        }
        let report: ImportReport = serde_json::from_value(metadata.clone())?;
        anyhow::ensure!(
            serde_json::to_vec(&metadata)?.len() <= REPORT_BYTES,
            "report metadata exceeds its budget"
        );
        let archived = report.archived_error_count() > 0;
        let updated = sqlx::query!(
            r#"UPDATE horae_jobs SET report = $3,
                 checkpoint = CASE WHEN checkpoint IS NULL THEN NULL
                     WHEN $4 THEN jsonb_set(jsonb_set(checkpoint, '{report}', $3), '{version}', '2')
                     ELSE jsonb_set(checkpoint, '{report}', $3) END,
                 claim_token = NULL,
                 lease_until = CASE WHEN status = 'running' THEN clock_timestamp() ELSE lease_until END
               WHERE id = $1 AND org_id = $2 AND octet_length($3::jsonb::text) <= $5"#,
            job.id,
            job.org_id,
            metadata,
            archived,
            REPORT_BYTES as i32,
        )
        .execute(&mut *tx)
        .await?;
        anyhow::ensure!(
            updated.rows_affected() == 1,
            "report metadata exceeds its database budget"
        );
        tx.commit().await?;
    }
}
