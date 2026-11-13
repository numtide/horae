//! People operations: adding people and archiving or restoring them.
//!
//! Adding people needs `PeopleWriteAll`, which the Administrator, Executive
//! Manager and People Admin profiles hold. Archiving and restoring also accept
//! `PeopleWriteManaged` for the people the actor manages. Who is an
//! administrator is decided only in the permission editor, so these operations
//! never name one and leave an administrator's account to another
//! administrator.
//!
//! Answers carry identity only (`UserListItem`): a people grant does not open
//! rates or sign-in details.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, UserPayload, active_transition};
use crate::server_fns::provisioning;

const NOT_FOUND: &str = "Person not found";
const WRITE_REQUIRED: &str = "Permission to manage people is required";
const MANAGED_REQUIRED: &str = "You can archive or restore only the people you manage";
const ACTIVE_REQUIRED: &str = "An active account is required";
const READ_REQUIRED: &str = "Permission to view everyone is required to list archived people";
const ADMINISTRATOR_PROFILE: &str =
    "Administrators are chosen in the permission editor, not when adding a person";
const PROFILE_NEEDS_ADMINISTRATOR: &str =
    "Only an administrator can add a person with a profile other than Member";
const ADMINISTRATOR_TARGET: &str = "Only an administrator can archive or restore an administrator";
const LAST_ADMINISTRATOR: &str = "The organization must keep at least one active administrator.";

/// Emails are unique across organizations, so an email can belong to an
/// account the actor may not know about. The refusal never says that an
/// account exists or where.
fn unavailable_email(email: &str) -> String {
    format!("The email {email} can't be used for a new person. Use another address.")
}

/// The most people one request may add, matching the importer's bounded list
/// of missing people.
pub(crate) const MAX_NEW_PEOPLE: usize = 500;

/// What adding people does with an email an account already uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Existing {
    /// Refuse the request: the person was meant to be new.
    Refuse,
    /// Leave the account alone and report the email as already present.
    Skip,
}

/// Whether an actor may archive or restore a person: with `PeopleWriteAll`,
/// or with `PeopleWriteManaged` for a person they manage, and an
/// administrator's account only as another administrator. The People page
/// asks the same question to decide which rows offer the action.
pub(crate) fn may_change_active(
    write_all: bool,
    write_managed: bool,
    manages: bool,
    actor_administrator: bool,
    target_administrator: bool,
) -> bool {
    (write_all || (write_managed && manages)) && (actor_administrator || !target_administrator)
}

