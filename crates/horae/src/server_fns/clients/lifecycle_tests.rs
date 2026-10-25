//! Client archive and restore (specs/012-clients-design/increments/lifecycle):
//! archive needs every linked project archived, restore changes only the
//! client, and both recheck current authority before revealing project state.

use super::*;
use crate::plugin::event::ActiveTransition;
use crate::server_fns::test_seed::{
    SeedIds, repeatable_read_pool, seed, time_entry, wait_for_blocked,
};
use horae_core::types::EntryState;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

const ARCHIVE_PROJECTS_FIRST: &str = "Archive this client's projects before archiving the client.";

/// A manager's organization whose client has one project plus history a status
/// change must keep: an invoiced time entry and import provenance.
async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Manager).await;
    let entry = time_entry(pool, &ids, EntryState::Open).await;
    let invoice = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
         VALUES ($1, $2, $3, 'LIFECYCLE-1', 'sent', '2026-09-07', '2026-10-07', 'EUR', 6000)",
        invoice,
        ids.org_id,
        ids.client_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
         VALUES ($1, $2, $3, 'Historical time', 60, 6000, 6000)",
        Uuid::now_v7(),
        invoice,
        entry,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO harvest_import_map (org_id, harvest_entity_type, harvest_id, horae_id)
         VALUES ($1, 'project', 42, $2)",
        ids.org_id,
        ids.project_id,
    )
    .execute(pool)
    .await
    .unwrap();
    // A finished import run with a stored report and an error chunk.
    let job = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO horae_jobs (id, org_id, kind, status, payload, original_requester_id, report)
         VALUES ($1, $2, 'harvest_csv_import', 'succeeded', '{\"source\":\"lifecycle\"}', $3, '{\"errors\":[]}')",
        job,
        ids.org_id,
        ids.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body)
         VALUES ($1, $2, $3, 0, $4)",
        Uuid::now_v7(),
        job,
        ids.org_id,
        b"retained error\n".as_slice(),
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

async fn archive_projects(pool: &PgPool, client_id: Uuid) {
    sqlx::query!(
        "UPDATE projects SET active = false WHERE client_id = $1",
        client_id
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Every record a client status change must leave exactly as it was. The
/// fixture seeds each part, so an unchanged snapshot compares real rows.
async fn history(pool: &PgPool) -> serde_json::Value {
    let snapshot = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
          'projects', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM projects t),
          'entries', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM time_entries t),
          'invoices', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM invoices t),
          'lines', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM invoice_line_items t),
          'imports', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.horae_id) FROM harvest_import_map t),
          'jobs', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM horae_jobs t),
          'report_chunks', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM horae_job_report_error_chunks t)
        ) AS "snapshot!""#
    )
    .fetch_one(pool)
    .await
    .unwrap();
    for (part, rows) in snapshot.as_object().unwrap() {
        assert!(rows.is_array(), "{part} has no rows to compare");
    }
    snapshot
}

