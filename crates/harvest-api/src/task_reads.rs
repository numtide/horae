use super::*;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &TaskFilters,
    task_id: Option<Uuid>,
    window: &Window,
) -> Result<Listed<TaskRow>, ApiError> {
    let mut read = begin_read(db, caller, "Task").await?;
    let result = visible_tasks(&mut read, caller, filters, task_id, window).await;
    finish_read(read, result).await
}

async fn visible_tasks(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    filters: &TaskFilters,
    task_id: Option<Uuid>,
    window: &Window,
) -> Result<Listed<TaskRow>, ApiError> {
    let horizon = list_horizon(read, task_id).await?;
    let position = window.position();
    // Count and rows share the authorized snapshot, even beyond the last page.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT t.id,t.harvest_id,t.name,t.active,t.billable_default,
                    CASE WHEN $7 THEN t.default_rate_cents END default_rate_cents,
                    t.default_rate_currency,t.created_at,t.updated_at
             FROM tasks t
             WHERE t.org_id=$1 AND $6
               AND ($2::boolean IS NULL OR t.active=$2)
               AND ($3::uuid IS NULL OR t.id=$3)
               AND ($8::timestamptz IS NULL OR t.updated_at >= $8)
               AND ($11::timestamptz IS NULL OR t.updated_at < $11)
           ), totals AS (
             SELECT COUNT(*) total,
               CASE WHEN $9::uuid IS NULL THEN $5
                 WHEN $10 THEN GREATEST(COUNT(*) FILTER (WHERE id > $9) - $4, 0)
                 ELSE COUNT(*) FILTER (WHERE id >= $9) END start,
               CASE WHEN $9::uuid IS NOT NULL AND $10
                 THEN LEAST(COUNT(*) FILTER (WHERE id > $9), $4) ELSE $4 END take
             FROM visible
           ), page AS (
             SELECT * FROM visible ORDER BY id DESC
             LIMIT (SELECT take FROM totals) OFFSET (SELECT start FROM totals)
           )
           SELECT totals.total AS "total!",totals.start AS "start!",t.id AS "uuid?",
                  t.harvest_id AS "id?",t.name AS "name?",
                  t.active AS "active?",t.billable_default AS "billable_default?",t.default_rate_cents,
                  t.default_rate_currency,
                  t.created_at AS "created_at?: chrono::DateTime<chrono::Utc>",
                  t.updated_at AS "updated_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page t ON true ORDER BY t.id DESC"#,
        caller.org_id,
        filters.is_active,
        task_id,
        window.per_page,
        window.offset,
        read.actor.has(Permission::TaskReadAll),
        read.actor.has(Permission::BillableRateReadAll),
        filters.updated_since as Option<DateTime<Utc>>,
        position.id,
        position.before,
        horizon as Option<DateTime<Utc>>,
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    let first = records
        .first()
        .ok_or_else(|| internal("Missing task count"))?;
    let (total, start) = (first.total, first.start);
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        // Only the row of an empty page has no id.
        let Some(uuid) = record.uuid else { continue };
        rows.push(TaskRow {
            uuid,
            updated_at: record
                .updated_at
                .ok_or_else(|| internal("Missing task update time"))?,
            created_at: record
                .created_at
                .ok_or_else(|| internal("Missing task creation time"))?,
            id: record.id.ok_or_else(|| internal("Missing api number"))?,
            name: record.name.ok_or_else(|| internal("Missing task name"))?,
            active: record
                .active
                .ok_or_else(|| internal("Missing task activity"))?,
            billable_default: record
                .billable_default
                .ok_or_else(|| internal("Missing task billing flag"))?,
            default_rate_cents: record.default_rate_cents,
            default_rate_currency: record.default_rate_currency,
        });
    }
    Ok(Listed { total, start, rows })
}
