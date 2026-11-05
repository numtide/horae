use super::canonical_read_tests::project_team;
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

#[sqlx::test(migrations = "./migrations")]
async fn creating_a_project_task_enables_it_with_its_defaults(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let (task, _) = create_task_for_project(
        &pool,
        ids.org_id,
        ids.user_id,
        "  Support  ",
        false,
        Some(ids.project_id),
        &TaskRateEdit::Preserve {},
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
            create_task_for_project(
                &pool,
                ids.org_id,
                ids.user_id,
                "Orphan",
                true,
                Some(project),
                &TaskRateEdit::Preserve {},
            )
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

#[sqlx::test(migrations = "./migrations")]
async fn enabling_tasks_only_inherits_rates_used_by_configured_projects(pool: PgPool) {
    for project_type in [
        ProjectType::TimeAndMaterials,
        ProjectType::FixedFee,
        ProjectType::NonBillable,
    ] {
        for mode in [None, Some("person"), Some("project"), Some("task")] {
            for default_rate in [None, Some(0_i64), Some(8000)] {
                let ids = seed(&pool, OrgRole::Admin).await;
                sqlx::query!(
                    "UPDATE projects SET project_type = $2 WHERE id = $1",
                    ids.project_id,
                    project_type as ProjectType
                )
                .execute(&pool)
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE tasks SET default_rate_cents = $2, default_rate_currency = 'EUR' WHERE id = $1",
                    ids.task_id,
                    default_rate
                )
                .execute(&pool)
                .await
                .unwrap();
                if let Some(mode) = mode {
                    sqlx::query!(
                        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1, $2, $3, $4, $5)",
                        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode,
                    ).execute(&pool).await.unwrap();
                }
                let mut tx = pool.begin().await.unwrap();
                enable_project_task(&mut tx, ids.org_id, ids.project_id, ids.task_id)
                    .await
                    .unwrap();
                tx.commit().await.unwrap();
                let expected = if mode.is_none()
                    || (project_type == ProjectType::TimeAndMaterials && mode == Some("task"))
                {
                    default_rate
                } else {
                    None
                };
                let stored = sqlx::query_scalar!(
                    "SELECT rate_cents FROM project_tasks WHERE project_id = $1 AND task_id = $2",
                    ids.project_id,
                    ids.task_id
                )
                .fetch_one(&pool)
                .await
                .unwrap();
                assert_eq!(
                    stored, expected,
                    "type={project_type:?}, mode={mode:?}, default={default_rate:?}"
                );
            }
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn enabling_tasks_rejects_incompatible_catalog_currency(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET currency = 'USD' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 8000, default_rate_currency = 'EUR' WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1, $2, $3, $4, 'task')",
        Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id,
    ).execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    let error = enable_project_task(&mut tx, ids.org_id, ids.project_id, ids.task_id)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("explicit rate in the project currency")
    );
    tx.commit().await.unwrap();
    assert_eq!(
        sqlx::query_scalar!(
            "SELECT count(*) FROM project_tasks WHERE project_id = $1",
            ids.project_id
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(0)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn unknown_catalog_currency_requires_explicit_configured_rates_but_preserves_legacy(
    pool: PgPool,
) {
    for rate in [0_i64, 8000] {
        for mode in [None, Some("task"), Some("person")] {
            let ids = seed(&pool, OrgRole::Admin).await;
            sqlx::query!(
                "UPDATE tasks SET default_rate_cents = $2 WHERE id = $1",
                ids.task_id,
                rate
            )
            .execute(&pool)
            .await
            .unwrap();
            if let Some(mode) = mode {
                sqlx::query!(
                    "INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,$5)",
                    Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id, mode,
                ).execute(&pool).await.unwrap();
            }
            let mut tx = pool.begin().await.unwrap();
            let result =
                enable_project_task(&mut tx, ids.org_id, ids.project_id, ids.task_id).await;
            if mode == Some("task") {
                let error = result.unwrap_err();
                assert!(
                    error
                        .to_string()
                        .contains("currency is unknown or incompatible"),
                    "{error}"
                );
            } else {
                result.unwrap();
                let stored = sqlx::query_scalar!(
                    "SELECT rate_cents FROM project_tasks WHERE project_id = $1 AND task_id = $2",
                    ids.project_id,
                    ids.task_id,
                )
                .fetch_one(&mut *tx)
                .await
                .unwrap();
                assert_eq!(stored, if mode.is_none() { Some(rate) } else { None });
            }
            tx.rollback().await.unwrap();
        }
    }
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

#[sqlx::test(migrations = "./migrations")]
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

#[sqlx::test(migrations = "./migrations")]
async fn assignments_are_scoped_to_the_viewers_organization(pool: PgPool) {
    let (mut viewer, project) = project_with_assignment(&pool).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
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
