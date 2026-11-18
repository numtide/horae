use super::*;
use crate::models::task::{TaskActivity, TaskCatalogPage, TaskCursor};

#[server]
pub async fn load_task_catalog(
    activity: TaskActivity,
    after: Option<TaskCursor>,
    expected_requester: Option<PermissionRequester>,
) -> Result<TaskCatalogPage, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let viewer = PermissionRequester {
        org_id: caller.org_id,
        user_id: caller.user_id,
    };
    fetch_task_catalog(
        &state.db,
        viewer,
        activity,
        after.as_ref(),
        expected_requester,
    )
    .await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_task_catalog(
    pool: &sqlx::PgPool,
    viewer: PermissionRequester,
    activity: TaskActivity,
    after: Option<&TaskCursor>,
    expected_requester: Option<PermissionRequester>,
) -> Result<TaskCatalogPage, ServerFnError> {
    let mut read = ops::begin_read(pool, Caller::session(viewer.org_id, viewer.user_id))
        .await
        .map_err(op_error)?;
    let result = ops::projects::task_catalog(
        &mut read.tx,
        &read.actor,
        activity,
        after,
        expected_requester,
    )
    .await;
    read.finish(result).await.map_err(op_error)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::super::canonical_read_tests::fixture;
    use super::*;
    use crate::testing::{seed, wait_for_blocked};
    use horae_core::permissions::catalog::{BuiltInProfile, PermissionSelection};
    use sqlx::PgPool;
    use uuid::Uuid;

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn catalog_pages_keep_duplicate_names_and_filter_activity_without_foreign_rows(
        pool: PgPool,
    ) {
        let (ids, _) = fixture(
            &pool,
            BuiltInProfile::Member,
            PermissionSelection::new(&[Permission::TaskReadAll]),
        )
        .await;
        let foreign = seed(&pool, BuiltInProfile::Administrator).await;
        let requester = PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        };
        sqlx::query!("UPDATE tasks SET active=false WHERE id=$1", ids.task_id)
            .execute(&pool)
            .await
            .unwrap();
        let mut expected = Vec::new();
        for _ in 0..103 {
            let id = Uuid::now_v7();
            sqlx::query!("INSERT INTO tasks (id,org_id,name,billable_default) VALUES ($1,$2,'Same name',true)", id, ids.org_id)
                .execute(&pool).await.unwrap();
            expected.push(id);
        }
        expected.sort();
        let first = fetch_task_catalog(&pool, requester, TaskActivity::Active, None, None)
            .await
            .unwrap();
        assert_eq!(first.tasks.len(), 100);
        let second = fetch_task_catalog(
            &pool,
            requester,
            TaskActivity::Active,
            first.next_after.as_ref(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(second.tasks.len(), 3);
        assert!(second.next_after.is_none());
        let actual: Vec<_> = first
            .tasks
            .iter()
            .chain(&second.tasks)
            .map(|task| task.id)
            .collect();
        assert_eq!(actual, expected);
        assert!(!actual.contains(&foreign.task_id));
        let archived = fetch_task_catalog(&pool, requester, TaskActivity::Archived, None, None)
            .await
            .unwrap();
        assert_eq!(
            archived
                .tasks
                .iter()
                .map(|task| task.id)
                .collect::<Vec<_>>(),
            [ids.task_id]
        );
        let all = fetch_task_catalog(&pool, requester, TaskActivity::All, None, None)
            .await
            .unwrap();
        let rest = fetch_task_catalog(
            &pool,
            requester,
            TaskActivity::All,
            all.next_after.as_ref(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(all.tasks.len() + rest.tasks.len(), 104);
        let invalid = TaskCursor {
            is_default: false,
            name: "bad\0cursor".into(),
            id: Uuid::now_v7(),
        };
        assert!(matches!(
            fetch_task_catalog(&pool, requester, TaskActivity::All, Some(&invalid), None).await,
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ));
    }

    /// The built-in profiles: Administrator edits tasks and their rates,
    /// Executive Manager edits tasks and reads rates, Member has no catalog.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn built_in_profiles_open_the_catalog_by_their_grants(pool: PgPool) {
        for (profile, allowed, edits_rates) in [
            (BuiltInProfile::Administrator, true, true),
            (BuiltInProfile::ExecutiveManager, true, false),
            (BuiltInProfile::Member, false, false),
        ] {
            let ids = seed(&pool, profile).await;
            let requester = PermissionRequester {
                org_id: ids.org_id,
                user_id: ids.user_id,
            };
            let page = fetch_task_catalog(&pool, requester, TaskActivity::All, None, None).await;
            if !allowed {
                assert!(
                    matches!(
                        page,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{profile:?}"
                );
                continue;
            }
            let page = page.unwrap();
            assert_eq!(
                page.tasks.iter().map(|task| task.id).collect::<Vec<_>>(),
                [ids.task_id],
                "{profile:?}"
            );
            assert!(page.can_edit && page.can_read_rates, "{profile:?}");
            assert_eq!(page.can_edit_rates, edits_rates, "{profile:?}");
            assert_eq!(page.rate_currency.is_some(), edits_rates, "{profile:?}");
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn catalog_observes_revocation_after_waiting_for_current_authority(pool: PgPool) {
        let (ids, _) = fixture(
            &pool,
            BuiltInProfile::Administrator,
            PermissionSelection::new(&[Permission::TaskReadAll]),
        )
        .await;
        let requester = PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id,
        };
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!(
            "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
            ids.org_id
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let pending_pool = pool.clone();
        let pending = tokio::spawn(async move {
            fetch_task_catalog(&pending_pool, requester, TaskActivity::All, None, None).await
        });
        wait_for_blocked(&pool, holder).await;
        let floor: Vec<String> =
            serde_json::from_value(serde_json::json!(PermissionSelection::new(&[]))).unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &floor
        )
        .execute(&mut *hold)
        .await
        .unwrap();
        hold.commit().await.unwrap();
        assert!(matches!(
            pending.await.unwrap(),
            Err(ServerFnError::ServerError {
                code: FORBIDDEN,
                ..
            })
        ));
    }
}
