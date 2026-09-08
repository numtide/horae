use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

async fn fixtures(pool: &PgPool, currency: &str) -> (SeedIds, SeedIds) {
    let first = seed(pool, OrgRole::Manager).await;
    let second = SeedIds {
        org_id: first.org_id,
        user_id: Uuid::now_v7(),
        client_id: Uuid::now_v7(),
        project_id: Uuid::now_v7(),
        task_id: Uuid::now_v7(),
    };
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Test User')",
        second.user_id,
        second.org_id,
        format!("{}@test.com", second.user_id)
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', $3)",
        second.client_id,
        second.org_id,
        currency
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents) VALUES ($1, $2, $3, 'Widget', $4, 12000)",
        second.project_id, second.org_id, second.client_id, currency)
        .execute(pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Dev')",
        second.task_id,
        second.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 6000, cost_rate_cents = 6030 WHERE org_id = $1",
        first.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) VALUES ($1, $2, true, 9000)",
        first.project_id, first.task_id)
        .execute(pool).await.unwrap();
    (first, second)
}

async fn entries(pool: &PgPool, ids: &SeedIds, rows: &[(i32, Option<i32>, bool)]) {
    for &(minutes, rounded, billable) in rows {
        sqlx::query!("INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, rounded_minutes, billable)
            VALUES ($1, $2, $3, $4, $5, '2026-09-07', $6, $7, $8)",
            Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id, ids.task_id, minutes, rounded, billable)
            .execute(pool).await.unwrap();
    }
}

async fn report(pool: &PgPool, org_id: Uuid, dimension: &str) -> Vec<ReportRow> {
    let day = "2026-09-07".parse().unwrap();
    fetch_report(pool, org_id, (day, day), dimension, None, None, None)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn same_names_keep_separate_groups_for_every_dimension(pool: PgPool) {
    let (first, second) = fixtures(&pool, "EUR").await;
    entries(
        &pool,
        &first,
        &[(60, None, true), (47, Some(45), true), (25, None, false)],
    )
    .await;
    entries(
        &pool,
        &second,
        &[(25, Some(30), true), (1, None, true), (0, None, true)],
    )
    .await;
    for dimension in ["project", "task", "client", "person", "unknown"] {
        let rows = report(&pool, first.org_id, dimension).await;
        assert_eq!(rows.len(), 2, "{dimension}: {rows:?}");
        assert_eq!(rows[0].label, rows[1].label);
        let mut expected_ids = match dimension {
            "task" => [first.task_id, second.task_id],
            "client" => [first.client_id, second.client_id],
            "person" => [first.user_id, second.user_id],
            _ => [first.project_id, second.project_id],
        };
        expected_ids.sort();
        assert_eq!([rows[0].group_id, rows[1].group_id], expected_ids);
        assert!(rows.iter().all(|row| row.currency == "EUR"));
        let mut totals: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    r.total_minutes,
                    r.rounded_minutes,
                    r.billable_minutes,
                    r.billable_cents,
                    r.cost_cents,
                )
            })
            .collect();
        totals.sort();
        assert_eq!(
            totals,
            [(26, 31, 31, 6200, 2614), (132, 130, 105, 15750, 13267)]
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn shared_person_and_task_are_partitioned_by_currency(pool: PgPool) {
    let (first, second) = fixtures(&pool, "USD").await;
    entries(&pool, &first, &[(60, None, true)]).await;
    let cross = SeedIds {
        user_id: first.user_id,
        task_id: first.task_id,
        ..second
    };
    entries(&pool, &cross, &[(60, None, true)]).await;
    for dimension in ["person", "task"] {
        let rows = report(&pool, first.org_id, dimension).await;
        assert_eq!(rows.len(), 2, "{dimension}: {rows:?}");
        assert_eq!(rows[0].group_id, rows[1].group_id);
        assert_eq!((&*rows[0].currency, &*rows[1].currency), ("EUR", "USD"));
        assert_eq!(rows[0].total_minutes, 60);
        assert_eq!(rows[1].total_minutes, 60);
        assert_eq!(
            (rows[0].billable_cents, rows[1].billable_cents),
            (9000, 12000)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn filters_and_renames_keep_report_identity_and_totals(pool: PgPool) {
    let (first, second) = fixtures(&pool, "EUR").await;
    entries(&pool, &first, &[(60, None, true)]).await;
    entries(&pool, &second, &[(30, None, true)]).await;
    let other = seed(&pool, OrgRole::Manager).await;
    entries(&pool, &other, &[(999, None, true)]).await;
    let day = "2026-09-07".parse().unwrap();
    for (client, project, user) in [
        (Some(second.client_id), None, None),
        (None, Some(second.project_id), None),
        (None, None, Some(second.user_id)),
        (
            Some(second.client_id),
            Some(second.project_id),
            Some(second.user_id),
        ),
    ] {
        let rows = fetch_report(
            &pool,
            first.org_id,
            (day, day),
            "project",
            client,
            project,
            user,
        )
        .await
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            (
                rows[0].group_id,
                rows[0].total_minutes,
                rows[0].billable_cents
            ),
            (second.project_id, 30, 6000)
        );
    }
    sqlx::query!(
        "UPDATE projects SET name = $2 WHERE id = $1",
        first.project_id,
        "alpha"
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET name = $2 WHERE id = $1",
        second.project_id,
        "Beta"
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = report(&pool, first.org_id, "project").await;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (&*rows[0].label, rows[0].group_id, rows[0].total_minutes),
        ("Beta", second.project_id, 30)
    );
    assert_eq!(
        (&*rows[1].label, rows[1].group_id, rows[1].total_minutes),
        ("alpha", first.project_id, 60)
    );
    let outside = "2026-09-08".parse().unwrap();
    assert!(
        fetch_report(
            &pool,
            first.org_id,
            (outside, outside),
            "project",
            None,
            None,
            None
        )
        .await
        .unwrap()
        .is_empty()
    );
}
