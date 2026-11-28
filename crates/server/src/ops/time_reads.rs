//! Scoped time reads: the Timesheet, its people picker, entry pages and the
//! ordinary time report. Neither contextual labels nor running timers grant
//! writes. Run them on [`super::begin_read`]: the actor's locks keep the
//! organization, its activity and grants stable until rows are materialized.

use chrono::{DateTime, NaiveDate, Utc};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use horae_core::types::EntryState;
use uuid::Uuid;

use super::{Actor, OpError, Tx};
use crate::models::TimeEntry;
use crate::models::people::PeopleCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{
    TimeEntryCursor, TimesheetPage, TimesheetPeoplePage, TimesheetPeopleQuery, TimesheetPerson,
    TimesheetQuery, VisibleTimeEntry,
};

/// Inclusive dates and optional narrowing filters, applied after scope checks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimeEntryQuery {
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub user_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub after: Option<TimeEntryCursor>,
}

/// Reauthorized on every page. Exhaust pages before presenting period totals.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TimeEntryPage {
    pub requester: PermissionRequester,
    pub entries: Vec<VisibleTimeEntry>,
    pub next_after: Option<TimeEntryCursor>,
}

pub mod reports;
#[cfg(test)]
mod tests;

const TIME_READS: [Permission; 3] = [
    Permission::TimeReadOwn,
    Permission::TimeReadManaged,
    Permission::TimeReadAll,
];

/// What [`list`] narrows the readable entries to; every filter is optional.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntryFilter {
    pub person_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    /// Inclusive spent dates.
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub is_running: Option<bool>,
    /// Entries whose `updated_at` is at or after this instant.
    pub updated_since: Option<DateTime<Utc>>,
}

const ENTRY_NOT_FOUND: &str = "Time entry not found";
const DATES_REVERSED: &str = "The start date is after the end date";

