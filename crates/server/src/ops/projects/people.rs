//! The people a project form offers for its team: names only, without the
//! directory's details or anyone's rates.

use horae_core::permissions::catalog::Permission;

use super::EDITING_REQUIRED;
use crate::models::people::PeopleCursor;
use crate::models::project_people::{
    ProjectPeopleContext, ProjectPeopleQuery, ProjectPeopleResult, ProjectPersonChoice,
};
use crate::ops::{Actor, OpError, Tx};

const INVALID_QUERY: &str = "Invalid project people query";

/// Searches or resolves active people for the form `context` names: creating
/// a project needs `ProjectCreateAll`, editing one needs authority to edit it.
pub async fn read(
    tx: &mut Tx,
    actor: &Actor,
    context: ProjectPeopleContext,
    query: &ProjectPeopleQuery,
) -> Result<ProjectPeopleResult, OpError> {
    let org_id = actor.org_id();
    let authorized = match context {
        ProjectPeopleContext::Create => actor.has(Permission::ProjectCreateAll),
        ProjectPeopleContext::Edit { project_id } => {
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM projects p WHERE p.org_id=$1 AND p.id=$2
                 AND ($4::bool OR ($5::bool AND EXISTS (
                   SELECT 1 FROM project_management_assignments m
                   WHERE m.org_id=p.org_id AND m.project_id=p.id AND m.manager_id=$3))))",
                org_id,
                project_id,
                actor.id(),
                actor.has(Permission::ProjectWriteAll),
                actor.has(Permission::ProjectWriteManaged),
            )
            .fetch_one(&mut **tx)
            .await?
                == Some(true)
        }
    };
    if !authorized {
        return Err(OpError::forbidden(EDITING_REQUIRED));
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
                return Err(OpError::bad_input(INVALID_QUERY));
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
            .fetch_all(&mut **tx)
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
                return Err(OpError::bad_input(INVALID_QUERY));
            }
            let people = sqlx::query_as!(
                ProjectPersonChoice,
                "SELECT id,name FROM users WHERE org_id=$1 AND active AND id=ANY($2)
                 ORDER BY name,id",
                org_id,
                ids,
            )
            .fetch_all(&mut **tx)
            .await?;
            (people, None)
        }
    };
    Ok(ProjectPeopleResult {
        requester: actor.requester(),
        people,
        next_after,
    })
}
