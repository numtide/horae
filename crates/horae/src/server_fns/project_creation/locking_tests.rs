use super::*;
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};
use sqlx::PgPool;
use tokio::task::JoinSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
enum Operation {
    CreateClient,
    LoadDraft,
    SaveDraft,
    DiscardDraft,
    SelectedCatalog,
    SelectedClient,
    Options,
    LoadEditor,
    SaveEditor,
    Finalize,
}

impl Operation {
    async fn run(
        self,
        pool: &PgPool,
        ids: &SeedIds,
        draft: Uuid,
        request: &ProjectEditRequest,
    ) -> Result<(), ServerFnError> {
        let (actor, org) = (ids.user_id, ids.org_id);
        match self {
            Self::CreateClient => create_client_record(pool, actor, org, "New client", "EUR", "")
                .await
                .map(|_| ()),
            Self::LoadDraft => load_draft_record(pool, actor, org).await.map(|_| ()),
            Self::SaveDraft => save_draft_record(pool, actor, org, draft, 1, &request.form)
                .await
                .map(|_| ()),
            Self::DiscardDraft => discard_draft_record(pool, actor, org, draft, 1).await,
            Self::SelectedCatalog => {
                load_selected_catalog(pool, actor, org, &[ids.task_id], &[actor])
                    .await
                    .map(|_| ())
            }
            Self::SelectedClient => load_selected_client(pool, actor, org, ids.client_id)
                .await
                .map(|_| ()),
            Self::Options => {
                load_creation_options(pool, actor, org, &CreationSearch::default(), false)
                    .await
                    .map(|_| ())
            }
            Self::LoadEditor => editing::load_editable_project(pool, actor, org, ids.project_id)
                .await
                .map(|_| ()),
            Self::SaveEditor => editing::save_editable_project(pool, actor, org, request, false)
                .await
                .map(|_| ()),
            Self::Finalize => {
                finalize_draft_record(pool, actor, org, draft, 1, &request.form, false)
                    .await
                    .map(|_| ())
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn every_project_entry_gates_before_actor_and_rechecks_revocation(pool: PgPool) {
    for operation in [
        Operation::CreateClient,
        Operation::LoadDraft,
        Operation::SaveDraft,
        Operation::DiscardDraft,
        Operation::SelectedCatalog,
        Operation::SelectedClient,
        Operation::Options,
        Operation::LoadEditor,
        Operation::SaveEditor,
        Operation::Finalize,
    ] {
        let ids = seed(&pool, OrgRole::Admin).await;
        let editor = editing::load_editable_project(&pool, ids.user_id, ids.org_id, ids.project_id)
            .await
            .unwrap();
        let request = ProjectEditRequest {
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
        let draft = Uuid::now_v7();
        save_draft_record(&pool, ids.user_id, ids.org_id, draft, 0, &request.form)
            .await
            .unwrap();
        let mut revocation = pool.begin().await.unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        if matches!(operation, Operation::SaveEditor | Operation::Finalize) {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR SHARE",
                ids.org_id
            )
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR NO KEY UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revocation)
            .await
            .unwrap();
        }
        let mut requests = JoinSet::new();
        let request_pool = pool.clone();
        let actor_id = ids.user_id;
        let org_id = ids.org_id;
        requests.spawn(async move {
            let result = operation.run(&request_pool, &ids, draft, &request).await;
            (result, ids, request)
        });
        wait_for_blocked(&pool, pid).await;
        // The waiting operation must not already own actor SHARE. This also
        // detects a late organization FK wait after actor authorization.
        sqlx::query!(
            "SELECT id FROM users WHERE id = $1 FOR UPDATE NOWAIT",
            actor_id
        )
        .fetch_one(&mut *revocation)
        .await
        .expect("organization must precede actor");
        sqlx::query!(
            "UPDATE users SET org_role = 'member' WHERE id = $1 AND org_id = $2",
            actor_id,
            org_id
        )
        .execute(&mut *revocation)
        .await
        .unwrap();
        revocation.commit().await.unwrap();
        let (result, ids, request) = requests.join_next().await.unwrap().unwrap();
        let expected = if matches!(operation, Operation::LoadEditor | Operation::SaveEditor) {
            CONFLICT
        } else {
            FORBIDDEN
        };
        assert!(
            matches!(result, Err(ServerFnError::ServerError { code, .. }) if code == expected),
            "{operation:?}: {result:?}"
        );
        let retry = operation.run(&pool, &ids, draft, &request).await;
        assert!(
            matches!(
                retry,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "fresh {operation:?}: {retry:?}"
        );
        let unchanged = sqlx::query!(
            "SELECT (SELECT count(*) FROM clients WHERE org_id=$1) AS clients,
                    (SELECT count(*) FROM projects WHERE org_id=$1) AS projects,
                    (SELECT count(*) FROM project_edit_requests WHERE org_id=$1) AS receipts,
                    revision, completed_project_id, discarded_at FROM project_drafts WHERE id=$2",
            org_id,
            draft,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (
                unchanged.clients,
                unchanged.projects,
                unchanged.receipts,
                unchanged.revision
            ),
            (Some(1), Some(1), Some(0), 1)
        );
        assert!(unchanged.completed_project_id.is_none() && unchanged.discarded_at.is_none());
    }
}
