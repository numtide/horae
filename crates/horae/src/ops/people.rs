//! People operations: adding people and archiving or restoring them.
//!
//! Adding and archiving need `PeopleWriteAll`, which the Administrator,
//! Executive Manager and People Admin profiles hold. Who is an administrator
//! is decided only in the permission editor, so these operations never name
//! one and leave an administrator's account to another administrator.

use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use uuid::Uuid;

use super::{Actor, Done, OpError, Tx};
use crate::models::User;
use crate::models::user::{Activation, AddedPeople, NewPerson, UserListItem};
use crate::plugin::AppEvent;
use crate::plugin::event::{ActiveTransition, active_transition};
use crate::server_fns::provisioning;

const NOT_FOUND: &str = "Person not found";
const WRITE_REQUIRED: &str = "Permission to manage people is required";
const ACTIVE_REQUIRED: &str = "An active account is required";
const READ_REQUIRED: &str = "Permission to view everyone is required to list archived people";
const ADMINISTRATOR_PROFILE: &str =
    "Administrators are chosen in the permission editor, not when adding a person";
const PROFILE_NEEDS_ADMINISTRATOR: &str =
    "Only an administrator can add a person with a profile other than Member";
const ADMINISTRATOR_TARGET: &str = "Only an administrator can archive or restore an administrator";
const LAST_ADMINISTRATOR: &str = "The organization must keep at least one active administrator.";

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

fn authorize_write(actor: &Actor) -> Result<(), OpError> {
    if actor.has(Permission::PeopleWriteAll) {
        Ok(())
    } else {
        Err(OpError::forbidden(WRITE_REQUIRED))
    }
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
    authorize_write(actor)?;
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
    for person in order {
        let taken = crate::db::lock_new_user_email(tx, &person.email).await?;
        match (taken, existing) {
            (false, _) => {}
            (true, Existing::Skip) => {
                present.push(person.email.clone());
                continue;
            }
            (true, Existing::Refuse) => {
                return Err(OpError::conflict(format!(
                    "A user with the email {} already exists",
                    person.email
                )));
            }
        }
        let user = sqlx::query_as!(
            User,
            r#"INSERT INTO users (id, org_id, email, name, org_role, active)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, org_id, email, name, oidc_subject,
                       org_role as "org_role: horae_core::types::OrgRole",
                       cost_rate_cents, billable_rate_cents, active,
                       created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
            Uuid::now_v7(),
            org_id,
            person.email,
            person.name,
            provisioning::placeholder_role(profile) as horae_core::types::OrgRole,
            activation == Activation::Active,
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| {
            match error
                .as_database_error()
                .and_then(|error| error.constraint())
            {
                Some("users_email_key") => OpError::conflict(format!(
                    "A user with the email {} already exists",
                    person.email
                )),
                _ => OpError::from(error),
            }
        })?;
        provisioning::provision(tx, org_id, user.id, profile).await?;
        added.push(user);
    }
    let now = chrono::Utc::now();
    let mut events = Vec::with_capacity(added.len());
    for user in &added {
        events.push(AppEvent::UserCreated {
            occurred_at: now,
            org_id,
            user: crate::server_fns::user_payload(
                &mut **tx,
                org_id,
                user.id,
                &user.email,
                &user.name,
            )
            .await?,
        });
    }
    Ok(Done {
        value: AddedPeople { added, present },
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
) -> Result<Done<User>, OpError> {
    authorize_write(actor)?;
    let org_id = actor.org_id();
    // The target is locked before the other administrators are, so two
    // administrators archiving each other cannot both pass the check below:
    // one waits for the other, or the deadlock check refuses it.
    let target = sqlx::query!(
        r#"SELECT u.active, COALESCE(p.is_administrator, false) AS "administrator!"
             FROM users u
             LEFT JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
            WHERE u.id = $1 AND u.org_id = $2
            FOR NO KEY UPDATE OF u"#,
        user_id,
        org_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found(NOT_FOUND))?;
    if target.administrator && !actor.administrator() {
        return Err(OpError::forbidden(ADMINISTRATOR_TARGET));
    }
    if target.active
        && !active
        && target.administrator
        && !other_administrator(tx, org_id, user_id).await?
    {
        return Err(OpError::conflict(LAST_ADMINISTRATOR));
    }
    let user = sqlx::query_as!(
        User,
        r#"UPDATE users SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: horae_core::types::OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        user_id,
        org_id,
        active,
    )
    .fetch_one(&mut **tx)
    .await?;
    let done = Done::new(user);
    // Plugins hear only about archiving; there is no restore event.
    if active_transition(Some(target.active), active) == Some(ActiveTransition::Deactivated) {
        let user = &done.value;
        let event = AppEvent::UserDeactivated {
            occurred_at: chrono::Utc::now(),
            org_id,
            user: crate::server_fns::user_payload(
                &mut **tx,
                org_id,
                user.id,
                &user.email,
                &user.name,
            )
            .await?,
        };
        return Ok(done.with(event));
    }
    Ok(done)
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
