use super::*;
use crate::server_fns::{PermissionStorageError, load_person_permissions};
use horae_core::permissions::catalog::{Permission, PermissionSelection};

async fn begin_project_transaction(pool: &PgPool) -> Result<Transaction<'_, Postgres>, StatusCode> {
    let mut tx = pool.begin().await.map_err(database_error)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    configure_deadlines(&mut tx).await?;
    Ok(tx)
}

async fn authorize_project_actor(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<(i32, PermissionSelection), StatusCode> {
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
        org_id,
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(database_error)?
    .ok_or(StatusCode::FORBIDDEN)?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id=$1 AND org_id=$2 AND active FOR SHARE",
        actor_id,
        org_id,
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(database_error)?
    .ok_or(StatusCode::FORBIDDEN)?;
    let grants = match policy {
        0 => PermissionSelection::new(&[]),
        1 => {
            load_person_permissions(connection, org_id, actor_id)
                .await
                .map_err(|error| match error {
                    PermissionStorageError::Database(error) => database_error(error),
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                })?
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                .grants
        }
        _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    Ok((policy, grants))
}

pub(in crate::reports) async fn projects(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    scope: &str,
) -> Result<Vec<ProjectExportRow>, StatusCode> {
    let mut tx = begin_project_transaction(pool).await?;
    let (policy, grants) = authorize_project_actor(&mut tx, org_id, actor_id).await?;
    // One fresh statement sees completed scope changes after the gate wait,
    // while keeping size checks and payload on the same database snapshot.
    let records = sqlx::query!(
        r#"WITH bounded AS MATERIALIZED (
             SELECT p.id, c.name client_name, p.code, p.name, p.project_type,
                    p.currency, p.budget_kind,
                    CASE WHEN $7 OR $10 OR ($11 AND management.id IS NOT NULL)
                      THEN p.budget_amount_cents END AS budget_amount_cents,
                    CASE WHEN $7 OR p.budget_kind='hours' THEN p.budget_minutes END AS budget_minutes, p.active
             FROM projects p JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
             LEFT JOIN project_read_access a ON a.project_id=p.id AND a.org_id=p.org_id AND a.user_id=$2
             LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
               AND management.project_id=p.id AND management.manager_id=$2
             LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
             LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
             WHERE p.org_id=$1 AND (CASE WHEN $7 THEN COALESCE(a.can_view_progress,false)
               ELSE $8 OR ($9 AND management.id IS NOT NULL)
                 OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility,'project_members')='project_members') END)
               AND CASE $3
               WHEN 'budgeted' THEN p.active AND p.budget_kind <> 'none'
               WHEN 'archived' THEN NOT p.active ELSE p.active END
             LIMIT $4::bigint + 1
           ), stats AS (
             SELECT COUNT(*) rows,
                    COALESCE(SUM(octet_length(client_name)::bigint + COALESCE(octet_length(code),0)
                      + octet_length(name) + octet_length(currency)),0)::bigint bytes,
                    COALESCE(MAX(GREATEST(octet_length(client_name), COALESCE(octet_length(code),0),
                      octet_length(name), octet_length(currency))),0) field_bytes
             FROM bounded
           )
           SELECT s.rows AS "rows!", s.bytes AS "bytes!", s.field_bytes AS "field_bytes!",
                  b.id AS "id?", b.client_name AS "client_name?", b.code, b.name AS "name?",
                  b.project_type AS "project_type?: horae_core::types::ProjectType",
                  b.currency AS "currency?", b.budget_kind AS "budget_kind?: horae_core::types::BudgetKind",
                  b.budget_amount_cents, b.budget_minutes, b.active AS "active?"
           FROM stats s LEFT JOIN bounded b ON s.rows <= $4 AND s.bytes <= $5 AND s.field_bytes <= $6
           ORDER BY b.client_name, b.name, b.id"#,
        org_id, actor_id, scope, XLSX.rows, XLSX.bytes, MAX_FIELD_BYTES,
        policy == 0, grants.contains(Permission::ProjectReadAll), grants.contains(Permission::ProjectReadManaged),
        grants.contains(Permission::BillableRateReadAll), grants.contains(Permission::BillableRateReadManaged),
    ).fetch_all(&mut *tx).await.map_err(database_error)?;
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        check(record.rows, record.bytes, record.field_bytes, XLSX)?;
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
    tx.commit().await.map_err(database_error)?;
    Ok(rows)
}

pub(in crate::reports) async fn authorize_projects(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    project_ids: &[Uuid],
    monetary_project_ids: &[Uuid],
) -> Result<(), StatusCode> {
    let mut tx = begin_project_transaction(pool).await?;
    authorize_project_rows(&mut tx, org_id, actor_id, project_ids, monetary_project_ids).await?;
    tx.commit().await.map_err(database_error)
}

pub(in crate::reports) async fn authorize_project_rows(
    connection: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    project_ids: &[Uuid],
    monetary_project_ids: &[Uuid],
) -> Result<(), StatusCode> {
    let (policy, grants) = authorize_project_actor(connection, org_id, actor_id).await?;
    let locked = sqlx::query_scalar!(
        "SELECT id FROM projects WHERE org_id=$1 AND id=ANY($2) ORDER BY id FOR SHARE",
        org_id,
        project_ids,
    )
    .fetch_all(&mut *connection)
    .await
    .map_err(database_error)?;
    if locked.len() != project_ids.len() {
        return Err(StatusCode::FORBIDDEN);
    }
    // Assignment/settings triggers write the parent. Refresh the relationship
    // snapshot only after acquiring those parents, not in the locking query.
    let allowed = sqlx::query!(
        r#"SELECT COUNT(*) AS "count!",
             COUNT(*) FILTER (WHERE p.id=ANY($4) AND ($5 OR $8 OR ($9 AND management.id IS NOT NULL))) AS "financial!"
           FROM projects p JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
           LEFT JOIN project_read_access a ON a.org_id=p.org_id AND a.project_id=p.id AND a.user_id=$2
           LEFT JOIN project_management_assignments management ON management.org_id=p.org_id
             AND management.project_id=p.id AND management.manager_id=$2
           LEFT JOIN assignments member ON member.project_id=p.id AND member.user_id=$2
           LEFT JOIN project_settings settings ON settings.org_id=p.org_id AND settings.project_id=p.id
           WHERE p.org_id=$1 AND p.id=ANY($3) AND CASE WHEN $5 THEN COALESCE(a.can_view_progress,false)
             ELSE $6 OR ($7 AND management.id IS NOT NULL)
               OR (member.id IS NOT NULL AND COALESCE(settings.report_visibility,'project_members')='project_members') END"#,
        org_id,
        actor_id,
        project_ids,
        monetary_project_ids,
        policy == 0, grants.contains(Permission::ProjectReadAll), grants.contains(Permission::ProjectReadManaged),
        grants.contains(Permission::BillableRateReadAll), grants.contains(Permission::BillableRateReadManaged),
    )
    .fetch_one(connection)
    .await
    .map_err(database_error)?;
    if usize::try_from(allowed.count).ok() != Some(project_ids.len())
        || usize::try_from(allowed.financial).ok() != Some(monetary_project_ids.len())
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
