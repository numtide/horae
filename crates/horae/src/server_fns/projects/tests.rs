use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;
use uuid::Uuid;

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
    let rows = assignments_for_viewer(&pool, &viewer, project)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_ne!(rows[0].user_id, viewer.id);
    assert_eq!(rows[0].rate_cents, None);
}

#[sqlx::test(migrations = "./migrations")]
async fn managers_and_admins_can_read_assignment_rates(pool: PgPool) {
    let (mut viewer, project) = project_with_assignment(&pool).await;
    for role in [OrgRole::Manager, OrgRole::Admin] {
        viewer.org_role = role;
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
