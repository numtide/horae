//! Group only currently readable time; entity labels are not directory access.

use horae_core::permissions::catalog::Permission;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{
    TimeReportGroup, TimeReportGroupCursor, TimeReportGroupPage, TimeReportGroupQuery,
    TimeReportGrouping, TimeReportTotals,
};
use crate::server_fns::permissions::time_entries::{TimeReadError, begin_read};

pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeReportGroupQuery,
) -> Result<TimeReportGroupPage, TimeReadError> {
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    if query
        .expected_requester
        .is_some_and(|expected| expected != requester)
    {
        return Err(TimeReadError::Forbidden);
    }
    let (mut tx, grants) = begin_read(pool, org_id, actor_id).await?;
    if query.date_from > query.date_to
        || query
            .after
            .as_ref()
            .is_some_and(|cursor| cursor.group_by != query.group_by || cursor.name.contains('\0'))
    {
        return Err(TimeReadError::InvalidQuery);
    }
    let dimension = match query.group_by {
        TimeReportGrouping::Client => "client",
        TimeReportGrouping::Project => "project",
        TimeReportGrouping::Task => "task",
        TimeReportGrouping::Person => "person",
    };
    // One statement keeps totals and grouped rows on the same data snapshot.
    // Never aggregate a fetched detail page or count groups as time entries.
    let rows = sqlx::query!(
        r#"WITH scoped AS (
           SELECT CASE $13::text WHEN 'client' THEN c.id WHEN 'project' THEN p.id
                    WHEN 'task' THEN t.id WHEN 'person' THEN u.id END AS group_id,
                  CASE $13::text WHEN 'client' THEN c.name WHEN 'project' THEN p.name
                    WHEN 'task' THEN t.name WHEN 'person' THEN u.name END AS name,
                  e.minutes,
                  effective_minutes(e.minutes,e.rounded_minutes,o.round_minutes,o.round_dir) AS rounded_minutes,
                  (e.billable AND (e.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                    AND COALESCE(pt.billable,t.billable_default)))) AS billable
           FROM time_entries e
           JOIN organizations o ON o.id=e.org_id
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
             AND (NOT $16::bool OR p.active)
             AND (cardinality($8::uuid[])=0 OR p.client_id=ANY($8))
             AND (cardinality($9::uuid[])=0 OR e.project_id=ANY($9))
             AND (cardinality($10::uuid[])=0 OR e.user_id=ANY($10))
             AND (cardinality($11::uuid[])=0 OR e.task_id=ANY($11))
             AND (cardinality($12::uuid[])=0 OR EXISTS (
               SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
               WHERE l.org_id=e.org_id AND l.project_id=e.project_id AND tag.id=ANY($12)))
           ), grouped AS (
             SELECT group_id,name,COUNT(*) AS entry_count,SUM(minutes)::bigint AS total_minutes,
                    SUM(rounded_minutes)::bigint AS rounded_minutes,
                    COALESCE(SUM(rounded_minutes) FILTER (WHERE billable),0)::bigint AS billable_minutes
             FROM scoped GROUP BY group_id,name
           ), stats AS (
             SELECT COALESCE(SUM(entry_count),0)::bigint AS entry_count,
                    COALESCE(SUM(total_minutes),0)::bigint AS total_minutes,
                    COALESCE(SUM(rounded_minutes),0)::bigint AS rounded_minutes,
                    COALESCE(SUM(billable_minutes),0)::bigint AS billable_minutes FROM grouped
           ), page AS (
             SELECT * FROM grouped
             WHERE ($14::text IS NULL OR (name COLLATE "C",group_id) > ($14 COLLATE "C",$15::uuid))
             ORDER BY name COLLATE "C",group_id LIMIT 501
           )
           SELECT s.entry_count AS "entry_count!",s.total_minutes AS "total_minutes!",
                  s.rounded_minutes AS "rounded_minutes!",s.billable_minutes AS "billable_minutes!",
                  p.group_id AS "id?",p.name AS "name?",p.entry_count AS "group_entry_count?",
                  p.total_minutes AS "group_total_minutes?",p.rounded_minutes AS "group_rounded_minutes?",
                  p.billable_minutes AS "group_billable_minutes?"
           FROM stats s LEFT JOIN page p ON true ORDER BY p.name COLLATE "C",p.group_id"#,
        org_id, actor_id, grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged), grants.contains(Permission::TimeReadAll),
        query.date_from as _, query.date_to as _, &query.client_ids, &query.project_ids,
        &query.user_ids, &query.task_ids, &query.tag_ids, dimension,
        query.after.as_ref().map(|cursor| cursor.name.as_str()),
        query.after.as_ref().map(|cursor| cursor.id),
        query.active_projects_only,
    ).fetch_all(&mut *tx).await?;
    let stats = rows.first().ok_or(TimeReadError::Unavailable)?;
    let totals = TimeReportTotals {
        entry_count: stats.entry_count,
        total_minutes: stats.total_minutes,
        rounded_minutes: stats.rounded_minutes,
        billable_minutes: stats.billable_minutes,
    };
    let mut groups = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(id) = row.id else { continue };
        groups.push(TimeReportGroup {
            id,
            name: row.name.ok_or(TimeReadError::Unavailable)?,
            totals: TimeReportTotals {
                entry_count: row.group_entry_count.ok_or(TimeReadError::Unavailable)?,
                total_minutes: row.group_total_minutes.ok_or(TimeReadError::Unavailable)?,
                rounded_minutes: row
                    .group_rounded_minutes
                    .ok_or(TimeReadError::Unavailable)?,
                billable_minutes: row
                    .group_billable_minutes
                    .ok_or(TimeReadError::Unavailable)?,
            },
        });
    }
    let next_after = if groups.len() > 500 {
        groups.truncate(500);
        groups.last().map(|group| TimeReportGroupCursor {
            group_by: query.group_by,
            name: group.name.clone(),
            id: group.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(TimeReportGroupPage {
        requester,
        groups,
        next_after,
        totals,
    })
}
