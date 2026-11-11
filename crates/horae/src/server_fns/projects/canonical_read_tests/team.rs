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
        OrgRole::Member,
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

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn canonical_member_does_not_inherit_legacy_admin_team(pool: PgPool) {
    let (ids, viewer) = fixture(&pool, OrgRole::Admin, PermissionSelection::new(&[])).await;
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
        OrgRole::Admin,
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
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll]),
    )
    .await;
    teammate(&pool, &ids).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
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
        OrgRole::Admin,
        PermissionSelection::new(&[Permission::ProjectReadAll, Permission::BillableRateReadAll]),
    )
    .await;
    let foreign = seed(&pool, OrgRole::Member).await;
    add_member(&pool, &ids, foreign.user_id).await;
    let local = teammate(&pool, &ids).await;
    assert_eq!(
        team_ids(&pool, &viewer, ids.project_id).await,
        Some(vec![local])
    );
}

/// Legacy Project Detail opens only with `can_view_progress`, so the legacy
/// `can_view_team` filter in `team_member_ids` is redundant while progress
/// access implies team access. It stays as defense in depth; this pins the
/// implication so a change to `project_read_access` cannot make that filter
/// the only, untested guard.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn legacy_progress_access_implies_team_access(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut viewers = vec![ids.user_id];
    for (role, assignment) in [
        (OrgRole::Manager, None),
        (OrgRole::Member, Some(ProjectRole::Lead)),
        (OrgRole::Member, Some(ProjectRole::Freelancer)),
        (OrgRole::Member, None),
    ] {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO users (id,org_id,email,name,org_role) VALUES ($1,$2,$3,'Viewer',$4)",
            id,
            ids.org_id,
            format!("{id}@test.invalid"),
            role as OrgRole,
        )
        .execute(&pool)
        .await
        .unwrap();
        if let Some(assignment) = assignment {
            sqlx::query!(
                "INSERT INTO assignments (id,project_id,user_id,role) VALUES ($1,$2,$3,$4)",
                Uuid::now_v7(),
                ids.project_id,
                id,
                assignment as ProjectRole,
            )
            .execute(&pool)
            .await
            .unwrap();
        } else if role == OrgRole::Member {
            // Own history gives an unassigned member a row, without progress.
            crate::server_fns::test_seed::time_entry(
                &pool,
                &SeedIds { user_id: id, ..ids },
                EntryState::Open,
            )
            .await;
        }
        viewers.push(id);
    }
    for visibility in [None, Some("project_members"), Some("managers")] {
        if let Some(visibility) = visibility {
            sqlx::query!(
                "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility)
                 VALUES ($1,$2,$3,$4,'project',$5)
                 ON CONFLICT (project_id) DO UPDATE SET report_visibility=EXCLUDED.report_visibility",
                Uuid::now_v7(),
                ids.org_id,
                ids.project_id,
                ids.user_id,
                visibility,
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let rows = sqlx::query!(
            r#"SELECT user_id as "user_id!", can_view_progress as "can_view_progress!",
                can_view_team as "can_view_team!"
             FROM project_read_access WHERE project_id=$1"#,
            ids.project_id,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let mut seen: Vec<_> = rows.iter().map(|row| row.user_id).collect();
        seen.sort();
        let mut expected = viewers.clone();
        expected.sort();
        assert_eq!(seen, expected, "{visibility:?}");
        assert!(
            rows.iter().any(|row| row.can_view_progress),
            "{visibility:?}"
        );
        for row in &rows {
            assert!(
                !row.can_view_progress || row.can_view_team,
                "{visibility:?}: {} reads progress without the team",
                row.user_id
            );
        }
    }
}
