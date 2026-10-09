//! Scoped time reads; neither contextual labels nor running timers grant writes.

use horae_core::permissions::catalog::Permission;
use horae_core::types::EntryState;
use sqlx::PgPool;
use uuid::Uuid;

use super::{PermissionStorageError, configure_administration, load_person_permissions};
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{
    TimeEntryCursor, TimeEntryPage, TimeEntryQuery, VisibleTimeEntry,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum TimeReadError {
    #[error("Current time-read authority is required")]
    Forbidden,
    #[error("Permission state is unavailable")]
    Unavailable,
    #[error("Invalid time-entry date range or cursor")]
    InvalidQuery,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Session-derived identity only; materialize rows while authority is stable.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeEntryQuery,
) -> Result<TimeEntryPage, TimeReadError> {
    let mut tx = pool.begin().await?;
    configure_administration(&mut tx).await?;
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id=$1 FOR SHARE",
        org_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    match policy {
        Some(1) => {}
        Some(0) | None => return Err(TimeReadError::Forbidden),
        Some(_) => return Err(TimeReadError::Unavailable),
    }
    let actor = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if actor.is_none() {
        return Err(TimeReadError::Forbidden);
    }
    let state = load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .ok_or(TimeReadError::Unavailable)?;
    let own = state.grants.contains(Permission::TimeReadOwn);
    let managed = state.grants.contains(Permission::TimeReadManaged);
    let all = state.grants.contains(Permission::TimeReadAll);
    if !own && !managed && !all {
        return Err(TimeReadError::Forbidden);
    }
    if query.date_from > query.date_to
        || query.after.as_ref().is_some_and(|cursor| {
            cursor.spent_date < query.date_from || cursor.spent_date > query.date_to
        })
    {
        return Err(TimeReadError::InvalidQuery);
    }
    let mut entries = sqlx::query_as!(
        VisibleTimeEntry,
        r#"SELECT e.id, e.user_id, u.name AS user_name,
            e.project_id, p.name AS project_name, e.task_id, t.name AS task_name,
            p.client_id, c.name AS client_name, e.spent_date AS "spent_date: _", e.minutes,
            e.rounded_minutes, e.notes, e.billable, e.is_running, e.started_at AS "started_at: _",
            e.start_minute, e.sort_order, e.state AS "state: EntryState", e.created_at AS "created_at: _"
         FROM time_entries e
         JOIN users u ON u.id=e.user_id AND u.org_id=e.org_id
         JOIN projects p ON p.id=e.project_id AND p.org_id=e.org_id
         JOIN tasks t ON t.id=e.task_id AND t.org_id=e.org_id
         JOIN clients c ON c.id=p.client_id AND c.org_id=e.org_id
         WHERE e.org_id=$1 AND (
             $5::bool OR ($3::bool AND e.user_id=$2) OR ($4::bool AND (
                 EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.managed_user_id=e.user_id)
                 OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.project_id=e.project_id))))
           AND e.spent_date BETWEEN $6 AND $7
           AND ($8::uuid IS NULL OR e.user_id=$8)
           AND ($9::uuid IS NULL OR e.project_id=$9)
           AND ($10::date IS NULL OR (e.spent_date,e.created_at,e.id)<($10,$11,$12))
         ORDER BY e.spent_date DESC,e.created_at DESC,e.id DESC LIMIT 501"#,
        org_id, actor_id, own, managed, all, query.date_from as _, query.date_to as _,
        query.user_id, query.project_id,
        query.after.as_ref().map(|cursor| cursor.spent_date) as _,
        query.after.as_ref().map(|cursor| cursor.created_at) as _,
        query.after.as_ref().map(|cursor| cursor.id),
    ).fetch_all(&mut *tx).await?;
    let next_after = if entries.len() > 500 {
        entries.truncate(500);
        entries.last().map(|entry| TimeEntryCursor {
            spent_date: entry.spent_date,
            created_at: entry.created_at,
            id: entry.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(TimeEntryPage {
        requester: PermissionRequester {
            org_id,
            user_id: actor_id,
        },
        entries,
        next_after,
    })
}
