//! Configured budget progress, alert mail, the sweep and plugin bands.

use crate::ops::testing::{Recorder, set_grants};
use crate::plugin::AppEvent;
use crate::server_fns::test_seed::seed;
use horae_core::permissions::catalog::{BuiltInProfile, Permission};
use sqlx::PgPool;

use crate::notifications::budgets::{
    BudgetProgress, announce_plugin_bands, claim_budget_band, configured_progress, enqueue_alerts,
    sweep,
};
use crate::server_fns::test_seed::{SeedIds, time_entry};
use horae_core::types::EntryState;
use uuid::Uuid;

/// Lifetime spent minutes per project, as the Projects list shows them.
async fn spent_minutes(pool: &PgPool, ids: &SeedIds) -> i32 {
    let mut read =
        crate::ops::begin_read(pool, crate::ops::Caller::session(ids.org_id, ids.user_id))
            .await
            .unwrap();
    let result = crate::ops::projects::spend(&mut read.tx, &read.actor, None).await;
    read.finish(result).await.unwrap()[0].spent_minutes as i32
}

async fn configured(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, BuiltInProfile::ExecutiveManager).await;
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 100, rate_cents = 6000 WHERE id = $1",
        ids.project_id
    ).execute(pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'project')",
        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id
    ).execute(pool).await.unwrap();
    ids
}

