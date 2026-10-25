//! Restoring a project needs its client active first (015 OP13 state
//! prerequisite; specs/012-clients-design/increments/lifecycle LC-005).

use super::*;
use crate::server_fns::test_seed::{
    SeedIds, repeatable_read_pool, seed, time_entry, wait_for_blocked,
};
use horae_core::types::EntryState;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

const RESTORE_CLIENT_FIRST: &str = "Restore the client before restoring this project.";

async fn project(pool: &PgPool, ids: &SeedIds, client_id: Uuid, active: bool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, active)
         VALUES ($1, $2, $3, 'Lifecycle', 'EUR', $4)",
        id,
        ids.org_id,
        client_id,
        active,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn set_client(pool: &PgPool, client_id: Uuid, active: bool) {
    sqlx::query!(
        "UPDATE clients SET active = $2 WHERE id = $1",
        client_id,
        active
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Projects, time and invoices: a refused restore must change none of them.
async fn history(pool: &PgPool) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
          'projects', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM projects t),
          'clients', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM clients t),
          'entries', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM time_entries t),
          'invoices', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM invoices t)
        ) AS "snapshot!""#
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

fn code_and_message(error: ServerFnError) -> (u16, String) {
    match error {
        ServerFnError::ServerError { code, message, .. } => (code, message),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn restoring_a_project_of_an_archived_client_is_refused_single_and_bulk(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let other_client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Still active', 'EUR')",
        other_client,
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    // Restorable on its own: a bulk request that also names the blocked
    // project must leave it archived too (all-or-nothing).
    let restorable = project(&pool, &ids, other_client, false).await;
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    set_client(&pool, ids.client_id, false).await;
    let before = history(&pool).await;

    let single = set_project_active_record(&pool, ids.org_id, ids.user_id, ids.project_id, true)
        .await
        .unwrap_err();
    let mut ordered = [restorable, ids.project_id];
    ordered.sort_unstable();
    let bulk = set_projects_active_records(&pool, ids.org_id, ids.user_id, &ordered, true)
        .await
        .unwrap_err();
    for error in [single, bulk] {
        assert_eq!(
            code_and_message(error),
            (CONFLICT, RESTORE_CLIENT_FIRST.to_owned())
        );
    }
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn an_already_active_project_of_an_archived_client_stays_a_noop(pool: PgPool) {
    // Data from before the archive prerequisite existed.
    let ids = seed(&pool, OrgRole::Manager).await;
    set_client(&pool, ids.client_id, false).await;
    let before = history(&pool).await;
    let (project, transition) =
        set_project_active_record(&pool, ids.org_id, ids.user_id, ids.project_id, true)
            .await
            .unwrap();
    assert_eq!((project.active, transition), (true, None));
    let results =
        set_projects_active_records(&pool, ids.org_id, ids.user_id, &[ids.project_id], true)
            .await
            .unwrap();
    assert_eq!(results[0].1, None);
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_project_restores_once_its_client_is_restored(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let bulk_project = project(&pool, &ids, ids.client_id, false).await;
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    set_client(&pool, ids.client_id, false).await;
    set_client(&pool, ids.client_id, true).await;
    let (project, transition) =
        set_project_active_record(&pool, ids.org_id, ids.user_id, ids.project_id, true)
            .await
            .unwrap();
    assert_eq!(
        (project.active, transition),
        (
            true,
            Some(crate::plugin::event::ActiveTransition::Reactivated)
        )
    );
    let results =
        set_projects_active_records(&pool, ids.org_id, ids.user_id, &[bulk_project], true)
            .await
            .unwrap();
    assert!(results[0].0.active);
}

/// Archive commits while a restoration waits for the client lock. The restore
/// must fail at READ COMMITTED (it reads the archived client) and at REPEATABLE
/// READ (its share lock on a row changed after its snapshot is a serialization
/// failure), and write nothing either way.
#[sqlx::test(migrations = "./migrations")]
async fn a_restore_waiting_for_a_client_archive_writes_nothing(pool: PgPool) {
    let rr = repeatable_read_pool(&pool).await;
    for bulk in [false, true] {
        for (isolation, restore_pool) in [("default", &pool), ("repeatable read", &rr)] {
            let ids = seed(&pool, OrgRole::Manager).await;
            sqlx::query!(
                "UPDATE projects SET active = false WHERE id = $1",
                ids.project_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let mut archive = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *archive)
                .await
                .unwrap()
                .unwrap();
            crate::db::lock_organization(&mut archive, ids.org_id, OrganizationLock::Shared)
                .await
                .unwrap();
            sqlx::query!(
                "SELECT id FROM clients WHERE id = $1 FOR UPDATE",
                ids.client_id
            )
            .fetch_one(&mut *archive)
            .await
            .unwrap();
            sqlx::query!(
                "UPDATE clients SET active = false WHERE id = $1",
                ids.client_id
            )
            .execute(&mut *archive)
            .await
            .unwrap();
            let run_pool = restore_pool.clone();
            let (org_id, user_id, project_id) = (ids.org_id, ids.user_id, ids.project_id);
            let restore = tokio::spawn(async move {
                if bulk {
                    set_projects_active_records(&run_pool, org_id, user_id, &[project_id], true)
                        .await
                        .map(|_| ())
                } else {
                    set_project_active_record(&run_pool, org_id, user_id, project_id, true)
                        .await
                        .map(|_| ())
                }
            });
            wait_for_blocked(&pool, blocker).await;
            archive.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), restore)
                .await
                .unwrap()
                .unwrap();
            let (code, message) = code_and_message(result.unwrap_err());
            if isolation == "default" {
                // It reads the committed archive and refuses.
                assert_eq!(
                    (code, message.as_str()),
                    (CONFLICT, RESTORE_CLIENT_FIRST),
                    "bulk={bulk}"
                );
            } else {
                // Its share lock meets a client changed after its snapshot: a
                // database failure, never the archived-client refusal or success.
                // The status code, not PostgreSQL's wording, is the contract.
                assert_eq!(code, INTERNAL_ERROR, "bulk={bulk}: {message}");
                assert_ne!(message, RESTORE_CLIENT_FIRST, "bulk={bulk}");
            }
            let active =
                sqlx::query_scalar!("SELECT active FROM projects WHERE id = $1", project_id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            assert!(!active, "bulk={bulk} at {isolation}");
        }
    }
}
