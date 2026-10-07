use super::*;
use crate::reports::bounded::ExportPermit;
use crate::server_fns::test_seed::wait_for_blocked;
use horae_core::permissions::catalog::{Permission, PermissionSelection};
use std::time::Duration;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn denied_time_export_releases_authority_before_pool_cleanup(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let returning = std::sync::Arc::new(tokio::sync::Notify::new());
    let cleanup = std::sync::Arc::new(tokio::sync::Notify::new());
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_release({
            let returning = std::sync::Arc::clone(&returning);
            let cleanup = std::sync::Arc::clone(&cleanup);
            move |_, _| {
                let returning = std::sync::Arc::clone(&returning);
                let cleanup = std::sync::Arc::clone(&cleanup);
                Box::pin(async move {
                    // Hold deferred rollback flushing until the writer has checked its lock.
                    returning.notify_one();
                    cleanup.notified().await;
                    Ok(true)
                })
            }
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    assert!(matches!(
        capture(&reader_pool, &ids).await,
        Err(StatusCode::FORBIDDEN)
    ));
    tokio::time::timeout(Duration::from_secs(5), returning.notified())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    let lock = sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await;
    cleanup.notify_one();
    writer.rollback().await.unwrap();
    reader_pool.close().await;
    assert!(lock.is_ok(), "denied export retained authority: {lock:?}");
}

async fn capture(pool: &PgPool, ids: &SeedIds) -> Result<time::TimeExport, StatusCode> {
    time::entries(
        pool,
        ids.org_id,
        ids.user_id,
        &params().time_query().unwrap(),
    )
    .await
}

async fn render(pool: &PgPool, export: time::TimeExport) -> Result<Vec<u8>, StatusCode> {
    let body = export
        .scope
        .render(ExportPermit::acquire().unwrap(), pool, move || {
            crate::reports::entries_xlsx(&export.rows)
        })
        .await?;
    Ok(
        axum::body::to_bytes(body, crate::reports::bounded::MAX_OUTPUT_BYTES)
            .await
            .unwrap()
            .to_vec(),
    )
}

async fn person(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other person')",
        id,
        org,
        format!("{id}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn project(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other project','EUR')", id, ids.org_id, ids.client_id)
        .execute(pool).await.unwrap();
    id
}

async fn scoped(pool: &PgPool, role: OrgRole, grants: &[Permission]) -> SeedIds {
    let ids = seed(pool, role).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(grants)).unwrap())
            .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,$4,false,'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id, &grants,
    ).execute(pool).await.unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_member_reads_own_rows_without_financial_or_directory_access(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    add_entries(&pool, &ids, 1).await;
    let rows = entries(&pool, ids.org_id, ids.user_id, &params())
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let workbook = crate::reports::entries_xlsx(&rows).unwrap();
    assert_text_cell(&workbook, "B2", "Widget");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_legacy_manager_role_does_not_expand_canonical_scope(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other person')",
        other,
        ids.org_id,
        format!("{other}@test.com")
    )
    .execute(&pool)
    .await
    .unwrap();
    add_entries(
        &pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        1,
    )
    .await;
    assert!(
        entries(&pool, ids.org_id, ids.user_id, &params())
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_scope_union_and_multi_filters_precede_size_checks(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
    let target = person(&pool, ids.org_id).await;
    let outsider = person(&pool, ids.org_id).await;
    let other = project(&pool, &ids).await;
    for user_id in [ids.user_id, target, outsider] {
        for project_id in [ids.project_id, other] {
            let entry = add_entries(
                &pool,
                &SeedIds {
                    user_id,
                    project_id,
                    ..ids
                },
                1,
            )
            .await[0];
            if user_id == outsider && project_id == other {
                sqlx::query!(
                    "UPDATE time_entries SET notes=repeat('x',40000) WHERE id=$1",
                    entry
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }
    }
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
    assert_eq!(capture(&pool, &ids).await.unwrap().rows.len(), 5);
    let mut query = params().time_query().unwrap();
    query.project_ids = vec![other, other, Uuid::now_v7()];
    query.user_ids = vec![target, outsider];
    query.task_ids = vec![ids.task_id];
    query.client_ids = vec![ids.client_id];
    let selected = time::entries(&pool, ids.org_id, ids.user_id, &query)
        .await
        .unwrap();
    assert_eq!(selected.rows.len(), 1);
    assert_eq!(selected.rows[0].project_name, "Other project");
    render(&pool, selected).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_reauthorizes_captured_project_not_reassigned_source(pool: PgPool) {
    for retain_original_access in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let owner = person(&pool, ids.org_id).await;
        let other = project(&pool, &ids).await;
        let entry = add_entries(
            &pool,
            &SeedIds {
                user_id: owner,
                ..ids
            },
            1,
        )
        .await[0];
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        let export = capture(&pool, &ids).await.unwrap();
        assert_eq!(export.rows.len(), 1);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let pending = {
            let pool = pool.clone();
            tokio::spawn(async move {
                export
                    .scope
                    .render(ExportPermit::acquire().unwrap(), &pool, move || {
                        started.send(()).unwrap();
                        wait.recv_timeout(Duration::from_secs(5)).unwrap();
                        crate::reports::entries_xlsx(&export.rows)
                    })
                    .await
            })
        };
        ready.await.unwrap();
        let mut change = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *change)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE time_entries SET project_id=$2,notes='New project data' WHERE id=$1",
            entry,
            other
        )
        .execute(&mut *change)
        .await
        .unwrap();
        if !retain_original_access {
            sqlx::query!(
                "UPDATE project_management_assignments SET project_id=$2 WHERE manager_id=$1",
                ids.user_id,
                other
            )
            .execute(&mut *change)
            .await
            .unwrap();
        }
        change.commit().await.unwrap();
        release.send(()).unwrap();
        let result = pending.await.unwrap();
        if retain_original_access {
            let bytes =
                axum::body::to_bytes(result.unwrap(), crate::reports::bounded::MAX_OUTPUT_BYTES)
                    .await
                    .unwrap();
            assert_text_cell(&bytes, "B2", "Widget");
            assert!(!xlsx_part(&bytes, "xl/sharedStrings.xml").contains("New project data"));
        } else {
            assert!(matches!(result, Err(StatusCode::FORBIDDEN)));
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_deletion_keeps_recorded_scope_but_loss_of_person_access_denies(pool: PgPool) {
    for revoke in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let target = person(&pool, ids.org_id).await;
        let entry = add_entries(
            &pool,
            &SeedIds {
                user_id: target,
                ..ids
            },
            1,
        )
        .await[0];
        add_entries(&pool, &ids, 1).await;
        sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, target).execute(&pool).await.unwrap();
        let export = capture(&pool, &ids).await.unwrap();
        assert_eq!(export.rows.len(), 2);
        sqlx::query!("DELETE FROM time_entries WHERE id=$1", entry)
            .execute(&pool)
            .await
            .unwrap();
        if revoke {
            sqlx::query!(
                "DELETE FROM person_management_assignments WHERE manager_id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let result = render(&pool, export).await;
        if revoke {
            assert_eq!(result, Err(StatusCode::FORBIDDEN));
        } else {
            assert_eq!(
                xlsx_part(&result.unwrap(), "xl/worksheets/sheet1.xml")
                    .matches("<row ")
                    .count(),
                3
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_empty_files_still_validate_state_actor_and_policy_at_release(pool: PgPool) {
    for change in ["unsupported", "missing", "inactive", "policy", "grants"] {
        let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadAll]).await;
        let export = capture(&pool, &ids).await.unwrap();
        assert!(export.rows.is_empty());
        match change {
            "unsupported" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
                    ids.user_id
                )
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
            "inactive" => {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&pool)
                    .await
                    .unwrap();
            }
            "policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
                    ids.org_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "grants" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=ARRAY['unknown'] WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let expected = if matches!(change, "inactive" | "policy") {
            StatusCode::FORBIDDEN
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        assert_eq!(render(&pool, export).await, Err(expected), "{change}");
        let first = ExportPermit::acquire().unwrap();
        let second = ExportPermit::acquire().unwrap();
        drop((first, second));
    }
    let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadAll]).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let export = capture(&pool, &ids).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(render(&pool, export).await, Err(StatusCode::FORBIDDEN));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_waits_for_scope_revocation_and_cancellation_releases_connection(pool: PgPool) {
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    for cancel in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let owner = person(&pool, ids.org_id).await;
        add_entries(
            &pool,
            &SeedIds {
                user_id: owner,
                ..ids
            },
            1,
        )
        .await;
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        let export = capture(&reader_pool, &ids).await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!(r#"SELECT pg_backend_pid() AS "pid!""#)
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        sqlx::query!(
            "DELETE FROM project_management_assignments WHERE manager_id=$1",
            ids.user_id
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        let pending = {
            let pool = reader_pool.clone();
            tokio::spawn(async move { render(&pool, export).await })
        };
        wait_for_blocked(&pool, pid).await;
        if cancel {
            pending.abort();
            assert!(pending.await.unwrap_err().is_cancelled());
            writer.rollback().await.unwrap();
        } else {
            writer.commit().await.unwrap();
            assert_eq!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN));
        }
        tokio::time::timeout(Duration::from_secs(5), capture(&reader_pool, &ids))
            .await
            .unwrap()
            .unwrap();
    }
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_rejects_paged_reversed_or_wrong_requester_queries(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let mut query = params().time_query().unwrap();
    query.date_to = "2026-09-06".parse().unwrap();
    assert!(matches!(
        time::entries(&pool, ids.org_id, ids.user_id, &query).await,
        Err(StatusCode::BAD_REQUEST)
    ));
    let mut query = params().time_query().unwrap();
    query.after = Some(crate::models::time_report::TimeReportCursor {
        spent_date: query.date_from,
        project_name: String::new(),
        task_name: String::new(),
        id: Uuid::now_v7(),
    });
    assert!(matches!(
        time::entries(&pool, ids.org_id, ids.user_id, &query).await,
        Err(StatusCode::BAD_REQUEST)
    ));
    let mut query = params().time_query().unwrap();
    query.expected_requester = Some(crate::models::permission_editor::PermissionRequester {
        org_id: ids.org_id,
        user_id: Uuid::now_v7(),
    });
    assert!(matches!(
        time::entries(&pool, ids.org_id, ids.user_id, &query).await,
        Err(StatusCode::FORBIDDEN)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_source_refreshes_grant_gains_and_losses_after_org_wait(pool: PgPool) {
    for gain in [false, true] {
        let ids = scoped(
            &pool,
            OrgRole::Member,
            &[if gain {
                Permission::TimeReadOwn
            } else {
                Permission::TimeReadAll
            }],
        )
        .await;
        let owner = person(&pool, ids.org_id).await;
        add_entries(&pool, &ids, 1).await;
        add_entries(
            &pool,
            &SeedIds {
                user_id: owner,
                ..ids
            },
            1,
        )
        .await;
        let grants: Vec<String> = serde_json::from_value(
            serde_json::to_value(PermissionSelection::new(&[if gain {
                Permission::TimeReadAll
            } else {
                Permission::TimeReadOwn
            }]))
            .unwrap(),
        )
        .unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        let pid = sqlx::query_scalar!(r#"SELECT pg_backend_pid() AS "pid!""#)
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
            ids.user_id,
            &grants
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        let pending = {
            let pool = pool.clone();
            tokio::spawn(async move { capture(&pool, &ids).await })
        };
        wait_for_blocked(&pool, pid).await;
        writer.commit().await.unwrap();
        assert_eq!(
            pending.await.unwrap().unwrap().rows.len(),
            if gain { 2 } else { 1 }
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_release_observes_direct_actor_update_commit_or_rollback(pool: PgPool) {
    for commit in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
        add_entries(&pool, &ids, 1).await;
        let export = capture(&pool, &ids).await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *writer)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!(r#"SELECT pg_backend_pid() AS "pid!""#)
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            tokio::spawn(async move { render(&pool, export).await })
        };
        wait_for_blocked(&pool, pid).await;
        if commit {
            writer.commit().await.unwrap();
            assert_eq!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN));
        } else {
            writer.rollback().await.unwrap();
            pending.await.unwrap().unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_xlsx_revoked_grants_after_render_deny_other_peoples_captured_rows(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Admin, &[Permission::TimeReadAll]).await;
    let owner = person(&pool, ids.org_id).await;
    add_entries(
        &pool,
        &SeedIds {
            user_id: owner,
            ..ids
        },
        1,
    )
    .await;
    let export = capture(&pool, &ids).await.unwrap();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let pending = {
        let pool = pool.clone();
        tokio::spawn(async move {
            export
                .scope
                .render(ExportPermit::acquire().unwrap(), &pool, move || {
                    started.send(()).unwrap();
                    wait.recv_timeout(Duration::from_secs(5)).unwrap();
                    crate::reports::entries_xlsx(&export.rows)
                })
                .await
        })
    };
    ready.await.unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TimeReadOwn])).unwrap(),
    )
    .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
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
    release.send(()).unwrap();
    assert!(matches!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN)));
}
