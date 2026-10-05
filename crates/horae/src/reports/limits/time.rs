//! Bounded time exports retain private source scope until final authorization.

use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::types::OrgRole;

use super::*;
use crate::models::DetailedReportRow;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::TimeReportQuery;
use crate::models::time_report::{TimeReportAccess, TimeReportPolicy};
use crate::reports::bounded::ExportPermit;
use crate::server_fns::{PermissionStorageError, load_person_permissions};

pub(in crate::reports) mod groups;

pub(in crate::reports) struct TimeExport {
    pub rows: Vec<DetailedReportRow>,
    pub scope: TimeExportScope,
}

pub(in crate::reports) struct TimeExportScope {
    requester: PermissionRequester,
    policy: i32,
    contexts: Vec<(Uuid, Uuid)>,
}

async fn begin(
    pool: &PgPool,
    requester: PermissionRequester,
) -> Result<(Transaction<'_, Postgres>, i32, PermissionSelection), StatusCode> {
    let mut tx = pool.begin().await.map_err(database_error)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED, READ WRITE")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    configure_deadlines(&mut tx).await?;
    let (policy, grants) = authorize_current(&mut tx, requester).await?;
    Ok((tx, policy, grants))
}

/// Resolve presentation mode with the same current authority as time downloads.
pub(crate) async fn read_access(
    pool: &PgPool,
    requester: PermissionRequester,
) -> Result<TimeReportAccess, StatusCode> {
    let (tx, policy, _) = begin(pool, requester).await?;
    let policy = match policy {
        0 => TimeReportPolicy::Legacy,
        1 => TimeReportPolicy::Scoped,
        _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    tx.commit().await.map_err(database_error)?;
    Ok(TimeReportAccess { requester, policy })
}

/// The caller configures the transaction and owns the lifetime of these gates.
pub(in crate::reports) async fn authorize_current(
    connection: &mut PgConnection,
    requester: PermissionRequester,
) -> Result<(i32, PermissionSelection), StatusCode> {
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
        requester.org_id,
    )
    .fetch_optional(&mut *connection)
    .await
    .map_err(database_error)?
    .ok_or(StatusCode::FORBIDDEN)?;
    let role = sqlx::query_scalar!(
        r#"SELECT org_role AS "role: OrgRole" FROM users WHERE org_id=$1 AND id=$2 AND active FOR SHARE"#,
        requester.org_id, requester.user_id,
    ).fetch_optional(&mut *connection).await.map_err(database_error)?.ok_or(StatusCode::FORBIDDEN)?;
    let grants = match policy {
        0 if matches!(role, OrgRole::Manager | OrgRole::Admin) => {
            PermissionSelection::new(&[Permission::TimeReadAll])
        }
        0 => return Err(StatusCode::FORBIDDEN),
        1 => {
            load_person_permissions(connection, requester.org_id, requester.user_id)
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
    if ![
        Permission::TimeReadOwn,
        Permission::TimeReadManaged,
        Permission::TimeReadAll,
    ]
    .iter()
    .any(|permission| grants.contains(*permission))
    {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok((policy, grants))
}

pub(in crate::reports) async fn entries(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeReportQuery,
    expected_policy: Option<TimeReportPolicy>,
) -> Result<TimeExport, StatusCode> {
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
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
    if expected_policy.is_some_and(|expected| match expected {
        TimeReportPolicy::Legacy => policy != 0,
        TimeReportPolicy::Scoped => policy != 1,
    }) {
        return Err(StatusCode::FORBIDDEN);
    }
    // A single snapshot both bounds and materializes the authorized projection.
    let records = sqlx::query!(
        r#"WITH bounded AS MATERIALIZED (
             SELECT e.id, e.user_id, e.project_id, e.spent_date, p.name project_name,
                    t.name task_name, u.name user_name, e.minutes,
                    effective_minutes(e.minutes,e.rounded_minutes,o.round_minutes,o.round_dir) rounded_minutes,
                    (e.billable AND (e.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable'
                      AND COALESCE(pt.billable,t.billable_default)))) billable, e.notes
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
               AND (NOT $16::bool OR p.active)
               AND (cardinality($8::uuid[])=0 OR p.client_id=ANY($8))
               AND (cardinality($9::uuid[])=0 OR e.project_id=ANY($9))
               AND (cardinality($10::uuid[])=0 OR e.user_id=ANY($10))
               AND (cardinality($11::uuid[])=0 OR e.task_id=ANY($11))
               AND (cardinality($12::uuid[])=0 OR EXISTS (
                 SELECT 1 FROM project_tag_links l JOIN project_tags tag ON tag.id=l.tag_id AND tag.org_id=l.org_id
                 WHERE l.org_id=e.org_id AND l.project_id=e.project_id AND tag.id=ANY($12)))
             LIMIT $13::bigint + 1
           ), stats AS (
             SELECT COUNT(*) rows,
                    COALESCE(SUM(octet_length(project_name)::bigint + octet_length(task_name)
                      + octet_length(user_name) + COALESCE(octet_length(notes),0)),0)::bigint bytes,
                    COALESCE(MAX(GREATEST(octet_length(project_name),octet_length(task_name),
                      octet_length(user_name),COALESCE(octet_length(notes),0))),0) field_bytes
             FROM bounded
           )
           SELECT s.rows AS "rows!", s.bytes AS "bytes!", s.field_bytes AS "field_bytes!",
                  b.id AS "id?", b.user_id AS "user_id?", b.project_id AS "project_id?",
                  b.spent_date AS "spent_date?: chrono::NaiveDate", b.project_name AS "project_name?",
                  b.task_name AS "task_name?", b.user_name AS "user_name?", b.minutes AS "minutes?",
                  b.rounded_minutes AS "rounded_minutes?", b.billable AS "billable?", b.notes
           FROM stats s LEFT JOIN bounded b ON s.rows <= $13 AND s.bytes <= $14 AND s.field_bytes <= $15
           ORDER BY b.spent_date,b.project_name COLLATE "C",b.task_name COLLATE "C",b.id"#,
        org_id, actor_id, grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged), grants.contains(Permission::TimeReadAll),
        query.date_from as _, query.date_to as _, &query.client_ids, &query.project_ids,
        &query.user_ids, &query.task_ids, &query.tag_ids, XLSX.rows, XLSX.bytes, MAX_FIELD_BYTES,
        query.active_projects_only,
    ).fetch_all(&mut *tx).await.map_err(database_error)?;
    let mut rows = Vec::with_capacity(records.len());
    let mut contexts = Vec::with_capacity(records.len());
    for record in records {
        check(record.rows, record.bytes, record.field_bytes, XLSX)?;
        if record.id.is_none() {
            continue;
        }
        contexts.push((
            record.user_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            record.project_id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
        ));
        rows.push(DetailedReportRow {
            spent_date: record.spent_date.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            project_name: record
                .project_name
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            task_name: record.task_name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            user_name: record.user_name.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            minutes: record.minutes.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            rounded_minutes: Some(
                record
                    .rounded_minutes
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            ),
            billable: record.billable.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
            notes: record.notes,
        });
    }
    tx.commit().await.map_err(database_error)?;
    contexts.sort_unstable();
    contexts.dedup();
    Ok(TimeExport {
        rows,
        scope: TimeExportScope {
            requester,
            policy,
            contexts,
        },
    })
}

