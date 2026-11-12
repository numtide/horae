use super::canonical_read_tests::project_team;
use super::*;
use crate::server_fns::test_seed::seed;
use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn project_rate_sql_matches_rust_for_missing_zero_and_present_sources(pool: PgPool) {
    use horae_core::project::RateMode;

    let rows = sqlx::query!(
        "SELECT mode, task, assignment, project, person, client,
                resolve_project_rate(mode, task, assignment, project, person, client) AS rate
         FROM unnest(ARRAY[NULL::text, 'legacy', 'person', 'task', 'project']) AS modes(mode)
         CROSS JOIN unnest(ARRAY[NULL::bigint, 0, 1234]) AS tasks(task)
         CROSS JOIN unnest(ARRAY[NULL::bigint, 0, 2345]) AS assignments(assignment)
         CROSS JOIN unnest(ARRAY[NULL::bigint, 0, 3456]) AS projects(project)
         CROSS JOIN unnest(ARRAY[NULL::bigint, 0, 4567]) AS people(person)
         CROSS JOIN unnest(ARRAY[NULL::bigint, 0, 5678]) AS clients(client)"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1215);
    for row in rows {
        let mode = match row.mode.as_deref() {
            None | Some("legacy") => RateMode::Legacy,
            Some("person") => RateMode::Person,
            Some("task") => RateMode::Task,
            Some("project") => RateMode::Project,
            other => panic!("unexpected test mode: {other:?}"),
        };
        let expected = horae_core::invoice::resolve_project_rate(
            mode,
            row.task,
            row.assignment,
            row.project,
            row.person,
            row.client,
        );
        assert_eq!(
            row.rate, expected,
            "mode={mode:?}; task={:?}, assignment={:?}, project={:?}, person={:?}, client={:?}",
            row.task, row.assignment, row.project, row.person, row.client
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn project_rate_helper_has_no_public_execute_grant(pool: PgPool) {
    let public_execute = sqlx::query_scalar!(
        r#"SELECT EXISTS (
           SELECT 1 FROM pg_proc p,
             LATERAL aclexplode(COALESCE(p.proacl, acldefault('f', p.proowner))) acl
           WHERE p.oid = 'public.resolve_project_rate(text,bigint,bigint,bigint,bigint,bigint)'::regprocedure
             AND acl.grantee = 0 AND acl.privilege_type = 'EXECUTE'
         ) AS "public_execute!""#
    ).fetch_one(&pool).await.unwrap();
    assert!(!public_execute);
}

async fn project_with_assignment(pool: &PgPool) -> (User, Uuid) {
    let ids = seed(pool, BuiltInProfile::Member).await;
    let colleague = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name)
         VALUES ($1, $2, $3, 'Colleague')",
        colleague,
        ids.org_id,
        format!("{colleague}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role, rate_cents)
         VALUES ($1, $2, $3, 'freelancer', 12345)",
        Uuid::now_v7(),
        ids.project_id,
        colleague,
    )
    .execute(pool)
    .await
    .unwrap();
    let viewer = User {
        id: ids.user_id,
        org_id: ids.org_id,
        email: format!("{}@test.com", ids.user_id),
        name: "Viewer".into(),
        oidc_subject: None,
        org_role: OrgRole::Member,
        cost_rate_cents: None,
        billable_rate_cents: None,
        active: true,
        created_at: chrono::Utc::now(),
    };
    (viewer, ids.project_id)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn members_see_the_project_team_without_rates(pool: PgPool) {
    let (viewer, project) = project_with_assignment(&pool).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        project,
        viewer.id
    )
    .execute(&pool)
    .await
    .unwrap();
    let team = project_team(&pool, &viewer, project)
        .await
        .unwrap()
        .expect("a member opens the project");
    assert_eq!(team.len(), 2);
    for member in team {
        let payload = serde_json::to_value(member).unwrap();
        assert!(payload.get("rate_cents").is_none(), "{payload}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn managers_and_admins_see_the_project_team(pool: PgPool) {
    let (mut viewer, project) = project_with_assignment(&pool).await;
    for role in [OrgRole::Manager, OrgRole::Admin] {
        viewer.org_role = role;
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            viewer.id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        let team = project_team(&pool, &viewer, project)
            .await
            .unwrap()
            .expect("managers open every project");
        assert_eq!(team.len(), 1, "{role:?}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn assignments_are_scoped_to_the_viewers_organization(pool: PgPool) {
    let (mut viewer, project) = project_with_assignment(&pool).await;
    let foreign = seed(&pool, BuiltInProfile::Administrator).await;
    viewer.org_id = foreign.org_id;
    viewer.id = foreign.user_id;
    viewer.org_role = OrgRole::Admin;
    assert!(
        project_team(&pool, &viewer, project)
            .await
            .unwrap()
            .is_none(),
        "a foreign project is not found"
    );
}
