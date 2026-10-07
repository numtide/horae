use super::*;
use crate::reports::ProjectExportRow;
use crate::server_fns::StoredPersonPermissions;

pub(in crate::reports::streaming) async fn declare_projects(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    scope: &str,
) -> Result<(), StatusCode> {
    sqlx::query!(
        "DECLARE horae_csv NO SCROLL CURSOR FOR
         WITH authority AS MATERIALIZED (
           SELECT o.permission_policy_version policy,u.active actor_active,
                  s.catalog_version,s.grants,s.is_administrator,s.source permission_source,
                  s.built_in_profile,s.template_id,s.applied_template_revision,s.revision,
                  COALESCE(octet_length(s.source),0)::bigint + COALESCE(octet_length(s.built_in_profile),0)::bigint
                    + COALESCE((SELECT SUM(octet_length(g)::bigint) FROM unnest(s.grants) g),0)::bigint authority_bytes
           FROM (VALUES ($1::uuid,$2::uuid)) identity(org_id,user_id)
           LEFT JOIN organizations o ON o.id=identity.org_id
           LEFT JOIN users u ON u.id=identity.user_id AND u.org_id=identity.org_id
           LEFT JOIN person_permission_states s ON s.org_id=o.id AND s.user_id=u.id
         )
         SELECT a.policy,a.actor_active,a.catalog_version,a.grants,a.is_administrator,
                a.permission_source,a.built_in_profile,a.template_id,a.applied_template_revision,a.revision,
                p.id,p.client_name,p.code,p.name,p.project_type,p.currency,
                p.budget_kind,p.budget_amount_cents,p.budget_minutes,p.active,
                256::bigint + a.authority_bytes + COALESCE(octet_length(p.client_name),0)::bigint
                  + COALESCE(octet_length(p.code),0)::bigint + COALESCE(octet_length(p.name),0)::bigint
                  + COALESCE(octet_length(p.currency),0)::bigint AS export_bytes
         FROM authority a LEFT JOIN LATERAL (
           SELECT p.id,c.name client_name,p.code,p.name,p.project_type,p.currency::text,
                  p.budget_kind,
                  CASE WHEN a.policy=0 OR 'billable_rate_read_all'=ANY(a.grants)
                    OR ('billable_rate_read_managed'=ANY(a.grants) AND management.id IS NOT NULL)
                    THEN p.budget_amount_cents END budget_amount_cents,
                  CASE WHEN a.policy=0 OR p.budget_kind='hours' THEN p.budget_minutes END budget_minutes,p.active
           FROM projects p JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
           LEFT JOIN project_read_access legacy ON legacy.project_id=p.id AND legacy.org_id=p.org_id AND legacy.user_id=$2
           LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
             AND management.project_id=p.id AND management.manager_id=$2
           LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
           LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
           WHERE p.org_id=$1 AND a.actor_active AND (
             (a.policy=0 AND legacy.can_view_progress) OR (a.policy=1 AND (
               'project_read_all'=ANY(a.grants)
               OR ('project_read_managed'=ANY(a.grants) AND management.id IS NOT NULL)
               OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility,'project_members')='project_members'))))
             AND CASE $3 WHEN 'budgeted' THEN p.active AND p.budget_kind <> 'none'
               WHEN 'archived' THEN NOT p.active ELSE p.active END
         ) p ON true
         ORDER BY p.client_name,p.name,p.id",
        org_id, actor_id, scope,
    ).execute(connection).await.map_err(database_error)?;
    Ok(())
}

pub(in crate::reports::streaming) async fn projects(
    connection: &mut PgConnection,
    limit: i32,
) -> Result<Vec<ProjectExportRow>, StatusCode> {
    let records = sqlx::query!(
        r#"SELECT policy,actor_active,catalog_version,grants,is_administrator,
                  permission_source,built_in_profile,template_id,applied_template_revision,revision,
                  id,client_name,code,name,project_type AS "project_type?: horae_core::types::ProjectType",currency,
                  budget_kind AS "budget_kind?: horae_core::types::BudgetKind",budget_amount_cents,budget_minutes,active
           FROM fetch_csv_export_rows($1) AS source(policy integer,actor_active boolean,
             catalog_version integer,grants text[],is_administrator boolean,permission_source text,
             built_in_profile text,template_id uuid,applied_template_revision bigint,revision bigint,
             id uuid,client_name text,code text,name text,project_type project_type,currency text,
             budget_kind budget_kind,budget_amount_cents bigint,budget_minutes bigint,active boolean,export_bytes bigint)"#,
        limit,
    ).fetch_all(connection).await.map_err(database_error)?;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        if record.actor_active != Some(true) {
            return Err(StatusCode::FORBIDDEN);
        }
        match record.policy {
            Some(0) => {}
            Some(1) => {
                StoredPersonPermissions {
                    catalog_version: record
                        .catalog_version
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    grants: record.grants.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    is_administrator: record
                        .is_administrator
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    source: record
                        .permission_source
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    built_in_profile: record.built_in_profile,
                    template_id: record.template_id,
                    applied_template_revision: record.applied_template_revision,
                    revision: record.revision.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                }
                .restore()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            None => return Err(StatusCode::FORBIDDEN),
            _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
        // Empty exports still validate the authority captured by the source.
        let Some(id) = record.id else { continue };
        rows.push(ProjectExportRow {
            id,
            client_name: record
                .client_name
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            code: record.code,
            name: record.name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            project_type: record
                .project_type
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            currency: record.currency.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            budget_kind: record
                .budget_kind
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            budget_amount_cents: record.budget_amount_cents,
            budget_minutes: record.budget_minutes,
            active: record.active.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        });
    }
    Ok(rows)
}
