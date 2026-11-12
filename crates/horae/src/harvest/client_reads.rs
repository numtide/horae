use super::*;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &ClientFilters,
    client_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<(i64, Vec<ClientRow>), ApiError> {
    let mut read = begin_read(db, caller, "Client").await?;
    // Count and page share one source snapshot, including exhausted pages.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT id,harvest_id,name,active,address,currency,created_at FROM clients
             WHERE org_id=$1 AND $5
               AND ($2::bool IS NULL OR active=$2)
               AND ($3::timestamptz IS NULL OR created_at >= $3)
               AND ($4::uuid IS NULL OR id=$4)
           ), totals AS (SELECT COUNT(*) total FROM visible), page AS (
             SELECT * FROM visible ORDER BY name,id LIMIT $6 OFFSET $7
           )
           SELECT totals.total AS "total!",c.harvest_id AS "id?",c.name AS "name?",
                  c.active AS "active?",c.address,c.currency AS "currency?",
                  c.created_at AS "created_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page c ON true ORDER BY c.name,c.id"#,
        caller.org_id,
        filters.is_active,
        filters.updated_since as Option<DateTime<Utc>>,
        client_id,
        read.actor.has(Permission::ClientReadAll),
        limit,
        offset,
    )
    .fetch_all(&mut *read.tx)
    .await
    .map_err(internal)?;
    let total = records
        .first()
        .ok_or_else(|| internal("Missing client count"))?
        .total;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        // Only the row of an empty page has no name.
        let Some(name) = record.name else { continue };
        rows.push(ClientRow {
            id: record.id.ok_or_else(|| internal("Missing api number"))?,
            name,
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
        });
    }
    read.tx.commit().await.map_err(internal)?;
    Ok((total, rows))
}
