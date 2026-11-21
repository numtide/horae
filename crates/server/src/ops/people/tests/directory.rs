//! `directory::page`: the whole People list with each person's profile and
//! the week's hours, which follow the time-read grants person by person.

use chrono::NaiveDate;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;
use uuid::Uuid;

use super::super::directory::page;
use crate::models::people::{PeopleActivity, PeoplePage, WeekMinutes};
use crate::ops::testing::provision;
use crate::ops::{Caller, OpError};
use crate::testing::{SeedIds, add_person, seed};

/// A Monday; the week runs to Sunday the 13th.
const WEEK: NaiveDate = NaiveDate::from_ymd_opt(2026, 9, 7).unwrap();

async fn read(pool: &PgPool, ids: &SeedIds) -> Result<PeoplePage, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(ids.org_id, ids.user_id)).await?;
    let result = page(&mut read.tx, &read.actor, PeopleActivity::All, WEEK).await;
    read.finish(result).await
}

async fn entry(pool: &PgPool, ids: &SeedIds, person: Uuid, day: &str, minutes: i32, billable: bool) {
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         VALUES ($1, $2, $3, $4, $5, $6::text::date, $7, $8)",
        Uuid::now_v7(),
        ids.org_id,
        person,
        ids.project_id,
        ids.task_id,
        day,
        minutes,
        billable,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn manage_person(pool: &PgPool, ids: &SeedIds, person: Uuid) {
    sqlx::query!(
        "INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        person,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn manage_project(pool: &PgPool, ids: &SeedIds) {
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, manager_id, project_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        ids.project_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

fn week_of(page: &PeoplePage, person: Uuid) -> Option<WeekMinutes> {
    page.people
        .iter()
        .find(|entry| entry.id == person)
        .expect("listed")
        .week
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_sees_everyones_profile_and_only_the_weeks_hours(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let teammate = add_person(&pool, &ids, BuiltInProfile::ProjectManager).await.user_id;
    entry(&pool, &ids, teammate, "2026-09-07", 120, true).await;
    entry(&pool, &ids, teammate, "2026-09-13", 45, false).await;
    // Outside the week on both sides.
    entry(&pool, &ids, teammate, "2026-09-06", 60, true).await;
    entry(&pool, &ids, teammate, "2026-09-14", 60, true).await;

    let listed = read(&pool, &ids).await.unwrap();

    assert_eq!(week_of(&listed, teammate), Some(WeekMinutes { total: 165, billable: 120 }));
    assert_eq!(week_of(&listed, ids.user_id), Some(WeekMinutes::default()));
    // Ordered by name: "Teammate", then "Test User".
    let profiles: Vec<_> = listed.people.iter().map(|p| p.profile.as_deref()).collect();
    assert_eq!(profiles, [Some("Project Manager"), Some("Administrator")]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reading_everyone_without_reading_their_time_shows_only_own_hours(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &ids, &[Permission::PeopleReadAll], false).await;
    let teammate = add_person(&pool, &ids, BuiltInProfile::Member).await.user_id;
    entry(&pool, &ids, teammate, "2026-09-08", 60, true).await;
    entry(&pool, &ids, ids.user_id, "2026-09-08", 30, true).await;

    let listed = read(&pool, &ids).await.unwrap();

    assert_eq!(week_of(&listed, teammate), None);
    assert_eq!(week_of(&listed, ids.user_id), Some(WeekMinutes { total: 30, billable: 30 }));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn managed_time_shows_managed_peoples_hours_but_not_a_managed_projects(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &ids, &[Permission::PeopleReadAll, Permission::TimeReadManaged], false).await;
    let managed = add_person(&pool, &ids, BuiltInProfile::Member).await.user_id;
    let on_my_project = add_person(&pool, &ids, BuiltInProfile::Member).await.user_id;
    manage_person(&pool, &ids, managed).await;
    manage_project(&pool, &ids).await;
    entry(&pool, &ids, managed, "2026-09-09", 90, false).await;
    entry(&pool, &ids, on_my_project, "2026-09-09", 90, true).await;

    let listed = read(&pool, &ids).await.unwrap();

    assert_eq!(week_of(&listed, managed), Some(WeekMinutes { total: 90, billable: 0 }));
    // A project's entries are not the person's week.
    assert_eq!(week_of(&listed, on_my_project), None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reading_managed_people_lists_only_them(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &ids, &[Permission::PeopleReadManaged], false).await;
    let managed = add_person(&pool, &ids, BuiltInProfile::Member).await.user_id;
    add_person(&pool, &ids, BuiltInProfile::Member).await;
    manage_person(&pool, &ids, managed).await;

    let listed = read(&pool, &ids).await.unwrap();

    let people: Vec<_> = listed.people.iter().map(|p| (p.id, p.profile.as_deref())).collect();
    assert_eq!(people, [(managed, Some("Member"))]);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_without_people_reads_is_refused(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;

    assert!(matches!(read(&pool, &ids).await, Err(OpError::Forbidden(_))));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn grants_chosen_one_by_one_read_as_individual(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    provision(&pool, &ids, &[Permission::PeopleReadAll], false).await;

    let listed = read(&pool, &ids).await.unwrap();

    assert_eq!(listed.people[0].profile.as_deref(), Some("Individual"));
}
