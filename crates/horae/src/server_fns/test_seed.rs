//! Shared fixture for the in-crate `#[sqlx::test]` guard tests: one minimal
//! tenant (org, user, client, project, task) to hang time entries off. These
//! tests live inside the crate because `tests/` cannot import a bin crate's
//! modules.

use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

pub struct SeedIds {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
}

/// Seed the minimal tenant, giving the user `role`, and return its ids.
pub async fn seed(pool: &PgPool, role: OrgRole) -> SeedIds {
    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Test Org')",
        org_id
    )
    .execute(pool)
    .await
    .unwrap();

    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, 'Test User', $4)",
        user_id,
        org_id,
        format!("{user_id}@test.com"),
        role as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();

    let client_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
        client_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    let project_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
        project_id,
        org_id,
        client_id,
    )
    .execute(pool)
    .await
    .unwrap();

    let task_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Dev')",
        task_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    SeedIds {
        org_id,
        user_id,
        client_id,
        project_id,
        task_id,
    }
}

pub async fn time_entry(
    pool: &PgPool,
    ids: &SeedIds,
    state: horae_core::types::EntryState,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', 60, true, $6)",
        id,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id,
        state as horae_core::types::EntryState,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}
