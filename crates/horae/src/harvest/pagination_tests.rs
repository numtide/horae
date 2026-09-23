use super::*;
use crate::server_fns::test_seed::{seed, time_entry};
use axum::{Extension, body::Body, http::Request};
use horae_core::types::{EntryState, OrgRole};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session};

async fn signed_in(pool: &PgPool, user_id: Uuid) -> Router {
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    crate::auth::session::set_session_user_id(&session, user_id)
        .await
        .unwrap();
    router(pool.clone()).layer(Extension(session))
}

async fn request(app: &Router, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

async fn page(app: &Router, uri: &str) -> Value {
    let (status, body) = request(app, uri).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    serde_json::from_str(&body).unwrap()
}

fn query_pairs(link: &str) -> Vec<(String, String)> {
    openidconnect::url::form_urlencoded::parse(link.split_once('?').unwrap().1.as_bytes())
        .into_owned()
        .filter(|(key, _)| key != "page" && key != "per_page")
        .collect()
}

async fn walk(app: &Router, resource: &str, query: &str, expected: &[Uuid]) {
    let first_uri = format!("/harvest/v2/{resource}?per_page=1&{query}");
    let filters = query_pairs(&first_uri);
    let mut uri = first_uri.clone();
    let mut seen = Vec::new();
    for (index, expected_id) in expected.iter().enumerate() {
        let body = page(app, &uri).await;
        assert_eq!(body["total_entries"], expected.len(), "{resource}");
        assert_eq!(body["total_pages"], expected.len());
        assert_eq!(body["page"], index + 1);
        assert_eq!(body["per_page"], 1);
        let rows = body[resource].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], expected_id.to_string(), "{resource}");
        seen.push(rows[0]["id"].as_str().unwrap().to_owned());
        for key in ["first", "last", "next", "previous"] {
            if let Some(link) = body["links"][key].as_str() {
                assert!(link.starts_with(&format!("/harvest/v2/{resource}?")));
                assert_eq!(query_pairs(link), filters, "{resource}: {key}");
            }
        }
        if index > 0 {
            let previous = page(app, body["links"]["previous"].as_str().unwrap()).await;
            assert_eq!(previous[resource][0]["id"], seen[index - 1]);
        } else {
            assert!(body["previous_page"].is_null());
            assert!(body["links"]["previous"].is_null());
        }
        if index + 1 < expected.len() {
            assert_eq!(body["next_page"], index + 2);
            uri = body["links"]["next"].as_str().unwrap().to_owned();
        } else {
            assert!(body["next_page"].is_null());
            assert!(body["links"]["next"].is_null());
            let first = page(app, body["links"]["first"].as_str().unwrap()).await;
            let last = page(app, body["links"]["last"].as_str().unwrap()).await;
            assert_eq!(first[resource][0]["id"], expected[0].to_string());
            assert_eq!(last[resource][0]["id"], expected_id.to_string());
        }
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_catalog_links_preserve_filters_and_break_name_ties(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let _other_org = seed(&pool, OrgRole::Manager).await;
    let second = Uuid::now_v7();
    let inactive = Uuid::now_v7();
    for (id, active) in [(inactive, false), (second, true)] {
        sqlx::query!("INSERT INTO clients (id, org_id, name, currency, active) VALUES ($1, $2, 'Acme', 'EUR', $3)", id, ids.org_id, active).execute(&pool).await.unwrap();
        sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency, active) VALUES ($1, $2, $3, 'Widget', 'EUR', $4)", id, ids.org_id, ids.client_id, active).execute(&pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO tasks (id, org_id, name, active) VALUES ($1, $2, 'Dev', $3)",
            id,
            ids.org_id,
            active
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!("INSERT INTO users (id, org_id, name, email, active) VALUES ($1, $2, 'Test User', $3, $4)", id, ids.org_id, format!("{id}@example.test"), active).execute(&pool).await.unwrap();
    }
    // Reinsert the original rows into the heap after the new rows. Name-only
    // ordering must not accidentally pass because insertion order matches UUIDs.
    sqlx::query!(
        "UPDATE clients SET name = name WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET name = name WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET name = name WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("UPDATE users SET name = name WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    for (resource, first) in [
        ("clients", ids.client_id),
        ("projects", ids.project_id),
        ("tasks", ids.task_id),
        ("users", ids.user_id),
    ] {
        let mut expected = [first, second];
        expected.sort();
        let mut query = "is_active=true&updated_since=2020-01-01T00%3A00%3A00%2B01%3A00&tag=caf%C3%A9%26a%3Db&tag=two+words".to_owned();
        if resource == "projects" {
            query.push_str(&format!("&client_id={}", ids.client_id));
        }
        walk(&app, resource, &query, &expected).await;
        walk(&app, resource, "is_active=false", &[inactive]).await;
        let future = page(
            &app,
            &format!("/harvest/v2/{resource}?updated_since=2100-01-01T00%3A00%3A00Z"),
        )
        .await;
        // Catalog timestamp limitations are part of the documented subset.
        assert_eq!(
            future["total_entries"],
            if matches!(resource, "tasks" | "users") {
                3
            } else {
                0
            }
        );
    }
    let no_client_projects = page(&app, &format!("/harvest/v2/projects?client_id={second}")).await;
    assert_eq!(no_client_projects["total_entries"], 0);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_time_entries_keep_the_filtered_set_and_date_ties(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let other = seed(&pool, OrgRole::Manager).await;
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.push(time_entry(&pool, &ids, EntryState::Open).await);
    }
    time_entry(&pool, &other, EntryState::Open).await;
    let excluded = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET spent_date = '2026-09-06' WHERE id = $1",
        excluded
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET created_at = '2026-09-07T09:00:00Z' WHERE org_id = $1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    expected.sort_by(|a, b| b.cmp(a));
    let app = signed_in(&pool, ids.user_id).await;
    let query = format!(
        "user_id={}&project_id={}&from=2026-09-07&to=2026-09-07&is_running=false&updated_since=2020-01-01T00%3A00%3A00%2B01%3A00",
        ids.user_id, ids.project_id
    );
    walk(&app, "time_entries", &query, &expected).await;
    for filter in [
        format!("user_id={}", other.user_id),
        format!("project_id={}", other.project_id),
        "from=2026-09-08".to_owned(),
        "to=2026-09-05".to_owned(),
        "is_running=true".to_owned(),
        "updated_since=2100-01-01T00%3A00%3A00Z".to_owned(),
    ] {
        let filtered = page(&app, &format!("/harvest/v2/time_entries?{filter}")).await;
        assert_eq!(filtered["total_entries"], 0, "{filter}");
        assert!(filtered["time_entries"].as_array().unwrap().is_empty());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_rejects_invalid_windows_on_every_collection(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;
    for resource in ["time_entries", "projects", "clients", "tasks", "users"] {
        for query in [
            "page=0",
            "page=-1",
            "per_page=0",
            "per_page=-1",
            "per_page=101",
            "page=9223372036854775807&per_page=100",
            "page=9223372036854775808",
            "page=nope",
        ] {
            let uri = format!("/harvest/v2/{resource}?{query}");
            let (status, body) = request(&app, &uri).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        }
    }
}

#[test]
fn harvest_pagination_total_pages_does_not_overflow() {
    let page =
        HarvestPagination::<Uuid>::new("tasks", Vec::new(), 1, 100, i64::MAX, "/harvest/v2/tasks");
    assert_eq!(page.total_pages, i64::MAX / 100 + 1);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_malformed_filters_are_client_errors(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;
    for uri in [
        "/harvest/v2/time_entries?user_id=not-a-uuid",
        "/harvest/v2/time_entries?project_id=not-a-uuid",
        "/harvest/v2/time_entries?from=2026-02-30",
        "/harvest/v2/time_entries?to=yesterday",
        "/harvest/v2/time_entries?updated_since=not-a-timestamp",
        "/harvest/v2/projects?client_id=not-a-uuid",
        "/harvest/v2/projects?updated_since=not-a-timestamp",
        "/harvest/v2/clients?updated_since=not-a-timestamp",
    ] {
        let (status, body) = request(&app, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_empty_and_out_of_range_pages_keep_the_contract(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;
    for resource in ["time_entries", "projects", "clients", "tasks", "users"] {
        let filter = if resource == "time_entries" {
            "from=2100-01-01"
        } else {
            "is_active=false"
        };
        for (requested_page, per_page) in [(1, 100), (3, 1), (i64::MAX, 1)] {
            let uri = format!(
                "/harvest/v2/{resource}?page={requested_page}&per_page={per_page}&{filter}"
            );
            let body = page(&app, &uri).await;
            assert!(body[resource].as_array().unwrap().is_empty());
            assert_eq!(body["page"], requested_page);
            assert_eq!(body["total_entries"], 0);
            assert_eq!(body["total_pages"], 1);
            assert!(body["next_page"].is_null());
            assert!(body["links"]["next"].is_null());
            assert_eq!(body["links"]["first"], body["links"]["last"]);
            assert_eq!(
                query_pairs(body["links"]["first"].as_str().unwrap()),
                query_pairs(&uri)
            );
            if requested_page > 1 {
                assert_eq!(body["previous_page"], requested_page - 1);
            }
        }
        let first = page(&app, &format!("/harvest/v2/{resource}")).await;
        assert_eq!(first["page"], 1);
        assert_eq!(first["per_page"], 100);
        let beyond = page(&app, &format!("/harvest/v2/{resource}?page=2")).await;
        assert!(beyond[resource].as_array().unwrap().is_empty());
        assert_eq!(beyond["total_entries"], first["total_entries"]);
        assert!(beyond["next_page"].is_null());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_pagination_router_preserves_session_and_tenant_guards(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = seed(&pool, OrgRole::Manager).await;
    let own_entry = time_entry(&pool, &ids, EntryState::Open).await;
    let other_entry = time_entry(&pool, &other, EntryState::Open).await;
    let unsigned = router(pool.clone());
    assert_eq!(
        request(&unsigned, "/harvest/v2/time_entries").await.0,
        StatusCode::UNAUTHORIZED
    );
    let app = signed_in(&pool, ids.user_id).await;
    assert_eq!(
        page(&app, "/harvest/v2/users/me").await["id"],
        ids.user_id.to_string()
    );
    assert_eq!(
        request(&app, "/harvest/v2/users").await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            &format!("/harvest/v2/time_entries?user_id={}", other.user_id)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    walk(&app, "time_entries", "is_running=false", &[own_entry]).await;
    for (resource, own, foreign) in [
        ("time_entries", own_entry, other_entry),
        ("projects", ids.project_id, other.project_id),
        ("clients", ids.client_id, other.client_id),
        ("tasks", ids.task_id, other.task_id),
    ] {
        assert_eq!(
            page(&app, &format!("/harvest/v2/{resource}/{own}")).await["id"],
            own.to_string()
        );
        assert_eq!(
            request(&app, &format!("/harvest/v2/{resource}/{foreign}"))
                .await
                .0,
            StatusCode::NOT_FOUND
        );
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "/harvest/v2/time_entries").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_project_progress_permissions_cover_counts_pages_and_details(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let foreign = seed(&pool, OrgRole::Admin).await;
    let own_entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'project')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 600 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let unassigned = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Hidden','EUR')",
        unassigned,
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    let detail = format!("/harvest/v2/projects/{}", ids.project_id);
    for query in ["", "?per_page=1", "?page=2&per_page=1", "?is_active=true"] {
        let body = page(&app, &format!("/harvest/v2/projects{query}")).await;
        assert_eq!(body["total_entries"], 0, "hidden projects counted");
        assert!(body["projects"].as_array().unwrap().is_empty());
    }
    assert_eq!(request(&app, &detail).await.0, StatusCode::NOT_FOUND);
    // Project privacy never takes away the owner's timesheet identity.
    assert_eq!(
        page(&app, &format!("/harvest/v2/time_entries/{own_entry}")).await["project"]["id"],
        ids.project_id.to_string()
    );
    for lead in [false, true] {
        sqlx::query!(
            "UPDATE assignments SET role = $2 WHERE project_id = $1 AND user_id = $3",
            ids.project_id,
            if lead {
                horae_core::types::ProjectRole::Lead
            } else {
                horae_core::types::ProjectRole::Freelancer
            } as horae_core::types::ProjectRole,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE project_settings SET report_visibility = $2 WHERE project_id = $1",
            ids.project_id,
            if lead { "managers" } else { "project_members" }
        )
        .execute(&pool)
        .await
        .unwrap();
        walk(&app, "projects", "is_active=true", &[ids.project_id]).await;
        let body = page(&app, &detail).await;
        assert_eq!(body["budget"], 10.0);
        assert!(body.get("rate_cents").is_none());
        assert!(body.get("admin_notes").is_none());
        assert_eq!(
            request(
                &app,
                &format!("/harvest/v2/projects/{}", foreign.project_id)
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(request(&app, &detail).await.0, StatusCode::NOT_FOUND);
    for role in [OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            ids.user_id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        walk(
            &app,
            "projects",
            "is_active=true",
            &[unassigned, ids.project_id],
        )
        .await;
        assert_eq!(page(&app, &detail).await["budget"], 10.0);
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "/harvest/v2/projects").await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(request(&app, &detail).await.0, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_task_access_filters_counts_and_preserves_rate_free_history(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let other = seed(&pool, OrgRole::Admin).await;
    let hidden = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name,default_rate_cents) VALUES ($1,$2,'Hidden',8765)",
        hidden,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET default_rate_cents = 1234 WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    assert_eq!(page(&app, "/harvest/v2/tasks").await["total_entries"], 0);
    assert_eq!(
        request(&app, &format!("/harvest/v2/tasks/{}", ids.task_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        ids.project_id,
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    walk(&app, "tasks", "is_active=true", &[ids.task_id]).await;
    for body in [
        page(&app, &format!("/harvest/v2/tasks/{}", ids.task_id)).await,
        page(&app, "/harvest/v2/tasks").await["tasks"][0].clone(),
    ] {
        assert!(
            body.get("default_hourly_rate").is_none(),
            "member received task rate: {body}"
        );
    }
    time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("UPDATE tasks SET active = false WHERE id = $1", ids.task_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        page(&app, "/harvest/v2/tasks?is_active=true").await["total_entries"],
        0
    );
    walk(&app, "tasks", "is_active=false", &[ids.task_id]).await;
    assert_eq!(
        request(&app, &format!("/harvest/v2/tasks/{}", other.task_id))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    for role in [OrgRole::Manager, OrgRole::Admin] {
        sqlx::query!(
            "UPDATE users SET org_role = $2 WHERE id = $1",
            ids.user_id,
            role as OrgRole
        )
        .execute(&pool)
        .await
        .unwrap();
        walk(&app, "tasks", "", &[ids.task_id, hidden]).await;
        assert_eq!(
            page(&app, &format!("/harvest/v2/tasks/{}", ids.task_id)).await["default_hourly_rate"],
            12.34
        );
    }
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(&app, "/harvest/v2/tasks").await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_own_time_omits_private_project_budget_metadata(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_settings (id,org_id,project_id,creator_id,rate_mode) VALUES ($1,$2,$3,$4,'project')", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    sqlx::query!(
        "UPDATE projects SET budget_kind = 'hours', budget_minutes = 600 WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    let detail = format!("/harvest/v2/time_entries/{entry}");
    for body in [
        page(&app, &detail).await,
        page(&app, "/harvest/v2/time_entries").await["time_entries"][0].clone(),
    ] {
        assert_eq!(body["hours"], 1.0);
        for field in ["budgeted", "billable_rate", "cost_rate"] {
            assert!(body.get(field).is_none(), "private field {field}: {body}");
        }
    }
    sqlx::query!(
        "UPDATE project_settings SET report_visibility = 'project_members' WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(page(&app, &detail).await["budgeted"], true);
    sqlx::query!(
        "DELETE FROM assignments WHERE project_id = $1 AND user_id = $2",
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(page(&app, &detail).await.get("budgeted").is_none());
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn harvest_project_cost_overrides_require_admin_without_fabricated_fallback(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        ids.project_id,
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET cost_rate_cents = 1000 WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO project_member_costs (id,org_id,project_id,user_id,cost_rate_cents) VALUES ($1,$2,$3,$4,1500)", Uuid::now_v7(), ids.org_id, ids.project_id, ids.user_id).execute(&pool).await.unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    let detail = format!("/harvest/v2/time_entries/{entry}");
    for body in [
        page(&app, &detail).await,
        page(&app, "/harvest/v2/time_entries").await["time_entries"][0].clone(),
    ] {
        assert!(
            body.get("cost_rate").is_none(),
            "manager received confidential cost: {body}"
        );
    }
    sqlx::query!(
        "UPDATE users SET org_role = 'admin' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(page(&app, &detail).await["cost_rate"], 15.0);
    assert_eq!(
        page(&app, "/harvest/v2/time_entries").await["time_entries"][0]["cost_rate"],
        15.0
    );
    sqlx::query!(
        "DELETE FROM project_member_costs WHERE project_id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role = 'manager' WHERE id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        page(&app, &detail).await["cost_rate"],
        10.0,
        "legacy profile costs stay unchanged"
    );
}
