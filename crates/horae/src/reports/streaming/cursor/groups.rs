//! One original scope pair per fragment; no group-sized context arrays.

use super::*;
use crate::models::time_report::{TimeReportGrouping, TimeReportQuery};
use crate::server_fns::StoredPersonPermissions;
use horae_core::permissions::catalog::Permission;

pub(in crate::reports::streaming) async fn declare(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeReportQuery,
    dimension: TimeReportGrouping,
) -> Result<(), StatusCode> {
    let dimension = match dimension {
        TimeReportGrouping::Client => "client",
        TimeReportGrouping::Project => "project",
        TimeReportGrouping::Task => "task",
        TimeReportGrouping::Person => "person",
    };
    sqlx::query!(
        r#"DECLARE horae_csv NO SCROLL CURSOR FOR
           WITH authority AS MATERIALIZED (
             SELECT o.permission_policy_version policy,u.active,
                    s.catalog_version,s.grants,s.is_administrator,s.source permission_source,
                    s.built_in_profile,s.template_id,s.applied_template_revision,s.revision,
                    COALESCE(octet_length(s.source),0)::bigint + COALESCE(octet_length(s.built_in_profile),0)::bigint
                      + COALESCE((SELECT SUM(octet_length(g)::bigint) FROM unnest(s.grants) g),0)::bigint authority_bytes
             FROM (VALUES ($1::uuid,$2::uuid)) identity(org_id,user_id)
             LEFT JOIN organizations o ON o.id=identity.org_id
             LEFT JOIN users u ON u.id=identity.user_id AND u.org_id=identity.org_id
             LEFT JOIN person_permission_states s ON s.org_id=o.id AND s.user_id=u.id
           ), scoped AS (
             SELECT CASE $10::text WHEN 'client' THEN c.id WHEN 'project' THEN p.id
                      WHEN 'task' THEN t.id WHEN 'person' THEN u.id END group_id,
                    CASE $10::text WHEN 'client' THEN c.name WHEN 'project' THEN p.name
                      WHEN 'task' THEN t.name WHEN 'person' THEN u.name END name,
                    te.user_id,te.project_id,
                    effective_minutes(te.minutes,te.rounded_minutes,o.round_minutes,o.round_dir) rounded_minutes,
                    (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                      AND COALESCE(pt.billable,t.billable_default)))) billable
             FROM authority a CROSS JOIN time_entries te JOIN organizations o ON o.id=te.org_id
             JOIN projects p ON p.id=te.project_id AND p.org_id=te.org_id
             JOIN tasks t ON t.id=te.task_id AND t.org_id=te.org_id
             JOIN users u ON u.id=te.user_id AND u.org_id=te.org_id
             JOIN clients c ON c.id=p.client_id AND c.org_id=te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id=p.id AND pt.task_id=t.id
             WHERE te.org_id=$1 AND a.active AND a.policy=1 AND (
                 'time_read_all'=ANY(a.grants) OR ('time_read_own'=ANY(a.grants) AND te.user_id=$2)
                 OR ('time_read_managed'=ANY(a.grants) AND (
                   EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=te.org_id AND m.manager_id=$2 AND m.managed_user_id=te.user_id)
                   OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=te.org_id AND m.manager_id=$2 AND m.project_id=te.project_id))))
               AND te.spent_date BETWEEN $3 AND $4
               AND (cardinality($5::uuid[])=0 OR p.client_id=ANY($5))
               AND (cardinality($6::uuid[])=0 OR te.project_id=ANY($6))
               AND (cardinality($7::uuid[])=0 OR te.user_id=ANY($7))
               AND (cardinality($8::uuid[])=0 OR te.task_id=ANY($8))
               AND (cardinality($9::uuid[])=0 OR EXISTS (
                 SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
                 WHERE l.org_id=te.org_id AND l.project_id=te.project_id AND tag.id=ANY($9)))
           ), contexts AS (
             SELECT group_id,name,user_id,project_id,SUM(rounded_minutes)::bigint rounded_minutes,
                    COALESCE(SUM(rounded_minutes) FILTER (WHERE billable),0)::bigint billable_minutes
             FROM scoped GROUP BY group_id,name,user_id,project_id
           ), fragments AS (
             SELECT group_id,name,user_id,project_id,
                    SUM(rounded_minutes) OVER (PARTITION BY group_id)::bigint rounded_minutes,
                    SUM(billable_minutes) OVER (PARTITION BY group_id)::bigint billable_minutes,
                    ROW_NUMBER() OVER (PARTITION BY group_id ORDER BY user_id DESC,project_id DESC)=1 last_context
             FROM contexts
           )
           SELECT a.policy,a.active,a.catalog_version,a.grants,a.is_administrator,
                  a.permission_source,a.built_in_profile,a.template_id,a.applied_template_revision,a.revision,
                  f.group_id,f.name,f.user_id,f.project_id,f.rounded_minutes,f.billable_minutes,f.last_context,
                  256::bigint + a.authority_bytes + COALESCE(octet_length(f.name),0)::bigint export_bytes
           FROM authority a LEFT JOIN fragments f ON true
           ORDER BY f.name COLLATE "C",f.group_id,f.user_id,f.project_id"#,
        org_id, actor_id, query.date_from as _, query.date_to as _, &query.client_ids,
        &query.project_ids, &query.user_ids, &query.task_ids, &query.tag_ids, dimension,
    ).execute(connection).await.map_err(database_error)?;
    Ok(())
}

