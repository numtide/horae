use super::detail::legacy_viewer;
use super::*;
use crate::models::project::ProjectDetailBudget;
use chrono::NaiveDate;
use horae_core::budget::BudgetHeadline;
use horae_core::types::BudgetKind;

fn exact(budget: i64, spent: i64, remaining: i64) -> BudgetHeadline {
    BudgetHeadline::Exact {
        budget,
        spent,
        remaining,
    }
}

fn date(value: &str) -> NaiveDate {
    value.parse().unwrap()
}

async fn budget_on(
    pool: &PgPool,
    viewer: &User,
    ids: &SeedIds,
    on: NaiveDate,
) -> Option<ProjectDetailBudget> {
    detail_view::fetch(pool, viewer, ids.project_id, None, on)
        .await
        .unwrap()
        .budget
}

async fn headline_of(pool: &PgPool, viewer: &User, ids: &SeedIds) -> BudgetHeadline {
    budget_on(pool, viewer, ids, today())
        .await
        .unwrap()
        .headline
}

async fn configure(pool: &PgPool, ids: &SeedIds, scope: &str, monthly: bool, nonbillable: bool) {
    sqlx::query!(
        "INSERT INTO project_settings
           (id,org_id,project_id,creator_id,rate_mode,budget_scope,monthly_reset,include_nonbillable)
         VALUES ($1,$2,$3,$4,'project',$5,$6,$7)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
        scope,
        monthly,
        nonbillable,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn hours_budget(pool: &PgPool, ids: &SeedIds, minutes: Option<i64>) {
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours', budget_minutes=$2 WHERE id=$1",
        ids.project_id,
        minutes,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn fee_budget(pool: &PgPool, ids: &SeedIds, cents: Option<i64>) {
    sqlx::query!(
        "UPDATE projects SET budget_kind='amount', budget_amount_cents=$2, rate_cents=6000 WHERE id=$1",
        ids.project_id,
        cents,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn entry(pool: &PgPool, ids: &SeedIds, task: Uuid, on: &str, minutes: i32, billable: bool) {
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state)
         VALUES ($1, $2, $3, $4, $5, $6::text::date, $7, $8, 'open')",
        Uuid::now_v7(),
        ids.org_id,
        ids.user_id,
        ids.project_id,
        task,
        on,
        minutes,
        billable,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn task(pool: &PgPool, ids: &SeedIds, name: &str, budget: Option<i64>) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, $3)",
        id,
        ids.org_id,
        name
    )
    .execute(pool)
    .await
    .unwrap();
    link_task(pool, ids, id, budget).await;
    id
}

async fn link_task(pool: &PgPool, ids: &SeedIds, task: Uuid, budget: Option<i64>) {
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        task,
    )
    .execute(pool)
    .await
    .unwrap();
    if budget.is_some() {
        sqlx::query!(
            "INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes)
             VALUES ($1,$2,$3,$4,$5)",
            Uuid::now_v7(),
            ids.org_id,
            ids.project_id,
            task,
            budget,
        )
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn reader(pool: &PgPool, permissions: &[Permission]) -> (SeedIds, User) {
    fixture(pool, OrgRole::Member, PermissionSelection::new(permissions)).await
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_budget_uses_lifetime_raw_minutes_and_list_fee_spend(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let viewer = legacy_viewer(&ids, OrgRole::Admin);
    assert_eq!(budget_on(&pool, &viewer, &ids, today()).await, None);
    entry(&pool, &ids, ids.task_id, "2026-01-01", 90, true).await;
    entry(&pool, &ids, ids.task_id, "2026-09-07", 60, false).await;
    hours_budget(&pool, &ids, Some(120)).await;
    let budget = budget_on(&pool, &viewer, &ids, today()).await.unwrap();
    assert_eq!(
        (
            budget.kind,
            budget.currency.as_str(),
            budget.period_key.as_str()
        ),
        (BudgetKind::Hours, "EUR", "lifetime")
    );
    assert_eq!(budget.headline, exact(120, 150, -30));
    fee_budget(&pool, &ids, Some(10_000)).await;
    let spend = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap()
        .remove(0)
        .spent_cents
        .unwrap();
    assert_eq!(spend, 9_000);
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(10_000, spend, 1_000)
    );
    fee_budget(&pool, &ids, None).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unallocated
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn configured_budget_applies_rounding_and_saved_nonbillable_inclusion(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "UPDATE organizations SET round_minutes=15, round_dir='up' WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    hours_budget(&pool, &ids, Some(120)).await;
    configure(&pool, &ids, "project", false, false).await;
    entry(&pool, &ids, ids.task_id, "2026-01-01", 50, true).await;
    entry(&pool, &ids, ids.task_id, "2026-09-07", 20, false).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(120, 60, 60));
    let list =
        crate::server_fns::budgets::progress_for_viewer(&pool, ids.org_id, ids.user_id, today())
            .await
            .unwrap();
    assert_eq!((list[0].budget, list[0].consumed), (Some(120), Some(60)));
    sqlx::query!(
        "UPDATE project_settings SET include_nonbillable=true WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(120, 90, 30));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn monthly_budget_counts_only_the_current_calendar_month(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, Some(600)).await;
    configure(&pool, &ids, "project", true, false).await;
    for (on, minutes) in [
        ("2028-01-31", 1),
        ("2028-02-01", 10),
        ("2028-02-29", 100),
        ("2028-03-01", 1000),
    ] {
        entry(&pool, &ids, ids.task_id, on, minutes, true).await;
    }
    let february = budget_on(&pool, &viewer, &ids, date("2028-02-29"))
        .await
        .unwrap();
    assert_eq!(
        (february.period_key.as_str(), february.headline),
        ("2028-02", exact(600, 110, 490))
    );
    let march = budget_on(&pool, &viewer, &ids, date("2028-03-01"))
        .await
        .unwrap();
    assert_eq!(
        (march.period_key.as_str(), march.headline),
        ("2028-03", exact(600, 1000, -400))
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_budgets_exclude_blank_count_zero_and_report_unallocated(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "task", false, false).await;
    link_task(&pool, &ids, ids.task_id, Some(60)).await;
    let blank = task(&pool, &ids, "Unbudgeted", None).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 90, true).await;
    entry(&pool, &ids, blank, "2026-09-01", 1000, true).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(60, 90, -30));
    sqlx::query!(
        "UPDATE project_task_settings SET budget_minutes=0 WHERE task_id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(0, 90, -90));
    sqlx::query!(
        "UPDATE project_task_settings SET budget_minutes=NULL WHERE task_id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unallocated
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_allowance_overflow_is_unavailable_not_unallocated(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "task", false, false).await;
    link_task(&pool, &ids, ids.task_id, Some(i64::MAX)).await;
    task(&pool, &ids, "Second", Some(1)).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 30, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn person_budget_uses_saved_member_allowances(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "person", false, false).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unallocated
    );
    sqlx::query!(
        "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes)
         VALUES ($1,$2,$3,$4,60)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, ids.task_id, "2026-09-01", 30, true).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(60, 30, 30));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn configured_budget_unit_none_ignores_leftover_legacy_columns(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind='none', budget_minutes=120 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    configure(&pool, &ids, "project", false, false).await;
    assert_eq!(budget_on(&pool, &viewer, &ids, today()).await, None);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn fee_figures_require_current_financial_authority(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, Some(10_000)).await;
    configure(&pool, &ids, "project", false, false).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(10_000, 6_000, 4_000)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let withheld = budget_on(&pool, &viewer, &ids, today()).await;
    assert_eq!(
        serde_json::to_value(withheld).unwrap(),
        serde_json::json!({"kind":"amount","currency":"EUR","period_key":"lifetime","headline":"unavailable"})
    );
    replace_grants(
        &pool,
        &ids,
        &[
            Permission::ProjectReadAll,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
    designate(&pool, &ids).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(10_000, 6_000, 4_000)
    );
    // Hour budgets need no financial authority.
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, Some(120)).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(120, 60, 60));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unallocated_fee_budget_is_still_withheld_without_financial_authority(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "task", false, false).await;
    link_task(&pool, &ids, ids.task_id, None).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unallocated
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_fee_figures_follow_spend_redaction_for_canonical_readers(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, Some(10_000)).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    let spent = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap()
        .remove(0)
        .spent_cents
        .unwrap();
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(10_000, spent, 10_000 - spent)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn archived_project_keeps_its_configured_budget(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, Some(120)).await;
    configure(&pool, &ids, "project", false, false).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 150, true).await;
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(120, 150, -30)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn foreign_client_parent_hides_configured_and_legacy_budgets(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, Some(10_000)).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    let other = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        other.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for configured in [false, true] {
        if configured {
            configure(&pool, &ids, "project", false, false).await;
        }
        assert!(
            matches!(
                detail_view::fetch(&pool, &viewer, ids.project_id, None, today()).await,
                Err(ServerFnError::ServerError {
                    code: NOT_FOUND,
                    ..
                })
            ),
            "configured={configured}"
        );
    }
}

async fn maximum_project_rate(pool: &PgPool, ids: &SeedIds) {
    sqlx::query!(
        "UPDATE projects SET rate_cents=$2 WHERE id=$1",
        ids.project_id,
        i64::MAX
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn configured_fee_valuation_overflow_is_unavailable_not_a_failed_detail(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, Some(10_000)).await;
    maximum_project_rate(&pool, &ids).await;
    configure(&pool, &ids, "project", false, false).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 120, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_hours_budget_ignores_unrelated_money_overflow(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let viewer = legacy_viewer(&ids, OrgRole::Admin);
    hours_budget(&pool, &ids, Some(120)).await;
    maximum_project_rate(&pool, &ids).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 120, true).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(120, 120, 0));
    // The Projects list still values money and keeps its existing error.
    assert!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .is_err()
    );
}

/// Sixty-one hour-long entries at `i64::MAX / 60` each fit, but their sum does not.
async fn summed_overflow(pool: &PgPool, ids: &SeedIds) {
    sqlx::query!(
        "UPDATE projects SET rate_cents=$2 WHERE id=$1",
        ids.project_id,
        i64::MAX / 60
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state)
         SELECT gen_random_uuid(),$1,$2,$3,$4,'2026-09-01',60,true,'open' FROM generate_series(1,61)",
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn configured_fee_sum_overflow_is_unavailable(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, Some(10_000)).await;
    configure(&pool, &ids, "project", false, false).await;
    summed_overflow(&pool, &ids).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn legacy_fee_line_and_sum_overflow_are_unavailable(pool: PgPool) {
    for summed in [false, true] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let viewer = legacy_viewer(&ids, OrgRole::Admin);
        fee_budget(&pool, &ids, Some(10_000)).await;
        if summed {
            summed_overflow(&pool, &ids).await;
        } else {
            maximum_project_rate(&pool, &ids).await;
            entry(&pool, &ids, ids.task_id, "2026-09-01", 120, true).await;
        }
        assert_eq!(
            headline_of(&pool, &viewer, &ids).await,
            BudgetHeadline::Unavailable,
            "summed={summed}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unallocated_configured_budgets_keep_metadata_and_value_nothing(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    fee_budget(&pool, &ids, None).await;
    maximum_project_rate(&pool, &ids).await;
    configure(&pool, &ids, "task", true, false).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 120, true).await;
    for linked in [false, true] {
        if linked {
            link_task(&pool, &ids, ids.task_id, None).await;
        }
        let budget = budget_on(&pool, &viewer, &ids, today()).await.unwrap();
        assert_eq!(
            (budget.kind, budget.period_key.as_str(), budget.headline),
            (BudgetKind::Amount, "2026-09", BudgetHeadline::Unallocated),
            "linked={linked}"
        );
    }
}

async fn task_rate(pool: &PgPool, ids: &SeedIds, task: Uuid, rate: i64) {
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents=$3 WHERE project_id=$1 AND task_id=$2",
        ids.project_id,
        task,
        rate
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn task_rate_fee_budget(pool: &PgPool, ids: &SeedIds) {
    fee_budget(pool, ids, None).await;
    configure(pool, ids, "task", false, false).await;
    sqlx::query!(
        "UPDATE project_settings SET rate_mode='task' WHERE project_id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn blank_scope_overflow_cannot_hide_an_exact_allocated_total(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    task_rate_fee_budget(&pool, &ids).await;
    link_task(&pool, &ids, ids.task_id, None).await;
    sqlx::query!(
        "INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_cents)
         VALUES ($1,$2,$3,$4,10000)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    task_rate(&pool, &ids, ids.task_id, 6_000).await;
    let blank = task(&pool, &ids, "Unbudgeted", None).await;
    task_rate(&pool, &ids, blank, i64::MAX).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    entry(&pool, &ids, blank, "2026-09-01", 120, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        exact(10_000, 6_000, 4_000)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn explicit_zero_allowance_overflow_is_unavailable(pool: PgPool) {
    let (ids, viewer) = reader(
        &pool,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    task_rate_fee_budget(&pool, &ids).await;
    link_task(&pool, &ids, ids.task_id, None).await;
    sqlx::query!(
        "INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_cents)
         VALUES ($1,$2,$3,$4,0)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.task_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    task_rate(&pool, &ids, ids.task_id, i64::MAX).await;
    entry(&pool, &ids, ids.task_id, "2026-09-01", 120, true).await;
    assert_eq!(
        headline_of(&pool, &viewer, &ids).await,
        BudgetHeadline::Unavailable
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn other_consumption_failures_still_fail_the_detail(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, Some(120)).await;
    configure(&pool, &ids, "project", false, false).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    sqlx::query!("SELECT set_config('statement_timeout', '300ms', false) AS timeout")
        .fetch_one(&reader_pool)
        .await
        .unwrap();
    // Only the consumption query reads invoice lines, so only it times out.
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE invoice_line_items IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let result = detail_view::fetch(&reader_pool, &viewer, ids.project_id, None, today()).await;
    hold.rollback().await.unwrap();
    assert!(
        matches!(
            &result,
            Err(ServerFnError::ServerError {
                code: INTERNAL_ERROR,
                ..
            })
        ),
        "{result:?}"
    );
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn shared_member_person_budget_is_aggregate_only(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[]).await;
    hours_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "person", false, false).await;
    sqlx::query!(
        "UPDATE project_settings SET report_visibility='project_members' WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Private teammate allocation')",
        other,
        ids.org_id,
        format!("{other}@test.com"),
    )
    .execute(&pool)
    .await
    .unwrap();
    for (person, allowance, minutes) in [(ids.user_id, 60_i64, 30), (other, 180_i64, 100)] {
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
            Uuid::now_v7(),
            ids.project_id,
            person,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes)
             VALUES ($1,$2,$3,$4,$5)",
            Uuid::now_v7(),
            ids.org_id,
            ids.project_id,
            person,
            allowance,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,state)
             VALUES ($1,$2,$3,$4,$5,'2026-09-01',$6,true,'open')",
            Uuid::now_v7(),
            ids.org_id,
            person,
            ids.project_id,
            ids.task_id,
            minutes,
        )
        .execute(&pool)
        .await
        .unwrap();
    }
    let view = detail_view::fetch(&pool, &viewer, ids.project_id, None, today())
        .await
        .unwrap();
    assert_eq!(view.budget.as_ref().unwrap().headline, exact(240, 130, 110));
    let payload = serde_json::to_string(&view.budget).unwrap();
    for private in [
        "Private teammate allocation",
        "180",
        "100",
        &other.to_string(),
    ] {
        assert!(!payload.contains(private), "{private} disclosed: {payload}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn detail_budget_sends_no_scope_details(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, None).await;
    configure(&pool, &ids, "task", false, false).await;
    sqlx::query!(
        "UPDATE tasks SET name='Private task label' WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    link_task(&pool, &ids, ids.task_id, Some(77)).await;
    let budget = serde_json::to_string(&budget_on(&pool, &viewer, &ids, today()).await).unwrap();
    assert_eq!(
        budget,
        r#"{"kind":"hours","currency":"EUR","period_key":"lifetime","headline":{"exact":{"budget":77,"spent":0,"remaining":77}}}"#
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn reading_the_budget_writes_nothing(pool: PgPool) {
    let (ids, viewer) = reader(&pool, &[Permission::ProjectReadAll]).await;
    hours_budget(&pool, &ids, Some(10)).await;
    configure(&pool, &ids, "project", false, false).await;
    sqlx::query!(
        "UPDATE project_settings SET alert_enabled=true, alert_threshold=0 WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    entry(&pool, &ids, ids.task_id, "2026-09-01", 60, true).await;
    let snapshot = |pool: PgPool| async move {
        sqlx::query!(
            r#"SELECT (SELECT xmin::text FROM projects WHERE id=$1) AS "project!",
                (SELECT xmin::text FROM project_settings WHERE project_id=$1) AS "settings!",
                (SELECT string_agg(xmin::text, ',' ORDER BY id) FROM time_entries WHERE project_id=$1) AS "entries!",
                (SELECT count(*) FROM project_budget_notifications) AS "notifications!",
                (SELECT count(*) FROM horae_outbox) AS "outbox!""#,
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .map(|row| (row.project, row.settings, row.entries, row.notifications, row.outbox))
        .unwrap()
    };
    let before = snapshot(pool.clone()).await;
    assert_eq!(headline_of(&pool, &viewer, &ids).await, exact(10, 60, -50));
    assert_eq!(snapshot(pool.clone()).await, before);
}
