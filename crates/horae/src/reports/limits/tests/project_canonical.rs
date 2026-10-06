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
async fn canonical_project_export_accepts_project_reader_with_legacy_member_role(pool: PgPool) {
    let ids = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let rows = projects(&pool, ids.org_id, ids.user_id, "active")
        .await
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ids.project_id]
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_export_does_not_inherit_legacy_administrator_scope(pool: PgPool) {
    let ids = canonical(&pool, OrgRole::Admin, &[]).await;
    let rows = projects(&pool, ids.org_id, ids.user_id, "active")
        .await
        .unwrap();
    assert!(
        rows.is_empty(),
        "legacy administrator exposed {} projects",
        rows.len()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_export_withholds_ordinary_monetary_budget(pool: PgPool) {
    let ids = canonical(&pool, OrgRole::Admin, &[Permission::ProjectReadAll]).await;
    let rows = projects(&pool, ids.org_id, ids.user_id, "active")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].budget_amount_cents, None);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_workbook_rejects_financial_revocation_during_render(pool: PgPool) {
    let ids = canonical(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let rows = projects(&pool, ids.org_id, ids.user_id, "active")
        .await
        .unwrap();
    assert_eq!(rows[0].budget_amount_cents, Some(86753));
    let project_ids = rows.iter().map(|row| row.id).collect();
    let monetary_project_ids = rows
        .iter()
        .filter(|row| row.has_monetary_budget())
        .map(|row| row.id)
        .collect();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let render_pool = pool.clone();
    let pending = tokio::spawn(async move {
        crate::reports::render_project_export(
            crate::reports::bounded::ExportPermit::acquire().unwrap(),
            &render_pool,
            ids.org_id,
            ids.user_id,
            project_ids,
            monetary_project_ids,
            move || {
                started.send(()).unwrap();
                wait.recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                crate::reports::projects_xlsx(&rows)
            },
        )
        .await
    });
    ready.await.unwrap();
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::ProjectReadAll])).unwrap(),
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
    release.send(()).unwrap();
    assert!(
        matches!(pending.await.unwrap(), Err(StatusCode::FORBIDDEN)),
        "retained project access must not release a workbook containing revoked money"
    );
}
