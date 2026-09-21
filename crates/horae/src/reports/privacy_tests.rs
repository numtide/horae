use super::*;
use crate::server_fns::test_seed::{SeedIds, seed};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

async fn fixture(pool: &PgPool) -> SeedIds {
    let ids = seed(pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id, org_id, project_id, creator_id, rate_mode) VALUES ($1,$2,$3,$4,'project')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(pool).await.unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 600 WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

async fn csv_rows(pool: &PgPool, ids: &SeedIds, scope: &str) -> Vec<csv::StringRecord> {
    let response = streaming::projects(
        pool.clone(),
        ids.org_id,
        ids.user_id,
        ProjectsExportParams {
            scope: Some(scope.into()),
        },
    )
    .await
    .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    csv::Reader::from_reader(bytes.as_ref())
        .records()
        .map(Result::unwrap)
        .collect()
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn project_downloads_follow_progress_visibility(pool: PgPool) {
    let ids = fixture(&pool).await;
    for scope in ["active", "budgeted"] {
        assert!(
            csv_rows(&pool, &ids, scope).await.is_empty(),
            "private budget leaked through CSV"
        );
        assert!(
            limits::projects(&pool, ids.org_id, ids.user_id, scope)
                .await
                .unwrap()
                .is_empty(),
            "private budget leaked through XLSX"
        );
    }
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    for scope in ["active", "budgeted"] {
        let csv = csv_rows(&pool, &ids, scope).await;
        let rows = limits::projects(&pool, ids.org_id, ids.user_id, scope)
            .await
            .unwrap();
        assert_eq!((csv.len(), rows.len()), (1, 1));
        assert_eq!(&csv[0][5], budget_cell(&rows[0]));
        assert_eq!(csv[0].len(), 7, "no rates or private fields added");
        assert!(!projects_xlsx(&rows).unwrap().is_empty());
    }
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(csv_rows(&pool, &ids, "active").await.is_empty());
    assert_eq!(csv_rows(&pool, &ids, "archived").await.len(), 1);
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(csv_rows(&pool, &ids, "archived").await.is_empty());
    assert!(
        limits::projects(&pool, ids.org_id, ids.user_id, "archived")
            .await
            .unwrap()
            .is_empty()
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn unauthorized_project_text_does_not_affect_export_size_limits(pool: PgPool) {
    let ids = fixture(&pool).await;
    sqlx::query!(
        "UPDATE projects SET name = repeat('x', 32768) WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        limits::projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .unwrap()
            .is_empty()
    );
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        limits::projects(&pool, ids.org_id, ids.user_id, "active").await,
        Err(StatusCode::PAYLOAD_TOO_LARGE)
    ));
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        limits::projects(&pool, ids.org_id, ids.user_id, "active")
            .await
            .unwrap()
            .is_empty()
    );
}
