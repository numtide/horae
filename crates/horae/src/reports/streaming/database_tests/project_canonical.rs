use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

async fn canonical(pool: &PgPool, role: OrgRole, permissions: &[Permission]) -> SeedIds {
    let ids = seed(pool, role).await;
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,$4,false,'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id, &grants,
    ).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind='amount', budget_amount_cents=86753 WHERE id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_csv_rejects_financial_revocation_after_capacity_wait(pool: PgPool) {
    let ids = canonical(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let (sender, mut receiver) = mpsc::channel(1);
    let (ready, waiting) = oneshot::channel();
    let worker_pool = pool.clone();
    let pending = tokio::spawn(async move {
        let mut tx = worker_pool.begin().await.unwrap();
        let authority = Authority {
            org_id: ids.org_id,
            actor_id: ids.user_id,
            purpose: Purpose::Projects,
        };
        authority.begin(&mut tx).await.unwrap();
        cursor::declare_projects(&mut tx, ids.org_id, ids.user_id, "active")
            .await
            .unwrap();
        let rows = cursor::projects(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].budget_amount_cents, Some(86753));
        let mut output = CsvBuffer::new(&["Budget"])?;
        for _ in 0..2 {
            output
                .writer
                .write_record([crate::reports::budget_cell(&rows[0])])
                .unwrap();
            output
                .project_record(&sender, &mut tx, &authority, &rows[0])
                .await?;
        }
        ready.send(()).unwrap();
        output.flush(&sender, &mut tx, &authority).await
    });
    waiting.await.unwrap();
    // A full channel cannot keep the old financial authority locked in place.
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id=$1 FOR NO KEY UPDATE NOWAIT",
        ids.org_id
    )
    .fetch_one(&mut *writer)
    .await
    .unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::ProjectReadAll])).unwrap(),
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
    let first = receiver.recv().await.unwrap();
    assert!(String::from_utf8(first).unwrap().contains("867.53"));
    assert_eq!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN));
    assert!(
        receiver.recv().await.is_none(),
        "revoked monetary block must not be released"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_csv_canonical_reader_exports_project_without_monetary_budget(pool: PgPool) {
    let ids = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let data = body(
        projects(
            pool,
            ids.org_id,
            ids.user_id,
            ProjectsExportParams::default(),
        )
        .await
        .unwrap(),
    )
    .await;
    let rows = csv::Reader::from_reader(data.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(&rows[0][2], "Widget");
    assert_eq!(&rows[0][5], "");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_csv_canonical_empty_scope_does_not_inherit_legacy_admin(pool: PgPool) {
    let ids = canonical(&pool, OrgRole::Admin, &[]).await;
    let data = body(
        projects(
            pool,
            ids.org_id,
            ids.user_id,
            ProjectsExportParams::default(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(
        csv::Reader::from_reader(data.as_slice()).records().count(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_csv_source_masks_money_using_declare_snapshot(pool: PgPool) {
    for visible in [false, true] {
        let ids = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
        let mut tx = pool.begin().await.unwrap();
        let authority = Authority {
            org_id: ids.org_id,
            actor_id: ids.user_id,
            purpose: Purpose::Projects,
        };
        authority.begin(&mut tx).await.unwrap();
        for granted in [visible, !visible] {
            let selection = if granted {
                vec![Permission::ProjectReadAll, Permission::BillableRateReadAll]
            } else {
                vec![Permission::ProjectReadAll]
            };
            let grants: Vec<String> = serde_json::from_value(
                serde_json::to_value(PermissionSelection::new(&selection)).unwrap(),
            )
            .unwrap();
            sqlx::query!(
                "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
                ids.user_id,
                &grants
            )
            .execute(&pool)
            .await
            .unwrap();
            if granted == visible {
                cursor::declare_projects(&mut tx, ids.org_id, ids.user_id, "active")
                    .await
                    .unwrap();
            }
        }
        let rows = cursor::projects(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].budget_amount_cents, visible.then_some(86753));
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_csv_invalid_source_state_cannot_be_repaired_before_fetch(pool: PgPool) {
    for populated in [false, true] {
        let permissions = if populated {
            vec![Permission::ProjectReadAll]
        } else {
            vec![]
        };
        let ids = canonical(&pool, OrgRole::Admin, &permissions).await;
        let mut tx = pool.begin().await.unwrap();
        let authority = Authority {
            org_id: ids.org_id,
            actor_id: ids.user_id,
            purpose: Purpose::Projects,
        };
        authority.begin(&mut tx).await.unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET catalog_version=999 WHERE user_id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        cursor::declare_projects(&mut tx, ids.org_id, ids.user_id, "active")
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET catalog_version=1 WHERE user_id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            cursor::projects(&mut tx, 128).await,
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        ));
        tx.rollback().await.unwrap();
    }
}
