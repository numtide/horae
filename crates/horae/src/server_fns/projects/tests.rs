use super::*;
use crate::server_fns::test_seed::seed;
use sqlx::PgPool;
use uuid::Uuid;

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
