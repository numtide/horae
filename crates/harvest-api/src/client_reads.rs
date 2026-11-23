use super::*;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &ClientFilters,
    client_id: Option<Uuid>,
    window: &Window,
) -> Result<Listed<ClientRow>, ApiError> {
    let mut read = begin_read(db, caller, "Client").await?;
    let result = visible_clients(&mut read, caller, filters, client_id, window).await;
    finish_read(read, result).await
}

async fn visible_clients(
    read: &mut ops::ReadTx,
    caller: &AuthUser,
    filters: &ClientFilters,
    client_id: Option<Uuid>,
    window: &Window,
) -> Result<Listed<ClientRow>, ApiError> {
    let horizon = list_horizon(read, client_id).await?;
    let position = window.position();
    // Count and page share one source snapshot, including exhausted pages.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT id,harvest_id,name,active,address,currency,created_at,updated_at FROM clients
             WHERE org_id=$1 AND $5
               AND ($2::bool IS NULL OR active=$2)
               AND ($3::timestamptz IS NULL OR updated_at >= $3)
               AND ($4::uuid IS NULL OR id=$4)
               AND ($10::timestamptz IS NULL OR updated_at < $10)
           ), totals AS (
             SELECT COUNT(*) total,
               CASE WHEN $8::uuid IS NULL THEN $7
                 WHEN $9 THEN GREATEST(COUNT(*) FILTER (WHERE id > $8) - $6, 0)
                 ELSE COUNT(*) FILTER (WHERE id >= $8) END start,
               CASE WHEN $8::uuid IS NOT NULL AND $9
                 THEN LEAST(COUNT(*) FILTER (WHERE id > $8), $6) ELSE $6 END take
             FROM visible
           ), page AS (
             SELECT * FROM visible ORDER BY id DESC
             LIMIT (SELECT take FROM totals) OFFSET (SELECT start FROM totals)
           )
           SELECT totals.total AS "total!",totals.start AS "start!",c.id AS "uuid?",
                  c.harvest_id AS "id?",c.name AS "name?",
                  c.active AS "active?",c.address,c.currency AS "currency?",
                  c.created_at AS "created_at?: chrono::DateTime<chrono::Utc>",
                  c.updated_at AS "updated_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page c ON true ORDER BY c.id DESC"#,
        caller.org_id,
        filters.is_active,
        filters.updated_since as Option<DateTime<Utc>>,
        client_id,
        read.actor.has(Permission::ClientReadAll),
        window.per_page,
        window.offset,
        position.id,
        position.before,
        horizon as Option<DateTime<Utc>>,
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    let first = records
        .first()
        .ok_or_else(|| internal("Missing client count"))?;
    let (total, start) = (first.total, first.start);
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        // Only the row of an empty page has no id.
        let Some(uuid) = record.uuid else { continue };
        rows.push(ClientRow {
            uuid,
            id: record.id.ok_or_else(|| internal("Missing api number"))?,
            name: record.name.ok_or_else(|| internal("Missing client name"))?,
            active: record
                .active
                .ok_or_else(|| internal("Missing client activity"))?,
            address: record.address,
            currency: record
                .currency
                .ok_or_else(|| internal("Missing client currency"))?,
            created_at: record
                .created_at
                .ok_or_else(|| internal("Missing client timestamp"))?,
            updated_at: record
                .updated_at
                .ok_or_else(|| internal("Missing client update time"))?,
        });
    }
    Ok(Listed { total, start, rows })
}
