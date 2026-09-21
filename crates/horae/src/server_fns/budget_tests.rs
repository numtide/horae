use super::claim_budget_band;
use crate::server_fns::test_seed::seed;
use horae_core::types::OrgRole;
use sqlx::PgPool;

use super::budgets::configured_progress;
use crate::server_fns::test_seed::{SeedIds, time_entry};
use horae_core::types::EntryState;
use uuid::Uuid;

async fn configured(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Manager).await;
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

async fn progress(pool: &PgPool, ids: &SeedIds, date: &str) -> Vec<super::budgets::BudgetProgress> {
    configured_progress(
        &mut pool.acquire().await.unwrap(),
        ids.org_id,
        ids.project_id,
        date.parse().unwrap(),
    )
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
async fn configured_budget_does_not_read_foreign_or_legacy_projects(pool: PgPool) {
    let ids = configured(&pool).await;
    let other = seed(&pool, OrgRole::Manager).await;
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

#[sqlx::test(migrations = "./migrations")]
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
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date),
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
    );
    assert_eq!(first.unwrap() + second.unwrap(), 1);
    sqlx::query!("UPDATE time_entries SET minutes = 0 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE time_entries SET minutes = 90 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
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
        super::budgets::enqueue_alerts(
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

#[sqlx::test(migrations = "./migrations")]
async fn configured_alerts_require_enabled_active_project_and_authorized_recipient(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let date = "2026-09-01".parse().unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
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
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
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
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        0
    );
    sqlx::query!("UPDATE users SET active = true WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        super::budgets::enqueue_alerts(&pool, ids.org_id, ids.project_id, date)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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
        super::budgets::enqueue_alerts(
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

#[sqlx::test(migrations = "./migrations")]
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
        super::budgets::enqueue_alerts(
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

#[sqlx::test(migrations = "./migrations")]
async fn configured_alert_and_outbox_roll_back_together(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    sqlx::query!("ALTER TABLE horae_outbox ADD CONSTRAINT reject_test_budget_email CHECK (event_kind <> 'budget_email')").execute(&pool).await.unwrap();
    assert!(
        super::budgets::enqueue_alerts(
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

#[sqlx::test(migrations = "./migrations")]
async fn configured_budget_sweep_recovers_missed_checks_without_duplicates(pool: PgPool) {
    let ids = configured(&pool).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let date = "2026-09-01".parse().unwrap();
    super::budgets::sweep(&pool, date).await.unwrap();
    super::budgets::sweep(&pool, date).await.unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM project_budget_notifications")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn configured_alert_recipients_are_active_same_org_managers_and_project_leads(pool: PgPool) {
    let ids = configured(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!("UPDATE project_settings SET alert_enabled = true, alert_threshold = 0 WHERE project_id = $1", ids.project_id).execute(&pool).await.unwrap();
    let mut expected = vec![ids.user_id];
    for (role, active, lead) in [
        (OrgRole::Admin, true, false),
        (OrgRole::Manager, true, true),
        (OrgRole::Member, true, true),
        (OrgRole::Member, true, false),
        (OrgRole::Admin, false, true),
    ] {
        let user = Uuid::now_v7();
        sqlx::query!("INSERT INTO users (id,org_id,email,name,org_role,active) VALUES ($1,$2,$3,'Recipient',$4,$5)", user, ids.org_id, format!("{user}@test.com"), role as OrgRole, active).execute(&pool).await.unwrap();
        if lead {
            sqlx::query!(
                "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'lead')",
                Uuid::now_v7(),
                ids.project_id,
                user
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        if active && (role != OrgRole::Member || lead) {
            expected.push(user);
        }
    }
    assert_eq!(
        super::budgets::enqueue_alerts(
            &pool,
            foreign.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        super::budgets::enqueue_alerts(
            &pool,
            ids.org_id,
            ids.project_id,
            "2026-09-01".parse().unwrap()
        )
        .await
        .unwrap(),
        expected.len()
    );
    let actual = sqlx::query_scalar!(
        "SELECT recipient_id FROM project_budget_notifications ORDER BY recipient_id"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[sqlx::test(migrations = "./migrations")]
async fn budget_notification_schema_enforces_tenants_and_no_public_access(pool: PgPool) {
    let ids = configured(&pool).await;
    let foreign = seed(&pool, OrgRole::Manager).await;
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

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_budget_checks_have_one_band_claimant(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
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