async fn progress(pool: &PgPool, ids: &SeedIds, date: &str) -> Vec<BudgetProgress> {
    configured_progress(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        ids.project_id,
        date.parse().unwrap(),
    )
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_monthly_budget_excludes_other_months_and_nonbillable_time(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let previous = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2026-08-31' WHERE id = $1",
        previous
    )
    .execute(&pool)
    .await
    .unwrap();
    let nonbillable = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET billable = false WHERE id = $1",
        nonbillable
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET monthly_reset = true WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = progress(&pool, &ids, "2026-09-30").await;
    assert_eq!(
        (
            rows[0].consumed,
            rows[0].budget,
            rows[0].period_key.as_str()
        ),
        (60, 100, "2026-09")
    );
    assert_eq!(progress(&pool, &ids, "2026-10-01").await[0].consumed, 0);
    sqlx::query!(
        "UPDATE project_settings SET include_nonbillable = true WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 120);
    sqlx::query!(
        "UPDATE project_settings SET monthly_reset = false WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let lifetime = progress(&pool, &ids, "2026-09-01").await;
    assert_eq!(
        (lifetime[0].consumed, lifetime[0].period_key.as_str()),
        (180, "lifetime")
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_nonbillable_projects_count_time_without_hidden_checkbox(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!(
        "UPDATE projects SET project_type = 'non_billable' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET billable = false WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 60);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_task_and_person_budgets_do_not_share_consumption(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes) VALUES ($1,$2,$3,$4,90)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.task_id).execute(&pool).await.unwrap();
    let other_task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Other')",
        other_task,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        other_task
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes) VALUES ($1,$2,$3,$4,120)", Uuid::now_v7(), ids.org_id, ids.project_id, other_task).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE project_settings SET budget_scope = 'task' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = progress(&pool, &ids, "2026-09-01").await;
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|r| r.task_id == Some(ids.task_id) && r.consumed == 60 && r.budget == 90)
    );
    assert!(
        rows.iter()
            .any(|r| r.task_id == Some(other_task) && r.consumed == 0 && r.budget == 120)
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
    sqlx::query!("INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes) VALUES ($1,$2,$3,$4,75)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE project_settings SET budget_scope = 'person' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = progress(&pool, &ids, "2026-09-01").await;
    assert_eq!(
        (
            rows.len(),
            rows[0].user_id,
            rows[0].consumed,
            rows[0].budget
        ),
        (1, Some(ids.user_id), 60, 75)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_amount_budget_uses_resolved_rates_and_explicit_zero(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE projects SET budget_kind = 'amount', budget_minutes = NULL, budget_amount_cents = 10000 WHERE id = $1", ids.project_id).execute(&pool).await.unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 6000);
    sqlx::query!(
        "UPDATE projects SET rate_cents = 0 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_budget_does_not_read_foreign_or_legacy_projects(pool: PgPool) {
    let ids = configured(&pool).await;
    let other = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    assert!(
        configured_progress(
            &mut pool.acquire().await.unwrap(),
            other.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .unwrap()
        .is_empty()
    );
    assert!(progress(&pool, &other, "2026-09-01").await.is_empty());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_alerts_deduplicate_concurrent_checks_and_recrossing(pool: PgPool) {
    let ids = configured(&pool).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE time_entries SET minutes = 80 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, monthly_reset = true WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let date = "2026-09-30".parse().unwrap();
    let (first, second) = tokio::join!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date),
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
    );
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    sqlx::query!("UPDATE time_entries SET minutes = 0 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE time_entries SET minutes = 90 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    let notification = sqlx::query_scalar!(
        "SELECT id FROM project_budget_notifications WHERE project_id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let payloads = sqlx::query_scalar!(
        "SELECT payload FROM horae_outbox WHERE org_id = $1 AND event_kind = 'budget_email'",
        ids.org_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        payloads,
        vec![serde_json::json!({"notification_id": notification})]
    );
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2026-10-01' WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        enqueue_alerts(
            &pool,
            ids.org_id,
            ids.project_id,
            "2026-10-01".parse().unwrap()
        )
        .await
        .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_alerts_require_enabled_active_project_and_authorized_recipient(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let date = "2026-09-01".parse().unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    // Managed read alone is not enough: the person must also manage this project.
    set_grants(
        &mut pool.acquire().await.unwrap(),
        &ids,
        &[Permission::ProjectReadManaged],
    )
    .await
    .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ($1,$2,$3,$4)",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!(
        "UPDATE projects SET active = true WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_monthly_budget_includes_leap_day_and_preserves_locked_rounding(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!(
        "UPDATE project_settings SET monthly_reset = true WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15, round_dir = 'up' WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let leap_day = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2024-02-29', minutes = 1 WHERE id = $1",
        leap_day
    )
    .execute(&pool)
    .await
    .unwrap();
    let locked = time_entry(&pool, &ids, EntryState::Approved).await;
    sqlx::query!("UPDATE time_entries SET spent_date = '2024-02-01', minutes = 1, rounded_minutes = 10 WHERE id = $1", locked).execute(&pool).await.unwrap();
    let march = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2024-03-01' WHERE id = $1",
        march
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(progress(&pool, &ids, "2024-02-15").await[0].consumed, 25);
    assert_eq!(progress(&pool, &ids, "2024-03-01").await[0].consumed, 60);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_task_fee_budget_respects_billability_and_alert_scope(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE projects SET budget_kind = 'amount', budget_minutes = NULL, budget_amount_cents = 10000 WHERE id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,false,12000)", ids.project_id, ids.task_id).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_cents) VALUES ($1,$2,$3,$4,10000)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.task_id).execute(&pool).await.unwrap();
    sqlx::query!("UPDATE project_settings SET rate_mode = 'task', budget_scope = 'task', alert_enabled = true WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 0);
    sqlx::query!(
        "UPDATE project_settings SET include_nonbillable = true WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = progress(&pool, &ids, "2026-09-01").await;
    assert_eq!(
        (rows[0].task_id, rows[0].consumed, rows[0].budget),
        (Some(ids.task_id), 12000, 10000)
    );
    assert_eq!(
        enqueue_alerts(
            &pool,
            ids.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .unwrap(),
        1
    );
    let scope = sqlx::query!(
        "SELECT task_id,user_id FROM project_budget_notifications WHERE project_id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((scope.task_id, scope.user_id), (Some(ids.task_id), None));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_person_budget_excludes_other_people_and_has_its_own_alert(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let other_user = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other')",
        other_user,
        ids.org_id,
        format!("{other_user}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    let other_entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET user_id = $2, minutes = 200 WHERE id = $1",
        other_entry,
        other_user
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes) VALUES ($1,$2,$3,$4,75)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!("UPDATE project_settings SET budget_scope = 'person', alert_enabled = true WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    assert_eq!(progress(&pool, &ids, "2026-09-01").await[0].consumed, 60);
    assert_eq!(
        enqueue_alerts(
            &pool,
            ids.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .unwrap(),
        1
    );
    let scope = sqlx::query!(
        "SELECT task_id,user_id FROM project_budget_notifications WHERE project_id = $1",
        ids.project_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((scope.task_id, scope.user_id), (None, Some(ids.user_id)));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_alert_and_outbox_roll_back_together(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!("ALTER TABLE horae_outbox ADD CONSTRAINT reject_test_budget_email CHECK (event_kind <> 'budget_email')").execute(&pool).await.unwrap();
    assert!(
        enqueue_alerts(
            &pool,
            ids.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_budget_notifications")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_budget_sweep_recovers_missed_checks_without_duplicates(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let now = "2026-09-01T12:00:00Z".parse().unwrap();
    sweep(&pool, now).await.unwrap();
    sweep(&pool, now).await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_budget_notifications")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn configured_budget_sweep_dates_the_period_in_the_workspace_zone(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0, monthly_reset = true WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET timezone = 'Europe/Madrid' WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    // 22:30 UTC on 31 August is already 00:30 on 1 September in Madrid.
    sweep(&pool, "2026-08-31T22:30:00Z".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT period_key FROM project_budget_notifications")
            .fetch_all(&pool)
            .await
            .unwrap(),
        ["2026-09"]
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn budget_notification_schema_enforces_tenants_and_no_public_access(pool: PgPool) {
    let ids = configured(&pool).await;
    let foreign = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    let error = sqlx::query!(
        "INSERT INTO project_budget_notifications (id,org_id,project_id,recipient_id,period_key,threshold)
         VALUES ($1,$2,$3,$4,'lifetime',80)",
        Uuid::now_v7(), ids.org_id, ids.project_id, foreign.user_id,
    ).execute(&pool).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("23503")
    );
    let public_access = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM pg_class c,
           LATERAL aclexplode(COALESCE(c.relacl, acldefault('r', c.relowner))) acl
           WHERE c.oid = 'public.project_budget_notifications'::regclass AND acl.grantee = 0) AS "access!""#
    ).fetch_one(&pool).await.unwrap();
    assert!(!public_access);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_budget_checks_have_one_band_claimant(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::ExecutiveManager).await;
    sqlx::query!(
        "UPDATE projects SET last_budget_alert_pct = NULL WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let first_pool = pool.clone();
    let first_barrier = std::sync::Arc::clone(&barrier);
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        claim_budget_band(&first_pool, ids.project_id, 0, 80).await
    });
    let second_pool = pool.clone();
    let second_barrier = std::sync::Arc::clone(&barrier);
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        claim_budget_band(&second_pool, ids.project_id, 0, 80).await
    });

    let (first, second) = tokio::join!(first, second);
    let claimed = [first.unwrap().unwrap(), second.unwrap().unwrap()];
    assert_eq!(claimed.iter().filter(|&&won| won).count(), 1);
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT last_budget_alert_pct FROM projects WHERE id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(80)
    );
}

/// `(over_budget, band, consumed_minutes)` for each plugin budget event, in
/// dispatch order.
fn plugin_bands(events: Vec<AppEvent>) -> Vec<(bool, i32, Option<i32>)> {
    events
        .into_iter()
        .map(|event| match event {
            AppEvent::ProjectOverBudget { budget, .. } => {
                (true, budget.threshold_pct, budget.consumed_minutes)
            }
            AppEvent::ProjectBudgetThresholdReached { budget, .. } => {
                (false, budget.threshold_pct, budget.consumed_minutes)
            }
            other => panic!("unexpected event {other:?}"),
        })
        .collect()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn plugin_budget_bands_count_the_spent_minutes_the_projects_list_shows(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    // Workspace rounding would value the open entries at 30 + 30 and the frozen
    // one at 40; Spent counts the tracked minutes only.
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15, round_dir = 'up' WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 60,
           last_budget_alert_pct = NULL WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let entry = |minutes: i32, rounded: Option<i32>, state: EntryState| {
        let pool = pool.clone();
        async move {
            sqlx::query!(
                "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id,
                   spent_date, minutes, rounded_minutes, billable, state)
                 VALUES ($1, $2, $3, $4, $5, '2026-09-07', $6, $7, true, $8)",
                Uuid::now_v7(),
                ids.org_id,
                ids.user_id,
                ids.project_id,
                ids.task_id,
                minutes,
                rounded,
                state as EntryState,
            )
            .execute(&pool)
            .await
            .unwrap();
        }
    };
    let spent = || spent_minutes(&pool, &ids);
    let events = Recorder::default();

    entry(25, None, EntryState::Open).await;
    entry(25, Some(40), EntryState::Approved).await;
    announce_plugin_bands(&pool, &events, ids.project_id)
        .await
        .unwrap();
    assert_eq!(spent().await, 50);
    assert_eq!(plugin_bands(events.take()), [(false, 80, Some(50))]);

    entry(15, None, EntryState::Open).await;
    announce_plugin_bands(&pool, &events, ids.project_id)
        .await
        .unwrap();
    assert_eq!(spent().await, 65);
    assert_eq!(plugin_bands(events.take()), [(true, 100, Some(65))]);
}
