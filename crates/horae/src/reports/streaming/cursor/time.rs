//! Native source rows include authority even when no time entry is visible.

use super::*;
use crate::models::{DetailedReportRow, time_report::TimeReportQuery};
use crate::server_fns::StoredPersonPermissions;
use horae_core::{permissions::catalog::Permission, types::OrgRole};

pub(in crate::reports::streaming) async fn declare_entries(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeReportQuery,
) -> Result<(), StatusCode> {
    sqlx::query!(
        r#"DECLARE horae_csv NO SCROLL CURSOR FOR
           WITH authority AS MATERIALIZED (
             SELECT o.permission_policy_version policy,u.org_role role,u.active,
                    s.catalog_version,s.grants,s.is_administrator,s.source permission_source,
                    s.built_in_profile,s.template_id,s.applied_template_revision,s.revision,
                    COALESCE(octet_length(s.source),0)::bigint + COALESCE(octet_length(s.built_in_profile),0)::bigint
                      + COALESCE((SELECT SUM(octet_length(g)::bigint) FROM unnest(s.grants) g),0)::bigint authority_bytes
             FROM (VALUES ($1::uuid,$2::uuid)) identity(org_id,user_id)
             LEFT JOIN organizations o ON o.id=identity.org_id
             LEFT JOIN users u ON u.id=identity.user_id AND u.org_id=identity.org_id
             LEFT JOIN person_permission_states s ON s.org_id=o.id AND s.user_id=u.id
           )
           SELECT a.policy,a.role,a.active,a.catalog_version,a.grants,a.is_administrator,
                  a.permission_source,a.built_in_profile,a.template_id,a.applied_template_revision,a.revision,
                  e.id,e.user_id,e.project_id,e.spent_date,e.project_name,e.task_name,e.user_name,
                  e.minutes,e.rounded_minutes,e.billable,e.notes,
                  256::bigint + a.authority_bytes + COALESCE(octet_length(e.project_name),0)::bigint
                    + COALESCE(octet_length(e.task_name),0)::bigint + COALESCE(octet_length(e.user_name),0)::bigint
                    + COALESCE(octet_length(e.notes),0)::bigint AS export_bytes
           FROM authority a LEFT JOIN LATERAL (
             SELECT te.id,te.user_id,te.project_id,te.spent_date,p.name project_name,t.name task_name,
                    u.name user_name,te.minutes,
                    effective_minutes(te.minutes,te.rounded_minutes,o.round_minutes,o.round_dir) rounded_minutes,
                    (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                      AND COALESCE(pt.billable,t.billable_default)))) billable,te.notes
             FROM time_entries te JOIN organizations o ON o.id=te.org_id
             JOIN projects p ON p.id=te.project_id AND p.org_id=te.org_id
             JOIN tasks t ON t.id=te.task_id AND t.org_id=te.org_id
             JOIN users u ON u.id=te.user_id AND u.org_id=te.org_id
             JOIN clients c ON c.id=p.client_id AND c.org_id=te.org_id
             LEFT JOIN project_tasks pt ON pt.project_id=p.id AND pt.task_id=t.id
             WHERE te.org_id=$1 AND a.active AND (
               (a.policy=0 AND a.role IN ('admin','manager')) OR (a.policy=1 AND (
                 'time_read_all'=ANY(a.grants) OR ('time_read_own'=ANY(a.grants) AND te.user_id=$2)
                 OR ('time_read_managed'=ANY(a.grants) AND (
                   EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=te.org_id AND m.manager_id=$2 AND m.managed_user_id=te.user_id)
                   OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=te.org_id AND m.manager_id=$2 AND m.project_id=te.project_id))))))
               AND te.spent_date BETWEEN $3 AND $4
               AND (NOT $10::bool OR p.active)
               AND ($11::bool IS NULL OR $11 = (te.billable AND (te.invoice_id IS NOT NULL
                 OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable,t.billable_default)))))
               AND (cardinality($5::uuid[])=0 OR p.client_id=ANY($5))
               AND (cardinality($6::uuid[])=0 OR te.project_id=ANY($6))
               AND (cardinality($7::uuid[])=0 OR te.user_id=ANY($7))
               AND (cardinality($8::uuid[])=0 OR te.task_id=ANY($8))
               AND (cardinality($9::uuid[])=0 OR EXISTS (
                 SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
                 WHERE l.org_id=te.org_id AND l.project_id=te.project_id AND tag.id=ANY($9)))
           ) e ON true
           ORDER BY e.spent_date,e.project_name COLLATE "C",e.task_name COLLATE "C",e.id"#,
        org_id, actor_id, query.date_from as _, query.date_to as _, &query.client_ids,
        &query.project_ids, &query.user_ids, &query.task_ids, &query.tag_ids,
        query.active_projects_only,
        query.billability.filter(),
    ).execute(connection).await.map_err(database_error)?;
    Ok(())
}

