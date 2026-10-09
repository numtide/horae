use super::*;
use crate::server_fns::test_seed::time_entry;

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn hidden_spend_overflow_does_not_prevent_authorized_hour_totals(pool: PgPool) {
    let ids = tracked(&pool, OrgRole::Admin, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=$2 WHERE id=$1",
        ids.project_id,
        i64::MAX
    )
    .execute(&pool)
    .await
    .unwrap();
    time_entry(&pool, &ids, EntryState::Open).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (120, None));
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    assert!(matches!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id).await,
        Err(ServerFnError::ServerError {
            code: INTERNAL_ERROR,
            ..
        })
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_excludes_entries_with_foreign_person_parents(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE time_entries SET user_id=$2 WHERE project_id=$1",
        ids.project_id,
        foreign.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (0, Some(0)));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn spend_rechecks_financial_only_revocation_after_organization_wait(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
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
    let pending =
        tokio::spawn(
            async move { fetch_project_spend(&pending_pool, ids.org_id, ids.user_id).await },
        );
    crate::server_fns::test_seed::wait_for_blocked(&pool, holder).await;
    let grants: Vec<String> =
        serde_json::from_value(serde_json::json!(PermissionSelection::new(&[
            Permission::ProjectReadAll
        ])))
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
    let rows = pending.await.unwrap().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].spent_minutes, rows[0].spent_cents), (60, None));
}

async fn tracked(pool: &PgPool, role: OrgRole, permissions: &[Permission]) -> SeedIds {
    let (ids, _) = fixture(pool, role, PermissionSelection::new(permissions)).await;
    sqlx::query!(
        "UPDATE projects SET rate_cents=12345 WHERE id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    time_entry(pool, &ids, EntryState::Open).await;
    ids
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_reads_do_not_require_a_legacy_manager_role(pool: PgPool) {
    let ids = tracked(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].project_id, rows[0].spent_minutes),
        (ids.project_id, 60)
    );
    assert!(
        serde_json::to_value(&rows[0])
            .unwrap()
            .get("spent_cents")
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_withholds_money_after_financial_revocation(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let initial = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&initial[0]).unwrap()["spent_cents"],
        serde_json::json!(12345)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 60);
    let payload = serde_json::to_value(&rows[0]).unwrap();
    assert!(
        payload.get("spent_cents").is_none(),
        "revoked amount: {payload}"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_managed_scope_requires_current_project_designation(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Manager,
        &[
            Permission::ProjectReadManaged,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    assert!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
    designate(&pool, &ids).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        serde_json::to_value(&rows[0]).unwrap()["spent_cents"],
        serde_json::json!(12345)
    );
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        fetch_project_spend(&pool, ids.org_id, ids.user_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_spend_zero_and_withheld_money_remain_distinct_without_entries(pool: PgPool) {
    let (ids, _) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 0);
    assert_eq!(
        serde_json::to_value(&rows[0]).unwrap()["spent_cents"],
        serde_json::json!(0)
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 0);
    assert!(
        serde_json::to_value(&rows[0])
            .unwrap()
            .get("spent_cents")
            .is_none()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_shared_progress_and_report_permissions_do_not_open_ordinary_money(pool: PgPool) {
    let ids = tracked(
        &pool,
        OrgRole::Member,
        &[Permission::ReportProfitabilityRead],
    )
    .await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,'freelancer')",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let rows = fetch_project_spend(&pool, ids.org_id, ids.user_id)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].spent_minutes, 60);
    let payload = serde_json::to_value(&rows[0]).unwrap();
    assert!(
        payload.get("spent_cents").is_none(),
        "report-only amount: {payload}"
    );
}
