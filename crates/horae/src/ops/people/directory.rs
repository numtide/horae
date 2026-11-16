//! The People page: a bounded directory page under the actor's people-read
//! grants. Workflow pickers use [`super::list`] or their own operation scope.

use horae_core::permissions::catalog::Permission;
use horae_core::permissions::person_management::may_change_active;

use crate::models::people::{PeopleActivity, PeopleCursor, PeoplePage, PersonSummary};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::{Actor, OpError, Tx};

const PAGE_SIZE: usize = 50;

/// One page of people, ordered by name then id, after `after`.
/// `PeopleReadAll` lists the organization; `PeopleReadManaged` only the people
/// the actor manages. The authority locks [`Actor::load`] took hold until the
/// page is materialized.
pub(crate) async fn page(
    tx: &mut Tx,
    actor: &Actor,
    activity: PeopleActivity,
    after: Option<&PeopleCursor>,
) -> Result<PeoplePage, OpError> {
    let all_people = actor.has(Permission::PeopleReadAll);
    if !all_people && !actor.has(Permission::PeopleReadManaged) {
        return Err(OpError::forbidden(
            "Current people-read authority is required",
        ));
    }
    if after.is_some_and(|cursor| cursor.name.contains('\0')) {
        return Err(OpError::invalid("after", "Invalid directory cursor"));
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
        actor.org_id(),
        actor.id(),
        all_people,
        active,
        after.map(|cursor| cursor.name.as_str()),
        after.map(|cursor| cursor.id),
    )
    .fetch_all(&mut **tx)
    .await?;
    let write_all = actor.has(Permission::PeopleWriteAll);
    let write_managed = actor.has(Permission::PeopleWriteManaged);
    let mut people: Vec<PersonSummary> = rows
        .into_iter()
        .map(|row| PersonSummary {
            can_change_active: may_change_active(
                write_all,
                write_managed,
                row.managed,
                actor.administrator(),
                row.administrator,
            ),
            id: row.id,
            name: row.name,
            email: row.email,
            active: row.active,
        })
        .collect();
    let next_after = if people.len() > PAGE_SIZE {
        people.truncate(PAGE_SIZE);
        people.last().map(|person| PeopleCursor {
            name: person.name.clone(),
            id: person.id,
        })
    } else {
        None
    };
    Ok(PeoplePage {
        requester: PermissionRequester {
            org_id: actor.org_id(),
            user_id: actor.id(),
        },
        people,
        next_after,
    })
}
