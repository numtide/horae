//! The People page: the whole directory under the actor's people-read grants,
//! with one week's hours. Workflow pickers use [`super::list`] or their own
//! operation scope.

use chrono::{Duration, NaiveDate};
use horae_core::permissions::catalog::{BuiltInProfile, Permission, profile_label};
use horae_core::permissions::person_management::may_change_active;

use crate::models::people::{PeopleActivity, PeoplePage, PersonSummary, WeekMinutes};
use crate::models::permission_editor::PermissionRequester;
use crate::ops::{Actor, OpError, Tx};

/// Every person the actor may see, ordered by name then id, with the minutes
/// of the seven days from `week`. `PeopleReadAll` lists the organization;
/// `PeopleReadManaged` only the people the actor manages. The authority locks
/// [`Actor::load`] took hold until the list is materialized.
///
/// Hours follow the time-read grants a person at a time: everyone with
/// `TimeReadAll`, the people the actor manages with `TimeReadManaged`, the
/// actor with `TimeReadOwn`. Managing a project opens only that project's
/// entries, never a person's whole week, so it shows no hours here.
pub async fn page(
    tx: &mut Tx,
    actor: &Actor,
    activity: PeopleActivity,
    week: NaiveDate,
) -> Result<PeoplePage, OpError> {
    let all_people = actor.has(Permission::PeopleReadAll);
    if !all_people && !actor.has(Permission::PeopleReadManaged) {
        return Err(OpError::forbidden(
            "Current people-read authority is required",
        ));
    }
    let week_end = week
        .checked_add_signed(Duration::days(6))
        .ok_or_else(|| OpError::invalid("week", "Invalid week"))?;
    let active = match activity {
        PeopleActivity::Active => Some(true),
        PeopleActivity::Archived => Some(false),
        PeopleActivity::All => None,
    };
    let rows = sqlx::query!(
        r#"SELECT u.id, u.name, u.email, u.active,
                  COALESCE(p.is_administrator, false) AS "administrator!", d.managed AS "managed!",
                  p.built_in_profile AS "built_in_profile?", t.name AS "template?",
                  (p.source = 'individual') AS "individual?",
                  w.total AS "total?", w.billable AS "billable?"
             FROM users u
             CROSS JOIN LATERAL (SELECT EXISTS (
               SELECT 1 FROM person_management_assignments m
               WHERE m.org_id=u.org_id AND m.manager_id=$2 AND m.managed_user_id=u.id) AS managed) d
             LEFT JOIN person_permission_states p ON p.org_id=u.org_id AND p.user_id=u.id
             LEFT JOIN permission_templates t ON t.org_id=p.org_id AND t.id=p.template_id
             LEFT JOIN LATERAL (
               SELECT COALESCE(SUM(e.minutes),0)::bigint AS total,
                      COALESCE(SUM(e.minutes) FILTER (WHERE e.billable),0)::bigint AS billable
                 FROM time_entries e
                WHERE e.org_id=u.org_id AND e.user_id=u.id AND e.spent_date BETWEEN $5 AND $6) w
               ON $7::bool OR ($8::bool AND d.managed) OR ($9::bool AND u.id=$2)
            WHERE u.org_id=$1
              AND ($3::bool OR d.managed)
              AND ($4::bool IS NULL OR u.active=$4)
            ORDER BY u.name,u.id"#,
        actor.org_id(),
        actor.id(),
        all_people,
        active,
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
    let people = rows
        .into_iter()
        .map(|row| PersonSummary {
            can_change_active: may_change_active(
                write_all,
                write_managed,
                row.managed,
                actor.administrator(),
                row.administrator,
            ),
            profile: row
                .built_in_profile
                .as_deref()
                .and_then(built_in_label)
                .map(str::to_owned)
                .or(row.template)
                .or_else(|| row.individual.unwrap_or(false).then(|| "Individual".into())),
            week: row.total.zip(row.billable).map(|(total, billable)| WeekMinutes { total, billable }),
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

/// The label of a stored built-in profile name (`project_manager`, …).
fn built_in_label(stored: &str) -> Option<&'static str> {
    let profile: BuiltInProfile = serde::Deserialize::deserialize(
        serde::de::value::StrDeserializer::<serde::de::value::Error>::new(stored),
    )
    .ok()?;
    Some(profile_label(profile))
}
