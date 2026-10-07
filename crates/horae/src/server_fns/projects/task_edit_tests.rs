use super::canonical_read_tests::fixture;
use super::*;
use crate::server_fns::test_seed::{seed, time_entry, wait_for_blocked};
use horae_core::permissions::catalog::PermissionSelection;
use sqlx::PgPool;

fn set_rate(amount_cents: i64) -> TaskRateEdit {
    TaskRateEdit::Set {
        amount_cents,
        currency: "EUR".into(),
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_name_editor_preserves_unknown_currency_and_hides_rate_even_on_noop(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents=8000,default_rate_currency=NULL WHERE id=$1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for changed in [true, false] {
        let (task, event) = update_task_record(
            &pool,
            ids.org_id,
            ids.user_id,
            ids.task_id,
            "  Renamed  ",
            true,
            &TaskRateEdit::Preserve {},
        )
        .await
        .unwrap();
        assert_eq!(task.name, "Renamed");
        assert_eq!(task.default_rate_cents, None);
        assert_eq!(event.is_some(), changed);
        if let Some(event) = event {
            assert_eq!(event.default_rate_cents, Some(8000));
        }
        let saved = sqlx::query!(
            "SELECT default_rate_cents,default_rate_currency FROM tasks WHERE id=$1",
            ids.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            (saved.default_rate_cents, saved.default_rate_currency),
            (Some(8000), None)
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_rate_edit_does_not_rewrite_project_overrides_or_time_history(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]),
    )
    .await;
    sqlx::query!("INSERT INTO project_tasks (project_id,task_id,billable,rate_cents) VALUES ($1,$2,true,1200)",
        ids.project_id, ids.task_id).execute(&pool).await.unwrap();
    let entry_id = time_entry(&pool, &ids, EntryState::Open).await;
    for (rate, expected) in [
        (set_rate(0), Some(0)),
        (set_rate(2500), Some(2500)),
        (TaskRateEdit::Preserve {}, Some(2500)),
        (TaskRateEdit::Clear {}, None),
    ] {
        let (task, _) = update_task_record(
            &pool,
            ids.org_id,
            ids.user_id,
            ids.task_id,
            "Catalog change",
            false,
            &rate,
        )
        .await
        .unwrap();
        assert_eq!(task.default_rate_cents, expected);
        let link = sqlx::query!(
            "SELECT billable,rate_cents FROM project_tasks WHERE project_id=$1 AND task_id=$2",
            ids.project_id,
            ids.task_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((link.billable, link.rate_cents), (true, Some(1200)));
        let entry = sqlx::query!(
            "SELECT minutes,billable FROM time_entries WHERE id=$1",
            entry_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!((entry.minutes, entry.billable), (60, true));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn explicit_task_rate_noops_still_require_global_write_authority(pool: PgPool) {
    for extra in [
        None,
        Some(Permission::BillableRateReadAll),
        Some(Permission::BillableRateWriteManaged),
        Some(Permission::ReportProfitabilityRead),
    ] {
        let mut grants = PermissionSelection::new(&[Permission::TaskWriteAll]);
        if let Some(extra) = extra {
            grants.add(extra);
        }
        let (ids, _) = fixture(&pool, OrgRole::Admin, grants).await;
        for (stored, edit) in [
            (None, TaskRateEdit::Clear {}),
            (Some(0), set_rate(0)),
            (Some(8000), set_rate(8000)),
        ] {
            sqlx::query!(
                "UPDATE tasks SET default_rate_cents=$2 WHERE id=$1",
                ids.task_id,
                stored
            )
            .execute(&pool)
            .await
            .unwrap();
            let result = update_task_record(
                &pool,
                ids.org_id,
                ids.user_id,
                ids.task_id,
                "Denied",
                false,
                &edit,
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
                "{extra:?}: {result:?}"
            );
            let after = sqlx::query!(
                "SELECT name,default_rate_cents FROM tasks WHERE id=$1",
                ids.task_id
            )
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(
                (after.name.as_str(), after.default_rate_cents),
                ("Dev", stored)
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_edit_validates_currency_amount_name_and_foreign_targets_atomically(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]),
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for (id, name, rate, code) in [
        (ids.task_id, "Negative", set_rate(-1), BAD_REQUEST),
        (ids.task_id, " ", TaskRateEdit::Preserve {}, CONFLICT),
        (
            ids.task_id,
            "Wrong currency",
            TaskRateEdit::Set {
                amount_cents: 8000,
                currency: "USD".into(),
            },
            CONFLICT,
        ),
        (
            foreign.task_id,
            "Foreign",
            TaskRateEdit::Preserve {},
            NOT_FOUND,
        ),
    ] {
        let error = update_task_record(&pool, ids.org_id, ids.user_id, id, name, true, &rate)
            .await
            .unwrap_err();
        assert!(
            matches!(error, ServerFnError::ServerError {code: actual, ..} if actual == code),
            "{error:?}"
        );
    }
    for id in [ids.task_id, foreign.task_id] {
        let row = sqlx::query!("SELECT name,default_rate_cents FROM tasks WHERE id=$1", id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!((row.name.as_str(), row.default_rate_cents), ("Dev", None));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_edit_observes_task_and_rate_revocation_after_organization_wait(pool: PgPool) {
    for revoke in [Permission::TaskWriteAll, Permission::BillableRateWriteAll] {
        let mut grants =
            PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]);
        let (ids, _) = fixture(&pool, OrgRole::Admin, grants.clone()).await;
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
            update_task_record(
                &pending_pool,
                ids.org_id,
                ids.user_id,
                ids.task_id,
                "Revoked",
                true,
                &TaskRateEdit::Clear {},
            )
            .await
        });
        wait_for_blocked(&pool, holder).await;
        grants.remove(revoke).unwrap();
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
            "{revoke:?}: {result:?}"
        );
        assert_eq!(
            sqlx::query_scalar!("SELECT name FROM tasks WHERE id=$1", ids.task_id)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "Dev"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_rate_edit_refuses_stale_currency_after_organization_wait(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll, Permission::BillableRateWriteAll]),
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET default_currency='USD' WHERE id=$1",
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
        update_task_record(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            ids.task_id,
            "Stale currency",
            true,
            &set_rate(8000),
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    hold.commit().await.unwrap();
    let result = pending.await.unwrap();
    assert!(
        matches!(
            result,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ),
        "{result:?}"
    );
    let row = sqlx::query!(
        "SELECT name,default_rate_cents,default_rate_currency FROM tasks WHERE id=$1",
        ids.task_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            row.name.as_str(),
            row.default_rate_cents,
            row.default_rate_currency
        ),
        ("Dev", None, None)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_edit_holds_authority_while_waiting_for_task_and_commit(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("SELECT id FROM tasks WHERE id=$1 FOR UPDATE", ids.task_id)
        .fetch_one(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *hold)
        .await
        .unwrap()
        .unwrap();
    let pending_pool = pool.clone();
    let pending = tokio::spawn(async move {
        update_task_record(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            ids.task_id,
            "Authorized edit",
            true,
            &TaskRateEdit::Preserve {},
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
    let mut revoke = pool.begin().await.unwrap();
    let error = sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *revoke)
    .await
    .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    revoke.rollback().await.unwrap();
    hold.commit().await.unwrap();
    let (task, event) = pending.await.unwrap().unwrap();
    assert_eq!(task.name, "Authorized edit");
    assert!(event.is_some());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn task_edit_rejects_actor_deactivated_after_waiting_for_actor_lock(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::TaskWriteAll]),
    )
    .await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
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
        update_task_record(
            &pending_pool,
            ids.org_id,
            ids.user_id,
            ids.task_id,
            "Inactive editor",
            true,
            &TaskRateEdit::Preserve {},
        )
        .await
    });
    wait_for_blocked(&pool, holder).await;
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
        "{result:?}"
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT name FROM tasks WHERE id=$1", ids.task_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "Dev"
    );
}
