//! Project creation and reassignment against a concurrent client archive
//! (specs/012-clients-design/increments/lifecycle LC-004). Archive's own side
//! of these races is in `server_fns/clients/lifecycle_tests.rs`.

use super::*;
use crate::server_fns::test_seed::{SeedIds, repeatable_read_pool, seed, wait_for_blocked};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
enum Writer {
    Finalize,
    Reassign,
}

/// Archive `client_id` in an open transaction holding archive's locks, and
/// return it with its backend pid. The caller commits it.
async fn hold_archive(
    pool: &PgPool,
    org_id: Uuid,
    client_id: Uuid,
) -> (sqlx::Transaction<'static, sqlx::Postgres>, i32) {
    let mut archive = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *archive)
        .await
        .unwrap();
    lock_organization(&mut archive, org_id, OrganizationLock::Shared)
        .await
        .unwrap();
    sqlx::query!("SELECT id FROM clients WHERE id = $1 FOR UPDATE", client_id)
        .fetch_one(&mut *archive)
        .await
        .unwrap();
    sqlx::query!("UPDATE clients SET active = false WHERE id = $1", client_id)
        .execute(&mut *archive)
        .await
        .unwrap();
    (archive, pid)
}

/// The writer's request and the client it targets, which has no projects yet,
/// so archiving it is valid.
async fn prepare(pool: &PgPool, ids: &SeedIds, writer: Writer) -> (Uuid, ProjectEditRequest, Uuid) {
    let target = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Target', 'EUR')",
        target,
        ids.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    let editor = editing::load_editable_project(pool, ids.user_id, ids.org_id, ids.project_id)
        .await
        .unwrap();
    let mut request = ProjectEditRequest {
        task_activity: Vec::new(),
        id: Uuid::now_v7(),
        project_id: ids.project_id,
        expected_revision: editor.revision,
        expected_requester: editor.access.as_ref().map(|access| access.requester),
        managers: editor
            .access
            .as_ref()
            .map(|access| (&access.managers).into()),
        form: editor.form,
        unchanged: Vec::new(),
    };
    request.form.client_id = Some(target);
    let draft = Uuid::now_v7();
    if matches!(writer, Writer::Finalize) {
        // A new project's own minimal valid form, not the existing project's.
        request.form = ProjectForm {
            client_id: Some(target),
            name: "Created under target".into(),
            rate_mode: horae_core::project::RateMode::Project,
            project_rate: "0".into(),
            ..Default::default()
        };
        save_draft_record(pool, ids.user_id, ids.org_id, draft, 0, &request.form)
            .await
            .unwrap();
    }
    (target, request, draft)
}

/// A writer that waits for an archive of its client must write nothing, at
/// READ COMMITTED (it then reads the archived client) and at REPEATABLE READ
/// (its share lock on a client changed after its snapshot fails to serialize).
#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn creation_and_reassignment_waiting_for_a_client_archive_write_nothing(pool: PgPool) {
    let rr = repeatable_read_pool(&pool).await;
    for writer in [Writer::Finalize, Writer::Reassign] {
        for (isolation, writer_pool) in [("default", &pool), ("repeatable read", &rr)] {
            let ids = seed(&pool, OrgRole::Admin).await;
            let (target, request, draft) = prepare(&pool, &ids, writer).await;
            let (archive, blocker) = hold_archive(&pool, ids.org_id, target).await;
            let run_pool = writer_pool.clone();
            let (actor, org) = (ids.user_id, ids.org_id);
            let retry = request.clone();
            let write =
                tokio::spawn(
                    async move { run(writer, &run_pool, actor, org, draft, &request).await },
                );
            wait_for_blocked(&pool, blocker).await;
            archive.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), write)
                .await
                .unwrap()
                .unwrap();
            let expected = match (writer, isolation) {
                // Finalization reads the committed archive and refuses the client.
                (Writer::Finalize, "default") => {
                    (NOT_FOUND, "Select an active client in this organization")
                }
                // Otherwise the share lock meets a client changed after the
                // snapshot, which `storage_error` reports as a conflict to reload.
                // The editor's save always sets SERIALIZABLE itself
                // (`editing/save.rs`), whatever the session default.
                _ => (
                    CONFLICT,
                    "The project changed while saving. Reload before retrying.",
                ),
            };
            match result {
                Err(ServerFnError::ServerError { code, message, .. }) => assert_eq!(
                    (code, message.as_str()),
                    expected,
                    "{writer:?} at {isolation}"
                ),
                other => panic!("{writer:?} at {isolation}: {other:?}"),
            }
            let linked = sqlx::query_scalar!(
                r#"SELECT count(*) AS "count!" FROM projects WHERE client_id = $1"#,
                target
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(linked, 0, "{writer:?} at {isolation}");
            // The same request succeeds once the client is active again, so the
            // refusal above came from the archive and not from the request.
            sqlx::query!("UPDATE clients SET active = true WHERE id = $1", target)
                .execute(&pool)
                .await
                .unwrap();
            run(writer, &pool, actor, org, draft, &retry)
                .await
                .unwrap_or_else(|error| panic!("{writer:?} at {isolation}: {error:?}"));
        }
    }
}

async fn run(
    writer: Writer,
    pool: &PgPool,
    actor: Uuid,
    org: Uuid,
    draft: Uuid,
    request: &ProjectEditRequest,
) -> Result<(), ServerFnError> {
    match writer {
        Writer::Finalize => finalize_draft_record(pool, actor, org, draft, 1, &request.form, false)
            .await
            .map(|_| ()),
        Writer::Reassign => editing::save_editable_project(pool, actor, org, request, false)
            .await
            .map(|_| ()),
    }
}