impl TimeExportScope {
    pub(in crate::reports) async fn render(
        self,
        permit: ExportPermit,
        pool: &PgPool,
        render: impl FnOnce() -> Result<Vec<u8>, StatusCode> + Send + 'static,
    ) -> Result<axum::body::Body, StatusCode> {
        let body = permit.render(render).await?;
        let (mut tx, policy, grants) = begin(pool, self.requester).await?;
        if policy != self.policy {
            return Err(StatusCode::FORBIDDEN);
        }
        authorize_rows(&mut tx, self.requester, &grants, &self.contexts).await?;
        tx.commit().await.map_err(database_error)?;
        Ok(body)
    }
}

pub(in crate::reports) async fn authorize_rows(
    connection: &mut PgConnection,
    requester: PermissionRequester,
    grants: &PermissionSelection,
    contexts: &[(Uuid, Uuid)],
) -> Result<(), StatusCode> {
    let (owners, projects): (Vec<_>, Vec<_>) = contexts.iter().copied().unzip();
    // Source reassignment/deletion cannot change the scope of rendered bytes.
    let allowed = sqlx::query_scalar!(
        r#"SELECT NOT EXISTS (
                 SELECT 1 FROM unnest($3::uuid[],$4::uuid[]) AS captured(owner_id,project_id)
                 WHERE NOT ($7::bool OR ($5::bool AND captured.owner_id=$2) OR ($6::bool AND (
                   EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=$1 AND m.manager_id=$2 AND m.managed_user_id=captured.owner_id)
                   OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=$1 AND m.manager_id=$2 AND m.project_id=captured.project_id))))
               ) AS "allowed!""#,
        requester.org_id,
        requester.user_id,
        &owners,
        &projects,
        grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged),
        grants.contains(Permission::TimeReadAll),
    )
    .fetch_one(connection)
    .await
    .map_err(database_error)?;
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}
