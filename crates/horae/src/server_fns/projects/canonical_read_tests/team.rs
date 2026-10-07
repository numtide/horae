use super::*;

async fn add_member(pool: &PgPool, ids: &SeedIds, user_id: Uuid) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id,rate_cents) VALUES ($1,$2,$3,76543)",
        id,
        ids.project_id,
        user_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

pub(super) async fn teammate(pool: &PgPool, ids: &SeedIds) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id,org_id,email,name,active) VALUES ($1,$2,$3,'Project teammate',false)",
        id, ids.org_id, format!("{id}@test.invalid"),
    ).execute(pool).await.unwrap();
    add_member(pool, ids, id).await;
    id
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_reader_can_read_retained_team_without_people_directory_or_rates(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let member = teammate(&pool, &ids).await;
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(
        team.iter().map(|row| row.user_id).collect::<Vec<_>>(),
        [member]
    );
    let payload = serde_json::to_value(&team[0]).unwrap();
    for field in [
        "rate_cents",
        "email",
        "cost_rate_cents",
        "org_role",
        "grants",
    ] {
        assert!(
            payload.get(field).is_none(),
            "unexpected {field}: {payload}"
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_member_does_not_inherit_legacy_admin_team_or_rates(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
    teammate(&pool, &ids).await;
    assert!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()
            .is_empty()
    );
    let own = add_member(&pool, &ids, ids.user_id).await;
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(team.iter().map(|row| row.id).collect::<Vec<_>>(), [own]);
    assert!(team[0].rate_cents.is_none());
    sqlx::query!("DELETE FROM assignments WHERE id=$1", own)
        .execute(&pool)
        .await
        .unwrap();
    replace_grants(&pool, &ids, &[Permission::BillableRateReadAll]).await;
    assert!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_team_and_override_rates_follow_independent_current_project_scope(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[
            Permission::ProjectReadManaged,
            Permission::BillableRateReadManaged,
        ]),
    )
    .await;
    teammate(&pool, &ids).await;
    assert!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()
            .is_empty()
    );
    designate(&pool, &ids).await;
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(team.len(), 1);
    assert_eq!(team[0].rate_cents, Some(76543));
    replace_grants(&pool, &ids, &[Permission::ProjectReadManaged]).await;
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(team.len(), 1);
    assert!(team[0].rate_cents.is_none());
    replace_grants(
        &pool,
        &ids,
        &[
            Permission::ProjectReadAll,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(team.len(), 1);
    assert!(team[0].rate_cents.is_none());
    replace_grants(
        &pool,
        &ids,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    assert_eq!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()[0]
            .rate_cents,
        Some(76543)
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn team_reader_denies_foreign_projects_and_unavailable_actors(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    teammate(&pool, &ids).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    add_member(&pool, &foreign, foreign.user_id).await;
    assert!(
        assignments_for_viewer(&pool, &viewer, foreign.project_id)
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        assignments_for_viewer(&pool, &viewer, ids.project_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn team_reader_rejects_malformed_canonical_state_including_empty_projects(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE org_id=$1 AND user_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let error = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    ));
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn team_reader_does_not_disclose_cross_tenant_assignment_targets(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    let foreign = seed(&pool, OrgRole::Member).await;
    add_member(&pool, &ids, foreign.user_id).await;
    let local = teammate(&pool, &ids).await;
    let team = assignments_for_viewer(&pool, &viewer, ids.project_id)
        .await
        .unwrap();
    assert_eq!(
        team.iter().map(|row| row.user_id).collect::<Vec<_>>(),
        [local]
    );
}
