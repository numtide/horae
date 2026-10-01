use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

async fn summary(pool: &PgPool, ids: &SeedIds) -> ProjectSummary {
    fetch_project_summary(
        pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        "2026-09-29".parse().unwrap(),
    )
    .await
    .unwrap()
}

async fn entry(pool: &PgPool, ids: &SeedIds, date: &str, minutes: i32, billable: bool) {
    sqlx::query!(
        "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,rounded_minutes,notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,15,'Private summary test note')",
        Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id, ids.task_id,
        date.parse::<chrono::NaiveDate>().unwrap() as chrono::NaiveDate, minutes, billable,
    ).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_empty_and_legacy_amount_budget_match_the_overview_without_writes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let empty = summary(&pool, &ids).await;
    assert_eq!(
        (
            empty.total_minutes,
            empty.billable_minutes,
            empty.non_billable_minutes
        ),
        (0, 0, 0)
    );
    assert_eq!(empty.budgets[0].kind, BudgetKind::None);
    assert_eq!(empty.internal_costs.unwrap().total_cents, Some(0));
    sqlx::query!("UPDATE projects SET budget_kind = 'amount', budget_amount_cents = 10000, rate_cents = 12000, active = false WHERE id = $1", ids.project_id).execute(&pool).await.unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    entry(&pool, &ids, "2026-10-01", 9, false).await;
    entry(&pool, &foreign, "2026-09-01", 900, true).await;
    let before = sqlx::query_scalar!(
        "SELECT xmin::text FROM projects WHERE id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let result = summary(&pool, &ids).await;
    let spend = super::super::fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(
        (
            result.total_minutes,
            result.billable_minutes,
            result.non_billable_minutes
        ),
        (16, 7, 9)
    );
    assert_eq!(result.budget_totals.consumed, spend[0].spent_cents);
    assert_eq!(
        (result.budget_totals.budget, result.budget_totals.remaining),
        (Some(10000), Some(7000))
    );
    assert!(!result.configured_budget);
    assert_eq!(result.budgets[0].period_key, "lifetime");
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT xmin::text FROM projects WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        before
    );
    assert!(
        !serde_json::to_string(&result)
            .unwrap()
            .contains("Private summary test note")
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_separates_lifetime_actual_hours_from_configured_monthly_billing_minutes(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours', budget_minutes=60 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,monthly_reset,include_nonbillable) VALUES ($1,$2,$3,$4,'project',true,false)",Uuid::now_v7(),ids.org_id,ids.project_id,ids.user_id).execute(&pool).await.unwrap();
    entry(&pool, &ids, "2026-08-31", 100, true).await;
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    entry(&pool, &ids, "2026-09-30", 9, false).await;
    entry(&pool, &ids, "2026-10-01", 100, true).await;
    let result = summary(&pool, &ids).await;
    assert_eq!(
        (
            result.total_minutes,
            result.billable_minutes,
            result.non_billable_minutes
        ),
        (216, 207, 9)
    );
    assert!(result.configured_budget);
    assert_eq!(result.budgets[0].period_key, "2026-09");
    assert_eq!(
        (
            result.budget_totals.consumed,
            result.budget_totals.remaining
        ),
        (15, Some(45))
    );
    let mut connection = pool.acquire().await.unwrap();
    let overview = crate::server_fns::budgets::progress_for_viewer(
        &mut connection,
        ids.org_id,
        ids.user_id,
        "2026-09-29".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(result.budgets, overview);
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_costs_distinguish_missing_zero_and_private_rates_in_workspace_currency(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE organizations SET default_currency='USD' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    let missing = summary(&pool, &ids).await.internal_costs.unwrap();
    assert_eq!(
        (
            missing.currency.as_str(),
            missing.total_cents,
            missing.missing_rate_minutes
        ),
        ("USD", None, 7)
    );
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=0 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        summary(&pool, &ids)
            .await
            .internal_costs
            .unwrap()
            .total_cents,
        Some(0)
    );
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=6000 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        summary(&pool, &ids)
            .await
            .internal_costs
            .unwrap()
            .total_cents,
        Some(700)
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,12000)",Uuid::now_v7(),ids.org_id,ids.project_id,ids.user_id).execute(&pool).await.unwrap();
    assert_eq!(
        summary(&pool, &ids)
            .await
            .internal_costs
            .unwrap()
            .total_cents,
        Some(1400)
    );
    for role in [OrgRole::Manager, OrgRole::Member] {
        sqlx::query!(
            "UPDATE users SET org_role=$2 WHERE id=$1",
            ids.user_id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        let result = summary(&pool, &ids).await;
        assert_eq!(result.total_minutes, 7);
        assert!(
            !serde_json::to_value(result)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("internal_costs")
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_rechecks_progress_permission_and_rejects_foreign_or_inactive_viewers(
    pool: PgPool,
) {
    let ids = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    for (org, viewer, project) in [
        (ids.org_id, ids.user_id, ids.project_id),
        (ids.org_id, foreign.user_id, ids.project_id),
        (ids.org_id, ids.user_id, foreign.project_id),
    ] {
        let error =
            fetch_project_summary(&pool, org, viewer, project, "2026-09-29".parse().unwrap())
                .await
                .unwrap_err();
        assert!(matches!(
            error,
            ServerFnError::ServerError {
                code: NOT_FOUND,
                ..
            }
        ));
    }
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(summary(&pool, &ids).await.total_minutes, 7);
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let error = fetch_project_summary(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        "2026-09-29".parse().unwrap(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_rechecks_configured_visibility_after_access_is_revoked(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'person')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    assert_eq!(summary(&pool, &ids).await.total_minutes, 7);
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'managers' WHERE project_id = $1",
        ids.project_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = fetch_project_summary(
        &pool,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        "2026-09-29".parse().unwrap(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_task_budgets_preserve_unallocated_scopes_and_individual_overruns(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours' WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,budget_scope) VALUES ($1,$2,$3,$4,'task','task')",Uuid::now_v7(),ids.org_id,ids.project_id,ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let extra = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Zero-time task')",
        extra,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        extra
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes) VALUES ($1,$2,$3,$4,10)",Uuid::now_v7(),ids.org_id,ids.project_id,ids.task_id).execute(&pool).await.unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    let partial = summary(&pool, &ids).await;
    assert_eq!(partial.budgets.len(), 2);
    assert_eq!(
        (
            partial.budget_totals.budget,
            partial.budget_totals.remaining,
            partial.budget_totals.unallocated_scopes,
            partial.budget_totals.overrun_scopes
        ),
        (None, None, 1, 1)
    );
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes) VALUES ($1,$2,$3,$4,50)",Uuid::now_v7(),ids.org_id,ids.project_id,extra).execute(&pool).await.unwrap();
    let allocated = summary(&pool, &ids).await;
    assert_eq!(
        (
            allocated.budget_totals.remaining,
            allocated.budget_totals.overrun_scopes
        ),
        (Some(45), 1)
    );
    assert!(
        allocated
            .budgets
            .iter()
            .any(|row| row.task_id == Some(extra) && row.consumed == 0)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn summary_mixed_missing_cost_rates_never_return_a_partial_total(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE users SET cost_rate_cents=6000 WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, "2026-09-01", 7, true).await;
    assert_eq!(
        summary(&pool, &ids)
            .await
            .internal_costs
            .unwrap()
            .total_cents,
        Some(700)
    );
    let other = Uuid::now_v7();
    sqlx::query!("INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,'missing@example.test','Missing cost')",other,ids.org_id).execute(&pool).await.unwrap();
    let second = SeedIds {
        user_id: other,
        org_id: ids.org_id,
        client_id: ids.client_id,
        project_id: ids.project_id,
        task_id: ids.task_id,
    };
    entry(&pool, &second, "2026-09-01", 3, false).await;
    let cost = summary(&pool, &ids).await.internal_costs.unwrap();
    assert_eq!((cost.total_cents, cost.missing_rate_minutes), (None, 3));
}