pub(in crate::reports::streaming) struct TimeRow {
    policy: Option<i32>,
    role: Option<OrgRole>,
    active: Option<bool>,
    catalog_version: Option<i32>,
    grants: Option<Vec<String>>,
    is_administrator: Option<bool>,
    permission_source: Option<String>,
    built_in_profile: Option<String>,
    template_id: Option<Uuid>,
    applied_template_revision: Option<i64>,
    revision: Option<i64>,
    id: Option<Uuid>,
    user_id: Option<Uuid>,
    project_id: Option<Uuid>,
    spent_date: Option<NaiveDate>,
    project_name: Option<String>,
    task_name: Option<String>,
    user_name: Option<String>,
    minutes: Option<i32>,
    rounded_minutes: Option<i32>,
    billable: Option<bool>,
    pub notes: Option<String>,
}

pub(in crate::reports::streaming) struct TimeEntry {
    pub row: DetailedReportRow,
    pub context: (Uuid, Uuid),
}

impl TimeRow {
    pub(in crate::reports::streaming) fn into_entry(
        self,
        expected_policy: i32,
    ) -> Result<Option<TimeEntry>, StatusCode> {
        let policy = self.policy.ok_or(StatusCode::FORBIDDEN)?;
        if !matches!(policy, 0 | 1) {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
        if policy != expected_policy || self.active != Some(true) {
            return Err(StatusCode::FORBIDDEN);
        }
        match policy {
            0 if matches!(self.role, Some(OrgRole::Admin | OrgRole::Manager)) => {}
            0 => return Err(StatusCode::FORBIDDEN),
            _ => {
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
            }
        }
        // Validate authority before discarding an empty-source sentinel.
        if self.id.is_none() {
            return Ok(None);
        }
        Ok(Some(TimeEntry {
            context: (
                self.user_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                self.project_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            ),
            row: DetailedReportRow {
                spent_date: self.spent_date.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                project_name: self.project_name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                task_name: self.task_name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                user_name: self.user_name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                minutes: self.minutes.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                rounded_minutes: Some(
                    self.rounded_minutes
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                ),
                billable: self.billable.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                notes: self.notes,
            },
        }))
    }
}

pub(in crate::reports::streaming) async fn entries(
    connection: &mut PgConnection,
    limit: i32,
) -> Result<Vec<TimeRow>, StatusCode> {
    sqlx::query_as!(TimeRow,
        r#"SELECT policy,role AS "role?: OrgRole",active,catalog_version,grants,is_administrator,
                  permission_source,built_in_profile,template_id,applied_template_revision,revision,
                  id,user_id,project_id,spent_date AS "spent_date?: NaiveDate",project_name,task_name,user_name,
                  minutes,rounded_minutes,billable,notes
           FROM fetch_csv_export_rows($1) AS source(policy integer,role org_role,active boolean,
             catalog_version integer,grants text[],is_administrator boolean,permission_source text,
             built_in_profile text,template_id uuid,applied_template_revision bigint,revision bigint,
             id uuid,user_id uuid,project_id uuid,spent_date date,project_name text,task_name text,user_name text,
             minutes integer,rounded_minutes integer,billable boolean,notes text,export_bytes bigint)"#,
        limit,
    ).fetch_all(connection).await.map_err(database_error)
}
