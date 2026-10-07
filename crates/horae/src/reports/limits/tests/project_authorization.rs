use super::*;
use crate::server_fns::test_seed::wait_for_blocked;
use std::time::Duration;

async fn assign(connection: &mut PgConnection, ids: &SeedIds) {
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(connection)
    .await
    .unwrap();
}

async fn visibility(connection: &mut PgConnection, ids: &SeedIds, value: &str) {
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
        VALUES ($1,$2,$3,$4,'person',$5) ON CONFLICT (project_id) DO UPDATE SET report_visibility=EXCLUDED.report_visibility",
        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, value)
        .execute(connection).await.unwrap();
}

async fn names(pool: &PgPool, org: Uuid, actor: Uuid) -> Result<Vec<String>, StatusCode> {
    Ok(projects(pool, org, actor, "active")
        .await?
        .into_iter()
        .map(|row| row.name)
        .collect())
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_require_current_tenant_bound_actor(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    assert!(
        projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .unwrap()
            .is_empty()
    );
    for (org, actor) in [
        (ids.org_id, Uuid::now_v7()),
        (ids.org_id, foreign.user_id),
        (Uuid::now_v7(), ids.user_id),
    ] {
        assert!(matches!(
            projects(&pool, org, actor, "active").await,
            Err(StatusCode::FORBIDDEN)
        ));
    }
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        projects(&pool, ids.org_id, ids.user_id, "active").await,
        Err(StatusCode::FORBIDDEN)
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_preserve_membership_visibility_and_history_rules(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    add_entries(&pool, &ids, 1).await;
    assert!(
        names(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    assert_eq!(
        names(&pool, ids.org_id, ids.user_id).await.unwrap(),
        ["Widget"]
    );
    visibility(&mut pool.acquire().await.unwrap(), &ids, "managers").await;
    assert!(
        names(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    for role in [
        horae_core::types::ProjectRole::Lead,
        horae_core::types::ProjectRole::Admin,
    ] {
        sqlx::query!(
            "UPDATE assignments SET role=$1 WHERE project_id=$2 AND user_id=$3",
            role as horae_core::types::ProjectRole,
            ids.project_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            names(&pool, ids.org_id, ids.user_id).await.unwrap(),
            ["Widget"]
        );
    }
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for role in [OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role=$1 WHERE id=$2",
            role as OrgRole,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            names(&pool, ids.org_id, ids.user_id).await.unwrap(),
            ["Widget"]
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_refresh_scope_after_winning_relationship_changes(pool: PgPool) {
    for change in [
        "add",
        "remove",
        "expand",
        "restrict",
        "insert_settings",
        "new_project",
    ] {
        let ids = seed(&pool, OrgRole::Member).await;
        if !matches!(change, "add" | "new_project") {
            assign(&mut pool.acquire().await.unwrap(), &ids).await;
        }
        if change == "expand" {
            visibility(&mut pool.acquire().await.unwrap(), &ids, "managers").await;
        }
        if change == "restrict" {
            visibility(&mut pool.acquire().await.unwrap(), &ids, "project_members").await;
        }
        let mut writer = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut writer,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            let (org, actor) = (ids.org_id, ids.user_id);
            tokio::spawn(async move { names(&pool, org, actor).await })
        };
        wait_for_blocked(&pool, pid).await;
        match change {
            "add" => assign(&mut writer, &ids).await,
            "remove" => {
                sqlx::query!(
                    "DELETE FROM assignments WHERE project_id=$1",
                    ids.project_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "expand" => visibility(&mut writer, &ids, "project_members").await,
            "restrict" | "insert_settings" => visibility(&mut writer, &ids, "managers").await,
            "new_project" => {
                let project = Uuid::now_v7();
                sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'New project','EUR')",
                    project, ids.org_id, ids.client_id).execute(&mut *writer).await.unwrap();
                sqlx::query!("INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
                    Uuid::now_v7(), project, ids.user_id).execute(&mut *writer).await.unwrap();
            }
            _ => unreachable!(),
        }
        writer.commit().await.unwrap();
        let result = pending.await.unwrap().unwrap();
        match change {
            "add" | "expand" => assert_eq!(result, ["Widget"], "{change}"),
            "new_project" => assert_eq!(result, ["New project"]),
            _ => assert!(result.is_empty(), "{change}: {result:?}"),
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_deny_actor_revoked_during_either_authority_wait(pool: PgPool) {
    for gate in [false, true] {
        let ids = seed(&pool, OrgRole::Member).await;
        assign(&mut pool.acquire().await.unwrap(), &ids).await;
        let mut writer = pool.begin().await.unwrap();
        if gate {
            crate::db::lock_organization(
                &mut writer,
                ids.org_id,
                crate::db::OrganizationLock::AccessChange,
            )
            .await
            .unwrap();
        }
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *writer)
            .await
            .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            tokio::spawn(async move { names(&pool, ids.org_id, ids.user_id).await })
        };
        wait_for_blocked(&pool, pid).await;
        writer.commit().await.unwrap();
        assert_eq!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_refresh_release_scope_after_parent_wait(pool: PgPool) {
    for change in ["remove", "insert_settings", "downgrade"] {
        let ids = seed(&pool, OrgRole::Member).await;
        assign(&mut pool.acquire().await.unwrap(), &ids).await;
        if change == "downgrade" {
            visibility(&mut pool.acquire().await.unwrap(), &ids, "managers").await;
            sqlx::query!(
                "UPDATE assignments SET role='lead' WHERE project_id=$1",
                ids.project_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        assert_eq!(
            names(&pool, ids.org_id, ids.user_id).await.unwrap(),
            ["Widget"]
        );
        let mut writer = pool.begin().await.unwrap();
        match change {
            "remove" => {
                sqlx::query!(
                    "DELETE FROM assignments WHERE project_id=$1",
                    ids.project_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            "insert_settings" => visibility(&mut writer, &ids, "managers").await,
            "downgrade" => {
                sqlx::query!(
                    "UPDATE assignments SET role='freelancer' WHERE project_id=$1",
                    ids.project_id
                )
                .execute(&mut *writer)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *writer)
            .await
            .unwrap();
        let pending = {
            let pool = pool.clone();
            tokio::spawn(async move {
                authorize_projects(&pool, ids.org_id, ids.user_id, &[ids.project_id]).await
            })
        };
        wait_for_blocked(&pool, pid).await;
        writer.commit().await.unwrap();
        assert_eq!(
            pending.await.unwrap(),
            Err(StatusCode::FORBIDDEN),
            "{change}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_release_every_captured_project_without_render_locks(pool: PgPool) {
    use crate::reports::{bounded::ExportPermit, render_project_export};
    for change in ["remove", "deactivate", "demote_retained"] {
        let ids = seed(&pool, OrgRole::Manager).await;
        assign(&mut pool.acquire().await.unwrap(), &ids).await;
        let second = Uuid::now_v7();
        sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Second','EUR')",
            second, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
            Uuid::now_v7(),
            second,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let pending = {
            let pool = pool.clone();
            let (org, actor) = (ids.org_id, ids.user_id);
            tokio::spawn(async move {
                let rows = projects(&pool, org, actor, "active").await.unwrap();
                assert_eq!(rows.len(), 2);
                render_project_export(
                    ExportPermit::acquire().unwrap(),
                    &pool,
                    org,
                    actor,
                    rows.iter().map(|row| row.id).collect(),
                    move || {
                        started.send(()).unwrap();
                        wait.recv_timeout(Duration::from_secs(5)).unwrap();
                        Ok(b"project export".to_vec())
                    },
                )
                .await
            })
        };
        ready.await.unwrap();
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
            ids.org_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
            ids.user_id
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "SELECT id FROM projects WHERE id=$1 FOR UPDATE NOWAIT",
            second
        )
        .fetch_one(&mut *writer)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE users SET org_role='member', active=$2 WHERE id=$1",
            ids.user_id,
            change != "deactivate"
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        if change == "remove" {
            sqlx::query!("DELETE FROM assignments WHERE project_id=$1", second)
                .execute(&mut *writer)
                .await
                .unwrap();
        }
        writer.commit().await.unwrap();
        release.send(()).unwrap();
        let result = pending.await.unwrap();
        if change == "demote_retained" {
            let body = result.unwrap();
            let mut writer = pool.begin().await.unwrap();
            sqlx::query!(
                "SELECT id FROM projects WHERE id=$1 FOR UPDATE NOWAIT",
                second
            )
            .fetch_one(&mut *writer)
            .await
            .unwrap();
            writer.rollback().await.unwrap();
            assert_eq!(
                axum::body::to_bytes(body, 100).await.unwrap().as_ref(),
                b"project export"
            );
        } else {
            assert!(matches!(result, Err(StatusCode::FORBIDDEN)), "{change}");
        }
        let first = ExportPermit::acquire().unwrap();
        let second = ExportPermit::acquire().unwrap();
        drop((first, second));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_bound_rows_and_text_after_scope_filtering(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let keys: Vec<_> = (1..XLSX.rows).map(|_| Uuid::now_v7()).collect();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency)
        SELECT id,$2,$3,'Additional','EUR' FROM unnest($1::uuid[]) id",
        &keys,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .unwrap()
            .len(),
        XLSX.rows as usize
    );
    let overflow = Uuid::now_v7();
    sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Overflow','EUR')",
        overflow, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
    assert!(matches!(
        projects(&pool, ids.org_id, ids.user_id, "active").await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    sqlx::query!(
        "UPDATE projects SET name=repeat('x',40000) WHERE org_id=$1 AND id<>$2",
        ids.org_id,
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        names(&pool, ids.org_id, ids.user_id).await.unwrap(),
        ["Widget"]
    );
    sqlx::query!(
        "UPDATE projects SET name=repeat('界',10923) WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        projects(&pool, ids.org_id, ids.user_id, "active").await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    sqlx::query!(
        "UPDATE projects SET name=repeat('界',10922) WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .unwrap()[0]
            .name
            .len(),
        32766
    );

    let aggregate = seed(&pool, OrgRole::Manager).await;
    let keys: Vec<_> = (0..300).map(|_| Uuid::now_v7()).collect();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency)
        SELECT id,$2,$3,repeat('n',32767),'EUR' FROM unnest($1::uuid[]) id",
        &keys,
        aggregate.org_id,
        aggregate.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        projects(&pool, aggregate.org_id, aggregate.user_id, "active").await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
}

async fn paused_project_pool(pool: &PgPool) -> PgPool {
    sqlx::query!("CREATE SCHEMA project_export_test")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query!(
        "CREATE FUNCTION project_export_test.pause() RETURNS boolean LANGUAGE plpgsql AS $$
        BEGIN PERFORM pg_advisory_xact_lock(715015); RETURN true; END $$"
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "CREATE VIEW project_export_test.project_read_access AS
        SELECT * FROM public.project_read_access WHERE project_export_test.pause()"
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET search_path=project_export_test,public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_share_one_size_and_payload_snapshot(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    let reader_pool = paused_project_pool(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT pg_advisory_xact_lock(715015) AS \"lock!: ()\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pending = {
        let reader_pool = reader_pool.clone();
        tokio::spawn(async move { projects(&reader_pool, ids.org_id, ids.user_id, "active").await })
    };
    wait_for_blocked(&pool, pid).await;
    sqlx::query!("UPDATE projects SET name=repeat('x',40000),budget_kind='hours',budget_minutes=300 WHERE id=$1", ids.project_id)
        .execute(&pool).await.unwrap();
    blocker.rollback().await.unwrap();
    let rows = pending.await.unwrap().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Widget");
    assert_eq!(rows[0].budget_minutes, None);
    assert!(matches!(
        projects(&reader_pool, ids.org_id, ids.user_id, "active").await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_release_cancelled_and_timed_out_checks(pool: PgPool) {
    use crate::reports::{bounded::ExportPermit, render_project_export};
    let ids = seed(&pool, OrgRole::Member).await;
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    let reader_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!("SET default_transaction_read_only=on")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET default_transaction_isolation='serializable'")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query!("SET lock_timeout='250ms'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    assert_eq!(
        names(&reader_pool, ids.org_id, ids.user_id).await.unwrap(),
        ["Widget"]
    );
    for cancel in [false, true] {
        let mut blocker = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut blocker,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
            .fetch_one(&mut *blocker)
            .await
            .unwrap();
        let pending = {
            let reader_pool = reader_pool.clone();
            let (org, actor, project) = (ids.org_id, ids.user_id, ids.project_id);
            tokio::spawn(async move {
                render_project_export(
                    ExportPermit::acquire().unwrap(),
                    &reader_pool,
                    org,
                    actor,
                    vec![project],
                    || Ok(b"rendered".to_vec()),
                )
                .await
            })
        };
        wait_for_blocked(&pool, pid).await;
        if cancel {
            pending.abort();
            assert!(pending.await.unwrap_err().is_cancelled());
        } else {
            assert!(matches!(
                pending.await.unwrap(),
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            ));
        }
        blocker.rollback().await.unwrap();
        assert_eq!(
            tokio::time::timeout(
                Duration::from_secs(5),
                names(&reader_pool, ids.org_id, ids.user_id)
            )
            .await
            .unwrap()
            .unwrap(),
            ["Widget"]
        );
        let settings = sqlx::query!("SELECT current_setting('transaction_read_only') AS \"read_only!\",
            current_setting('transaction_isolation') AS \"isolation!\", current_setting('lock_timeout') AS \"timeout!\"")
            .fetch_one(&reader_pool).await.unwrap();
        assert_eq!(
            (
                settings.read_only.as_str(),
                settings.isolation.as_str(),
                settings.timeout.as_str()
            ),
            ("on", "serializable", "250ms")
        );
        let first = ExportPermit::acquire().unwrap();
        let second = ExportPermit::acquire().unwrap();
        drop((first, second));
    }
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_reject_missing_foreign_and_unreadable_captured_ids(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    for project in [Uuid::now_v7(), foreign.project_id] {
        assert_eq!(
            authorize_projects(&pool, ids.org_id, ids.user_id, &[ids.project_id, project]).await,
            Err(StatusCode::FORBIDDEN)
        );
    }
    authorize_projects(&pool, ids.org_id, ids.user_id, &[])
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role='member' WHERE id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        authorize_projects(&pool, ids.org_id, ids.user_id, &[ids.project_id]).await,
        Err(StatusCode::FORBIDDEN)
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        authorize_projects(&pool, ids.org_id, ids.user_id, &[]).await,
        Err(StatusCode::FORBIDDEN)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_retain_parent_authority_until_release_check_finishes(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    let reader_pool = paused_project_pool(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT pg_advisory_xact_lock(715015) AS \"lock!: ()\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pending = {
        let reader_pool = reader_pool.clone();
        tokio::spawn(async move {
            authorize_projects(&reader_pool, ids.org_id, ids.user_id, &[ids.project_id]).await
        })
    };
    wait_for_blocked(&pool, pid).await;
    let reader_pid = sqlx::query_scalar!(
        r#"SELECT pid AS "pid!" FROM pg_stat_activity
           WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)) LIMIT 1"#,
        pid,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let writer = {
        let pool = pool.clone();
        tokio::spawn(async move {
            sqlx::query!(
                "DELETE FROM assignments WHERE project_id=$1",
                ids.project_id
            )
            .execute(&pool)
            .await
            .unwrap()
            .rows_affected()
        })
    };
    wait_for_blocked(&pool, reader_pid).await;
    blocker.rollback().await.unwrap();
    pending.await.unwrap().unwrap();
    assert_eq!(writer.await.unwrap(), 1);
    assert_eq!(
        authorize_projects(&reader_pool, ids.org_id, ids.user_id, &[ids.project_id]).await,
        Err(StatusCode::FORBIDDEN)
    );
    reader_pool.close().await;
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_exports_cancel_loading_without_retaining_authority(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    assign(&mut pool.acquire().await.unwrap(), &ids).await;
    let reader_pool = paused_project_pool(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!("SELECT pg_advisory_xact_lock(715015) AS \"lock!: ()\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pid = sqlx::query_scalar!("SELECT pg_backend_pid() as \"pid!\"")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let pending = {
        let reader_pool = reader_pool.clone();
        tokio::spawn(async move { names(&reader_pool, ids.org_id, ids.user_id).await })
    };
    wait_for_blocked(&pool, pid).await;
    pending.abort();
    assert!(pending.await.unwrap_err().is_cancelled());
    blocker.rollback().await.unwrap();
    assert_eq!(
        tokio::time::timeout(
            Duration::from_secs(5),
            names(&reader_pool, ids.org_id, ids.user_id)
        )
        .await
        .unwrap()
        .unwrap(),
        ["Widget"]
    );
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    sqlx::query!(
        "SELECT id FROM users WHERE id=$1 FOR UPDATE NOWAIT",
        ids.user_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    writer.rollback().await.unwrap();
    reader_pool.close().await;
}
