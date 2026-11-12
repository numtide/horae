//! Scoped time reads; neither contextual labels nor running timers grant writes.

use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::types::EntryState;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{PermissionStorageError, ReadIsolation, begin_lock_only_read_async_commit};
use crate::models::people::PeopleCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{
    TimeEntryCursor, TimeEntryPage, TimeEntryQuery, TimesheetPage, TimesheetPeoplePage,
    TimesheetPeopleQuery, TimesheetPerson, TimesheetQuery, VisibleTimeEntry,
};
use crate::ops::{Actor, Caller, OpError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum TimeReadError {
    #[error("Current time-read authority is required")]
    Forbidden,
    #[error("Permission state is unavailable")]
    Unavailable,
    #[error("Invalid time-read query")]
    InvalidQuery,
    #[error(transparent)]
    Storage(#[from] PermissionStorageError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Op(OpError),
}

const TIME_READS: [Permission; 3] = [
    Permission::TimeReadOwn,
    Permission::TimeReadManaged,
    Permission::TimeReadAll,
];

/// Keep the organization, actor activity and grants stable until rows are
/// materialized. Authority comes from [`Actor::load`]; the returned selection
/// holds only the actor's time-read grants.
pub(super) async fn begin_read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<(Transaction<'static, Postgres>, PermissionSelection), TimeReadError> {
    let mut tx = begin_lock_only_read_async_commit(pool, ReadIsolation::ReadCommitted).await?;
    let actor = Actor::load(&mut tx, Caller::session(org_id, actor_id))
        .await
        .map_err(|error| match error {
            // Only missing or invalid stored state is refused while loading.
            OpError::Forbidden(_) => TimeReadError::Unavailable,
            error => TimeReadError::Op(error),
        })?;
    let held: Vec<Permission> = TIME_READS
        .into_iter()
        .filter(|permission| actor.has(*permission))
        .collect();
    if held.is_empty() {
        return Err(TimeReadError::Forbidden);
    }
    Ok((tx, PermissionSelection::new(&held)))
}

/// Resolve selection and entries together; selecting a person never widens row scope.
pub(crate) async fn sheet(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimesheetQuery,
) -> Result<TimesheetPage, TimeReadError> {
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    if query
        .expected_requester
        .is_some_and(|expected| expected != requester)
    {
        return Err(TimeReadError::Forbidden);
    }
    let (mut tx, grants) = begin_read(pool, org_id, actor_id).await?;
    let subject_id = query.subject_id.unwrap_or(actor_id);
    // A teammate can be archived independently of the requester. Hold its
    // activity stable across discovery and the subsequent entry query too.
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
        org_id,
        subject_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(TimeReadError::Forbidden)?;
    let subject = fetch_people(
        &mut tx,
        requester,
        &grants,
        &TimesheetPeopleQuery {
            user_id: Some(subject_id),
            ..TimesheetPeopleQuery::default()
        },
    )
    .await?
    .into_iter()
    .next()
    .ok_or(TimeReadError::Forbidden)?;
    let page = fetch_entries(
        &mut tx,
        requester,
        &grants,
        &TimeEntryQuery {
            date_from: query.date_from,
            date_to: query.date_to,
            user_id: Some(subject_id),
            project_id: None,
            after: query.after.clone(),
        },
    )
    .await?;
    let zone = crate::timezone::person_zone(&mut *tx, subject_id).await?;
    tx.commit().await?;
    Ok(TimesheetPage {
        requester,
        subject,
        subject_utc_offset_seconds: crate::timezone::utc_offset_seconds(&zone, chrono::Utc::now()),
        entries: page.entries,
        next_after: page.next_after,
    })
}

/// Discover identities without requiring time in the displayed date range.
pub(crate) async fn people(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimesheetPeopleQuery,
) -> Result<TimesheetPeoplePage, TimeReadError> {
    let (mut tx, grants) = begin_read(pool, org_id, actor_id).await?;
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    let mut people = fetch_people(&mut tx, requester, &grants, query).await?;
    let next_after = if people.len() > 50 {
        people.truncate(50);
        people.last().map(|person| PeopleCursor {
            name: person.name.clone(),
            id: person.id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(TimesheetPeoplePage {
        requester,
        people,
        next_after,
    })
}

async fn fetch_people(
    tx: &mut Transaction<'_, Postgres>,
    requester: PermissionRequester,
    grants: &PermissionSelection,
    query: &TimesheetPeopleQuery,
) -> Result<Vec<TimesheetPerson>, TimeReadError> {
    let PermissionRequester {
        org_id,
        user_id: actor_id,
    } = requester;
    let search = query.search.trim();
    if search.contains('\0')
        || search.chars().count() > 100
        || query
            .after
            .as_ref()
            .is_some_and(|cursor| cursor.name.contains('\0'))
    {
        return Err(TimeReadError::InvalidQuery);
    }
    let people = sqlx::query_as!(
        TimesheetPerson,
        "SELECT u.id,u.name FROM users u
         WHERE u.org_id=$1 AND u.active AND (
           $5::bool OR ($3::bool AND u.id=$2) OR ($4::bool AND (
             EXISTS (SELECT 1 FROM person_management_assignments m
               WHERE m.org_id=u.org_id AND m.manager_id=$2 AND m.managed_user_id=u.id)
             OR EXISTS (SELECT 1 FROM project_management_assignments m
               JOIN projects p ON p.id=m.project_id AND p.org_id=m.org_id
               JOIN clients c ON c.id=p.client_id AND c.org_id=p.org_id
               WHERE m.org_id=u.org_id AND m.manager_id=$2 AND (
                 EXISTS (SELECT 1 FROM assignments a
                   WHERE a.project_id=p.id AND a.user_id=u.id)
                 OR EXISTS (SELECT 1 FROM time_entries e
                   JOIN tasks t ON t.id=e.task_id AND t.org_id=e.org_id
                   WHERE e.org_id=u.org_id AND e.project_id=p.id AND e.user_id=u.id))))))
           AND strpos(lower(u.name),lower($6)) > 0
           AND ($7::uuid IS NULL OR u.id=$7)
           AND ($8::text IS NULL OR (u.name,u.id)>($8::text,$9::uuid))
         ORDER BY u.name,u.id LIMIT 51",
        org_id,
        actor_id,
        grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged),
        grants.contains(Permission::TimeReadAll),
        search,
        query.user_id,
        query.after.as_ref().map(|cursor| cursor.name.as_str()),
        query.after.as_ref().map(|cursor| cursor.id),
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(people)
}

/// Session-derived identity only; materialize rows while authority is stable.
pub(crate) async fn read(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    query: &TimeEntryQuery,
) -> Result<TimeEntryPage, TimeReadError> {
    let (mut tx, grants) = begin_read(pool, org_id, actor_id).await?;
    let result = fetch_entries(
        &mut tx,
        PermissionRequester {
            org_id,
            user_id: actor_id,
        },
        &grants,
        query,
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}

async fn fetch_entries(
    tx: &mut Transaction<'_, Postgres>,
    requester: PermissionRequester,
    grants: &PermissionSelection,
    query: &TimeEntryQuery,
) -> Result<TimeEntryPage, TimeReadError> {
    let PermissionRequester {
        org_id,
        user_id: actor_id,
    } = requester;
    let own = grants.contains(Permission::TimeReadOwn);
    let managed = grants.contains(Permission::TimeReadManaged);
    let all = grants.contains(Permission::TimeReadAll);
    if query.date_from > query.date_to
        || query.after.as_ref().is_some_and(|cursor| {
            cursor.spent_date < query.date_from || cursor.spent_date > query.date_to
        })
    {
        return Err(TimeReadError::InvalidQuery);
    }
    let mut entries = sqlx::query_as!(
        VisibleTimeEntry,
        r#"SELECT e.id, e.user_id, u.name AS user_name,
            e.project_id, p.name AS project_name, e.task_id, t.name AS task_name,
            p.client_id, c.name AS client_name, e.spent_date AS "spent_date: _", e.minutes,
            e.rounded_minutes, e.notes, e.billable, e.is_running, e.started_at AS "started_at: _",
            e.start_minute, e.sort_order, e.state AS "state: EntryState", e.created_at AS "created_at: _"
         FROM time_entries e
         JOIN users u ON u.id=e.user_id AND u.org_id=e.org_id
         JOIN projects p ON p.id=e.project_id AND p.org_id=e.org_id
         JOIN tasks t ON t.id=e.task_id AND t.org_id=e.org_id
         JOIN clients c ON c.id=p.client_id AND c.org_id=e.org_id
         WHERE e.org_id=$1 AND (
             $5::bool OR ($3::bool AND e.user_id=$2) OR ($4::bool AND (
                 EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.managed_user_id=e.user_id)
                 OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.project_id=e.project_id))))
           AND e.spent_date BETWEEN $6 AND $7
           AND ($8::uuid IS NULL OR e.user_id=$8)
           AND ($9::uuid IS NULL OR e.project_id=$9)
           AND ($10::date IS NULL OR (e.spent_date,e.created_at,e.id)<($10,$11,$12))
         ORDER BY e.spent_date DESC,e.created_at DESC,e.id DESC LIMIT 501"#,
        org_id, actor_id, own, managed, all, query.date_from as _, query.date_to as _,
        query.user_id, query.project_id,
        query.after.as_ref().map(|cursor| cursor.spent_date) as _,
        query.after.as_ref().map(|cursor| cursor.created_at) as _,
        query.after.as_ref().map(|cursor| cursor.id),
    ).fetch_all(&mut **tx).await?;
    let next_after = if entries.len() > 500 {
        entries.truncate(500);
        entries.last().map(|entry| TimeEntryCursor {
            spent_date: entry.spent_date,
            created_at: entry.created_at,
            id: entry.id,
        })
    } else {
        None
    };
    Ok(TimeEntryPage {
        requester,
        entries,
        next_after,
    })
}
