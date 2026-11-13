//! Report access and rate access are separate grants (contract C02).

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::catalog::Permission::{self, *};
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::provision;
use crate::ops::{Caller, OpError, begin_read};
use crate::server_fns::test_seed::{SeedIds, seed};

/// The day every fixture entry is on.
const DAY: &str = "2026-09-07";

/// The viewer's organization with two people besides the viewer, each with
/// one hour on their own project at 100.00 an hour and a 40.00 hourly cost.
struct Fixture {
    viewer: SeedIds,
    managed: Person,
    other: Person,
}

struct Person {
    user_id: Uuid,
    project_id: Uuid,
}

async fn person(pool: &PgPool, viewer: &SeedIds, name: &str) -> Person {
    let user_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, cost_rate_cents) VALUES ($1, $2, $3, $4, 4000)",
        user_id,
        viewer.org_id,
        format!("{user_id}@test.com"),
        name,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents)
         VALUES ($1, $2, $3, $4, 'EUR', 10000)",
        project_id,
        viewer.org_id,
        viewer.client_id,
        format!("{name}'s project"),
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', 60, true)",
        Uuid::now_v7(),
        viewer.org_id,
        user_id,
        project_id,
        viewer.task_id,
    )
    .execute(pool)
    .await
    .unwrap();
    Person {
        user_id,
        project_id,
    }
}

