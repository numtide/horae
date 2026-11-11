//! A person's default billable and cost rates, always in the workspace's
//! default currency.

use super::*;
use crate::models::permission_editor::PermissionRequester;
#[cfg(feature = "server")]
use crate::models::user::PersonRate;
use crate::models::user::PersonRates;
use horae_core::permissions::rates::RateEdit;
#[cfg(feature = "server")]
use uuid::Uuid;

/// Read a person's rates, each withheld unless the viewer may read it.
#[server]
pub async fn load_person_rates(
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
) -> Result<PersonRates, ServerFnError> {
    let actor = rate_actor(expected_requester).await?;
    let state = crate::state::global_state().await;
    read_person_rates(&state.db, actor.org_id, actor.id, person_id).await
}

/// Change a person's rates. `currency` is the one the form showed, so an
/// amount typed for another currency is refused rather than reinterpreted.
#[server]
pub async fn update_person_rates(
    person_id: uuid::Uuid,
    expected_requester: PermissionRequester,
    currency: String,
    billable: RateEdit,
    cost: RateEdit,
) -> Result<PersonRates, ServerFnError> {
    let actor = rate_actor(expected_requester).await?;
    let state = crate::state::global_state().await;
    write_person_rates(
        &state.db,
        actor.org_id,
        actor.id,
        person_id,
        &currency,
        billable,
        cost,
    )
    .await
}

#[cfg(feature = "server")]
async fn rate_actor(expected: PermissionRequester) -> Result<User, ServerFnError> {
    let actor = require_user().await?;
    if expected.org_id != actor.org_id || expected.user_id != actor.id {
        return Err(conflict("Your session changed. Reload this page."));
    }
    Ok(actor)
}

/// What the actor may do with each of one person's rates.
#[cfg(feature = "server")]
#[derive(Debug, Clone, Copy)]
struct RateAccess {
    read_billable: bool,
    write_billable: bool,
    read_cost: bool,
    write_cost: bool,
}

/// Lock the actor and decide its access under the active policy. Without
/// detailed permissions an administrator holds both rates and a manager the
/// billable one, as for task and project rates; with them only the
/// BillableRate and CostRate grants count, plus read access to the person.
#[cfg(feature = "server")]
async fn authorize(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    actor_id: Uuid,
    person_id: Uuid,
) -> Result<RateAccess, ServerFnError> {
    use horae_core::permissions::{
        AccessScope, Actor, ManagementAssignments, ScopedResource,
        catalog::Permission,
        rates::{
            BillableRateOwner, BillableRateResource, RateAction, billable_rate_access,
            cost_rate_access,
        },
    };

    crate::server_fns::permissions::configure_administration(tx)
        .await
        .map_err(server_err)?;
    crate::db::lock_organization(tx, org_id, crate::db::OrganizationLock::Shared)
        .await
        .map_err(server_err)?;
    let actor = sqlx::query!(
        r#"SELECT u.org_role AS "role: OrgRole", o.permission_policy_version
           FROM users u JOIN organizations o ON o.id = u.org_id
           WHERE u.org_id = $1 AND u.id = $2 AND u.active FOR SHARE OF u"#,
        org_id,
        actor_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| forbidden("Current rate access is required"))?;
    match actor.permission_policy_version {
        0 => {
            let admin = actor.role == OrgRole::Admin;
            let billable = actor.role.is_manager_or_above();
            Ok(RateAccess {
                read_billable: billable,
                write_billable: billable,
                read_cost: admin,
                write_cost: admin,
            })
        }
        1 => {
            let permissions =
                crate::server_fns::permissions::load_person_permissions(tx, org_id, actor_id)
                    .await
                    .map_err(|error| {
                        tracing::error!(%error, "Unable to load rate permissions");
                        server_err("Rate permission state is unavailable")
                    })?
                    .ok_or_else(|| forbidden("Current rate access is required"))?;
            let managed = sqlx::query_scalar!(
                "SELECT managed_user_id FROM person_management_assignments
                 WHERE org_id = $1 AND manager_id = $2 AND managed_user_id = $3",
                org_id,
                actor_id,
                person_id,
            )
            .fetch_all(&mut **tx)
            .await
            .map_err(server_err)?;
            let actor = Actor {
                id: actor_id,
                org_id,
                active: true,
            };
            let assignments = ManagementAssignments {
                actor_id,
                org_id,
                people: &managed,
                projects: &[],
            };
            let grants = &permissions.grants;
            let people_scope = if grants.contains(Permission::PeopleReadAll) {
                AccessScope::ORGANIZATION
            } else if grants.contains(Permission::PeopleReadManaged) {
                AccessScope::MANAGED_PEOPLE
            } else {
                AccessScope::NONE
            };
            let sees_person = people_scope.covers(
                &actor,
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
                        &actor,
                        &BillableRateResource {
                            org_id,
                            owner: BillableRateOwner::Person(person_id),
                        },
                        &assignments,
                    )
            };
            let cost = |action| sees_person && cost_rate_access(grants, action, &actor, org_id);
            Ok(RateAccess {
                read_billable: billable(RateAction::Read),
                write_billable: billable(RateAction::Write),
                read_cost: cost(RateAction::Read),
                write_cost: cost(RateAction::Write),
            })
        }
        _ => Err(server_err("Rate permission state is unavailable")),
    }
}

