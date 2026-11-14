use super::*;
use horae_core::permissions::catalog::BuiltInProfile;

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

/// The visible team's ids; `None` when the viewer cannot open the project.
async fn team_ids(pool: &PgPool, viewer: &User, project_id: Uuid) -> Option<Vec<Uuid>> {
    project_team(pool, viewer, project_id)
        .await
        .unwrap()
        .map(|team| team.into_iter().map(|member| member.id).collect())
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn project_reader_can_read_retained_team_without_people_directory_or_rates(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Member,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    let member = teammate(&pool, &ids).await;
    let team = project_team(&pool, &viewer, ids.project_id)
        .await
        .unwrap()
        .expect("a project reader opens the project");
    assert_eq!(
        team.iter().map(|member| member.id).collect::<Vec<_>>(),
        [member]
    );
    let payload = serde_json::to_value(&team[0]).unwrap();
    for field in ["rate_cents", "email", "cost_rate_cents", "grants"] {
        assert!(
            payload.get(field).is_none(),
            "unexpected {field}: {payload}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn administrator_profile_without_grants_sees_only_its_own_membership(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[]),
    )
    .await;
    teammate(&pool, &ids).await;
    assert_eq!(team_ids(&pool, &viewer, ids.project_id).await, None);
    // Membership opens the project, but the teammate stays hidden.
    let own = add_member(&pool, &ids, ids.user_id).await;
    assert_eq!(
        team_ids(&pool, &viewer, ids.project_id).await,
        Some(vec![ids.user_id])
    );
    sqlx::query!("DELETE FROM assignments WHERE id=$1", own)
        .execute(&pool)
        .await
        .unwrap();
    replace_grants(&pool, &ids, &[Permission::BillableRateReadAll]).await;
    assert_eq!(team_ids(&pool, &viewer, ids.project_id).await, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn project_team_follows_current_project_scope(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[Permission::ProjectReadManaged]),
    )
    .await;
    let member = teammate(&pool, &ids).await;
    assert_eq!(team_ids(&pool, &viewer, ids.project_id).await, None);
    designate(&pool, &ids).await;
    assert_eq!(
        team_ids(&pool, &viewer, ids.project_id).await,
        Some(vec![member])
    );
    replace_grants(&pool, &ids, &[Permission::ProjectReadAll]).await;
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE org_id=$1 AND manager_id=$2",
        ids.org_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        team_ids(&pool, &viewer, ids.project_id).await,
        Some(vec![member])
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn team_reader_denies_foreign_projects_and_unavailable_actors(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    teammate(&pool, &ids).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    add_member(&pool, &foreign, foreign.user_id).await;
    assert_eq!(team_ids(&pool, &viewer, foreign.project_id).await, None);
    sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        project_team(&pool, &viewer, ids.project_id).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn team_reader_rejects_malformed_canonical_state_including_empty_projects(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
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
    let error = project_team(&pool, &viewer, ids.project_id)
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn team_reader_does_not_disclose_cross_tenant_assignment_targets(pool: PgPool) {
    let (ids, viewer) = fixture(
        &pool,
        BuiltInProfile::Administrator,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    let foreign = seed(&pool, BuiltInProfile::Member).await;
    add_member(&pool, &ids, foreign.user_id).await;
    let local = teammate(&pool, &ids).await;
    assert_eq!(
        team_ids(&pool, &viewer, ids.project_id).await,
        Some(vec![local])
    );
}
