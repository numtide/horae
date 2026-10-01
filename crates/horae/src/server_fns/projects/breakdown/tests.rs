use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

async fn read(
    pool: &PgPool,
    ids: &SeedIds,
    interval: Option<ProjectActivityInterval>,
) -> Result<ProjectBreakdown, ServerFnError> {
    fetch_project_breakdown(pool, ids.org_id, ids.user_id, ids.project_id, interval).await
}

async fn entry(pool: &PgPool, ids: &SeedIds, day: &str, minutes: i32, billable: bool) {
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'Private breakdown note')",
        Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id, ids.task_id,
        day.parse::<chrono::NaiveDate>().unwrap() as chrono::NaiveDate, minutes, billable,
    ).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn breakdown_reconciles_periods_and_preserves_zero_time_and_inactive_history(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=1000 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-08-31", 20, true).await;
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    entry(&pool, &ids, "2026-09-30", 9, false).await;
    entry(&pool, &ids, "2026-10-01", 20, true).await;
    entry(&pool, &foreign, "2026-09-01", 999, true).await;
    sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    let zero_task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Zero-time task')",
        zero_task,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        zero_task
    )
    .execute(&pool)
    .await
    .unwrap();
    let zero_person = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,'zero@example.test','Zero-time person')", zero_person, ids.org_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        zero_person
    )
    .execute(&pool)
    .await
    .unwrap();
    let interval = Some(ProjectActivityInterval {
        from: "2026-09-01".parse().unwrap(),
        to: "2026-09-30".parse().unwrap(),
    });
    let result = read(&pool, &ids, interval).await.unwrap();
    assert_eq!(result.interval, interval);
    assert_eq!(result.totals.total.minutes, 16);
    assert_eq!(result.totals.total.billable_minutes, 7);
    assert_eq!(result.totals.total.cost_cents, Some(267));
    assert_eq!(result.cells.len(), 1);
    assert!(
        result
            .tasks
            .iter()
            .any(|row| row.id == ids.task_id && !row.active && !row.current)
    );
    assert!(
        result
            .tasks
            .iter()
            .any(|row| row.id == zero_task && row.current)
    );
    assert!(
        result
            .people
            .iter()
            .any(|row| row.id == zero_person && row.current)
    );
    assert_eq!(
        result.totals.by_task[&ids.task_id],
        result.totals.by_person[&ids.user_id]
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("Private breakdown note")
    );
    assert_eq!(
        read(&pool, &ids, None).await.unwrap().totals.total.minutes,
        56
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn breakdown_costs_preserve_missing_zero_private_and_current_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    assert_eq!(
        read(&pool, &ids, None)
            .await
            .unwrap()
            .totals
            .total
            .cost_cents,
        None
    );
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=0 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        read(&pool, &ids, None)
            .await
            .unwrap()
            .totals
            .total
            .cost_cents,
        Some(0)
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,12000)",Uuid::now_v7(),ids.org_id,ids.project_id,ids.user_id).execute(&pool).await.unwrap();
    assert_eq!(
        read(&pool, &ids, None)
            .await
            .unwrap()
            .totals
            .total
            .cost_cents,
        Some(1400)
    );
    sqlx::query!(
        "UPDATE users SET org_role='manager' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let hidden = serde_json::to_string(&read(&pool, &ids, None).await.unwrap()).unwrap();
    assert!(
        !hidden.contains("cost_cents") && !hidden.contains("cost_currency"),
        "{hidden}"
    );
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(read(&pool, &ids, None).await.unwrap().cost_currency, None);
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'person')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    assert!(matches!(
        read(&pool, &ids, None).await.unwrap_err(),
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        read(&pool, &ids, None).await.unwrap().totals.total.minutes,
        7
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(read(&pool, &ids, None).await.is_err());
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn breakdown_empty_foreign_and_reversed_intervals_are_explicit(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let empty = read(&pool, &ids, None).await.unwrap();
    assert!(empty.tasks.is_empty() && empty.people.is_empty() && empty.cells.is_empty());
    assert_eq!(empty.totals.total.cost_cents, Some(0));
    let reversed = Some(ProjectActivityInterval {
        from: "2026-10-01".parse().unwrap(),
        to: "2026-09-01".parse().unwrap(),
    });
    assert!(matches!(
        read(&pool, &ids, reversed).await.unwrap_err(),
        ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        }
    ));
    assert!(matches!(
        fetch_project_breakdown(&pool, ids.org_id, foreign.user_id, ids.project_id, reversed)
            .await
            .unwrap_err(),
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn breakdown_multiple_people_and_tasks_reconcile_with_workspace_currency(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE organizations SET default_currency='USD' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=1000 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let user_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id,org_id,email,name,cost_rate_cents,active) VALUES ($1,$2,'historical@example.test','Historical person',2000,false)", user_id, ids.org_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Second task')",
        task_id,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    let second = SeedIds {
        user_id,
        task_id: ids.task_id,
        org_id: ids.org_id,
        project_id: ids.project_id,
        client_id: ids.client_id,
    };
    entry(&pool, &second, "2026-09-01", 9, false).await;
    let third = SeedIds {
        user_id: ids.user_id,
        task_id,
        org_id: ids.org_id,
        project_id: ids.project_id,
        client_id: ids.client_id,
    };
    entry(&pool, &third, "2026-09-01", 11, true).await;
    let result = read(&pool, &ids, None).await.unwrap();
    assert_eq!(result.cost_currency.as_deref(), Some("USD"));
    assert_eq!(result.totals.total.minutes, 27);
    assert_eq!(result.totals.total.billable_minutes, 18);
    assert_eq!(result.totals.total.cost_cents, Some(600));
    assert_eq!(result.cells.len(), 3);
    for rows in [&result.totals.by_task, &result.totals.by_person] {
        assert_eq!(rows.values().map(|row| row.minutes).sum::<i64>(), 27);
        assert_eq!(
            rows.values()
                .map(|row| row.cost_cents.unwrap())
                .sum::<i64>(),
            600
        );
    }
    assert!(
        result
            .people
            .iter()
            .any(|row| row.id == user_id && !row.active && !row.current)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn breakdown_excessive_groups_fail_instead_of_returning_a_partial_report(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let task_ids: Vec<_> = (0..5001).map(|_| Uuid::now_v7()).collect();
    let entry_ids: Vec<_> = (0..5001).map(|_| Uuid::now_v7()).collect();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) SELECT id,$1,'Bounded task' FROM unnest($2::uuid[]) id",
        ids.org_id,
        &task_ids
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable) SELECT e.id,$1,$2,$3,e.task_id,'2026-09-01',1,true FROM unnest($4::uuid[],$5::uuid[]) AS e(id,task_id)", ids.org_id, ids.user_id, ids.project_id, &entry_ids, &task_ids).execute(&pool).await.unwrap();
    let error = read(&pool, &ids, None).await.unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: BAD_REQUEST,
            ..
        }
    ));
    assert!(error.to_string().contains("5000 task/person groups"));
}