/// Load the rates the actor may read, refusing when it may read neither.
#[cfg(feature = "server")]
async fn visible_rates(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    person_id: Uuid,
    access: RateAccess,
) -> Result<PersonRates, ServerFnError> {
    if !access.read_billable && !access.read_cost {
        return Err(forbidden("Current rate access is required"));
    }
    let row = sqlx::query!(
        r#"SELECT u.billable_rate_cents, u.cost_rate_cents,
                  upper(btrim(o.default_currency)) AS "currency!"
           FROM users u JOIN organizations o ON o.id = u.org_id
           WHERE u.org_id = $1 AND u.id = $2"#,
        org_id,
        person_id,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Person not found"))?;
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

#[cfg(feature = "server")]
pub(crate) async fn read_person_rates(
    db: &sqlx::PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    person_id: Uuid,
) -> Result<PersonRates, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let access = authorize(&mut tx, org_id, actor_id, person_id).await?;
    let rates = visible_rates(&mut tx, org_id, person_id, access).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(rates)
}

#[cfg(feature = "server")]
pub(crate) async fn write_person_rates(
    db: &sqlx::PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    person_id: Uuid,
    currency: &str,
    billable: RateEdit,
    cost: RateEdit,
) -> Result<PersonRates, ServerFnError> {
    let mut tx = db.begin().await.map_err(server_err)?;
    let access = authorize(&mut tx, org_id, actor_id, person_id).await?;
    let denied = |_| forbidden("Current rate editing authority is required");
    let billable = billable.authorize(access.write_billable).map_err(denied)?;
    let cost = cost.authorize(access.write_cost).map_err(denied)?;
    let current = visible_rates(&mut tx, org_id, person_id, access).await?;
    for edit in [billable, cost] {
        if let RateEdit::Set(cents) = edit {
            if cents < 0 {
                return Err(err(BAD_REQUEST, "A rate cannot be negative"));
            }
            if !currency.trim().eq_ignore_ascii_case(&current.currency) {
                return Err(conflict(
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
        org_id,
        person_id,
        keep_billable,
        billable,
        keep_cost,
        cost,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;
    let rates = visible_rates(&mut tx, org_id, person_id, access).await?;
    tx.commit().await.map_err(server_err)?;
    Ok(rates)
}

/// Whether to keep the column, and its new value otherwise.
#[cfg(feature = "server")]
fn column(edit: RateEdit) -> (bool, Option<i64>) {
    match edit {
        RateEdit::Unchanged => (true, None),
        RateEdit::Reset => (false, None),
        RateEdit::Set(cents) => (false, Some(cents)),
    }
}
