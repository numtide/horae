use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

mod billability;
mod grouped;

async fn scoped(pool: &PgPool, role: OrgRole, selection: &[Permission]) -> SeedIds {
    let ids = seed(pool, role).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let grants: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(selection)).unwrap())
            .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_member_downloads_own_rows_without_financial_grants(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    add_entries(&pool, &ids, 1).await;
    let response = entries(pool.clone(), ids.org_id, ids.user_id, params())
        .await
        .unwrap();
    let data = body(response).await;
    let records = csv::Reader::from_reader(data.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(&records[0][1], "Widget");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_legacy_manager_does_not_expand_own_scope(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other')",
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
    let data = body(
        entries(pool.clone(), ids.org_id, ids.user_id, params())
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(data.as_slice()).records().count(),
        0
    );
}

async fn person(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Other')",
        id,
        org,
        format!("{id}@test.com")
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn grants(pool: &PgPool, actor: Uuid, permissions: &[Permission]) {
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        actor,
        &grants
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_source_uses_declare_authority_not_initial_or_fetch_authority(pool: PgPool) {
    for gain in [false, true] {
        let initial = if gain {
            Permission::TimeReadOwn
        } else {
            Permission::TimeReadAll
        };
        let captured = if gain {
            Permission::TimeReadAll
        } else {
            Permission::TimeReadOwn
        };
        let ids = scoped(&pool, OrgRole::Member, &[initial]).await;
        let other = person(&pool, ids.org_id).await;
        add_entries(&pool, &ids, 1).await;
        add_entries(
            &pool,
            &SeedIds {
                user_id: other,
                ..ids
            },
            1,
        )
        .await;
        let mut tx = pool.begin().await.unwrap();
        let (_, policy) = Authority::time(&mut tx, ids.org_id, ids.user_id)
            .await
            .unwrap();
        grants(&pool, ids.user_id, &[captured]).await;
        cursor::declare_entries(
            &mut tx,
            ids.org_id,
            ids.user_id,
            &params().time_query().unwrap(),
        )
        .await
        .unwrap();
        grants(&pool, ids.user_id, &[initial]).await;
        let rows = cursor::entries(&mut tx, 128).await.unwrap();
        let count = rows
            .into_iter()
            .filter_map(|row| row.into_entry(policy).unwrap())
            .count();
        assert_eq!(count, if gain { 2 } else { 1 });
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_source_uses_management_edges_at_declare(pool: PgPool) {
    for gain in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let other = person(&pool, ids.org_id).await;
        add_entries(
            &pool,
            &SeedIds {
                user_id: other,
                ..ids
            },
            1,
        )
        .await;
        let edge = Uuid::now_v7();
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", edge, ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        if gain {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE id=$1",
                edge
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let mut tx = pool.begin().await.unwrap();
        let (_, policy) = Authority::time(&mut tx, ids.org_id, ids.user_id)
            .await
            .unwrap();
        if gain {
            sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", edge, ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        } else {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE id=$1",
                edge
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        cursor::declare_entries(
            &mut tx,
            ids.org_id,
            ids.user_id,
            &params().time_query().unwrap(),
        )
        .await
        .unwrap();
        if gain {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE id=$1",
                edge
            )
            .execute(&pool)
            .await
            .unwrap();
        } else {
            sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", edge, ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        }
        let rows = cursor::entries(&mut tx, 128).await.unwrap();
        assert_eq!(
            rows.into_iter()
                .filter_map(|row| row.into_entry(policy).unwrap())
                .count(),
            usize::from(gain)
        );
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_invalid_captured_state_cannot_be_repaired_before_fetch(pool: PgPool) {
    for populated in [false, true] {
        let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
        if populated {
            add_entries(&pool, &ids, 1).await;
        }
        for invalid in ["catalog", "grants", "missing", "inactive", "policy"] {
            let mut tx = pool.begin().await.unwrap();
            let (_, policy) = Authority::time(&mut tx, ids.org_id, ids.user_id)
                .await
                .unwrap();
            match invalid {
                "catalog" => {
                    sqlx::query!(
                        "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
                        ids.user_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                "grants" => {
                    sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['unknown'] WHERE user_id=$1", ids.user_id).execute(&pool).await.unwrap();
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
                _ => unreachable!(),
            }
            cursor::declare_entries(
                &mut tx,
                ids.org_id,
                ids.user_id,
                &params().time_query().unwrap(),
            )
            .await
            .unwrap();
            sqlx::query!("UPDATE users SET active=true WHERE id=$1", ids.user_id)
                .execute(&pool)
                .await
                .unwrap();
            sqlx::query!(
                "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
                ids.org_id
            )
            .execute(&pool)
            .await
            .unwrap();
            sqlx::query!(
                "DELETE FROM person_permission_states WHERE user_id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
            let restored: Vec<String> = serde_json::from_value(
                serde_json::to_value(PermissionSelection::new(&[Permission::TimeReadOwn])).unwrap(),
            )
            .unwrap();
            sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &restored).execute(&pool).await.unwrap();
            let rows = cursor::entries(&mut tx, 128).await.unwrap();
            assert_eq!(rows.len(), 1, "{invalid}, populated={populated}");
            let expected = if matches!(invalid, "inactive" | "policy") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            assert_eq!(
                rows.into_iter().next().unwrap().into_entry(policy).err(),
                Some(expected),
                "{invalid}, populated={populated}"
            );
            tx.rollback().await.unwrap();
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_missing_identity_has_invalid_sentinel_and_metadata_is_bounded(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
    for (org, actor) in [(Uuid::now_v7(), ids.user_id), (ids.org_id, Uuid::now_v7())] {
        let mut tx = pool.begin().await.unwrap();
        cursor::declare_entries(&mut tx, org, actor, &params().time_query().unwrap())
            .await
            .unwrap();
        let rows = cursor::entries(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.into_iter().next().unwrap().into_entry(1).err(),
            Some(StatusCode::FORBIDDEN)
        );
        tx.rollback().await.unwrap();
    }
    add_entries(&pool, &ids, 3).await;
    // Legacy policy ignores canonical state, but the native batch still accounts
    // for any captured authority metadata before transferring it to Rust.
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY[repeat('x',65536)] WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let (_, policy) = Authority::time(&mut tx, ids.org_id, ids.user_id)
        .await
        .unwrap();
    cursor::declare_entries(
        &mut tx,
        ids.org_id,
        ids.user_id,
        &params().time_query().unwrap(),
    )
    .await
    .unwrap();
    for _ in 0..3 {
        let mut rows = cursor::entries(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows.pop().unwrap().into_entry(policy).unwrap().is_some());
    }
    assert!(cursor::entries(&mut tx, 128).await.unwrap().is_empty());
    tx.rollback().await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_revocation_under_backpressure_interrupts_body_without_holding_locks(
    pool: PgPool,
) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadAll]).await;
    let other = person(&pool, ids.org_id).await;
    add_entries(
        &pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        1000,
    )
    .await;
    let response = entries(pool.clone(), ids.org_id, ids.user_id, params())
        .await
        .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!("SET LOCAL lock_timeout='1s'")
        .execute(&mut *writer)
        .await
        .unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['time_read_own'] WHERE user_id=$1",
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let mut stream = response.into_body().into_data_stream();
    let mut bytes = Vec::new();
    let mut failed = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => bytes.extend_from_slice(&chunk),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "revocation must not become a successful EOF");
    assert!(csv::Reader::from_reader(bytes.as_slice()).records().count() <= 1 + CHUNK_ROWS);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_rechecks_every_pending_context_but_not_previous_blocks(pool: PgPool) {
    for revoke_previous in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let owners = [
            person(&pool, ids.org_id).await,
            person(&pool, ids.org_id).await,
            person(&pool, ids.org_id).await,
        ];
        for owner in owners {
            sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, owner).execute(&pool).await.unwrap();
        }
        let (sender, mut receiver) = mpsc::channel(1);
        let (ready, waiting) = oneshot::channel();
        let worker_pool = pool.clone();
        let pending = tokio::spawn(async move {
            let mut connection = worker_pool.acquire().await.unwrap();
            connection.close_on_drop();
            let mut tx = connection.begin().await.unwrap();
            let (authority, _) = Authority::time(&mut tx, ids.org_id, ids.user_id)
                .await
                .unwrap();
            let mut output = CsvBuffer::new(&["Owner"])?;
            for owner in owners {
                output.writer.write_record([owner.to_string()]).unwrap();
                output
                    .time_record(&sender, &mut tx, &authority, (owner, ids.project_id))
                    .await?;
            }
            ready.send(()).unwrap();
            output.flush(&sender, &mut tx, &authority).await
        });
        waiting.await.unwrap();
        let revoked = owners[if revoke_previous { 0 } else { 2 }];
        let mut writer = pool.begin().await.unwrap();
        sqlx::query!("SET LOCAL lock_timeout='1s'")
            .execute(&mut *writer)
            .await
            .unwrap();
        crate::db::lock_organization(
            &mut writer,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        sqlx::query!("DELETE FROM person_management_assignments WHERE org_id=$1 AND manager_id=$2 AND managed_user_id=$3", ids.org_id, ids.user_id, revoked).execute(&mut *writer).await.unwrap();
        writer.commit().await.unwrap();
        assert_eq!(
            receiver.recv().await.unwrap(),
            format!("Owner\n{}\n", owners[0]).as_bytes()
        );
        let result = pending.await.unwrap();
        if revoke_previous {
            result.unwrap();
            assert_eq!(
                receiver.recv().await.unwrap(),
                format!("{}\n{}\n", owners[1], owners[2]).as_bytes()
            );
        } else {
            assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
        }
        assert!(receiver.recv().await.is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_empty_header_rechecks_policy_and_state_after_capacity(pool: PgPool) {
    for change_policy in [false, true] {
        let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
        let (sender, mut receiver) = mpsc::channel(1);
        sender.send(b"queued".to_vec()).await.unwrap();
        let (ready, waiting) = oneshot::channel();
        let worker_pool = pool.clone();
        let pending = tokio::spawn(async move {
            let mut connection = worker_pool.acquire().await.unwrap();
            connection.close_on_drop();
            let mut tx = connection.begin().await.unwrap();
            let (authority, _) = Authority::time(&mut tx, ids.org_id, ids.user_id)
                .await
                .unwrap();
            let mut output = CsvBuffer::new(&["Date"])?;
            ready.send(()).unwrap();
            output.flush(&sender, &mut tx, &authority).await
        });
        waiting.await.unwrap();
        if change_policy {
            sqlx::query!(
                "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
                ids.org_id
            )
            .execute(&pool)
            .await
            .unwrap();
        } else {
            sqlx::query!(
                "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
                ids.user_id
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        assert_eq!(receiver.recv().await.unwrap(), b"queued");
        assert_eq!(
            pending.await.unwrap().unwrap_err(),
            if change_policy {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        );
        assert!(receiver.recv().await.is_none());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn scoped_csv_reassigned_or_deleted_entries_keep_captured_context(pool: PgPool) {
    for retain_original in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let owner = person(&pool, ids.org_id).await;
        let other_project = Uuid::now_v7();
        sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Reassigned','EUR')", other_project, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, ids.project_id).execute(&pool).await.unwrap();
        add_entries(
            &pool,
            &SeedIds {
                user_id: owner,
                ..ids
            },
            400,
        )
        .await;
        let response = entries(pool.clone(), ids.org_id, ids.user_id, params())
            .await
            .unwrap();
        let mut writer = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut writer,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE time_entries SET project_id=$2,notes='Changed',minutes=120 WHERE org_id=$1",
            ids.org_id,
            other_project
        )
        .execute(&mut *writer)
        .await
        .unwrap();
        if retain_original {
            sqlx::query!("DELETE FROM time_entries WHERE org_id=$1", ids.org_id)
                .execute(&mut *writer)
                .await
                .unwrap();
        } else {
            sqlx::query!(
                "UPDATE project_management_assignments SET project_id=$2 WHERE org_id=$1",
                ids.org_id,
                other_project
            )
            .execute(&mut *writer)
            .await
            .unwrap();
        }
        writer.commit().await.unwrap();
        if retain_original {
            let data = body(response).await;
            let records = csv::Reader::from_reader(data.as_slice())
                .records()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(records.len(), 400);
            assert!(
                records
                    .iter()
                    .all(|row| &row[1] == "Widget" && &row[4] == "1.00" && row[7].is_empty())
            );
        } else {
            assert!(
                axum::body::to_bytes(response.into_body(), 1024 * 1024)
                    .await
                    .is_err()
            );
        }
    }
}