async fn client_active(pool: &PgPool, client_id: Uuid) -> bool {
    sqlx::query_scalar!("SELECT active FROM clients WHERE id = $1", client_id)
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
async fn archive_is_refused_while_any_linked_project_is_active(pool: PgPool) {
    let ids = fixture(&pool).await;
    // A second, archived project must not hide the active one.
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, active)
         VALUES ($1, $2, $3, 'Finished', 'EUR', false)",
        Uuid::now_v7(),
        ids.org_id,
        ids.client_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = history(&pool).await;
    let error = set_client_active_record(&pool, ids.org_id, ids.user_id, ids.client_id, false)
        .await
        .unwrap_err();
    let (code, message) = code_and_message(error);
    // The reason names no project and counts none.
    assert_eq!((code, message.as_str()), (CONFLICT, ARCHIVE_PROJECTS_FIRST));
    assert!(client_active(&pool, ids.client_id).await);
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn archive_succeeds_without_active_projects_and_repeats_as_a_noop(pool: PgPool) {
    for linked in ["none", "archived only"] {
        let ids = fixture(&pool).await;
        let target = if linked == "none" {
            let id = Uuid::now_v7();
            sqlx::query!(
                "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'No projects', 'EUR')",
                id,
                ids.org_id,
            )
            .execute(&pool)
            .await
            .unwrap();
            id
        } else {
            archive_projects(&pool, ids.client_id).await;
            ids.client_id
        };
        let before = history(&pool).await;
        let (client, transition) =
            set_client_active_record(&pool, ids.org_id, ids.user_id, target, false)
                .await
                .unwrap();
        assert_eq!(
            (transition, client.active),
            (Some(ActiveTransition::Deactivated), false),
            "{linked}"
        );
        assert_eq!(history(&pool).await, before, "{linked}");
        let (client, transition) =
            set_client_active_record(&pool, ids.org_id, ids.user_id, target, false)
                .await
                .unwrap();
        assert_eq!((transition, client.active), (None, false), "{linked}");
        assert_eq!(history(&pool).await, before, "{linked}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn restore_changes_only_the_client(pool: PgPool) {
    let ids = fixture(&pool).await;
    archive_projects(&pool, ids.client_id).await;
    set_client_active_record(&pool, ids.org_id, ids.user_id, ids.client_id, false)
        .await
        .unwrap();
    let before = history(&pool).await;
    let (client, transition) =
        set_client_active_record(&pool, ids.org_id, ids.user_id, ids.client_id, true)
            .await
            .unwrap();
    assert_eq!(
        (transition, client.active),
        (Some(ActiveTransition::Reactivated), true)
    );
    // Projects stay archived until restored on their own.
    assert_eq!(history(&pool).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn status_changes_recheck_current_authority_before_project_state(pool: PgPool) {
    for change in [
        "member",
        "inactive",
        "policy one",
        "unknown policy",
        "foreign actor",
    ] {
        // The project stays active: a caller without authority must get the
        // authority error, never the archive prerequisite.
        let ids = fixture(&pool).await;
        let foreign = seed(&pool, OrgRole::Admin).await;
        let mut actor = ids.user_id;
        match change {
            "member" => {
                sqlx::query!(
                    "UPDATE users SET org_role = 'member' WHERE id = $1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "inactive" => {
                sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "policy one" | "unknown policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version = $2 WHERE id = $1",
                    ids.org_id,
                    if change == "policy one" { 1 } else { 2 },
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => actor = foreign.user_id,
        }
        let before = history(&pool).await;
        for active in [false, true] {
            let error = set_client_active_record(&pool, ids.org_id, actor, ids.client_id, active)
                .await
                .unwrap_err();
            let (code, message) = code_and_message(error);
            assert_eq!(code, FORBIDDEN, "{change}");
            assert_ne!(message, ARCHIVE_PROJECTS_FIRST, "{change}");
        }
        assert!(client_active(&pool, ids.client_id).await, "{change}");
        assert_eq!(history(&pool).await, before, "{change}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn status_change_waits_for_a_role_change_and_then_fails_closed(pool: PgPool) {
    let ids = fixture(&pool).await;
    archive_projects(&pool, ids.client_id).await;
    let (org_id, user_id, client_id) = (ids.org_id, ids.user_id, ids.client_id);
    let mut change = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *change)
        .await
        .unwrap()
        .unwrap();
    crate::db::lock_organization(
        &mut change,
        org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'member' WHERE id = $1",
        user_id
    )
    .execute(&mut *change)
    .await
    .unwrap();
    let run_pool = pool.clone();
    let write = tokio::spawn(async move {
        set_client_active_record(&run_pool, org_id, user_id, client_id, false).await
    });
    wait_for_blocked(&pool, blocker).await;
    change.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), write)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(code_and_message(error).0, FORBIDDEN);
    assert!(client_active(&pool, client_id).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn foreign_clients_are_indistinguishable_from_unknown_ones(pool: PgPool) {
    let ids = fixture(&pool).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let mut errors = Vec::new();
    for client_id in [Uuid::now_v7(), ids.client_id] {
        for active in [false, true] {
            let error =
                set_client_active_record(&pool, other.org_id, other.user_id, client_id, active)
                    .await
                    .unwrap_err();
            errors.push(code_and_message(error));
        }
    }
    assert!(errors.iter().all(|error| error == &errors[0]), "{errors:?}");
    assert_eq!(errors[0].0, NOT_FOUND);
    assert!(client_active(&pool, ids.client_id).await);
}

/// The other side of an archive race: a writer that makes a project active
/// under the client while holding the locks that writer really takes.
#[derive(Clone, Copy, Debug)]
enum Writer {
    /// Draft finalization or an import creating a new active project.
    Create,
    /// The project editor moving an active project to this client.
    Reassign,
    /// Single or bulk project restoration.
    Restore,
}

impl Writer {
    /// Run the writer's statements in `tx` and leave them uncommitted.
    async fn hold(
        self,
        pool: &PgPool,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ids: &SeedIds,
    ) {
        crate::db::lock_organization(tx, ids.org_id, crate::db::OrganizationLock::Shared)
            .await
            .unwrap();
        match self {
            Self::Create => {
                sqlx::query!(
                    "SELECT id FROM clients WHERE id = $1 AND active FOR SHARE",
                    ids.client_id
                )
                .fetch_one(&mut **tx)
                .await
                .unwrap();
                sqlx::query!(
                    "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Created', 'EUR')",
                    Uuid::now_v7(),
                    ids.org_id,
                    ids.client_id,
                )
                .execute(&mut **tx)
                .await
                .unwrap();
            }
            Self::Reassign => {
                let elsewhere = Uuid::now_v7();
                sqlx::query!(
                    "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Elsewhere', 'EUR')",
                    elsewhere,
                    ids.org_id,
                )
                .execute(pool)
                .await
                .unwrap();
                let moved = Uuid::now_v7();
                sqlx::query!(
                    "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Moved', 'EUR')",
                    moved,
                    ids.org_id,
                    elsewhere,
                )
                .execute(pool)
                .await
                .unwrap();
                sqlx::query!("SELECT id FROM projects WHERE id = $1 FOR UPDATE", moved)
                    .fetch_one(&mut **tx)
                    .await
                    .unwrap();
                sqlx::query!(
                    "SELECT id FROM clients WHERE id = $1 FOR SHARE",
                    ids.client_id
                )
                .fetch_one(&mut **tx)
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE projects SET client_id = $2 WHERE id = $1",
                    moved,
                    ids.client_id
                )
                .execute(&mut **tx)
                .await
                .unwrap();
            }
            Self::Restore => {
                sqlx::query!(
                    "SELECT id FROM projects WHERE id = $1 FOR UPDATE",
                    ids.project_id
                )
                .fetch_one(&mut **tx)
                .await
                .unwrap();
                sqlx::query!(
                    "SELECT id FROM clients WHERE id = $1 FOR SHARE",
                    ids.client_id
                )
                .fetch_one(&mut **tx)
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE projects SET active = true WHERE id = $1",
                    ids.project_id
                )
                .execute(&mut **tx)
                .await
                .unwrap();
            }
        }
    }
}

/// A writer commits a newly active project while archive waits for the client
/// lock. Archive must then see it, even when the session default is REPEATABLE
/// READ (the writer leaves the client row unchanged, so no serialization error
/// would warn an archive reading its older snapshot).
#[sqlx::test(migrations = "./migrations")]
async fn archive_waiting_for_a_project_writer_sees_its_committed_project(pool: PgPool) {
    let rr = repeatable_read_pool(&pool).await;
    for writer in [Writer::Create, Writer::Reassign, Writer::Restore] {
        for (isolation, archive_pool) in [("default", &pool), ("repeatable read", &rr)] {
            let ids = fixture(&pool).await;
            archive_projects(&pool, ids.client_id).await;
            let mut tx = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *tx)
                .await
                .unwrap()
                .unwrap();
            writer.hold(&pool, &mut tx, &ids).await;
            let run_pool = archive_pool.clone();
            let (org_id, user_id, client_id) = (ids.org_id, ids.user_id, ids.client_id);
            let archive = tokio::spawn(async move {
                set_client_active_record(&run_pool, org_id, user_id, client_id, false).await
            });
            wait_for_blocked(&pool, blocker).await;
            tx.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), archive)
                .await
                .unwrap()
                .unwrap();
            let error = match result {
                Err(error) => error,
                Ok(archived) => panic!("{writer:?} at {isolation}: archived {archived:?}"),
            };
            let (code, message) = code_and_message(error);
            assert_eq!(
                (code, message.as_str()),
                (CONFLICT, ARCHIVE_PROJECTS_FIRST),
                "{writer:?} at {isolation}"
            );
            assert!(
                client_active(&pool, client_id).await,
                "{writer:?} at {isolation}: an archived client has an active project"
            );
        }
    }
}