/// Identities for the directory pickers. Archived people need `PeopleReadAll`;
/// active people are listed for any active person, as the pickers need them.
pub(crate) async fn list(
    tx: &mut Tx,
    actor: &Actor,
    include_inactive: bool,
) -> Result<Vec<UserListItem>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden(ACTIVE_REQUIRED));
    }
    if include_inactive && !actor.has(Permission::PeopleReadAll) {
        return Err(OpError::forbidden(READ_REQUIRED));
    }
    Ok(sqlx::query_as!(
        UserListItem,
        "SELECT id, email, name, active FROM users
         WHERE org_id = $2 AND ($1::bool OR active)
         ORDER BY name ASC",
        include_inactive,
        actor.org_id(),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Adds people with `profile`'s permission state. A person with more than the
/// Member profile is effectively a permission change, so only an
/// administrator may choose one, and nobody becomes an administrator here.
pub(crate) async fn add(
    tx: &mut Tx,
    actor: &Actor,
    people: &[NewPerson],
    profile: BuiltInProfile,
    activation: Activation,
    existing: Existing,
) -> Result<Done<AddedPeople>, OpError> {
    if !actor.has(Permission::PeopleWriteAll) {
        return Err(OpError::forbidden(WRITE_REQUIRED));
    }
    if profile == BuiltInProfile::Administrator {
        return Err(OpError::invalid("profile", ADMINISTRATOR_PROFILE));
    }
    if profile != BuiltInProfile::Member && !actor.administrator() {
        return Err(OpError::forbidden(PROFILE_NEEDS_ADMINISTRATOR));
    }
    if people.is_empty() || people.len() > MAX_NEW_PEOPLE {
        return Err(OpError::invalid(
            "people",
            format!("Add between 1 and {MAX_NEW_PEOPLE} people at a time"),
        ));
    }
    let people = people
        .iter()
        .map(|person| {
            person
                .clone()
                .normalized()
                .ok_or_else(|| OpError::invalid("email", "Every person needs an email address"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    // Each email is locked after the organization (taken by `Actor::load`),
    // in one order so two batches cannot deadlock. Emails compare trimmed and
    // ignoring case.
    let key = |person: &NewPerson| person.email.to_lowercase();
    let mut order: Vec<_> = people.iter().collect();
    order.sort_by_key(|person| key(person));
    for pair in order.windows(2) {
        if key(pair[0]) == key(pair[1]) {
            return Err(OpError::invalid(
                "email",
                format!("The email {} is listed twice", pair[1].email),
            ));
        }
    }
    let org_id = actor.org_id();
    let mut added = Vec::with_capacity(order.len());
    let mut present = Vec::new();
    let mut unavailable = Vec::new();
    let mut events = Vec::new();
    let now = chrono::Utc::now();
    for person in order {
        let owner = if crate::db::lock_new_user_email(tx, &person.email).await? {
            sqlx::query_scalar!(
                "SELECT org_id FROM users WHERE lower(email) = lower($1)",
                person.email,
            )
            .fetch_optional(&mut **tx)
            .await?
        } else {
            None
        };
        match (owner, existing) {
            (None, _) => {}
            (Some(owner), Existing::Skip) => {
                if owner == org_id {
                    present.push(person.email.clone());
                } else {
                    unavailable.push(person.email.clone());
                }
                continue;
            }
            (Some(owner), Existing::Refuse) => {
                return Err(OpError::conflict(if owner == org_id {
                    format!("A user with the email {} already exists", person.email)
                } else {
                    unavailable_email(&person.email)
                }));
            }
        }
        let user = sqlx::query!(
            r#"INSERT INTO users (id, org_id, email, name, active)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, email, name, active"#,
            Uuid::now_v7(),
            org_id,
            person.email,
            person.name,
            activation == Activation::Active,
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| {
            match error
                .as_database_error()
                .and_then(|error| error.constraint())
            {
                Some("users_email_key") => OpError::conflict(unavailable_email(&person.email)),
                _ => OpError::from(error),
            }
        })?;
        provisioning::provision(tx, org_id, user.id, profile).await?;
        let (person, payload) =
            identity(tx, org_id, user.id, user.email, user.name, user.active).await?;
        events.push(AppEvent::UserCreated {
            occurred_at: now,
            org_id,
            user: payload,
        });
        added.push(person);
    }
    Ok(Done {
        value: AddedPeople {
            added,
            present,
            unavailable,
        },
        events,
    })
}

/// Archives (`active = false`) or restores a person. Their time and history
/// stay. The organization keeps at least one active administrator.
pub(crate) async fn set_active(
    tx: &mut Tx,
    actor: &Actor,
    user_id: Uuid,
    active: bool,
) -> Result<Done<UserListItem>, OpError> {
    let write_all = actor.has(Permission::PeopleWriteAll);
    let write_managed = actor.has(Permission::PeopleWriteManaged);
    if !write_all && !write_managed {
        return Err(OpError::forbidden(WRITE_REQUIRED));
    }
    let org_id = actor.org_id();
    // The target is locked before the other administrators are, so two
    // administrators archiving each other cannot both pass the check below:
    // one waits for the other, or the deadlock check refuses it.
    let target = sqlx::query!(
        r#"SELECT u.active, COALESCE(p.is_administrator, false) AS "administrator!",
                  EXISTS (SELECT 1 FROM person_management_assignments m
                           WHERE m.org_id = u.org_id AND m.manager_id = $3
                             AND m.managed_user_id = u.id) AS "managed!"
             FROM users u
             LEFT JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
            WHERE u.id = $1 AND u.org_id = $2
            FOR NO KEY UPDATE OF u"#,
        user_id,
        org_id,
        actor.id(),
    )
    .fetch_optional(&mut **tx)
    .await?;
    // Someone who manages only some people learns nothing about the rest: a
    // person they do not manage and no person at all get the same refusal.
    let Some(target) = target.filter(|target| write_all || target.managed) else {
        return Err(if write_all {
            OpError::not_found(NOT_FOUND)
        } else {
            OpError::forbidden(MANAGED_REQUIRED)
        });
    };
    if !may_change_active(
        write_all,
        write_managed,
        target.managed,
        actor.administrator(),
        target.administrator,
    ) {
        return Err(OpError::forbidden(ADMINISTRATOR_TARGET));
    }
    if target.active
        && !active
        && target.administrator
        && !other_administrator(tx, org_id, user_id).await?
    {
        return Err(OpError::conflict(LAST_ADMINISTRATOR));
    }
    let user = sqlx::query!(
        r#"UPDATE users SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, email, name, active"#,
        user_id,
        org_id,
        active,
    )
    .fetch_one(&mut **tx)
    .await?;
    let (person, payload) =
        identity(tx, org_id, user.id, user.email, user.name, user.active).await?;
    let done = Done::new(person);
    // Plugins hear only about archiving; there is no restore event.
    if active_transition(Some(target.active), active) == Some(ActiveTransition::Deactivated) {
        let event = AppEvent::UserDeactivated {
            occurred_at: chrono::Utc::now(),
            org_id,
            user: payload,
        };
        return Ok(done.with(event));
    }
    Ok(done)
}

/// The answer for the caller and the plugin payload for one person, whose
/// profile and Administrator identity come from their stored state.
async fn identity(
    tx: &mut Tx,
    org_id: Uuid,
    id: Uuid,
    email: String,
    name: String,
    active: bool,
) -> Result<(UserListItem, UserPayload), OpError> {
    let payload = crate::server_fns::user_payload(tx, org_id, id, &email, &name).await?;
    Ok((
        UserListItem {
            id,
            email,
            name,
            active,
        },
        payload,
    ))
}

/// Whether another active administrator remains. Their accounts are locked
/// `FOR SHARE` until commit, as the permission editor's check does, so they
/// cannot be archived meanwhile.
async fn other_administrator(tx: &mut Tx, org_id: Uuid, exclude: Uuid) -> Result<bool, OpError> {
    let found = sqlx::query_scalar!(
        "SELECT u.id FROM users u
           JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
          WHERE u.org_id = $1 AND u.id <> $2 AND u.active AND p.is_administrator
          ORDER BY u.id
          FOR SHARE OF u",
        org_id,
        exclude,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(!found.is_empty())
}
