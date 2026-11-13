//! Canonical people reads; workflow pickers require their own operation scope.

use horae_core::permissions::catalog::Permission;
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    PermissionStorageError, ReadIsolation, begin_lock_only_read_async_commit,
    load_person_permissions,
};
use crate::models::people::{PeopleActivity, PeopleCursor, PeoplePage, PersonSummary};
use crate::models::permission_editor::PermissionRequester;

#[derive(Debug, thiserror::Error)]
pub(crate) enum DirectoryError {
    #[error("Current people-read authority is required")]
    Forbidden,
    #[error("Permission state is unavailable")]
    Unavailable,
    #[error("Invalid directory cursor")]
    InvalidCursor,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Session-derived IDs only. Keep authority stable through the materialized read.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    activity: PeopleActivity,
    after: Option<&PeopleCursor>,
) -> Result<PeoplePage, DirectoryError> {
    let mut tx = begin_lock_only_read_async_commit(pool, ReadIsolation::ReadCommitted).await?;
    let organization =
        sqlx::query_scalar!("SELECT id FROM organizations WHERE id=$1 FOR SHARE", org_id)
            .fetch_optional(&mut *tx)
            .await?;
    if organization.is_none() {
        return Err(DirectoryError::Forbidden);
    }
    let actor = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if actor.is_none() {
        return Err(DirectoryError::Forbidden);
    }
    let state = load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .ok_or(DirectoryError::Unavailable)?;
    let all_people = state.grants.contains(Permission::PeopleReadAll);
    if !all_people && !state.grants.contains(Permission::PeopleReadManaged) {
        return Err(DirectoryError::Forbidden);
    }
    if after.is_some_and(|cursor| cursor.name.contains('\0')) {
        return Err(DirectoryError::InvalidCursor);
    }
    let active = match activity {
        PeopleActivity::Active => Some(true),
        PeopleActivity::Archived => Some(false),
        PeopleActivity::All => None,
    };
    let rows = sqlx::query!(
        r#"SELECT u.id, u.name, u.email, u.active,
                  COALESCE(p.is_administrator, false) AS "administrator!", d.managed AS "managed!"
             FROM users u
             CROSS JOIN LATERAL (SELECT EXISTS (
               SELECT 1 FROM person_management_assignments m
               WHERE m.org_id=u.org_id AND m.manager_id=$2 AND m.managed_user_id=u.id) AS managed) d
             LEFT JOIN person_permission_states p ON p.org_id=u.org_id AND p.user_id=u.id
            WHERE u.org_id=$1
              AND ($3::bool OR d.managed)
              AND ($4::bool IS NULL OR u.active=$4)
              AND ($5::text IS NULL OR (u.name,u.id)>($5::text,$6::uuid))
            ORDER BY u.name,u.id LIMIT 51"#,
        org_id,
        actor_id,
        all_people,
        active,
        after.map(|cursor| cursor.name.as_str()),
        after.map(|cursor| cursor.id),
    )
    .fetch_all(&mut *tx)
    .await?;
    let write_all = state.grants.contains(Permission::PeopleWriteAll);
    let write_managed = state.grants.contains(Permission::PeopleWriteManaged);
    let mut people: Vec<PersonSummary> = rows
        .into_iter()
        .map(|row| PersonSummary {
            can_change_active: crate::ops::people::may_change_active(
                write_all,
                write_managed,
                row.managed,
                state.is_administrator,
                row.administrator,
            ),
            id: row.id,
            name: row.name,
            email: row.email,
            active: row.active,
        })
        .collect();
    let next_after = if people.len() > 50 {
        people.truncate(50);
        people.last().map(|person| PeopleCursor {
            name: person.name.clone(),
            id: person.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(PeoplePage {
        requester: PermissionRequester {
            org_id,
            user_id: actor_id,
        },
        people,
        next_after,
    })
}
