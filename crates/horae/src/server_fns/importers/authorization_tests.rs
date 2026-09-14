//! Real registered server functions with PostgreSQL-backed session cookies.

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    extract::{Path, Request},
    http::StatusCode,
    middleware,
    routing::post,
};
use dioxus::prelude::{
    DioxusRouterExt,
    dioxus_server::{FullstackState, ServerFunction},
};
use dioxus_fullstack::reqwest;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use super::*;

struct Api {
    base: String,
    client: reqwest::Client,
}

impl Api {
    async fn cookie(&self, id: Uuid) -> String {
        let response = self
            .client
            .post(format!("{}/test/login/{id}", self.base))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    }

    async fn call(
        &self,
        name: &str,
        body: Value,
        cookie: Option<&str>,
        unread_upload: bool,
    ) -> reqwest::Response {
        let path = if name == "csv" {
            format!(
                "/api/import/harvest/csv-job/{}",
                body["mode"].as_str().unwrap()
            )
        } else {
            let routes = ServerFunction::collect();
            let matches: Vec<_> = routes
                .iter()
                .filter(|route| route.path().contains(&format!("/{name}")))
                .collect();
            assert_eq!(matches.len(), 1, "registered endpoint {name}");
            assert_eq!(matches[0].method(), axum::http::Method::POST);
            matches[0].path().to_owned()
        };
        let mut request = self.client.post(format!("{}{path}", self.base));
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        if name == "csv" {
            request = request
                .header("X-Horae-Import", "csv")
                .body("Date,Client,Project,Task,Hours,Email\n");
            if unread_upload {
                request = request.header("X-Test-Unread-Upload", "true");
            }
        } else {
            request = request
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        request.send().await.unwrap()
    }

    async fn json(&self, name: &str, body: Value, cookie: &str) -> Value {
        let response = self.call(name, body, Some(cookie), false).await;
        let status = response.status();
        let text = response.text().await.unwrap();
        assert_eq!(status, StatusCode::OK, "{name}: {text}");
        serde_json::from_str(&text).unwrap()
    }
}

async fn user(pool: &PgPool, org_id: Uuid, role: OrgRole) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, 'Test User', $4)",
        id,
        org_id,
        format!("{id}@test.com"),
        role as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

