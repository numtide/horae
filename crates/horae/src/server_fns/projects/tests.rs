use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
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

#[test]
fn project_rate_distinguishes_inheritance_zero_and_exact_amounts() {
    assert_eq!(parse_project_rate("  ").unwrap(), None);
    assert_eq!(parse_project_rate("0").unwrap(), Some(0));
    assert_eq!(parse_project_rate(" 120.50 ").unwrap(), Some(12050));
    assert_eq!(
        parse_project_rate("92233720368547758.07").unwrap(),
        Some(i64::MAX)
    );
    for invalid in ["-1", "NaN", "inf", "12.345", "92233720368547758.08"] {
        assert!(parse_project_rate(invalid).is_err(), "accepted {invalid}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn creating_a_project_task_enables_it_with_its_defaults(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let task = create_task_for_project(
        &pool,
        ids.org_id,
        "  Support  ",
        false,
        Some(ids.project_id),
    )
    .await
    .unwrap();
    assert_eq!(task.name, "Support");
    let context = sqlx::query!("SELECT billable FROM time_entry_contexts WHERE user_id = $1 AND project_id = $2 AND task_id = $3", ids.user_id, ids.project_id, task.id).fetch_one(&pool).await.unwrap();
    assert_eq!(context.billable, Some(false));
}

#[sqlx::test(migrations = "./migrations")]
async fn failed_project_task_creation_leaves_no_orphan_task(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = seed(&pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for project in [ids.project_id, other.project_id, uuid::Uuid::now_v7()] {
        assert!(
            create_task_for_project(&pool, ids.org_id, "Orphan", true, Some(project))
                .await
                .is_err()
        );
    }
    assert_eq!(
        sqlx::query_scalar!("SELECT count(*) FROM tasks WHERE name = 'Orphan'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn enabling_is_tenant_scoped_even_for_existing_links_and_preserves_overrides(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = seed(&pool, OrgRole::Manager).await;
    let mut tx = pool.begin().await.unwrap();
    enable_project_task(&mut tx, ids.org_id, ids.project_id, ids.task_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    sqlx::query!(
        "UPDATE project_tasks SET billable = false, rate_cents = 123 WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(
        enable_project_task(&mut tx, other.org_id, ids.project_id, ids.task_id)
            .await
            .is_err()
    );
    enable_project_task(&mut tx, ids.org_id, ids.project_id, ids.task_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let link = sqlx::query!(
        "SELECT billable, rate_cents FROM project_tasks WHERE project_id = $1 AND task_id = $2",
        ids.project_id,
        ids.task_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((link.billable, link.rate_cents), (false, Some(123)));
}
async fn project_with_assignment(pool: &PgPool) -> (User, Uuid) {
    let ids = seed(pool, OrgRole::Member).await;
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

#[sqlx::test(migrations = "./migrations")]
async fn members_can_list_identities_but_not_assignment_rates(pool: PgPool) {
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
    let rows = assignments_for_viewer(&pool, &viewer, project)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|row| row.rate_cents.is_none()));
}

#[sqlx::test(migrations = "./migrations")]
async fn managers_and_admins_can_read_assignment_rates(pool: PgPool) {
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
        let rows = assignments_for_viewer(&pool, &viewer, project)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rate_cents, Some(12345));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn assignments_are_scoped_to_the_viewers_organization(pool: PgPool) {
    let (mut viewer, project) = project_with_assignment(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    viewer.org_id = foreign.org_id;
    viewer.id = foreign.user_id;
    viewer.org_role = OrgRole::Admin;
    assert!(
        assignments_for_viewer(&pool, &viewer, project)
            .await
            .unwrap()
            .is_empty()
    );
}