/// The entries the actor may read, with the Timesheet's scope (own, managed
/// or all), in `id` order after `after`, `limit` at most.
pub async fn list(
    tx: &mut Tx,
    actor: &Actor,
    filter: &EntryFilter,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<TimeEntry>, OpError> {
    let grants = require_time_reads(actor, TIME_READ_REQUIRED)?;
    if let (Some(from), Some(to)) = (filter.from, filter.to)
        && from > to
    {
        return Err(OpError::invalid("from", DATES_REVERSED));
    }
    readable(tx, actor, &grants, filter, None, after, limit).await
}

/// One entry the actor may read. One outside their scope is not found, as
/// in the web app.
pub async fn get(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<TimeEntry, OpError> {
    let grants = time_reads(actor).ok_or_else(|| OpError::not_found(ENTRY_NOT_FOUND))?;
    readable(
        tx,
        actor,
        &grants,
        &EntryFilter::default(),
        Some(id),
        None,
        1,
    )
    .await?
    .pop()
    .ok_or_else(|| OpError::not_found(ENTRY_NOT_FOUND))
}

async fn readable(
    tx: &mut Tx,
    actor: &Actor,
    grants: &PermissionSelection,
    filter: &EntryFilter,
    id: Option<Uuid>,
    after: Option<Uuid>,
    limit: i64,
) -> Result<Vec<TimeEntry>, OpError> {
    // The scope is `fetch_entries`'s: own, managed people and projects, or all.
    Ok(sqlx::query_as!(
        TimeEntry,
        r#"SELECT e.id, e.org_id, e.user_id, e.project_id, e.task_id,
                e.spent_date AS "spent_date: NaiveDate", e.minutes, e.start_minute, e.sort_order,
                e.rounded_minutes, e.notes, e.billable, e.is_running,
                e.started_at AS "started_at: DateTime<Utc>",
                e.state AS "state: EntryState", e.invoice_id,
                e.created_at AS "created_at: DateTime<Utc>",
                e.updated_at AS "updated_at: DateTime<Utc>"
         FROM time_entries e
         JOIN projects p ON p.id=e.project_id AND p.org_id=e.org_id
         WHERE e.org_id=$1 AND (
             $5::bool OR ($3::bool AND e.user_id=$2) OR ($4::bool AND (
                 EXISTS (SELECT 1 FROM person_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.managed_user_id=e.user_id)
                 OR EXISTS (SELECT 1 FROM project_management_assignments m
                     WHERE m.org_id=e.org_id AND m.manager_id=$2 AND m.project_id=e.project_id))))
           AND ($6::uuid IS NULL OR e.id=$6)
           AND ($7::uuid IS NULL OR e.user_id=$7)
           AND ($8::uuid IS NULL OR e.project_id=$8)
           AND ($9::uuid IS NULL OR e.task_id=$9)
           AND ($10::uuid IS NULL OR p.client_id=$10)
           AND ($11::date IS NULL OR e.spent_date>=$11)
           AND ($12::date IS NULL OR e.spent_date<=$12)
           AND ($13::bool IS NULL OR e.is_running=$13)
           AND ($14::timestamptz IS NULL OR e.updated_at>=$14)
           AND ($15::uuid IS NULL OR e.id>$15)
         ORDER BY e.id LIMIT $16"#,
        actor.org_id(),
        actor.id(),
        grants.contains(Permission::TimeReadOwn),
        grants.contains(Permission::TimeReadManaged),
        grants.contains(Permission::TimeReadAll),
        id,
        filter.person_id,
        filter.project_id,
        filter.task_id,
        filter.client_id,
        filter.from as Option<NaiveDate>,
        filter.to as Option<NaiveDate>,
        filter.is_running,
        filter.updated_since as Option<DateTime<Utc>>,
        after,
        limit,
    )
    .fetch_all(&mut **tx)
    .await?)
}

const SHEET_FORBIDDEN: &str = "Timesheet access or identity has changed; refresh the page";
const SHEET_INVALID: &str = "Invalid Timesheet date range or cursor";
const TIME_READ_REQUIRED: &str = "Current time-read authority is required";
const PEOPLE_INVALID: &str = "Invalid timesheet people query";

/// The actor's time-read grants, `None` when it holds none. A selection
/// always carries the Member floor, so an empty one cannot mean "none".
pub fn time_reads(actor: &Actor) -> Option<PermissionSelection> {
    let held: Vec<Permission> = TIME_READS
        .into_iter()
        .filter(|permission| actor.has(*permission))
        .collect();
    (!held.is_empty()).then(|| PermissionSelection::new(&held))
}

fn require_time_reads(actor: &Actor, refusal: &str) -> Result<PermissionSelection, OpError> {
    time_reads(actor).ok_or_else(|| OpError::forbidden(refusal))
}

/// Resolve selection and entries together; selecting a person never widens row scope.
pub async fn sheet(
    tx: &mut Tx,
    actor: &Actor,
    query: &TimesheetQuery,
) -> Result<TimesheetPage, OpError> {
    let requester = actor.expect_requester(query.expected_requester, SHEET_FORBIDDEN)?;
    let grants = require_time_reads(actor, SHEET_FORBIDDEN)?;
    let (org_id, actor_id) = (actor.org_id(), actor.id());
    let subject_id = query.subject_id.unwrap_or(actor_id);
    // A teammate can be archived independently of the requester. Hold its
    // activity stable across discovery and the subsequent entry query too.
    // The actor's own row is already held: `Actor::load` locked it, active.
    if subject_id != actor_id {
        sqlx::query_scalar!(
            "SELECT id FROM users WHERE org_id = $1 AND id = $2 AND active FOR SHARE",
            org_id,
            subject_id
        )
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| OpError::forbidden(SHEET_FORBIDDEN))?;
    }
    let subject = fetch_people(
        tx,
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
    .ok_or_else(|| OpError::forbidden(SHEET_FORBIDDEN))?;
    let entries = TimeEntryQuery {
        date_from: query.date_from,
        date_to: query.date_to,
        user_id: Some(subject_id),
        project_id: None,
        after: query.after.clone(),
    };
    if !entry_query_is_valid(&entries) {
        return Err(OpError::bad_input(SHEET_INVALID));
    }
    let page = fetch_entries(tx, requester, &grants, &entries).await?;
    let zone = crate::timezone::person_zone(&mut **tx, subject_id).await?;
    Ok(TimesheetPage {
        requester,
        subject,
        subject_utc_offset_seconds: crate::timezone::utc_offset_seconds(&zone, chrono::Utc::now()),
        entries: page.entries,
        next_after: page.next_after,
    })
}

/// Discover identities without requiring time in the displayed date range.
pub async fn people(
    tx: &mut Tx,
    actor: &Actor,
    query: &TimesheetPeopleQuery,
) -> Result<TimesheetPeoplePage, OpError> {
    let grants = require_time_reads(actor, TIME_READ_REQUIRED)?;
    let search = query.search.trim();
    if search.contains('\0')
        || search.chars().count() > 100
        || query
            .after
            .as_ref()
            .is_some_and(|cursor| cursor.name.contains('\0'))
    {
        return Err(OpError::bad_input(PEOPLE_INVALID));
    }
    let requester = actor.requester();
    let mut people = fetch_people(tx, requester, &grants, query).await?;
    let next_after = if people.len() > 50 {
        people.truncate(50);
        people.last().map(|person| PeopleCursor {
            name: person.name.clone(),
            id: person.id,
        })
    } else {
        None
    };
    Ok(TimesheetPeoplePage {
        requester,
        people,
        next_after,
    })
}

async fn fetch_people(
    tx: &mut Tx,
    requester: PermissionRequester,
    grants: &PermissionSelection,
    query: &TimesheetPeopleQuery,
) -> Result<Vec<TimesheetPerson>, OpError> {
    let PermissionRequester {
        org_id,
        user_id: actor_id,
    } = requester;
    let search = query.search.trim();
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

fn entry_query_is_valid(query: &TimeEntryQuery) -> bool {
    query.date_from <= query.date_to
        && !query.after.as_ref().is_some_and(|cursor| {
            cursor.spent_date < query.date_from || cursor.spent_date > query.date_to
        })
}

async fn fetch_entries(
    tx: &mut Tx,
    requester: PermissionRequester,
    grants: &PermissionSelection,
    query: &TimeEntryQuery,
) -> Result<TimeEntryPage, OpError> {
    let PermissionRequester {
        org_id,
        user_id: actor_id,
    } = requester;
    let own = grants.contains(Permission::TimeReadOwn);
    let managed = grants.contains(Permission::TimeReadManaged);
    let all = grants.contains(Permission::TimeReadAll);
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
