use super::*;
use crate::models::task::{TaskActivity, TaskCatalogPage, TaskCursor};
#[cfg(feature = "server")]
use crate::models::task::{TaskCatalogEntry, opens_task_catalog};

#[server]
pub async fn load_task_catalog(
    activity: TaskActivity,
    after: Option<TaskCursor>,
    expected_requester: Option<PermissionRequester>,
) -> Result<TaskCatalogPage, ServerFnError> {
    let viewer = require_user().await?;
    let requester = project_requester(&viewer, expected_requester)?;
    let state = crate::state::global_state().await;
    fetch_task_catalog(&state.db, requester, activity, after.as_ref()).await
}

#[cfg(feature = "server")]
pub(super) async fn fetch_task_catalog(
    pool: &sqlx::PgPool,
    requester: PermissionRequester,
    activity: TaskActivity,
    after: Option<&TaskCursor>,
) -> Result<TaskCatalogPage, ServerFnError> {
    let mut access = read_access::ReadAccess::begin(pool, requester.org_id, requester.user_id)
        .await?
        .ok_or_else(|| forbidden("Task catalog access is unavailable"))?;
    let policy_zero_role = if access.legacy() {
        Some(
            sqlx::query_scalar!(
                r#"SELECT org_role as "org_role: OrgRole" FROM users WHERE org_id=$1 AND id=$2"#,
                requester.org_id,
                requester.user_id,
            )
            .fetch_one(&mut *access.tx)
            .await
            .map_err(server_err)?,
        )
    } else {
        None
    };
    if !opens_task_catalog(policy_zero_role, access.has(Permission::TaskReadAll)) {
        return Err(forbidden("Task catalog access is unavailable"));
    }
    if after.is_some_and(|cursor| cursor.name.contains('\0')) {
        return Err(err(BAD_REQUEST, "Invalid task catalog cursor"));
    }
    // A legacy reader is an admin or manager, who may already edit tasks and rates.
    let legacy = policy_zero_role.is_some();
    let can_edit = legacy || access.has(Permission::TaskWriteAll);
    let can_read_rates = legacy || access.has(Permission::BillableRateReadAll);
    let can_edit_rates = can_edit && (legacy || access.has(Permission::BillableRateWriteAll));
    let rate_currency = if can_edit_rates {
        Some(sqlx::query_scalar!(
            "SELECT upper(btrim(default_currency)) AS \"currency!\" FROM organizations WHERE id=$1",
            requester.org_id,
        ).fetch_one(&mut *access.tx).await.map_err(server_err)?)
    } else {
        None
    };
    let (active_only, archived_only) = (
        activity == TaskActivity::Active,
        activity == TaskActivity::Archived,
    );
    // Common tasks first, so the page splits into its two groups in order.
    let mut tasks = sqlx::query_as!(
        TaskCatalogEntry,
        "SELECT id,name,billable_default,is_default,active,
                CASE WHEN $2 THEN default_rate_cents END AS default_rate_cents,
                CASE WHEN $2 AND default_rate_cents IS NOT NULL
                     THEN default_rate_currency END AS default_rate_currency
         FROM tasks
         WHERE org_id=$1 AND (NOT $3 OR active) AND (NOT $4 OR NOT active)
           AND ($5::text IS NULL OR (NOT is_default,name,id) > (NOT $7::boolean,$5,$6::uuid))
         ORDER BY NOT is_default,name,id LIMIT 101",
        requester.org_id,
        can_read_rates,
        active_only,
        archived_only,
        after.map(|cursor| cursor.name.as_str()),
        after.map(|cursor| cursor.id),
        after.map(|cursor| cursor.is_default),
    )
    .fetch_all(&mut *access.tx)
    .await
    .map_err(server_err)?;
    let next_after = if tasks.len() > 100 {
        tasks.truncate(100);
        tasks.last().map(|task| TaskCursor {
            is_default: task.is_default,
            name: task.name.clone(),
            id: task.id,
        })
    } else {
        None
    };
    access.tx.commit().await.map_err(server_err)?;
    Ok(TaskCatalogPage {
        requester,
        can_edit,
        can_read_rates,
        can_edit_rates,
        rate_currency,
        tasks,
        next_after,
    })
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::super::canonical_read_tests::fixture;
    use super::*;
    use crate::server_fns::test_seed::{seed, wait_for_blocked};
    use horae_core::permissions::catalog::PermissionSelection;
    use sqlx::PgPool;
    use uuid::Uuid;

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn catalog_pages_keep_duplicate_names_and_filter_activity_without_foreign_rows(
        pool: PgPool,
    ) {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[Permission::TaskReadAll]),
        )
        .await;
        let foreign = seed(&pool, OrgRole::Admin).await;
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
        let first = fetch_task_catalog(&pool, requester, TaskActivity::Active, None)
            .await
            .unwrap();
        assert_eq!(first.tasks.len(), 100);
        let second = fetch_task_catalog(
            &pool,
            requester,
            TaskActivity::Active,
            first.next_after.as_ref(),
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
        let archived = fetch_task_catalog(&pool, requester, TaskActivity::Archived, None)
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
        let all = fetch_task_catalog(&pool, requester, TaskActivity::All, None)
            .await
            .unwrap();
        let rest = fetch_task_catalog(&pool, requester, TaskActivity::All, all.next_after.as_ref())
            .await
            .unwrap();
        assert_eq!(all.tasks.len() + rest.tasks.len(), 104);
        let invalid = TaskCursor {
            is_default: false,
            name: "bad\0cursor".into(),
            id: Uuid::now_v7(),
        };
        assert!(matches!(
            fetch_task_catalog(&pool, requester, TaskActivity::All, Some(&invalid)).await,
            Err(ServerFnError::ServerError {
                code: BAD_REQUEST,
                ..
            })
        ));
    }

    /// Policy zero opens the catalog to the legacy roles that may already edit
    /// it, with rates, and keeps it closed to members.
    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn policy_zero_opens_the_catalog_to_admins_and_managers_only(pool: PgPool) {
        for (role, allowed) in [
            (OrgRole::Admin, true),
            (OrgRole::Manager, true),
            (OrgRole::Member, false),
        ] {
            let ids = seed(&pool, role).await;
            let requester = PermissionRequester {
                org_id: ids.org_id,
                user_id: ids.user_id,
            };
            let page = fetch_task_catalog(&pool, requester, TaskActivity::All, None).await;
            if !allowed {
                assert!(
                    matches!(
                        page,
                        Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            ..
                        })
                    ),
                    "{role:?}"
                );
                continue;
            }
            let page = page.unwrap();
            assert_eq!(
                page.tasks.iter().map(|task| task.id).collect::<Vec<_>>(),
                [ids.task_id],
                "{role:?}"
            );
            assert!(
                page.can_edit && page.can_read_rates && page.can_edit_rates,
                "{role:?}"
            );
            assert!(page.rate_currency.is_some(), "{role:?}");
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn catalog_observes_revocation_after_waiting_for_current_authority(pool: PgPool) {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
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
            fetch_task_catalog(&pending_pool, requester, TaskActivity::All, None).await
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
