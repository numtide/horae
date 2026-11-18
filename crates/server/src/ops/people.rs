//! People operations: adding people and archiving or restoring them. The
//! People page's directory and each person's rates live in [`directory`] and
//! [`rates`].
//!
//! Adding people needs `PeopleWriteAll`, which the Administrator, Executive
//! Manager and People Admin profiles hold; the CLI and OIDC sign-up add people
//! through their own channels (see [`add`]). Archiving and restoring also accept
//! `PeopleWriteManaged` for the people the actor manages. Who is an
//! administrator is decided in the permission editor, so these operations
//! never name one for a signed-in person (only the server's operator can) and
//! leave an administrator's account to another administrator.
//!
//! Answers carry identity only (`UserListItem`): a people grant does not open
//! rates or sign-in details.

use chrono::{DateTime, Utc};
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use horae_core::permissions::person_management::may_change_active;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::permissions::provisioning;
use super::{Actor, Channel, Done, OpError, Tx};
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, UserPayload, active_transition};

pub mod directory;
pub mod rates;

#[cfg(test)]
mod channel_tests;
#[cfg(test)]
mod tests;

const NOT_FOUND: &str = "Person not found";
const WRITE_REQUIRED: &str = "Permission to manage people is required";
const MANAGED_REQUIRED: &str = "You can archive or restore only the people you manage";
const ACTIVE_REQUIRED: &str = "An active account is required";
const READ_REQUIRED: &str = "Permission to view everyone is required to list archived people";
const ADMINISTRATOR_PROFILE: &str =
    "Administrators are chosen in the permission editor, not when adding a person";
const PROFILE_NEEDS_ADMINISTRATOR: &str =
    "Only an administrator can add a person with a profile other than Member";
const SYSTEM_ADDS_MEMBERS: &str = "Automatic sign-up adds members only";
const TOKEN_ADDS_NO_ONE: &str = "People can't be added with an API token";
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
const MAX_NEW_PEOPLE: usize = 500;

/// The whole account row, rates and sign-in binding included. Server-only:
/// answers to the browser use `UserListItem` or `CurrentUser`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub oidc_subject: Option<String>,
    pub cost_rate_cents: Option<i64>,
    pub billable_rate_cents: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

/// Trimmed fields, the email standing in for a blank name. `None` when there
/// is no usable email.
pub fn normalize(person: &NewPerson) -> Option<NewPerson> {
    let email = person.email.trim();
    if email.is_empty() || !email.contains('@') {
        return None;
    }
    let name = person.name.trim();
    Some(NewPerson {
        email: email.to_owned(),
        name: if name.is_empty() { email } else { name }.to_owned(),
    })
}

/// What adding people does with an email an account already uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Existing {
    /// Refuse the request: the person was meant to be new.
    Refuse,
    /// Leave the account alone and report the email as already present.
    Skip,
}

/// Identities for the directory pickers. Archived people need `PeopleReadAll`;
/// active people are listed for any active person, as the pickers need them.
pub async fn list(
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

/// A person as [`list`] shows them, and when their account last changed.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonRecord {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which people [`page`] lists, and where its page starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersonQuery {
    /// Active people, or archived ones (which need `PeopleReadAll`).
    pub active: bool,
    /// Only people whose `updated_at` is at or after it.
    pub updated_since: Option<DateTime<Utc>>,
    /// The last id of the previous page.
    pub after: Option<Uuid>,
    pub limit: u32,
}

/// The people of [`list`] in `id` order: up to `limit + 1` rows, the extra
/// one saying that another page follows.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    query: &PersonQuery,
) -> Result<Vec<PersonRecord>, OpError> {
    if !query.active && !actor.has(Permission::PeopleReadAll) {
        return Err(OpError::forbidden(READ_REQUIRED));
    }
    records(tx, actor, None, Some(query)).await
}

/// One person of [`page`]. An archived person is not found without
/// `PeopleReadAll`.
pub async fn get(tx: &mut Tx, actor: &Actor, person_id: Uuid) -> Result<PersonRecord, OpError> {
    records(tx, actor, Some(person_id), None)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| OpError::not_found(NOT_FOUND))
}

/// [`list`]'s rules, for one person or a page of them.
async fn records(
    tx: &mut Tx,
    actor: &Actor,
    person_id: Option<Uuid>,
    query: Option<&PersonQuery>,
) -> Result<Vec<PersonRecord>, OpError> {
    if !actor.is_active() {
        return Err(OpError::forbidden(ACTIVE_REQUIRED));
    }
    Ok(sqlx::query_as!(
        PersonRecord,
        r#"SELECT id, name, email, active,
                  created_at as "created_at: DateTime<Utc>",
                  updated_at as "updated_at: DateTime<Utc>"
           FROM users
           WHERE org_id = $1 AND (active OR $2)
             AND ($3::uuid IS NULL OR id = $3) AND ($4::bool IS NULL OR active = $4)
             AND ($5::timestamptz IS NULL OR updated_at >= $5)
             AND id > $6
           ORDER BY id LIMIT $7"#,
        actor.org_id(),
        actor.has(Permission::PeopleReadAll),
        person_id,
        query.map(|query| query.active),
        query.and_then(|query| query.updated_since) as Option<DateTime<Utc>>,
        query.and_then(|query| query.after).unwrap_or(Uuid::nil()),
        query.map_or(1, |query| i64::from(query.limit) + 1),
    )
    .fetch_all(&mut **tx)
    .await?)
}

/// Adds people with `profile`'s permission state.
///
/// What each channel may add:
/// - a signed-in person: anyone with `PeopleWriteAll` adds members; a person
///   with more than the Member profile is effectively a permission change, so
///   only an administrator may choose one, and nobody becomes an administrator
///   here;
/// - the server's operator (`horae user create`): any built-in profile,
///   Administrator included;
/// - the system (an OIDC first sign-in): members only;
/// - a personal access token: nobody, since the API adds no one.
pub async fn add(
    tx: &mut Tx,
    actor: &Actor,
    people: &[NewPerson],
    profile: BuiltInProfile,
    activation: Activation,
    existing: Existing,
) -> Result<Done<AddedPeople>, OpError> {
    match actor.channel() {
        Channel::Session => {
            if !actor.has(Permission::PeopleWriteAll) {
                return Err(OpError::forbidden(WRITE_REQUIRED));
            }
            if profile == BuiltInProfile::Administrator {
                return Err(OpError::invalid("profile", ADMINISTRATOR_PROFILE));
            }
            if profile != BuiltInProfile::Member && !actor.administrator() {
                return Err(OpError::forbidden(PROFILE_NEEDS_ADMINISTRATOR));
            }
        }
        Channel::Token { .. } => return Err(OpError::forbidden(TOKEN_ADDS_NO_ONE)),
        Channel::Operator => {}
        Channel::System => {
            if profile != BuiltInProfile::Member {
                return Err(OpError::forbidden(SYSTEM_ADDS_MEMBERS));
            }
        }
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
            normalize(person)
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
pub async fn set_active(
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
    let payload = crate::plugin::event::user_payload(tx, org_id, id, &email, &name).await?;
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
