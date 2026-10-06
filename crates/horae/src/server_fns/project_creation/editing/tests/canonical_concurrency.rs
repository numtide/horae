use super::canonical_fields::{canonical_actor, keep_protected};
use super::*;
use crate::models::project_creation::{ProjectFieldAccess, ProtectedProjectField};
use crate::models::project_managers::{ProjectManagersCommand, ProjectManagersCommandKind};
use crate::server_fns::{permissions::project_management, test_seed::wait_for_blocked};
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use tokio::task::JoinSet;

#[derive(Clone, Copy, Debug)]
enum Operation {
    Read,
    Save,
    Replay,
}

impl Operation {
    async fn run(
        self,
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        request: &ProjectEditRequest,
    ) -> Result<(), ServerFnError> {
        match self {
            Self::Read => load_editable_project(pool, actor, org, request.project_id)
                .await
                .map(|_| ()),
            Self::Save | Self::Replay => save_editable_project(pool, actor, org, request, false)
                .await
                .map(|_| ()),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Revocation {
    ProjectGrant,
    Designation,
    FinancialAccess,
}

async fn saved_state(pool: &PgPool, project: Uuid) -> serde_json::Value {
    sqlx::query_scalar!(
        r#"SELECT jsonb_build_array(
          (SELECT to_jsonb(p) FROM projects p WHERE id=$1),
          (SELECT jsonb_agg(to_jsonb(t) ORDER BY task_id) FROM project_tasks t WHERE project_id=$1),
          (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM project_task_settings t WHERE project_id=$1),
          (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM project_edit_requests r WHERE project_id=$1)
        ) AS "state!""#,
        project,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn revoked_while_waiting(pool: &PgPool, operation: Operation) {
    for revocation in [
        Revocation::ProjectGrant,
        Revocation::Designation,
        Revocation::FinancialAccess,
    ] {
        let (owner, original) = configured_fixture(pool).await;
        canonical_actor(
            pool,
            &owner,
            &PermissionSelection::new(&[
                Permission::ProjectWriteManaged,
                match revocation {
                    Revocation::FinancialAccess => Permission::BillableRateWriteManaged,
                    _ => Permission::BillableRateWriteAll,
                },
            ]),
        )
        .await;
        sqlx::query!(
            "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(), owner.org_id, owner.user_id, original.id,
        ).execute(pool).await.unwrap();
        let editor = load_editable_project(pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        let mut request = edit_request(editor);
        request.unchanged = keep_protected(&request.form);
        request.form.name = "Authorized financial edit".into();
        request.form.tasks[0].rate = "37.00".into();
        request
            .unchanged
            .retain(|field| *field != ProtectedProjectField::TaskRate(request.form.tasks[0].id));
        if matches!(operation, Operation::Replay) {
            save_editable_project(pool, owner.user_id, owner.org_id, &request, false)
                .await
                .unwrap();
        }
        let before = saved_state(pool, original.id).await;
        let mut revoke = pool.begin().await.unwrap();
        lock_organization(&mut revoke, owner.org_id, OrganizationLock::AccessChange)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *revoke)
            .await
            .unwrap()
            .unwrap();
        match revocation {
            Revocation::Designation => {
                project_management::execute_in_transaction(
                    &mut revoke,
                    owner.org_id,
                    owner.user_id,
                    &ProjectManagersCommand {
                        request_id: Uuid::now_v7(),
                        expected_access_revision: request
                            .managers
                            .as_ref()
                            .unwrap()
                            .expected_access_revision,
                        project_id: original.id,
                        manager_ids: vec![],
                        kind: ProjectManagersCommandKind::ReplaceProjectManagers,
                    },
                )
                .await
                .unwrap();
            }
            Revocation::ProjectGrant | Revocation::FinancialAccess => {
                let selection = match revocation {
                    Revocation::ProjectGrant => PermissionSelection::new(&[
                        Permission::ProjectReadManaged,
                        Permission::BillableRateWriteAll,
                    ]),
                    _ => PermissionSelection::new(&[Permission::ProjectWriteManaged]),
                };
                let grants: Vec<String> =
                    serde_json::from_value(serde_json::to_value(selection).unwrap()).unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                    owner.user_id,
                    &grants
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
                // Access writers publish the revision with the grants under this gate.
                sqlx::query!(
                    "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
                    owner.org_id
                )
                .execute(&mut *revoke)
                .await
                .unwrap();
            }
        }
        let mut pending = JoinSet::new();
        let request_pool = pool.clone();
        let pending_request = request.clone();
        pending.spawn(async move {
            operation
                .run(&request_pool, owner.org_id, owner.user_id, &pending_request)
                .await
        });
        wait_for_blocked(pool, pid).await;
        revoke.commit().await.unwrap();
        let result = pending.join_next().await.unwrap().unwrap();
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError { code: CONFLICT, .. })
            ),
            "{operation:?}/{revocation:?} must discard the pre-revocation snapshot: {result:?}"
        );
        let fresh = operation
            .run(pool, owner.org_id, owner.user_id, &request)
            .await;
        if matches!(
            (operation, revocation),
            (Operation::Read, Revocation::FinancialAccess)
        ) {
            fresh.unwrap();
            let editor = load_editable_project(pool, owner.user_id, owner.org_id, original.id)
                .await
                .unwrap();
            assert_eq!(
                editor.access.unwrap().billable,
                ProjectFieldAccess::Withheld
            );
            assert!(editor.form.tasks[0].rate.is_empty());
        } else {
            assert!(
                matches!(
                    fresh,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "fresh {operation:?}/{revocation:?} must reauthorize: {fresh:?}"
            );
        }
        assert_eq!(saved_state(pool, original.id).await, before);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_read_discards_snapshot_after_concurrent_revocation(pool: PgPool) {
    revoked_while_waiting(&pool, Operation::Read).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_save_rechecks_authority_after_concurrent_revocation(pool: PgPool) {
    revoked_while_waiting(&pool, Operation::Save).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_replay_rechecks_recorded_effects_after_concurrent_revocation(pool: PgPool) {
    revoked_while_waiting(&pool, Operation::Replay).await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_editor_operations_finish_before_waiting_designation_revocation(pool: PgPool) {
    for operation in [Operation::Read, Operation::Save, Operation::Replay] {
        let (owner, original) = configured_fixture(&pool).await;
        canonical_actor(
            &pool,
            &owner,
            &PermissionSelection::new(&[Permission::ProjectWriteManaged]),
        )
        .await;
        sqlx::query!(
            "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(), owner.org_id, owner.user_id, original.id,
        ).execute(&pool).await.unwrap();
        let editor = load_editable_project(&pool, owner.user_id, owner.org_id, original.id)
            .await
            .unwrap();
        let mut request = edit_request(editor);
        request.unchanged = keep_protected(&request.form);
        request.form.name = "Saved before revocation".into();
        if matches!(operation, Operation::Replay) {
            operation
                .run(&pool, owner.org_id, owner.user_id, &request)
                .await
                .unwrap();
        }
        let before = saved_state(&pool, original.id).await;
        let mut blocker = pool.begin().await.unwrap();
        // Both readers and writers pause on the project after taking their org gate.
        sqlx::query!("LOCK TABLE projects IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let blocker_pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap()
            .unwrap();
        let mut edits = JoinSet::new();
        let edit_pool = pool.clone();
        let pending_request = request.clone();
        edits.spawn(async move {
            operation
                .run(&edit_pool, owner.org_id, owner.user_id, &pending_request)
                .await
        });
        wait_for_blocked(&pool, blocker_pid).await;
        let editor_pid = sqlx::query_scalar!(
            "SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))",
            blocker_pid,
        ).fetch_one(&pool).await.unwrap().expect("blocked editor has a backend pid");
        let mut revocations = JoinSet::new();
        let revoke_pool = pool.clone();
        let command = ProjectManagersCommand {
            kind: ProjectManagersCommandKind::ReplaceProjectManagers,
            request_id: Uuid::now_v7(),
            expected_access_revision: request.managers.as_ref().unwrap().expected_access_revision,
            project_id: original.id,
            manager_ids: vec![],
        };
        revocations.spawn(async move {
            project_management::execute(&revoke_pool, owner.org_id, owner.user_id, &command).await
        });
        wait_for_blocked(&pool, editor_pid).await;
        blocker.commit().await.unwrap();
        edits.join_next().await.unwrap().unwrap().unwrap();
        assert!(
            revocations
                .join_next()
                .await
                .unwrap()
                .unwrap()
                .unwrap()
                .changed
        );
        let after = saved_state(&pool, original.id).await;
        if matches!(operation, Operation::Save) {
            assert_eq!(after[0]["name"], serde_json::json!(request.form.name));
            assert_eq!(
                after[0]["edit_revision"],
                serde_json::json!(request.expected_revision + 1)
            );
            assert_eq!(after[1], before[1]);
            assert_eq!(after[2], before[2]);
            assert_eq!(after[3].as_array().unwrap().len(), 1);
        } else {
            assert_eq!(after, before);
        }
        let fresh = operation
            .run(&pool, owner.org_id, owner.user_id, &request)
            .await;
        assert!(
            matches!(
                fresh,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "fresh {operation:?} after committed revocation: {fresh:?}"
        );
        assert_eq!(saved_state(&pool, original.id).await, after);
    }
}
