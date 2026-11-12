use super::*;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &TaskFilters,
    task_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<(i64, Vec<TaskRow>), ApiError> {
    let mut read = begin_read(db, caller, "Task").await?;
    // Count and rows share the authorized snapshot, even beyond the last page.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT t.id,t.harvest_id,t.name,t.active,t.billable_default,
                    CASE WHEN $7 THEN t.default_rate_cents END default_rate_cents,
                    t.default_rate_currency
             FROM tasks t
             WHERE t.org_id=$1 AND $6
               AND ($2::boolean IS NULL OR t.active=$2)
               AND ($3::uuid IS NULL OR t.id=$3)
           ), totals AS (SELECT COUNT(*) total FROM visible), page AS (
             SELECT * FROM visible ORDER BY name,id LIMIT $4 OFFSET $5
           )
           SELECT totals.total AS "total!",t.harvest_id AS "id?",t.name AS "name?",
                  t.active AS "active?",t.billable_default AS "billable_default?",t.default_rate_cents,
                  t.default_rate_currency
           FROM totals LEFT JOIN page t ON true ORDER BY t.name,t.id"#,
        caller.org_id,
        filters.is_active,
        task_id,
        limit,
        offset,
        read.actor.has(Permission::TaskReadAll),
        read.actor.has(Permission::BillableRateReadAll),
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    let total = records
        .first()
        .ok_or_else(|| internal("Missing task count"))?
        .total;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        // Only the row of an empty page has no name.
        let Some(name) = record.name else { continue };
        rows.push(TaskRow {
            id: record.id.ok_or_else(|| internal("Missing api number"))?,
            name,
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
    read.tx.commit().await.map_err(internal)?;
    Ok((total, rows))
}
