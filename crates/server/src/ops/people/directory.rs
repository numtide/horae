//! The People page: the whole directory under the actor's people-read grants,
//! with one week's hours. Workflow pickers use [`super::list`] or their own
//! operation scope.

use chrono::{Duration, NaiveDate};
use horae_core::permissions::catalog::Permission;
use horae_core::permissions::person_management::may_change_active;

use crate::models::people::{PeoplePage, PersonProfile, PersonSummary, WeekMinutes};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::permissions::provisioning::parse_profile;
use crate::ops::{Actor, OpError, Tx};

/// Every person the actor may see, active and archived, ordered by name then
/// id, with the minutes of the seven days from `week`, which the client passes
/// as its week start. `PeopleReadAll` lists the organization;
/// `PeopleReadManaged` only the people the actor manages. The authority locks
/// [`Actor::load`] took hold until the list is materialized.
///
/// Hours follow the time-read grants a person at a time: everyone with
/// `TimeReadAll`, the people the actor manages with `TimeReadManaged`, the
/// actor with `TimeReadOwn`. Managing a project opens only that project's
/// entries, never a person's whole week, so it shows no hours here.
pub async fn page(tx: &mut Tx, actor: &Actor, week: NaiveDate) -> Result<PeoplePage, OpError> {
    let all_people = actor.has(Permission::PeopleReadAll);
    if !all_people && !actor.has(Permission::PeopleReadManaged) {
        return Err(OpError::forbidden(
            "Current people-read authority is required",
        ));
    }
    let week_end = week
        .checked_add_signed(Duration::days(6))
        .ok_or_else(|| OpError::invalid("week", "Invalid week"))?;
    // The week is summed once for the organization through the date index,
    // not per person over their whole history.
    let rows = sqlx::query!(
        r#"WITH wk AS (
             SELECT user_id, SUM(minutes)::bigint AS total,
                    COALESCE(SUM(minutes) FILTER (WHERE billable), 0)::bigint AS billable
               FROM time_entries
              WHERE org_id=$1 AND spent_date BETWEEN $4 AND $5
              GROUP BY user_id)
           SELECT u.id, u.name, u.email, u.active,
                  COALESCE(p.is_administrator, false) AS "administrator!", d.managed AS "managed!",
                  p.source AS "source?", p.built_in_profile AS "built_in_profile?",
                  t.name AS "template?",
                  ($6::bool OR ($7::bool AND d.managed) OR ($8::bool AND u.id=$2)) AS "hours_visible!",
                  wk.total AS "total?", wk.billable AS "billable?"
             FROM users u
             CROSS JOIN LATERAL (SELECT EXISTS (
               SELECT 1 FROM person_management_assignments m
               WHERE m.org_id=u.org_id AND m.manager_id=$2 AND m.managed_user_id=u.id) AS managed) d
             LEFT JOIN person_permission_states p ON p.org_id=u.org_id AND p.user_id=u.id
             LEFT JOIN permission_templates t ON t.org_id=p.org_id AND t.id=p.template_id
             LEFT JOIN wk ON wk.user_id=u.id
            WHERE u.org_id=$1
              AND ($3::bool OR d.managed)
            ORDER BY u.name,u.id"#,
        actor.org_id(),
        actor.id(),
        all_people,
        week as _,
        week_end as _,
        actor.has(Permission::TimeReadAll),
        actor.has(Permission::TimeReadManaged),
        actor.has(Permission::TimeReadOwn),
    )
    .fetch_all(&mut **tx)
    .await?;
    let write_all = actor.has(Permission::PeopleWriteAll);
    let write_managed = actor.has(Permission::PeopleWriteManaged);
    // The same rule as `people::rates`: every listed person is already one the
    // actor may see.
    let rates_all =
        actor.has(Permission::BillableRateReadAll) || actor.has(Permission::CostRateReadAll);
    let rates_managed = actor.has(Permission::BillableRateReadManaged);
    let people = rows
        .into_iter()
        .map(|row| PersonSummary {
            // `set_active` refuses archiving yourself.
            can_change_active: row.id != actor.id()
                && may_change_active(
                    write_all,
                    write_managed,
                    row.managed,
                    actor.administrator(),
                    row.administrator,
                ),
            can_read_rates: rates_all || (rates_managed && row.managed),
            profile: match row.source.as_deref() {
                Some("built_in") => row
                    .built_in_profile
                    .as_deref()
                    .and_then(parse_profile)
                    .map(PersonProfile::BuiltIn),
                Some("template") => row.template.map(PersonProfile::Custom),
                Some("individual") => Some(PersonProfile::Individual),
                _ => None,
            },
            week: row.hours_visible.then(|| WeekMinutes {
                total: row.total.unwrap_or(0),
                billable: row.billable.unwrap_or(0),
            }),
            id: row.id,
            name: row.name,
            email: row.email,
            active: row.active,
        })
        .collect();
    Ok(PeoplePage {
        requester: PermissionRequester {
            org_id: actor.org_id(),
            user_id: actor.id(),
        },
        people,
    })
}
