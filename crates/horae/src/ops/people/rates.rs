//! A person's default billable and cost rates, always in the workspace's
//! default currency.
//!
//! Each rate follows its own grants (`horae-core`'s `billable_rate_access` and
//! `cost_rate_access`), and both need read access to the person: a rate grant
//! without a People grant opens no one's rates.

use horae_core::permissions::catalog::Permission;
use horae_core::permissions::rates::{
    BillableRateOwner, BillableRateResource, RateAction, RateEdit, billable_rate_access,
    cost_rate_access,
};
use horae_core::permissions::{AccessScope, ManagementAssignments, ScopedResource};
use uuid::Uuid;

use crate::models::permission_editor::PermissionRequester;
use crate::models::user::{PersonRate, PersonRates};
use crate::ops::{Actor, Done, OpError, Tx};

const ACCESS_REQUIRED: &str = "Current rate access is required";
const EDIT_REQUIRED: &str = "Current rate editing authority is required";
const SESSION_CHANGED: &str = "Your session changed. Reload this page.";

/// What the actor may do with each of one person's rates.
#[derive(Debug, Clone, Copy)]
struct RateAccess {
    read_billable: bool,
    write_billable: bool,
    read_cost: bool,
    write_cost: bool,
}

/// Reads a person's rates, each withheld unless the actor may read it.
/// `expected` is the requester the page was loaded for.
pub(crate) async fn load(
    tx: &mut Tx,
    actor: &Actor,
    person_id: Uuid,
    expected: PermissionRequester,
) -> Result<PersonRates, OpError> {
    same_requester(actor, expected)?;
    let access = access(tx, actor, person_id).await?;
    visible(tx, actor, person_id, access).await
}

/// Changes a person's rates. `currency` is the one the form showed, so an
/// amount typed for another currency is refused rather than reinterpreted.
pub(crate) async fn update(
    tx: &mut Tx,
    actor: &Actor,
    person_id: Uuid,
    expected: PermissionRequester,
    currency: &str,
    billable: RateEdit,
    cost: RateEdit,
) -> Result<Done<PersonRates>, OpError> {
    same_requester(actor, expected)?;
    let access = access(tx, actor, person_id).await?;
    let denied = |_| OpError::forbidden(EDIT_REQUIRED);
    let billable = billable.authorize(access.write_billable).map_err(denied)?;
    let cost = cost.authorize(access.write_cost).map_err(denied)?;
    let current = visible(tx, actor, person_id, access).await?;
    for (field, edit) in [("billable", billable), ("cost", cost)] {
        if let RateEdit::Set(cents) = edit {
            if cents < 0 {
                return Err(OpError::invalid(field, "A rate cannot be negative"));
            }
            if !currency.trim().eq_ignore_ascii_case(&current.currency) {
                return Err(OpError::conflict(
                    "The workspace currency changed. Reload and enter the rates again.",
                ));
            }
        }
    }
    let (keep_billable, billable) = column(billable);
    let (keep_cost, cost) = column(cost);
    sqlx::query!(
        "UPDATE users SET
           billable_rate_cents = CASE WHEN $3 THEN billable_rate_cents ELSE $4 END,
           cost_rate_cents = CASE WHEN $5 THEN cost_rate_cents ELSE $6 END
         WHERE org_id = $1 AND id = $2",
        actor.org_id(),
        person_id,
        keep_billable,
        billable,
        keep_cost,
        cost,
    )
    .execute(&mut **tx)
    .await?;
    Ok(Done::new(visible(tx, actor, person_id, access).await?))
}

/// Refuses a page loaded for another requester. The rates page answers this
/// 409 by asking for a reload, so it stays a conflict rather than the shared 403.
fn same_requester(actor: &Actor, expected: PermissionRequester) -> Result<(), OpError> {
    actor
        .expect_requester(Some(expected), SESSION_CHANGED)
        .map(drop)
        .map_err(|_| OpError::conflict(SESSION_CHANGED))
}

/// The actor's access to each of the person's rates, from its stored grants
/// and the people it manages.
async fn access(tx: &mut Tx, actor: &Actor, person_id: Uuid) -> Result<RateAccess, OpError> {
    let Some(grants) = actor.grants() else {
        return Err(OpError::forbidden(ACCESS_REQUIRED));
    };
    let org_id = actor.org_id();
    let managed = sqlx::query_scalar!(
        "SELECT managed_user_id FROM person_management_assignments
         WHERE org_id = $1 AND manager_id = $2 AND managed_user_id = $3",
        org_id,
        actor.id(),
        person_id,
    )
    .fetch_all(&mut **tx)
    .await?;
    let assignments = ManagementAssignments {
        actor_id: actor.id(),
        org_id,
        people: &managed,
        projects: &[],
    };
    let people_scope = if grants.contains(Permission::PeopleReadAll) {
        AccessScope::ORGANIZATION
    } else if grants.contains(Permission::PeopleReadManaged) {
        AccessScope::MANAGED_PEOPLE
    } else {
        AccessScope::NONE
    };
    let sees_person = people_scope.covers(
        actor.identity(),
        &ScopedResource {
            org_id,
            person_id: Some(person_id),
            project_id: None,
        },
        &assignments,
    );
    let billable = |action| {
        sees_person
            && billable_rate_access(
                grants,
                action,
                actor.identity(),
                &BillableRateResource {
                    org_id,
                    owner: BillableRateOwner::Person(person_id),
                },
                &assignments,
            )
    };
    let cost = |action| sees_person && cost_rate_access(grants, action, actor.identity(), org_id);
    Ok(RateAccess {
        read_billable: billable(RateAction::Read),
        write_billable: billable(RateAction::Write),
        read_cost: cost(RateAction::Read),
        write_cost: cost(RateAction::Write),
    })
}

/// The rates the actor may read, refusing when it may read neither.
async fn visible(
    tx: &mut Tx,
    actor: &Actor,
    person_id: Uuid,
    access: RateAccess,
) -> Result<PersonRates, OpError> {
    if !access.read_billable && !access.read_cost {
        return Err(OpError::forbidden(ACCESS_REQUIRED));
    }
    let row = sqlx::query!(
        r#"SELECT u.billable_rate_cents, u.cost_rate_cents,
                  upper(btrim(o.default_currency)) AS "currency!"
           FROM users u JOIN organizations o ON o.id = u.org_id
           WHERE u.org_id = $1 AND u.id = $2"#,
        actor.org_id(),
        person_id,
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found("Person not found"))?;
    let rate = |read: bool, cents: Option<i64>, editable: bool| {
        read.then_some(PersonRate { cents, editable })
    };
    Ok(PersonRates {
        person_id,
        currency: row.currency,
        billable: rate(
            access.read_billable,
            row.billable_rate_cents,
            access.write_billable,
        ),
        cost: rate(access.read_cost, row.cost_rate_cents, access.write_cost),
    })
}

/// Whether to keep the column, and its new value otherwise.
fn column(edit: RateEdit) -> (bool, Option<i64>) {
    match edit {
        RateEdit::Unchanged => (true, None),
        RateEdit::Reset => (false, None),
        RateEdit::Set(cents) => (false, Some(cents)),
    }
}