// Keep the HTTP matrix in one test: production AppState is initialized once per
// process. Other importer tests use their own pools without that singleton.
#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn job_endpoints_enforce_session_role_and_organization(pool: PgPool) {
    let owner = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    let foreign = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    crate::state::init_state(
        pool.clone(),
        Arc::new(crate::plugin::PluginRegistry::empty()),
        None,
        None,
        Default::default(),
    )
    .await;
    let router = Router::new()
        .register_server_functions()
        .route(
            "/test/login/{id}",
            post(|session: Session, Path(id): Path<Uuid>| async move {
                crate::auth::session::set_session_user_id(&session, id)
                    .await
                    .unwrap();
                StatusCode::NO_CONTENT
            }),
        )
        .with_state(FullstackState::headless())
        .layer(middleware::from_fn(
            |request: Request, next: middleware::Next| async move {
                let request = if request.headers().contains_key("X-Test-Unread-Upload") {
                    request.map(|_| {
                        Body::from_stream(futures_util::stream::poll_fn(
                            |_| -> std::task::Poll<
                                Option<Result<axum::body::Bytes, std::io::Error>>,
                            > {
                                panic!("unauthorized CSV job read its upload");
                            },
                        ))
                    })
                } else {
                    request
                };
                next.run(request).await
            },
        ))
        .layer(
            crate::auth::make_session_layer(pool.clone(), false)
                .await
                .unwrap(),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let api = Api {
        base: format!("http://{}", listener.local_addr().unwrap()),
        client: reqwest::Client::builder()
            .cookie_store(false)
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
    };
    let mut server = tokio::task::JoinSet::new();
    server.spawn(async move { axum::serve(listener, router).await.unwrap() });
    let admin = api.cookie(owner.user_id).await;
    let outsider = api.cookie(foreign.user_id).await;
    let member = api
        .cookie(user(&pool, owner.org_id, OrgRole::Member).await)
        .await;
    let manager = api
        .cookie(user(&pool, owner.org_id, OrgRole::Manager).await)
        .await;
    let inactive_id = user(&pool, owner.org_id, OrgRole::Admin).await;
    let inactive = api.cookie(inactive_id).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", inactive_id)
        .execute(&pool)
        .await
        .unwrap();
    let missing = api.cookie(Uuid::now_v7()).await;
    let demoted_id = user(&pool, owner.org_id, OrgRole::Admin).await;
    let demoted = api.cookie(demoted_id).await;
    sqlx::query!(
        "UPDATE users SET org_role = $2 WHERE id = $1",
        demoted_id,
        OrgRole::Member as OrgRole
    )
    .execute(&pool)
    .await
    .unwrap();

    let target = Uuid::now_v7();
    for (cookie, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(member.as_str()), StatusCode::FORBIDDEN),
        (Some(manager.as_str()), StatusCode::FORBIDDEN),
        (Some(inactive.as_str()), StatusCode::UNAUTHORIZED),
        (Some(missing.as_str()), StatusCode::UNAUTHORIZED),
        (Some(demoted.as_str()), StatusCode::FORBIDDEN),
    ] {
        for (name, body) in [
            (
                "start_harvest_api_import",
                json!({"mode":"DryRun","sync":"Full"}),
            ),
            (
                "start_harvest_api_import",
                json!({"mode":"Commit","sync":"Incremental"}),
            ),
            ("csv", json!({"mode":"DryRun"})),
            ("csv", json!({"mode":"Commit"})),
            ("get_harvest_import_job", json!({"job_id":target})),
            ("list_harvest_import_jobs", json!({"before":null})),
            ("cancel_harvest_import_job", json!({"job_id":target})),
            ("retry_harvest_import_job", json!({"job_id":target})),
        ] {
            let response = api.call(name, body, cookie, true).await;
            let status = response.status();
            let text = response.text().await.unwrap();
            assert_eq!(status, expected, "{name}: {text}");
        }
    }
    assert!(
        crate::jobs::list(&pool, owner.org_id, 100, None)
            .await
            .unwrap()
            .is_empty()
    );
    for name in ["start_harvest_api_import", "csv"] {
        let started: crate::models::JobStatus = serde_json::from_value(
            api.json(
                name,
                json!({"mode":"DryRun","sync":"Full","org_id":foreign.org_id}),
                &admin,
            )
            .await,
        )
        .unwrap();
        let id = started.id;
        assert_eq!(started.status, "queued");
        assert_eq!(started.processed_count, 0);
        assert!(started.report.is_some());
        assert!(
            crate::jobs::status(&pool, owner.org_id, id)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            crate::jobs::status(&pool, foreign.org_id, id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            api.json("get_harvest_import_job", json!({"job_id":id}), &outsider)
                .await,
            Value::Null
        );
        assert_eq!(
            api.json(
                "list_harvest_import_jobs",
                json!({"before":null}),
                &outsider
            )
            .await,
            json!([])
        );
        assert_eq!(
            api.json("list_harvest_import_jobs", json!({"before":id}), &outsider)
                .await,
            json!([])
        );
        assert_eq!(
            api.call(
                "cancel_harvest_import_job",
                json!({"job_id":id}),
                Some(&outsider),
                false
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            api.json("get_harvest_import_job", json!({"job_id":id}), &admin)
                .await["status"],
            "queued"
        );
        let cancelled = api
            .json("cancel_harvest_import_job", json!({"job_id":id}), &admin)
            .await;
        assert_eq!(cancelled["id"], json!(id));
        assert_eq!(cancelled["status"], "cancelled");
        assert!(cancelled.get("payload").is_none());
        assert!(cancelled.get("checkpoint").is_none());
        assert_eq!(
            api.call(
                "retry_harvest_import_job",
                json!({"job_id":id}),
                Some(&outsider),
                false
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            api.json("get_harvest_import_job", json!({"job_id":id}), &admin)
                .await["status"],
            "cancelled"
        );
        let retried = api
            .json("retry_harvest_import_job", json!({"job_id":id}), &admin)
            .await;
        assert_eq!(retried["id"], json!(id));
        assert_eq!(retried["status"], "queued");
        assert_eq!(
            api.json("get_harvest_import_job", json!({"job_id":id}), &admin)
                .await["status"],
            "queued"
        );
    }
    let history = api
        .json("list_harvest_import_jobs", json!({"before":null}), &admin)
        .await;
    assert_eq!(history.as_array().unwrap().len(), 2);
    for limit in [0, 1] {
        let limited = api
            .json(
                "list_harvest_import_jobs",
                json!({"before":null,"limit":limit}),
                &admin,
            )
            .await;
        assert_eq!(
            limited.as_array().unwrap(),
            &history.as_array().unwrap()[..1]
        );
    }
    let (lease, stop) = crate::jobs::claim_lease_for_test(&pool).await;
    let running = crate::jobs::list(&pool, owner.org_id, 20, None)
        .await
        .unwrap()
        .into_iter()
        .find(|job| job.status == "running")
        .unwrap();
    let accepted = api
        .json(
            "cancel_harvest_import_job",
            json!({"job_id":running.id}),
            &admin,
        )
        .await;
    assert_eq!(accepted["id"], json!(running.id));
    assert_eq!(accepted["status"], "running");
    assert_eq!(accepted["phase"], "cancelling");
    tokio::time::timeout(
        Duration::from_secs(10),
        crate::jobs::run_claimed(&pool, &lease, stop, lease.run(std::future::pending())),
    )
    .await
    .expect("cooperative cancellation must be acknowledged")
    .unwrap();
    assert_eq!(
        api.json(
            "get_harvest_import_job",
            json!({"job_id":running.id}),
            &admin
        )
        .await["status"],
        "cancelled"
    );
    server.abort_all();
    while server.join_next().await.is_some() {}
}
