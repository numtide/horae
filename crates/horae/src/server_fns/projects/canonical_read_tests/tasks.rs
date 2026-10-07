use super::*;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_task_reader_requires_current_manager_designation(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for designated in [false, true, false] {
        if designated {
            designate(&pool, &ids).await;
        } else {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
                ids.org_id,
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let rows = tasks_for_viewer(&pool, &viewer, Some(ids.project_id), TaskRead::Catalog)
            .await
            .unwrap();
        assert_eq!(rows.len(), usize::from(designated));
        assert!(rows.iter().all(|row| row.default_rate_cents.is_none()));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_workflow_identity_rejects_foreign_client_parents(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    crate::server_fns::test_seed::time_entry(&pool, &ids, horae_core::types::EntryState::Open)
        .await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for valid_parent in [true, false, true] {
        sqlx::query!(
            "UPDATE projects SET client_id=$2 WHERE id=$1",
            ids.project_id,
            if valid_parent {
                ids.client_id
            } else {
                foreign.client_id
            }
        )
        .execute(&pool)
        .await
        .unwrap();
        for (project_id, purpose) in [
            (None, TaskRead::Tracking),
            (Some(ids.project_id), TaskRead::Catalog),
        ] {
            let rows = tasks_for_viewer(&pool, &viewer, project_id, purpose)
                .await
                .unwrap();
            assert_eq!(rows.len(), usize::from(valid_parent));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_catalog_accepts_current_read_grant_without_legacy_role(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskReadAll]),
    )
    .await;
    let rows = tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
        .await
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.task_id]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_reads_do_not_inherit_legacy_administrator_catalog(pool: PgPool) {
    let (_, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    for purpose in [TaskRead::Catalog, TaskRead::Tracking] {
        let rows = tasks_for_viewer(&pool, &viewer, None, purpose)
            .await
            .unwrap();
        assert!(
            rows.is_empty(),
            "legacy role disclosed unrelated tasks: {rows:?}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_catalog_withholds_global_rates_without_all_rate_read(pool: PgPool) {
    for rates in [None, Some(Permission::BillableRateReadManaged)] {
        let mut permissions = vec![Permission::TaskReadAll];
        permissions.extend(rates);
        let (ids, viewer) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&permissions),
        )
        .await;
        sqlx::query!(
            "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
            ids.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let rows = tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            serde_json::to_value(&rows[0])
                .unwrap()
                .get("default_rate_cents")
                .is_none()
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_catalog_all_rate_grant_is_independent_of_legacy_role(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].default_rate_cents, Some(999));
    let tracking = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
        .await
        .unwrap();
    assert!(tracking.iter().all(|row| row.default_rate_cents.is_none()));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_reader_rejects_unknown_policy_even_with_legacy_admin(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 99 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn tracking_history_is_not_task_catalog_authority(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    crate::server_fns::test_seed::time_entry(&pool, &ids, horae_core::types::EntryState::Open)
        .await;
    sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    let tracking = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
        .await
        .unwrap();
    assert_eq!(
        tracking.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.task_id]
    );
    assert!(tracking[0].default_rate_cents.is_none());
    assert!(
        tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_task_labels_require_project_scope_not_global_task_scope(pool: PgPool) {
    for grants in [
        vec![Permission::TaskReadAll],
        vec![Permission::ProjectReadAll],
    ] {
        let (ids, viewer) =
            fixture(&pool, OrgRole::Member, PermissionSelection::new(&grants)).await;
        sqlx::query!(
            "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
            ids.project_id,
            ids.task_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let rows = tasks_for_viewer(&pool, &viewer, Some(ids.project_id), TaskRead::Catalog)
            .await
            .unwrap();
        assert_eq!(
            rows.len(),
            usize::from(grants.contains(&Permission::ProjectReadAll))
        );
        assert!(rows.iter().all(|row| row.default_rate_cents.is_none()));
        let global = tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap();
        assert_eq!(
            global.len(),
            usize::from(grants.contains(&Permission::TaskReadAll))
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn assigned_tracking_task_survives_private_progress_without_catalog_grant(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Member, PermissionSelection::new(&[])).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
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
        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
        VALUES ($1,$2,$3,$4,'project','managers')",
        Uuid::now_v7(),
        ids.org_id,
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let tracking = tasks_for_viewer(&pool, &viewer, None, TaskRead::Tracking)
        .await
        .unwrap();
    assert_eq!(
        tracking.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.task_id]
    );
    assert!(
        tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
    let scoped = tasks_for_viewer(&pool, &viewer, Some(ids.project_id), TaskRead::Catalog)
        .await
        .unwrap();
    assert_eq!(
        scoped.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.task_id]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_catalog_uses_current_actor_and_never_foreign_tasks(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskReadAll]),
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let rows = tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
        .await
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.task_id]
    );
    assert!(
        tasks_for_viewer(&pool, &viewer, Some(foreign.project_id), TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        tasks_for_viewer(&pool, &viewer, None, TaskRead::Catalog)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_catalog_refreshes_permissions_after_organization_wait(pool: PgPool) {
    use crate::server_fns::test_seed::wait_for_blocked;
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 999 WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET access_revision=access_revision+1 WHERE id=$1",
        ids.org_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *writer)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        tasks_for_viewer(&pending_pool, &viewer, None, TaskRead::Catalog).await
    });
    wait_for_blocked(&pool, holder).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TaskReadAll])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let rows = pending.await.unwrap().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].default_rate_cents.is_none());
}
