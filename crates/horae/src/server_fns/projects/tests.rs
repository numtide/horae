use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;

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