pub(in crate::reports::streaming) struct GroupRow {
    policy: Option<i32>,
    active: Option<bool>,
    catalog_version: Option<i32>,
    grants: Option<Vec<String>>,
    is_administrator: Option<bool>,
    permission_source: Option<String>,
    built_in_profile: Option<String>,
    template_id: Option<Uuid>,
    applied_template_revision: Option<i64>,
    revision: Option<i64>,
    group_id: Option<Uuid>,
    name: Option<String>,
    user_id: Option<Uuid>,
    project_id: Option<Uuid>,
    rounded_minutes: Option<i64>,
    billable_minutes: Option<i64>,
    last_context: Option<bool>,
}

pub(in crate::reports::streaming) struct Fragment {
    pub group_id: Uuid,
    pub name: String,
    pub context: (Uuid, Uuid),
    pub rounded_minutes: i64,
    pub billable_minutes: i64,
    pub last_context: bool,
}

impl GroupRow {
    pub(in crate::reports::streaming) fn into_fragment(
        self,
    ) -> Result<Option<Fragment>, StatusCode> {
        if self.policy != Some(1) || self.active != Some(true) {
            return Err(StatusCode::FORBIDDEN);
        }
        let state = StoredPersonPermissions {
            catalog_version: self
                .catalog_version
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            grants: self.grants.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            is_administrator: self
                .is_administrator
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            source: self
                .permission_source
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            built_in_profile: self.built_in_profile,
            template_id: self.template_id,
            applied_template_revision: self.applied_template_revision,
            revision: self.revision.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        }
        .restore()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if ![
            Permission::TimeReadOwn,
            Permission::TimeReadManaged,
            Permission::TimeReadAll,
        ]
        .iter()
        .any(|permission| state.grants.contains(*permission))
        {
            return Err(StatusCode::FORBIDDEN);
        }
        let Some(group_id) = self.group_id else {
            return Ok(None);
        };
        Ok(Some(Fragment {
            group_id,
            name: self.name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            context: (
                self.user_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                self.project_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            ),
            rounded_minutes: self
                .rounded_minutes
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            billable_minutes: self
                .billable_minutes
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            last_context: self.last_context.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        }))
    }
}

pub(in crate::reports::streaming) async fn fetch(
    connection: &mut PgConnection,
    limit: i32,
) -> Result<Vec<GroupRow>, StatusCode> {
    sqlx::query_as!(GroupRow,
        r#"SELECT policy,active,catalog_version,grants,is_administrator,
                  permission_source,built_in_profile,template_id,applied_template_revision,revision,
                  group_id,name,user_id,project_id,rounded_minutes,billable_minutes,last_context
           FROM fetch_csv_export_rows($1) AS source(policy integer,active boolean,
             catalog_version integer,grants text[],is_administrator boolean,permission_source text,
             built_in_profile text,template_id uuid,applied_template_revision bigint,revision bigint,
             group_id uuid,name text,user_id uuid,project_id uuid,rounded_minutes bigint,
             billable_minutes bigint,last_context boolean,export_bytes bigint)"#,
        limit,
    ).fetch_all(connection).await.map_err(database_error)
}
