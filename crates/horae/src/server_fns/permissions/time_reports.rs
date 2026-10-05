//! Ordinary time reports share entry scope, not directory or financial authority.

pub(crate) mod groups;

use horae_core::permissions::catalog::Permission;
use sqlx::PgPool;
use uuid::Uuid;

use super::time_entries::{TimeReadError, begin_read};
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{
    TimeReportCursor, TimeReportEntry, TimeReportPage, TimeReportQuery, TimeReportTotals,
};

pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeReportQuery,
) -> Result<TimeReportPage, TimeReadError> {
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
        || query.after.as_ref().is_some_and(|cursor| {
            cursor.spent_date < query.date_from
                || cursor.spent_date > query.date_to
                || cursor.project_name.contains('\0')
                || cursor.task_name.contains('\0')
        })
    {
        return Err(TimeReadError::InvalidQuery);
    }
    // One snapshot keeps full-period totals consistent with this page. Inline
    // the relation so aggregating minutes need not materialize every entry's text.
    let records = sqlx::query!(
        r#"WITH scoped AS NOT MATERIALIZED (
           SELECT e.id, e.spent_date, p.name AS project_name,
                  t.name AS task_name, u.name AS user_name, e.minutes,
                  effective_minutes(e.minutes,e.rounded_minutes,o.round_minutes,o.round_dir) AS rounded_minutes,
                  (e.billable AND (e.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                    AND COALESCE(pt.billable,t.billable_default)))) AS billable, e.notes
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
             AND (cardinality($8::uuid[])=0 OR p.client_id=ANY($8))
             AND (cardinality($9::uuid[])=0 OR e.project_id=ANY($9))
             AND (cardinality($10::uuid[])=0 OR e.user_id=ANY($10))
             AND (cardinality($11::uuid[])=0 OR e.task_id=ANY($11))
             AND (cardinality($12::uuid[])=0 OR EXISTS (
               SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
               WHERE l.org_id=e.org_id AND l.project_id=e.project_id AND tag.id=ANY($12)))
           ), stats AS (
             SELECT COUNT(*) AS entry_count,
                    COALESCE(SUM(minutes),0)::bigint AS total_minutes,
                    COALESCE(SUM(rounded_minutes),0)::bigint AS rounded_minutes,
                    COALESCE(SUM(rounded_minutes) FILTER (WHERE billable),0)::bigint AS billable_minutes
             FROM scoped
           ), page AS (
             SELECT * FROM scoped
             WHERE ($13::date IS NULL OR (spent_date,project_name COLLATE "C",task_name COLLATE "C",id)
               > ($13,$14::text COLLATE "C",$15::text COLLATE "C",$16::uuid))
             ORDER BY spent_date,project_name COLLATE "C",task_name COLLATE "C",id LIMIT 501
           )
           SELECT s.entry_count AS "entry_count!", s.total_minutes AS "total_minutes!",
                  s.rounded_minutes AS "total_rounded_minutes!", s.billable_minutes AS "billable_minutes!",
                  p.id AS "id?", p.spent_date AS "spent_date?: chrono::NaiveDate",
                  p.project_name AS "project_name?", p.task_name AS "task_name?", p.user_name AS "user_name?",
                  p.minutes AS "minutes?", p.rounded_minutes AS "rounded_minutes?", p.billable AS "billable?", p.notes
           FROM stats s LEFT JOIN page p ON true
           ORDER BY p.spent_date,p.project_name COLLATE "C",p.task_name COLLATE "C",p.id"#,
        org_id, actor_id, grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged), grants.contains(Permission::TimeReadAll),
        query.date_from as _, query.date_to as _, &query.client_ids, &query.project_ids,
        &query.user_ids, &query.task_ids, &query.tag_ids,
        query.after.as_ref().map(|cursor| cursor.spent_date) as _,
        query.after.as_ref().map(|cursor| cursor.project_name.as_str()),
        query.after.as_ref().map(|cursor| cursor.task_name.as_str()),
        query.after.as_ref().map(|cursor| cursor.id),
    ).fetch_all(&mut *tx).await?;
    let stats = records.first().ok_or(TimeReadError::Unavailable)?;
    let totals = TimeReportTotals {
        entry_count: stats.entry_count,
        total_minutes: stats.total_minutes,
        rounded_minutes: stats.total_rounded_minutes,
        billable_minutes: stats.billable_minutes,
    };
    let mut entries = Vec::with_capacity(records.len());
    for record in records {
        let Some(id) = record.id else { continue };
        entries.push(TimeReportEntry {
            id,
            spent_date: record.spent_date.ok_or(TimeReadError::Unavailable)?,
            project_name: record.project_name.ok_or(TimeReadError::Unavailable)?,
            task_name: record.task_name.ok_or(TimeReadError::Unavailable)?,
            user_name: record.user_name.ok_or(TimeReadError::Unavailable)?,
            minutes: record.minutes.ok_or(TimeReadError::Unavailable)?,
            rounded_minutes: record.rounded_minutes.ok_or(TimeReadError::Unavailable)?,
            billable: record.billable.ok_or(TimeReadError::Unavailable)?,
            notes: record.notes,
        });
    }
    let next_after = if entries.len() > 500 {
        entries.truncate(500);
        entries.last().map(|entry| TimeReportCursor {
            spent_date: entry.spent_date,
            project_name: entry.project_name.clone(),
            task_name: entry.task_name.clone(),
            id: entry.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(TimeReportPage {
        requester,
        entries,
        next_after,
        totals,
    })
}
