use super::canonical_read_tests::fixture;
use super::*;
use crate::server_fns::test_seed::{seed, time_entry, wait_for_blocked};
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
#[serial_test::serial]
async fn link_requires_matching_default_currency_even_without_project_settings(pool: PgPool) {
    for mode in [None, Some("task"), Some("person")] {
        for currency in [None, Some("USD"), Some("EUR")] {
            for amount in [0_i64, 8000] {
                let (ids, _) = fixture(
                    &pool,
                    OrgRole::Member,
                    PermissionSelection::new(&[
                        Permission::ProjectWriteAll,
                        Permission::BillableRateWriteAll,
                    ]),
                )
                .await;
                sqlx::query!(
                    "UPDATE tasks SET default_rate_cents=$2,default_rate_currency=$3 WHERE id=$1",
                    ids.task_id,
                    amount,
                    currency
                )
                .execute(&pool)
                .await
                .unwrap();
                if let Some(mode) = mode {
                    sqlx::query!(
                        "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,$5)",
                        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode,
                    ).execute(&pool).await.unwrap();
                }
                let result = link_project_task_record(
                    &pool,
                    ids.org_id,
                    ids.user_id,
                    ids.project_id,
                    ids.task_id,
                    None,
                )
                .await;
                let expected_rate = if mode != Some("person") && currency != Some("EUR") {
                    assert!(
                        matches!(
                            result,
                            Err(ServerFnError::ServerError { code: CONFLICT, .. })
                        ),
                        "mode={mode:?}, currency={currency:?}, amount={amount}: {result:?}"
                    );
                    assert_eq!(
                        sqlx::query_scalar!(
                            "SELECT COUNT(*) FROM project_tasks WHERE project_id=$1",
                            ids.project_id
                        )
                        .fetch_one(&pool)
                        .await
                        .unwrap(),
                        Some(0)
                    );
                    link_project_task_record(
                        &pool,
                        ids.org_id,
                        ids.user_id,
                        ids.project_id,
                        ids.task_id,
                        Some(&ProjectTaskRate {
                            amount: "12.34".into(),
                            currency: "EUR".into(),
                        }),
                    )
                    .await
                    .unwrap();
                    Some(1234)
                } else {
                    result.unwrap();
                    (mode != Some("person")).then_some(amount)
                };
                // Re-linking preserves the stored override even if the catalog
                // denomination is unknown or incompatible with this project.
                link_project_task_record(
                    &pool,
                    ids.org_id,
                    ids.user_id,
                    ids.project_id,
                    ids.task_id,
                    None,
                )
                .await
                .unwrap();
                assert_eq!(
                    sqlx::query_scalar!(
                        "SELECT rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
                        ids.project_id,
                        ids.task_id,
                    )
                    .fetch_one(&pool)
                    .await
                    .unwrap(),
                    expected_rate
                );
            }
        }
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn link_rechecks_project_rate_and_designation_after_organization_wait(pool: PgPool) {
    for revoked in ["project", "rate", "designation", "rate_designation"] {
        let managed = revoked == "designation";
        let managed_rate = revoked == "rate_designation";
        let (ids, _) = fixture(
            &pool,
            OrgRole::Member,
            PermissionSelection::new(&[
                if managed {
                    Permission::ProjectWriteManaged
                } else {
                    Permission::ProjectWriteAll
                },
                if managed_rate {
                    Permission::BillableRateWriteManaged
                } else {
                    Permission::BillableRateWriteAll
                },
            ]),
        )
        .await;
        if managed || managed_rate {
            sqlx::query!("INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id)
                .execute(&pool).await.unwrap();
        }
        let mut hold = pool.begin().await.unwrap();
        lock_organization(&mut hold, ids.org_id, OrganizationLock::AccessChange)
            .await
            .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *hold)
            .await
            .unwrap()
            .unwrap();
        let pending_pool = pool.clone();
        let pending = tokio::spawn(async move {
            link_project_task_record(
                &pending_pool,
                ids.org_id,
                ids.user_id,
                ids.project_id,
                ids.task_id,
                Some(&ProjectTaskRate {
                    amount: "0".into(),
                    currency: "EUR".into(),
                }),
            )
            .await
        });
        wait_for_blocked(&pool, holder).await;
        if managed || managed_rate {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
                ids.org_id,
                ids.user_id
            )
            .execute(&mut *hold)
            .await
            .unwrap();
        } else {
            let grants = PermissionSelection::new(&[if revoked == "rate" {
                Permission::ProjectWriteAll
            } else {
                Permission::BillableRateWriteAll
            }]);
            let stored: Vec<String> =
                serde_json::from_value(serde_json::to_value(grants).unwrap()).unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &stored
            )
            .execute(&mut *hold)
            .await
            .unwrap();
        }
        hold.commit().await.unwrap();
        let result = pending.await.unwrap();
        assert!(
            matches!(
                result,
                Err(ServerFnError::ServerError {
                    code: FORBIDDEN,
                    ..
                })
            ),
            "{revoked}: {result:?}"
        );
        assert_eq!(
            sqlx::query_scalar!(
                "SELECT COUNT(*) FROM project_tasks WHERE project_id=$1",
                ids.project_id
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            Some(0)
        );
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn link_holds_authority_while_waiting_for_task(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let mut task = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM tasks WHERE id=$1 FOR UPDATE", ids.task_id)
        .fetch_one(&mut *task)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *task)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        link_project_task_record(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            ids.project_id,
            ids.task_id,
            None,
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    let mut revocation = pool.begin().await.unwrap();
    let error = sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *revocation)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    revocation.rollback().await.unwrap();
    task.rollback().await.unwrap();
    pending.await.unwrap().unwrap();
}

#[sqlx::test]
#[serial_test::serial]
async fn link_noops_reject_unavailable_authority_without_changing_storage(pool: PgPool) {
    for case in [
        "missing",
        "malformed",
        "unknown_policy",
        "inactive",
        "no_grant",
    ] {
        let (ids, _) = fixture(
            &pool,
            OrgRole::Admin,
            PermissionSelection::new(&[Permission::ProjectWriteAll]),
        )
        .await;
        sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,true,2500)", ids.project_id, ids.task_id)
            .execute(&pool).await.unwrap();
        match case {
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "malformed" => {
                sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown.permission'] WHERE user_id=$1", ids.user_id).execute(&pool).await.unwrap();
            }
            "unknown_policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version=2 WHERE id=$1",
                    ids.org_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "no_grant" => {
                let floor: Vec<String> = serde_json::from_value(
                    serde_json::to_value(PermissionSelection::new(&[])).unwrap(),
                )
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                    ids.user_id,
                    &floor
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let before = sqlx::query_scalar!(
            "SELECT xmin::text FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            ids.project_id,
            ids.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let result = link_project_task_record(
            &pool,
            ids.org_id,
            ids.user_id,
            ids.project_id,
            ids.task_id,
            None,
        )
        .await;
        assert!(result.is_err(), "{case}: {result:?}");
        let after = sqlx::query_scalar!(
            "SELECT xmin::text FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            ids.project_id,
            ids.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(before, after);
    }
}

#[sqlx::test]
#[serial_test::serial]
async fn link_does_not_restore_archived_associations_or_touch_foreign_records(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectWriteAll]),
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents,active) VALUES ($1,$2,true,2500,false)", ids.project_id, ids.task_id)
        .execute(&pool).await.unwrap();
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    for (project, task, code) in [
        (ids.project_id, ids.task_id, CONFLICT),
        (foreign.project_id, ids.task_id, NOT_FOUND),
        (ids.project_id, foreign.task_id, NOT_FOUND),
    ] {
        let result =
            link_project_task_record(&pool, ids.org_id, ids.user_id, project, task, None).await;
        assert!(
            matches!(result, Err(ServerFnError::ServerError { code: actual, .. }) if actual == code),
            "{result:?}"
        );
    }
    let saved = sqlx::query!(
        "SELECT active,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
        ids.project_id,
        ids.task_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((saved.active, saved.rate_cents), (false, Some(2500)));
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", entry)
            .fetch_one(&pool)
            .await
            .unwrap(),
        60
    );
}
