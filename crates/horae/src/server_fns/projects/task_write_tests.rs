use super::canonical_read_tests::fixture;
use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;
use uuid::Uuid;

async fn task_count(pool: &PgPool, org_id: Uuid) -> i64 {
    sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn initial_task_rate_requires_global_financial_and_task_authority(pool: PgPool) {
    for grants in [
        vec![Permission::TaskWriteAll],
        vec![Permission::BillableRateWriteAll],
        vec![Permission::TaskWriteAll, Permission::BillableRateReadAll],
        vec![
            Permission::TaskWriteAll,
            Permission::BillableRateWriteManaged,
        ],
        vec![
            Permission::TaskWriteAll,
            Permission::ReportProfitabilityRead,
        ],
    ] {
        let (ids, _) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&grants)).await;
        for rate in [
            TaskRateEdit::Clear {},
            TaskRateEdit::Set {
                amount_cents: 0,
                currency: "EUR".into(),
            },
        ] {
            let result = create_catalog_task(
                &pool,
                ids.org_id,
                ids.user_id,
                "Denied rate",
                true,
                false,
                &rate,
            )
            .await;
            assert!(
                matches!(
                    result,
                    Err(ServerFnError::ServerError {
                        code: FORBIDDEN,
                        ..
                    })
                ),
                "{grants:?}: {result:?}"
            );
            assert_eq!(task_count(&pool, ids.org_id).await, 1);
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn initial_task_rate_commits_with_service_payload(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]),
    )
    .await;
    for amount in [0, 12345] {
        let (task, event) = create_catalog_task(
            &pool,
            ids.org_id,
            ids.user_id,
            "With initial rate",
            true,
            false,
            &TaskRateEdit::Set {
                amount_cents: amount,
                currency: " eur ".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(task.default_rate_cents, Some(amount));
        assert_eq!(event.default_rate_cents, Some(amount));
        assert_eq!(event.id, task.id);
        let row = sqlx::query!(
            "SELECT default_rate_cents,default_rate_currency FROM tasks WHERE id=$1",
            task.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (row.default_rate_cents, row.default_rate_currency.as_deref()),
            (Some(amount), Some("EUR"))
        );
    }
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT COUNT(*) FROM project_tasks WHERE project_id=$1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0),
        "creating a catalog task links it to no project"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn initial_task_rate_rechecks_financial_authority_and_currency_after_wait(pool: PgPool) {
    for change_currency in [false, true] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]),
        )
        .await;
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
            create_catalog_task(
                &pending_pool,
                ids.org_id,
                ids.user_id,
                "Stale creation",
                true,
                false,
                &TaskRateEdit::Set {
                    amount_cents: 0,
                    currency: "EUR".into(),
                },
            )
            .await
        });
        wait_for_blocked(&pool, holder).await;
        if change_currency {
            sqlx::query!(
                "UPDATE organizations SET default_currency='USD' WHERE id=$1",
                ids.org_id
            )
            .execute(&mut *hold)
            .await
            .unwrap();
        } else {
            let grants: Vec<String> = serde_json::from_value(
                serde_json::to_value(PermissionSelection::new(&[Permission::TaskWriteAll]))
                    .unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &grants
            )
            .execute(&mut *hold)
            .await
            .unwrap();
        }
        hold.commit().await.unwrap();
        let result = pending.await.unwrap();
        let expected = if change_currency { CONFLICT } else { FORBIDDEN };
        assert!(
            matches!(result, Err(ServerFnError::ServerError {code, ..}) if code == expected),
            "{result:?}"
        );
        assert_eq!(task_count(&pool, ids.org_id).await, 1);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn project_edit_authority_does_not_create_global_tasks(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let result = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "Unauthorized catalog task",
        true,
        false,
        &TaskRateEdit::Preserve {},
    )
    .await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(task_count(&pool, ids.org_id).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn task_creation_fails_closed_for_unavailable_actor_permissions(pool: PgPool) {
    for invalid in ["inactive", "missing", "unknown_grant", "unknown_catalog"] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
        )
        .await;
        match invalid {
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "unknown_grant" => {
                sqlx::query!("UPDATE person_permission_states SET grants=array_append(grants,'unknown.grant') WHERE user_id=$1", ids.user_id)
                .execute(&pool).await.unwrap();
            }
            "unknown_catalog" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let result = create_catalog_task(
            &pool,
            ids.org_id,
            ids.user_id,
            "Unavailable actor",
            true,
            false,
            &TaskRateEdit::Preserve {},
        )
        .await;
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{invalid}: {result:?}"
        );
        assert_eq!(task_count(&pool, ids.org_id).await, 1, "{invalid}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn task_creation_rechecks_revoked_grants_after_organization_wait(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::ProjectWriteAll]),
    )
    .await;
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
        create_catalog_task(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            "Revoked task",
            true,
            false,
            &TaskRateEdit::Preserve {},
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::ProjectWriteAll])).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        ids.user_id,
        &grants
    )
    .execute(&mut *hold)
    .await
    .unwrap();
    hold.commit().await.unwrap();
    let result = pending.await.unwrap();
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(task_count(&pool, ids.org_id).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn global_task_creation_accepts_current_grant_without_legacy_manager(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let (task, _) = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "  Canonical task  ",
        true,
        false,
        &TaskRateEdit::Preserve {},
    )
    .await
    .unwrap();
    assert_eq!(task.name, "Canonical task");
    assert_eq!(task.org_id, ids.org_id);
    assert!(task.default_rate_cents.is_none());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn global_task_creation_does_not_inherit_legacy_administrator_authority(pool: PgPool) {
    let (ids, _) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    let result = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "Forbidden task",
        true,
        false,
        &TaskRateEdit::Preserve {},
    )
    .await;
    assert!(matches!(
        result,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1),
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn global_task_creation_rejects_unknown_policy_without_inserting(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 99 WHERE id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = create_catalog_task(
        &pool,
        ids.org_id,
        ids.user_id,
        "Unknown-policy task",
        true,
        false,
        &TaskRateEdit::Preserve {},
    )
    .await;
    assert!(result.is_err());
    assert_eq!(
        sqlx::query_scalar!("SELECT COUNT(*) FROM tasks WHERE org_id=$1", ids.org_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(1),
    );
}
