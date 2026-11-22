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
    let (cursor_at, cursor_id, before) = window.position();
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
           ), totals AS (
             SELECT COUNT(*) total, CASE WHEN $9::timestamptz IS NULL THEN $5
               WHEN $11 THEN GREATEST(COUNT(*) FILTER (WHERE (updated_at,id) < ($9,$10::uuid)) - $4, 0)
               ELSE COUNT(*) FILTER (WHERE (updated_at,id) <= ($9,$10::uuid)) END start
             FROM visible
           ), page AS (
             SELECT * FROM visible ORDER BY updated_at,id LIMIT $4 OFFSET (SELECT start FROM totals)
           )
           SELECT totals.total AS "total!",totals.start AS "start!",t.id AS "uuid?",
                  t.harvest_id AS "id?",t.name AS "name?",
                  t.active AS "active?",t.billable_default AS "billable_default?",t.default_rate_cents,
                  t.default_rate_currency,
                  t.created_at AS "created_at?: chrono::DateTime<chrono::Utc>",
                  t.updated_at AS "updated_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page t ON true ORDER BY t.updated_at,t.id"#,
        caller.org_id,
        filters.is_active,
        task_id,
        window.per_page,
        window.offset,
        read.actor.has(Permission::TaskReadAll),
        read.actor.has(Permission::BillableRateReadAll),
        filters.updated_since as Option<DateTime<Utc>>,
        cursor_at as Option<DateTime<Utc>>,
        cursor_id,
        before,
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
            key: Key {
                at: record
                    .updated_at
                    .ok_or_else(|| internal("Missing task update time"))?,
                id: uuid,
            },
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
