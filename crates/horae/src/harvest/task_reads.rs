use super::*;
use crate::server_fns::ProjectReadAccess;
use horae_core::permissions::catalog::Permission;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &TaskFilters,
    task_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<(i64, Vec<TaskRow>), (StatusCode, String)> {
    let mut access = ProjectReadAccess::begin(db, caller.org_id, caller.user_id)
        .await
        .map_err(|error| match error {
            dioxus::prelude::ServerFnError::ServerError { code, .. }
                if code == StatusCode::FORBIDDEN.as_u16() =>
            {
                forbidden("Task permission state is unavailable")
            }
            error => internal(error),
        })?
        .ok_or_else(|| forbidden("Task access is unavailable"))?;
    // Count and rows share the authorized snapshot, even beyond the last page.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT t.id,t.name,t.active,t.billable_default,
                    CASE WHEN (CASE WHEN $7 THEN legacy.can_view_rates ELSE $9 END)
                      THEN t.default_rate_cents END default_rate_cents
             FROM tasks t
             LEFT JOIN task_read_access legacy ON legacy.org_id=t.org_id
               AND legacy.task_id=t.id AND legacy.user_id=$2
             WHERE t.org_id=$1 AND (CASE WHEN $7 THEN legacy.user_id IS NOT NULL ELSE $8 END)
               AND ($3::boolean IS NULL OR t.active=$3)
               AND ($4::uuid IS NULL OR t.id=$4)
           ), totals AS (SELECT COUNT(*) total FROM visible), page AS (
             SELECT * FROM visible ORDER BY name,id LIMIT $5 OFFSET $6
           )
           SELECT totals.total AS "total!",t.id AS "id?",t.name AS "name?",
                  t.active AS "active?",t.billable_default AS "billable_default?",t.default_rate_cents
           FROM totals LEFT JOIN page t ON true ORDER BY t.name,t.id"#,
        caller.org_id,
        caller.user_id,
        filters.is_active,
        task_id,
        limit,
        offset,
        access.legacy(),
        access.has(Permission::TaskReadAll),
        access.has(Permission::BillableRateReadAll),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(internal)?;
    let total = records
        .first()
        .ok_or_else(|| internal("Missing task count"))?
        .total;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let Some(id) = record.id else { continue };
        rows.push(TaskRow {
            id,
            name: record.name.ok_or_else(|| internal("Missing task name"))?,
            active: record
                .active
                .ok_or_else(|| internal("Missing task activity"))?,
            billable_default: record
                .billable_default
                .ok_or_else(|| internal("Missing task billing flag"))?,
            default_rate_cents: record.default_rate_cents,
        });
    }
    access.tx.commit().await.map_err(internal)?;
    Ok((total, rows))
}
