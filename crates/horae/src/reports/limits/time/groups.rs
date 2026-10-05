//! Grouped workbooks retain the scope of every contributing time entry.

use super::*;
use crate::models::time_report::{TimeReportGroup, TimeReportGrouping, TimeReportTotals};

pub(in crate::reports) struct GroupExport {
    pub rows: Vec<TimeReportGroup>,
    pub scope: TimeExportScope,
}

pub(in crate::reports) async fn read(
    pool: &PgPool,
    requester: PermissionRequester,
    query: &TimeReportQuery,
    dimension: TimeReportGrouping,
) -> Result<GroupExport, StatusCode> {
    if query
        .expected_requester
        .is_some_and(|expected| expected != requester)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if query.date_from > query.date_to || query.after.is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let (mut tx, policy, grants) = begin(pool, requester).await?;
    if policy != 1 {
        return Err(StatusCode::FORBIDDEN);
    }
    let dimension = match dimension {
        TimeReportGrouping::Client => "client",
        TimeReportGrouping::Project => "project",
        TimeReportGrouping::Task => "task",
        TimeReportGrouping::Person => "person",
    };
    // Both bounded payloads derive from the complete source in one snapshot.
    // Scope tuples are separate records, never a repeated array per group.
    let records = sqlx::query!(
        r#"WITH scoped AS MATERIALIZED (
             SELECT CASE $13::text WHEN 'client' THEN c.id WHEN 'project' THEN p.id
                      WHEN 'task' THEN t.id WHEN 'person' THEN u.id END group_id,
                    CASE $13::text WHEN 'client' THEN c.name WHEN 'project' THEN p.name
                      WHEN 'task' THEN t.name WHEN 'person' THEN u.name END name,
                    e.user_id,e.project_id,e.minutes,
                    effective_minutes(e.minutes,e.rounded_minutes,o.round_minutes,o.round_dir) rounded_minutes,
                    (e.billable AND (e.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                      AND COALESCE(pt.billable,t.billable_default)))) billable
             FROM time_entries e JOIN organizations o ON o.id=e.org_id
             JOIN users u ON u.id=e.user_id AND u.org_id=e.org_id
             JOIN projects p ON p.id=e.project_id AND p.org_id=e.org_id
             JOIN tasks t ON t.id=e.task_id AND t.org_id=e.org_id
             JOIN clients c ON c.id=p.client_id AND c.org_id=e.org_id
             LEFT JOIN project_tasks pt ON pt.project_id=p.id AND pt.task_id=t.id
             WHERE e.org_id=$1 AND (
               $5::bool OR ($3::bool AND e.user_id=$2) OR ($4::bool AND (
                 EXISTS (SELECT 1 FROM person_management_assignments m
                   WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.managed_user_id=e.user_id)
                 OR EXISTS (SELECT 1 FROM project_management_assignments m
                   WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.project_id=e.project_id))))
               AND e.spent_date BETWEEN $6 AND $7
               AND (cardinality($8::uuid[])=0 OR p.client_id=ANY($8))
               AND (cardinality($9::uuid[])=0 OR e.project_id=ANY($9))
               AND (cardinality($10::uuid[])=0 OR e.user_id=ANY($10))
               AND (cardinality($11::uuid[])=0 OR e.task_id=ANY($11))
               AND (cardinality($12::uuid[])=0 OR EXISTS (
                 SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
                 WHERE l.org_id=e.org_id AND l.project_id=e.project_id AND tag.id=ANY($12)))
           ), grouped AS MATERIALIZED (
             SELECT group_id,name,COUNT(*) entry_count,SUM(minutes)::bigint total_minutes,
                    SUM(rounded_minutes)::bigint rounded_minutes,
                    COALESCE(SUM(rounded_minutes) FILTER (WHERE billable),0)::bigint billable_minutes
             FROM scoped GROUP BY group_id,name
             ORDER BY name COLLATE "C",group_id LIMIT $14::bigint + 1
           ), contexts AS MATERIALIZED (
             SELECT DISTINCT user_id,project_id FROM scoped LIMIT ($15::bigint / 32) + 1
           ), stats AS (
             SELECT COUNT(*) rows,
                    COALESCE(SUM(octet_length(name)::bigint),0)::bigint
                      + (SELECT COUNT(*) * 32 FROM contexts) bytes,
                    COALESCE(MAX(octet_length(name)),0) field_bytes FROM grouped
           ), payload AS (
             SELECT 1 kind,group_id,name,entry_count,total_minutes,rounded_minutes,billable_minutes,
                    NULL::uuid user_id,NULL::uuid project_id FROM grouped
             UNION ALL
             SELECT 2,NULL,NULL,NULL,NULL,NULL,NULL,user_id,project_id FROM contexts
           )
           SELECT s.rows AS "rows!",s.bytes AS "bytes!",s.field_bytes AS "field_bytes!",
                  p.kind AS "kind?",p.group_id AS "group_id?",p.name AS "name?",
                  p.entry_count AS "entry_count?",p.total_minutes AS "total_minutes?",
                  p.rounded_minutes AS "rounded_minutes?",p.billable_minutes AS "billable_minutes?",
                  p.user_id AS "user_id?",p.project_id AS "project_id?"
           FROM stats s LEFT JOIN payload p ON s.rows <= $14 AND s.bytes <= $15 AND s.field_bytes <= $16
           ORDER BY p.kind,p.name COLLATE "C",p.group_id,p.user_id,p.project_id"#,
        requester.org_id, requester.user_id, grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged), grants.contains(Permission::TimeReadAll),
        query.date_from as _, query.date_to as _, &query.client_ids, &query.project_ids,
        &query.user_ids, &query.task_ids, &query.tag_ids, dimension, XLSX.rows, XLSX.bytes, MAX_FIELD_BYTES,
    ).fetch_all(&mut *tx).await.map_err(database_error)?;
    let mut rows = Vec::new();
    let mut contexts = Vec::new();
    for record in records {
        check(record.rows, record.bytes, record.field_bytes, XLSX)?;
        match record.kind {
            Some(1) => rows.push(TimeReportGroup {
                id: record.group_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                name: record.name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                totals: TimeReportTotals {
                    entry_count: record
                        .entry_count
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    total_minutes: record
                        .total_minutes
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    rounded_minutes: record
                        .rounded_minutes
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    billable_minutes: record
                        .billable_minutes
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                },
            }),
            Some(2) => contexts.push((
                record.user_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                record.project_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            )),
            None if record.rows == 0 && record.bytes == 0 => {}
            _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
    tx.commit().await.map_err(database_error)?;
    Ok(GroupExport {
        rows,
        scope: TimeExportScope {
            requester,
            policy,
            contexts,
        },
    })
}
