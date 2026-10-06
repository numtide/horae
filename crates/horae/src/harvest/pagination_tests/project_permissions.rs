use super::*;
use crate::server_fns::test_seed::SeedIds;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

mod concurrency;

pub(super) async fn selection(pool: &PgPool, user_id: Uuid, permissions: &[Permission]) {
    let grants: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        user_id,
        &grants,
    )
    .execute(pool)
    .await
    .unwrap();
}

pub(super) async fn canonical(
    pool: &PgPool,
    role: OrgRole,
    permissions: &[Permission],
) -> (SeedIds, Router) {
    let ids = seed(pool, role).await;
    sqlx::query!(
        "INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source)
         VALUES ($1,$2,$3,1,'{}',false,'individual')",
        Uuid::now_v7(), ids.org_id, ids.user_id,
    ).execute(pool).await.unwrap();
    selection(pool, ids.user_id, permissions).await;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind='amount',budget_amount_cents=86753 WHERE id=$1",
        ids.project_id,
    )
    .execute(pool)
    .await
    .unwrap();
    let app = signed_in(pool, ids.user_id).await;
    (ids, app)
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_reader_uses_list_count_and_detail_without_legacy_role(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    walk(&app, "projects", "is_active=true", &[ids.project_id]).await;
    let detail = page(&app, &format!("/harvest/v2/projects/{}", ids.project_id)).await;
    assert_eq!(detail["id"], ids.project_id.to_string());
    assert!(detail["budget"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_does_not_inherit_legacy_administrator_scope(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
    for query in ["", "?per_page=1", "?per_page=1&page=2", "?is_active=true"] {
        let result = page(&app, &format!("/harvest/v2/projects{query}")).await;
        assert_eq!(result["total_entries"], 0);
        assert!(result["projects"].as_array().unwrap().is_empty());
    }
    assert_eq!(
        request(&app, &format!("/harvest/v2/projects/{}", ids.project_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_rechecks_financial_only_revocation(pool: PgPool) {
    let (ids, app) = canonical(
        &pool,
        OrgRole::Admin,
        &[Permission::ProjectReadAll, Permission::BillableRateReadAll],
    )
    .await;
    let detail_uri = format!("/harvest/v2/projects/{}", ids.project_id);
    assert!(!page(&app, &detail_uri).await["budget"].is_null());
    selection(&pool, ids.user_id, &[Permission::ProjectReadAll]).await;
    let list = page(&app, "/harvest/v2/projects").await;
    assert_eq!(list["total_entries"], 1);
    assert!(list["projects"][0]["budget"].is_null());
    assert!(page(&app, &detail_uri).await["budget"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_managed_scope_requires_current_designation(pool: PgPool) {
    let (ids, app) = canonical(
        &pool,
        OrgRole::Admin,
        &[
            Permission::ProjectReadManaged,
            Permission::BillableRateReadManaged,
        ],
    )
    .await;
    let detail_uri = format!("/harvest/v2/projects/{}", ids.project_id);
    assert_eq!(page(&app, "/harvest/v2/projects").await["total_entries"], 0);
    let edge = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id) VALUES ($1,$2,$3,$4)",
        edge, ids.org_id, ids.user_id, ids.project_id,
    ).execute(&pool).await.unwrap();
    walk(&app, "projects", "is_active=true", &[ids.project_id]).await;
    assert!(!page(&app, &detail_uri).await["budget"].is_null());
    sqlx::query!(
        "DELETE FROM project_management_assignments WHERE id=$1",
        edge
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(page(&app, "/harvest/v2/projects").await["total_entries"], 0);
    assert_eq!(request(&app, &detail_uri).await.0, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_rejects_malformed_state_even_without_visible_rows(pool: PgPool) {
    for invalid in ["catalog", "grants", "missing", "policy"] {
        let (ids, app) = canonical(&pool, OrgRole::Admin, &[]).await;
        match invalid {
            "catalog" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET catalog_version=999 WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "grants" => {
                sqlx::query!(
                    "UPDATE person_permission_states SET grants=ARRAY['unknown'] WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "missing" => {
                sqlx::query!(
                    "DELETE FROM person_permission_states WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            "policy" => {
                sqlx::query!(
                    "UPDATE organizations SET permission_policy_version=999 WHERE id=$1",
                    ids.org_id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
            _ => unreachable!(),
        }
        for uri in [
            "/harvest/v2/projects".to_owned(),
            format!("/harvest/v2/projects/{}", ids.project_id),
        ] {
            assert_eq!(
                request(&app, &uri).await.0,
                StatusCode::FORBIDDEN,
                "{invalid}"
            );
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_retains_filtered_total_past_last_page(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let second = Uuid::now_v7();
    let archived = Uuid::now_v7();
    for (id, active) in [(second, true), (archived, false)] {
        sqlx::query!("INSERT INTO projects (id,org_id,client_id,name,currency,active) VALUES ($1,$2,$3,'Widget','EUR',$4)", id, ids.org_id, ids.client_id, active).execute(&pool).await.unwrap();
    }
    let filter = format!(
        "is_active=true&client_id={}&updated_since=2000-01-01T00:00:00Z",
        ids.client_id
    );
    walk(&app, "projects", &filter, &[ids.project_id, second]).await;
    let past = page(
        &app,
        &format!("/harvest/v2/projects?{filter}&per_page=1&page=3"),
    )
    .await;
    assert_eq!(past["total_entries"], 2);
    assert!(past["projects"].as_array().unwrap().is_empty());
    let future = page(
        &app,
        "/harvest/v2/projects?updated_since=2999-01-01T00:00:00Z",
    )
    .await;
    assert_eq!(future["total_entries"], 0);
    let archive = page(&app, "/harvest/v2/projects?is_active=false").await;
    assert_eq!(archive["total_entries"], 1);
    assert_eq!(archive["projects"][0]["id"], archived.to_string());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_shared_member_keeps_hours_but_not_money(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[]).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let detail_uri = format!("/harvest/v2/projects/{}", ids.project_id);
    assert!(page(&app, &detail_uri).await["budget"].is_null());
    sqlx::query!(
        "UPDATE projects SET budget_kind='hours',budget_minutes=600 WHERE id=$1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(page(&app, &detail_uri).await["budget"].to_string(), "10.0");
    walk(&app, "projects", "is_active=true", &[ids.project_id]).await;
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode,report_visibility) VALUES ($1,$2,$3,$4,'project','managers')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    assert_eq!(page(&app, "/harvest/v2/projects").await["total_entries"], 0);
    assert_eq!(request(&app, &detail_uri).await.0, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn canonical_project_api_excludes_foreign_projects_and_foreign_client_parents(pool: PgPool) {
    let (ids, app) = canonical(&pool, OrgRole::Member, &[Permission::ProjectReadAll]).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    assert_eq!(
        request(
            &app,
            &format!("/harvest/v2/projects/{}", foreign.project_id)
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        page(
            &app,
            &format!("/harvest/v2/projects?client_id={}", foreign.client_id)
        )
        .await["total_entries"],
        0
    );
    sqlx::query!(
        "UPDATE projects SET client_id=$2 WHERE id=$1",
        ids.project_id,
        foreign.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(page(&app, "/harvest/v2/projects").await["total_entries"], 0);
    assert_eq!(
        request(&app, &format!("/harvest/v2/projects/{}", ids.project_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
}
