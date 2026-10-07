use super::*;

async fn begin_project_read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<Transaction<'_, Postgres>, StatusCode> {
    let mut tx = pool.begin().await.map_err(database_error)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    configure_deadlines(&mut tx).await?;
    crate::db::lock_organization(&mut tx, org_id, crate::db::OrganizationLock::Shared)
        .await
        .map_err(|error| match error {
            sqlx::Error::RowNotFound => StatusCode::FORBIDDEN,
            other => database_error(other),
        })?;
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE id=$1 AND org_id=$2 AND active FOR SHARE",
        actor_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(database_error)?
    .ok_or(StatusCode::FORBIDDEN)?;
    Ok(tx)
}

pub(in crate::reports) async fn projects(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    scope: &str,
) -> Result<Vec<ProjectExportRow>, StatusCode> {
    let mut tx = begin_project_read(pool, org_id, actor_id).await?;
    // One fresh statement sees completed scope changes after the gate wait,
    // while keeping size checks and payload on the same database snapshot.
    let records = sqlx::query!(
        r#"WITH bounded AS MATERIALIZED (
             SELECT p.id, c.name client_name, p.code, p.name, p.project_type,
                    p.currency, p.budget_kind, p.budget_amount_cents, p.budget_minutes, p.active
             FROM projects p JOIN clients c ON c.id=p.client_id
             JOIN project_read_access a ON a.project_id=p.id AND a.org_id=p.org_id
             WHERE p.org_id=$1 AND a.user_id=$2 AND a.can_view_progress AND CASE $3
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
) -> Result<(), StatusCode> {
    let mut tx = begin_project_read(pool, org_id, actor_id).await?;
    let locked = sqlx::query_scalar!(
        "SELECT id FROM projects WHERE org_id=$1 AND id=ANY($2) ORDER BY id FOR SHARE",
        org_id,
        project_ids,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(database_error)?;
    if locked.len() != project_ids.len() {
        return Err(StatusCode::FORBIDDEN);
    }
    // Assignment/settings triggers write the parent. Refresh the relationship
    // snapshot only after acquiring those parents, not in the locking query.
    let allowed = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM project_read_access
           WHERE org_id=$1 AND user_id=$2 AND project_id=ANY($3) AND can_view_progress"#,
        org_id,
        actor_id,
        project_ids,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(database_error)?;
    if usize::try_from(allowed).ok() != Some(project_ids.len()) {
        return Err(StatusCode::FORBIDDEN);
    }
    tx.commit().await.map_err(database_error)
}
