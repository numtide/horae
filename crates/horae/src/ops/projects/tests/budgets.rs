//! The budget progress the Projects overview shows: current period, scope
//! rows and who may read them.

use chrono::NaiveDate;
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{ProjectBudgetProgress, ProjectSpend};
use crate::notifications::budgets::{BudgetProgress, configured_progress};
use crate::ops::testing::set_grants;
use crate::ops::{Caller, OpError, begin_read};
use crate::testing::{SeedIds, seed, time_entry};

/// `list_project_budget_progress` on a pinned `date`, one row per scope.
async fn visible_scopes(
    pool: &PgPool,
    org_id: Uuid,
    viewer_id: Uuid,
    date: NaiveDate,
) -> Result<Vec<ProjectBudgetProgress>, OpError> {
    let mut read = begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = super::budget_progress_on(&mut read.tx, &read.actor, date, None).await;
    let projects = read.finish(result).await?;
    Ok(projects
        .into_iter()
        .flat_map(|project| project.breakdown)
        .collect())
}

/// `get_project_spend`: lifetime minutes per project.
async fn spend(pool: &PgPool, org_id: Uuid, viewer_id: Uuid) -> Result<Vec<ProjectSpend>, OpError> {
    let mut read = begin_read(pool, Caller::session(org_id, viewer_id)).await?;
    let result = super::spend(&mut read.tx, &read.actor, None).await;
    read.finish(result).await
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
async fn overview_budget_progress_is_current_period_scoped_and_authorized(pool: PgPool) {
    let ids = configured(&pool).await;
    let foreign = configured(&pool).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let old = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2026-08-31' WHERE id = $1",
        old
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
    let date = "2026-09-07".parse().unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].project_id, rows[0].budget, rows[0].consumed),
        (ids.project_id, Some(100), 60)
    );
    assert_eq!(rows[0].period_key, "2026-09");
    let lifetime = spend(&pool, ids.org_id, ids.user_id).await.unwrap();
    assert_eq!(lifetime[0].spent_minutes, 120);
    assert!(
        visible_scopes(&pool, ids.org_id, foreign.user_id, date)
            .await
            .unwrap()
            .is_empty()
    );
    set_grants(&mut pool.acquire().await.unwrap(), &ids, &[])
        .await
        .unwrap();
    assert!(
        visible_scopes(&pool, ids.org_id, ids.user_id, date)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    assert_eq!(rows[0].consumed, 60);
    let payload = serde_json::to_string(&rows).unwrap();
    for private in [
        "rate_cents",
        "admin_notes",
        "cost_rate",
        "notes",
        &entry.to_string(),
    ] {
        assert!(
            !payload.contains(private),
            "private field in progress: {private}"
        );
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        visible_scopes(&pool, ids.org_id, ids.user_id, date)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn overview_budget_progress_batches_projects_without_mixing_scopes(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let second = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency,budget_kind,budget_minutes)
        VALUES ($1,$2,$3,'Second project','USD','hours',300)",
        second,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,budget_scope,rate_mode)
        VALUES ($1,$2,$3,$4,'task','project')",
        Uuid::now_v7(),
        ids.org_id,
        second,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        second,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_task_settings (id,org_id,project_id,task_id,budget_minutes)
        VALUES ($1,$2,$3,$4,30)",
        Uuid::now_v7(),
        ids.org_id,
        second,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET project_id = $1, minutes = 120 WHERE id = $2",
        second,
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let date = "2026-09-01".parse().unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let first = rows
        .iter()
        .find(|row| row.project_id == ids.project_id)
        .unwrap();
    let second_row = rows.iter().find(|row| row.project_id == second).unwrap();
    assert_eq!(
        (first.budget, first.consumed, first.scope.as_str()),
        (Some(100), 60, "project")
    );
    assert_eq!(
        (
            second_row.budget,
            second_row.consumed,
            second_row.scope.as_str()
        ),
        (Some(30), 120, "task")
    );
    assert!(second_row.label.is_some());
    sqlx::query!(
        "UPDATE project_task_settings SET budget_minutes = NULL WHERE project_id = $1",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    let row = rows.iter().find(|row| row.project_id == second).unwrap();
    assert_eq!((row.budget, row.consumed), (None, 120));
    assert!(
        configured_progress(&mut connection, ids.org_id, second, date)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!(
        "DELETE FROM project_task_settings WHERE project_id = $1",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    let row = rows.iter().find(|row| row.project_id == second).unwrap();
    assert_eq!((row.budget, row.consumed), (None, 120));
    assert!(
        row.label.is_some(),
        "Linked tasks without settings remain visible"
    );
    sqlx::query!(
        "UPDATE project_settings SET budget_scope = 'person' WHERE project_id = $1",
        second
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    let row = rows.iter().find(|row| row.project_id == second).unwrap();
    assert_eq!(
        (row.budget, row.consumed, row.label.as_deref()),
        (None, 120, None)
    );
    sqlx::query!("DELETE FROM project_settings WHERE project_id = $1", second)
        .execute(&pool)
        .await
        .unwrap();
    let rows = visible_scopes(&pool, ids.org_id, ids.user_id, date)
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "Legacy projects do not acquire configured budgets"
    );
    assert_eq!(rows[0].project_id, ids.project_id);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn overview_budget_progress_includes_people_without_an_allowance(pool: PgPool) {
    let ids = configured(&pool).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE project_settings SET budget_scope = 'person' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other')",
        other,
        ids.org_id,
        format!("{other}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        other
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_member_budgets (id,org_id,project_id,user_id,budget_minutes) VALUES ($1,$2,$3,$4,75)",
        Uuid::now_v7(), ids.org_id, ids.project_id, other).execute(&pool).await.unwrap();
    let rows = visible_scopes(
        &pool,
        ids.org_id,
        ids.user_id,
        "2026-09-01".parse().unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(
        rows.len(),
        2,
        "An assigned person without an allowance still has progress"
    );
    assert!(
        rows.iter().any(|row| row.user_id == Some(ids.user_id)
            && row.budget.is_none()
            && row.consumed == 60)
    );
    assert!(
        rows.iter()
            .any(|row| row.user_id == Some(other) && row.budget == Some(75) && row.consumed == 0)
    );
    let alerts = progress(&pool, &ids, "2026-09-01").await;
    assert_eq!(
        alerts.len(),
        1,
        "Unallocated people do not gain an alert threshold"
    );
    assert_eq!(alerts[0].user_id, Some(other));
}
