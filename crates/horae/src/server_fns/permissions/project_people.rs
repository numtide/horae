//! Project-team selection without directory or financial access.

use horae_core::permissions::catalog::Permission;
use sqlx::PgPool;
use uuid::Uuid;

use super::{PermissionStorageError, configure_administration, load_person_permissions};
use crate::models::people::PeopleCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::models::project_people::{
    ProjectPeopleContext, ProjectPeopleQuery, ProjectPeopleResult, ProjectPersonChoice,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectPeopleError {
    #[error("Current project editing authority is required")]
    Forbidden,
    #[error("Permission state is unavailable")]
    Unavailable,
    #[error("Invalid project people query")]
    InvalidInput,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// Derive actor and organization from authentication; reauthorize every request.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    context: ProjectPeopleContext,
    query: &ProjectPeopleQuery,
) -> Result<ProjectPeopleResult, ProjectPeopleError> {
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
        Some(0) | None => return Err(ProjectPeopleError::Forbidden),
        Some(_) => return Err(ProjectPeopleError::Unavailable),
    }
    let actor = sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        actor_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if actor.is_none() {
        return Err(ProjectPeopleError::Forbidden);
    }
    let state = load_person_permissions(&mut tx, org_id, actor_id)
        .await?
        .ok_or(ProjectPeopleError::Unavailable)?;
    let authorized = match context {
        ProjectPeopleContext::Create => state.grants.contains(Permission::ProjectCreateAll),
        ProjectPeopleContext::Edit { project_id } => {
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM projects p WHERE p.org_id=$1 AND p.id=$2
                 AND ($4::bool OR ($5::bool AND EXISTS (
                   SELECT 1 FROM project_management_assignments m
                   WHERE m.org_id=p.org_id AND m.project_id=p.id AND m.manager_id=$3))))",
                org_id,
                project_id,
                actor_id,
                state.grants.contains(Permission::ProjectWriteAll),
                state.grants.contains(Permission::ProjectWriteManaged),
            )
            .fetch_one(&mut *tx)
            .await?
                == Some(true)
        }
    };
    if !authorized {
        return Err(ProjectPeopleError::Forbidden);
    }
    let (people, next_after) = match query {
        ProjectPeopleQuery::Search { query, after } => {
            let query = query.trim();
            if query.contains('\0')
                || query.chars().count() > 100
                || after
                    .as_ref()
                    .is_some_and(|cursor| cursor.name.contains('\0'))
            {
                return Err(ProjectPeopleError::InvalidInput);
            }
            let mut people = sqlx::query_as!(
                ProjectPersonChoice,
                "SELECT id,name FROM users WHERE org_id=$1 AND active
                 AND strpos(lower(name),lower($2)) > 0
                 AND ($3::text IS NULL OR (name,id)>($3::text,$4::uuid))
                 ORDER BY name,id LIMIT 51",
                org_id,
                query,
                after.as_ref().map(|cursor| cursor.name.as_str()),
                after.as_ref().map(|cursor| cursor.id),
            )
            .fetch_all(&mut *tx)
            .await?;
            let next_after = if people.len() > 50 {
                people.truncate(50);
                people.last().map(|person| PeopleCursor {
                    name: person.name.clone(),
                    id: person.id,
                })
            } else {
                None
            };
            (people, next_after)
        }
        ProjectPeopleQuery::Resolve { ids } => {
            if ids.len() > 500 {
                return Err(ProjectPeopleError::InvalidInput);
            }
            let people = sqlx::query_as!(
                ProjectPersonChoice,
                "SELECT id,name FROM users WHERE org_id=$1 AND active AND id=ANY($2)
                 ORDER BY name,id",
                org_id,
                ids,
            )
            .fetch_all(&mut *tx)
            .await?;
            (people, None)
        }
    };
    tx.commit().await?;
    Ok(ProjectPeopleResult {
        requester: PermissionRequester {
            org_id,
            user_id: actor_id,
        },
        people,
        next_after,
    })
}
