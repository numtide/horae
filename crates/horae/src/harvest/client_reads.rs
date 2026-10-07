use super::*;
use crate::server_fns::ProjectReadAccess;
use horae_core::permissions::catalog::Permission;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &ClientFilters,
    client_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<(i64, Vec<ClientRow>), (StatusCode, String)> {
    let mut access = ProjectReadAccess::begin(db, caller.org_id, caller.user_id)
        .await
        .map_err(|error| match error {
            dioxus::prelude::ServerFnError::ServerError { code, .. }
                if code == StatusCode::FORBIDDEN.as_u16() =>
            {
                forbidden("Client permission state is unavailable")
            }
            error => internal(error),
        })?
        .ok_or_else(|| forbidden("Client access is unavailable"))?;
    // Count and page share one source snapshot, including exhausted pages.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT id,name,active,address,currency,created_at FROM clients
             WHERE org_id=$1 AND $5
               AND ($2::bool IS NULL OR active=$2)
               AND ($3::timestamptz IS NULL OR created_at >= $3)
               AND ($4::uuid IS NULL OR id=$4)
           ), totals AS (SELECT COUNT(*) total FROM visible), page AS (
             SELECT * FROM visible ORDER BY name,id LIMIT $6 OFFSET $7
           )
           SELECT totals.total AS "total!",c.id AS "id?",c.name AS "name?",
                  c.active AS "active?",c.address,c.currency AS "currency?",
                  c.created_at AS "created_at?: chrono::DateTime<chrono::Utc>"
           FROM totals LEFT JOIN page c ON true ORDER BY c.name,c.id"#,
        caller.org_id,
        filters.is_active,
        filters.updated_since as Option<DateTime<Utc>>,
        client_id,
        access.legacy() || access.has(Permission::ClientReadAll),
        limit,
        offset,
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(internal)?;
    let total = records
        .first()
        .ok_or_else(|| internal("Missing client count"))?
        .total;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let Some(id) = record.id else { continue };
        rows.push(ClientRow {
            id,
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
        });
    }
    access.tx.commit().await.map_err(internal)?;
    Ok((total, rows))
}