/// `managed` is managed by the viewer as a person, and their project as a
/// project; `other` is not managed at all.
async fn fixture(pool: &PgPool) -> Fixture {
    let viewer = seed(pool, BuiltInProfile::Member).await;
    let managed = person(pool, &viewer, "Managed").await;
    let other = person(pool, &viewer, "Other").await;
    sqlx::query!(
        "INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        viewer.org_id,
        viewer.user_id,
        managed.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_management_assignments (id, org_id, manager_id, project_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        viewer.org_id,
        viewer.user_id,
        managed.project_id,
    )
    .execute(pool)
    .await
    .unwrap();
    Fixture {
        viewer,
        managed,
        other,
    }
}

fn period() -> (NaiveDate, NaiveDate) {
    let day = DAY.parse().unwrap();
    (day, day)
}

async fn grouped(
    pool: &PgPool,
    viewer: &SeedIds,
    grants: &[Permission],
    group_by: &str,
) -> Result<Vec<ReportRow>, OpError> {
    provision(pool, viewer, grants, false).await;
    let mut read = begin_read(pool, Caller::session(viewer.org_id, viewer.user_id)).await?;
    let result = time(
        &mut read.tx,
        &read.actor,
        period(),
        group_by,
        ReportFilters::default(),
    )
    .await;
    read.finish(result).await
}

async fn entries(
    pool: &PgPool,
    viewer: &SeedIds,
    grants: &[Permission],
) -> Result<Vec<DetailedReportRow>, OpError> {
    provision(pool, viewer, grants, false).await;
    let mut read = begin_read(pool, Caller::session(viewer.org_id, viewer.user_id)).await?;
    let result = detailed(
        &mut read.tx,
        &read.actor,
        period(),
        ReportFilters::default(),
    )
    .await;
    read.finish(result).await
}

fn row(rows: &[ReportRow], group_id: Uuid) -> &ReportRow {
    rows.iter().find(|row| row.group_id == group_id).unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reports_need_managed_or_all_time(pool: PgPool) {
    let fixture = fixture(&pool).await;
    for grants in [
        &[][..],
        &[TimeReadOwn, BillableRateReadAll, CostRateReadAll],
    ] {
        let error = grouped(&pool, &fixture.viewer, grants, "project")
            .await
            .unwrap_err();
        assert!(
            matches!(error, OpError::Forbidden(_)),
            "{grants:?}: {error:?}"
        );
        let error = entries(&pool, &fixture.viewer, grants).await.unwrap_err();
        assert!(
            matches!(error, OpError::Forbidden(_)),
            "{grants:?}: {error:?}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reading_time_without_rates_shows_no_amounts(pool: PgPool) {
    let fixture = fixture(&pool).await;
    let rows = grouped(&pool, &fixture.viewer, &[TimeReadAll], "project")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(row.total_minutes, 60);
        let payload = serde_json::to_value(row).unwrap();
        assert!(payload.get("billable_cents").is_none(), "{payload}");
        assert!(payload.get("cost_cents").is_none(), "{payload}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reading_rates_without_costs_shows_no_costs(pool: PgPool) {
    let fixture = fixture(&pool).await;
    let rows = grouped(
        &pool,
        &fixture.viewer,
        &[TimeReadAll, BillableRateReadAll, ReportProfitabilityRead],
        "project",
    )
    .await
    .unwrap();
    for row in &rows {
        assert_eq!((row.billable_cents, row.cost_cents), (Some(10000), None));
    }

    let rows = grouped(
        &pool,
        &fixture.viewer,
        &[TimeReadAll, BillableRateReadAll, CostRateReadAll],
        "person",
    )
    .await
    .unwrap();
    for row in &rows {
        assert_eq!(
            (row.billable_cents, row.cost_cents),
            (Some(10000), Some(4000))
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn costs_need_only_the_cost_grant(pool: PgPool) {
    let fixture = fixture(&pool).await;
    let rows = grouped(
        &pool,
        &fixture.viewer,
        &[TimeReadAll, CostRateReadAll],
        "project",
    )
    .await
    .unwrap();
    for row in &rows {
        assert_eq!((row.billable_cents, row.cost_cents), (None, Some(4000)));
    }
    // A project-specific cost is a cost like any other.
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role) VALUES ($1, $2, $3, 'freelancer')",
        Uuid::now_v7(),
        fixture.other.project_id,
        fixture.other.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_member_costs (id, org_id, project_id, user_id, cost_rate_cents)
         VALUES ($1, $2, $3, $4, 7777)",
        Uuid::now_v7(),
        fixture.viewer.org_id,
        fixture.other.project_id,
        fixture.other.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = grouped(
        &pool,
        &fixture.viewer,
        &[TimeReadAll, CostRateReadAll],
        "project",
    )
    .await
    .unwrap();
    assert_eq!(row(&rows, fixture.other.project_id).cost_cents, Some(7777));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn managed_rates_show_only_on_managed_projects(pool: PgPool) {
    let fixture = fixture(&pool).await;
    let grants = [TimeReadAll, BillableRateReadManaged];
    let rows = grouped(&pool, &fixture.viewer, &grants, "project")
        .await
        .unwrap();
    assert_eq!(
        row(&rows, fixture.managed.project_id).billable_cents,
        Some(10000)
    );
    assert_eq!(row(&rows, fixture.other.project_id).billable_cents, None);

    // A group mixing a visible and a hidden rate shows neither: the sum would
    // reveal the hidden one.
    let rows = grouped(&pool, &fixture.viewer, &grants, "client")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].total_minutes, rows[0].billable_cents), (120, None));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn managed_time_covers_managed_people_and_projects_only(pool: PgPool) {
    let fixture = fixture(&pool).await;
    // The other person's hour on the managed project is covered by project
    // management; their hour on their own project is not covered at all.
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', 30, true)",
        Uuid::now_v7(),
        fixture.viewer.org_id,
        fixture.other.user_id,
        fixture.managed.project_id,
        fixture.viewer.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = grouped(&pool, &fixture.viewer, &[TimeReadManaged], "project")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].group_id, rows[0].total_minutes),
        (fixture.managed.project_id, 90)
    );
    let details = entries(&pool, &fixture.viewer, &[TimeReadManaged])
        .await
        .unwrap();
    assert_eq!(
        details.iter().map(|row| row.minutes).collect::<Vec<_>>(),
        [60, 30]
    );

    let details = entries(&pool, &fixture.viewer, &[TimeReadAll])
        .await
        .unwrap();
    assert_eq!(details.len(), 3);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn foreign_and_inactive_viewers_are_refused(pool: PgPool) {
    let fixture = fixture(&pool).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    let mut read = begin_read(
        &pool,
        Caller::session(foreign.org_id, fixture.viewer.user_id),
    )
    .await
    .unwrap();
    let error = time(
        &mut read.tx,
        &read.actor,
        period(),
        "project",
        ReportFilters::default(),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, OpError::Forbidden(_)), "{error:?}");
    drop(read);

    sqlx::query!(
        "UPDATE users SET active = false WHERE id = $1",
        fixture.viewer.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = grouped(&pool, &fixture.viewer, &[TimeReadAll], "project")
        .await
        .unwrap_err();
    assert!(matches!(error, OpError::Forbidden(_)), "{error:?}");
}
