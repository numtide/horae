use super::*;
use crate::server_fns::ProjectReadAccess;
use horae_core::permissions::catalog::Permission;

pub(super) async fn read(
    db: &PgPool,
    caller: &AuthUser,
    filters: &ProjectFilters,
    project_id: Option<Uuid>,
    limit: i64,
    offset: i64,
) -> Result<(i64, Vec<ProjectRow>), (StatusCode, String)> {
    let mut access = ProjectReadAccess::begin(db, caller.org_id, caller.user_id)
        .await
        .map_err(|error| match error {
            dioxus::prelude::ServerFnError::ServerError { code, .. }
                if code == StatusCode::FORBIDDEN.as_u16() =>
            {
                forbidden("Project permission state is unavailable")
            }
            error => internal(error),
        })?
        .ok_or_else(|| forbidden("Project access is unavailable"))?;
    // Count and page must use the same visible set, including out-of-range pages.
    let records = sqlx::query!(
        r#"WITH visible AS MATERIALIZED (
             SELECT p.id,p.name,p.code,p.project_type::text project_type,p.active,
                    p.budget_kind::text budget_kind,
                    CASE WHEN $9 OR $12 OR ($13 AND management.id IS NOT NULL)
                      THEN p.budget_amount_cents END budget_amount_cents,
                    CASE WHEN $9 OR p.budget_kind='hours' THEN p.budget_minutes END budget_minutes,
                    p.starts_on,p.ends_on,p.created_at,p.client_id,c.name client_name
             FROM projects p JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
             LEFT JOIN project_read_access legacy ON legacy.org_id=p.org_id
               AND legacy.project_id=p.id AND legacy.user_id=$2
             LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
               AND management.project_id=p.id AND management.manager_id=$2
             LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
             LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
             WHERE p.org_id=$1 AND (CASE WHEN $9 THEN COALESCE(legacy.can_view_progress,false)
               ELSE $10 OR ($11 AND management.id IS NOT NULL)
                 OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility,'project_members')='project_members') END)
               AND ($3::boolean IS NULL OR p.active=$3)
               AND ($4::uuid IS NULL OR p.client_id=$4)
               AND ($5::timestamptz IS NULL OR p.created_at >= $5)
               AND ($6::uuid IS NULL OR p.id=$6)
           ), totals AS (SELECT COUNT(*) total FROM visible), page AS (
             SELECT * FROM visible ORDER BY name,id LIMIT $7 OFFSET $8
           )
           SELECT totals.total AS "total!",p.id AS "id?",p.name AS "name?",p.code,
                  p.project_type AS "project_type?",p.active AS "active?",p.budget_kind AS "budget_kind?",
                  p.budget_amount_cents,p.budget_minutes,p.starts_on AS "starts_on?: chrono::NaiveDate",
                  p.ends_on AS "ends_on?: chrono::NaiveDate",p.created_at AS "created_at?: chrono::DateTime<chrono::Utc>",
                  p.client_id AS "client_id?",p.client_name AS "client_name?"
           FROM totals LEFT JOIN page p ON true ORDER BY p.name,p.id"#,
        caller.org_id, caller.user_id, filters.is_active, filters.client_id,
        filters.updated_since as Option<DateTime<Utc>>, project_id, limit, offset,
        access.legacy(), access.has(Permission::ProjectReadAll), access.has(Permission::ProjectReadManaged),
        access.has(Permission::BillableRateReadAll), access.has(Permission::BillableRateReadManaged),
    ).fetch_all(&mut *access.tx).await.map_err(internal)?;
    let total = records
        .first()
        .ok_or_else(|| internal("Missing project count"))?
        .total;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let Some(id) = record.id else { continue };
        rows.push(ProjectRow {
            id,
            name: record
                .name
                .ok_or_else(|| internal("Missing project name"))?,
            code: record.code,
            project_type: record
                .project_type
                .ok_or_else(|| internal("Missing project type"))?,
            active: record
                .active
                .ok_or_else(|| internal("Missing project activity"))?,
            budget_kind: record
                .budget_kind
                .ok_or_else(|| internal("Missing budget kind"))?,
            budget_amount_cents: record.budget_amount_cents,
            budget_minutes: record.budget_minutes,
            starts_on: record.starts_on,
            ends_on: record.ends_on,
            created_at: record
                .created_at
                .ok_or_else(|| internal("Missing project creation date"))?,
            client_id: record
                .client_id
                .ok_or_else(|| internal("Missing project client"))?,
            client_name: record
                .client_name
                .ok_or_else(|| internal("Missing client name"))?,
        });
    }
    access.tx.commit().await.map_err(internal)?;
    Ok((total, rows))
}
