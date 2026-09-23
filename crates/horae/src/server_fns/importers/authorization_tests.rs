//! Real registered server functions with PostgreSQL-backed session cookies.

use std::{sync::Arc, time::Duration};

use axum::{
    Router,
    body::Body,
    extract::{Path, Request},
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use dioxus::prelude::{
    DioxusRouterExt,
    dioxus_server::{FullstackState, ServerFunction},
};
use dioxus_fullstack::reqwest;
use horae_core::importers::harvest::types::ImportReport;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use super::*;

mod cli;

#[cfg(target_os = "linux")]
mod report_stress;

#[test]
fn request_bound_import_endpoints_are_not_registered() {
    let legacy: Vec<_> = ServerFunction::collect()
        .into_iter()
        .filter(|route| {
            route.path().starts_with("/api/import_harvest_api")
                || route.path().starts_with("/api/import/harvest/csv/")
        })
        .map(|route| route.path().to_owned())
        .collect();
    assert!(legacy.is_empty(), "request-bound import routes: {legacy:?}");
}

#[test]
fn cli_job_endpoints_have_stable_registered_paths() {
    let routes = ServerFunction::collect();
    for path in [
        "/api/import/harvest/connection",
        "/api/import/harvest/start",
        "/api/import/harvest/status",
        "/api/import/harvest/history",
        "/api/import/harvest/cancel",
        "/api/import/harvest/retry",
    ] {
        assert!(
            routes
                .iter()
                .any(|route| route.path() == path && route.method() == axum::http::Method::POST),
            "missing {path}"
        );
    }
}

struct Api {
    base: String,
    client: reqwest::Client,
}

impl Api {
    async fn errors(&self, job_id: Uuid, cookie: Option<&str>) -> reqwest::Response {
        let mut request = self.client.get(format!(
            "{}/api/import/harvest/jobs/{job_id}/errors",
            self.base
        ));
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        request.send().await.unwrap()
    }

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
            let explicit = match name {
                "start_harvest_api_import" => Some("/api/import/harvest/start"),
                "harvest_connection_status" => Some("/api/import/harvest/connection"),
                "get_harvest_import_job" => Some("/api/import/harvest/status"),
                "list_harvest_import_jobs" => Some("/api/import/harvest/history"),
                "cancel_harvest_import_job" => Some("/api/import/harvest/cancel"),
                "retry_harvest_import_job" => Some("/api/import/harvest/retry"),
                _ => None,
            };
            let routes = ServerFunction::collect();
            let matches: Vec<_> = routes
                .iter()
                .filter(|route| {
                    explicit.map_or_else(
                        || route.path().contains(&format!("/{name}")),
                        |path| route.path() == path,
                    )
                })
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
        Some(crate::config::HarvestConfig {
            client_id: "test".into(),
            client_secret: "test".into(),
            redirect_url: "http://localhost/auth/harvest/callback".into(),
            encryption_key_hex: "11".repeat(32),
        }),
        Default::default(),
        None,
    )
    .await;
    let router = Router::new()
        .register_server_functions()
        .route(
            "/api/import/harvest/jobs/{job_id}/errors",
            get(crate::jobs::report::download),
        )
        .route(
            "/test/login/{id}",
            post(|session: Session, Path(id): Path<Uuid>| async move {
                crate::auth::session::set_session_user_id(&session, id)
                    .await
                    .unwrap();
                StatusCode::NO_CONTENT
            }),
        )
        .route(
            "/test/expire",
            post(|session: Session| async move {
                session.set_expiry(Some(tower_sessions::Expiry::AtDateTime(
                    session.expiry_date() - Duration::from_secs(365 * 24 * 60 * 60),
                )));
                session.save().await.unwrap();
                StatusCode::NO_CONTENT
            }),
        )
        .with_state(FullstackState::headless())
        .merge(crate::importers::harvest::callback_router())
        .route(
            "/test/harvest-attempt",
            get(|session: Session| async move {
                axum::Json(
                    session
                        .get::<String>(crate::importers::harvest::OAUTH_STATE_KEY)
                        .await
                        .unwrap()
                        .map(|encoded| {
                            serde_json::from_str::<crate::importers::harvest::OAuthAttempt>(
                                &encoded,
                            )
                            .unwrap()
                        }),
                )
            }),
        )
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
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap(),
    };
    let mut server = tokio::task::JoinSet::new();
    server.spawn(async move { axum::serve(listener, router).await.unwrap() });
    let admin = api.cookie(owner.user_id).await;
    let expired = api.cookie(owner.user_id).await;
    assert_eq!(
        api.client
            .post(format!("{}/test/expire", api.base))
            .header("cookie", &expired)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
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
        (Some(expired.as_str()), StatusCode::UNAUTHORIZED),
        (Some(demoted.as_str()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(api.errors(target, cookie).await.status(), expected);
        for (name, body) in [
            ("harvest_connection_status", json!({})),
            ("harvest_connect_start", json!({})),
            ("harvest_disconnect", json!({})),
            (
                "harvest_change_account",
                json!({"expected_account":"test-account","expected_generation":0,"expected_revision":1}),
            ),
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
    assert_eq!(
        api.call(
            "start_harvest_api_import",
            json!({"mode":"DryRun","sync":"Full"}),
            Some(&admin),
            false
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    crate::importers::harvest::credentials::store(
        &pool,
        owner.org_id,
        &"11".repeat(32),
        "test-account",
        "test-access",
        "test-refresh",
        None,
        None,
    )
    .await
    .unwrap();
    let private_status = api
        .json("harvest_connection_status", json!({}), &admin)
        .await;
    assert_eq!(private_status["account_id"], "test-account");
    assert_eq!(private_status["account_generation"], 0);
    assert_eq!(private_status["connection_revision"], 1);
    assert!(!private_status.to_string().contains("test-access"));
    assert!(!private_status.to_string().contains("test-refresh"));
    let foreign_status = api
        .json(
            "harvest_connection_status",
            json!({"org_id":owner.org_id}),
            &outsider,
        )
        .await;
    assert_eq!(foreign_status["account_id"], Value::Null);
    assert_eq!(api.call("harvest_change_account", json!({"org_id":owner.org_id,"expected_account":"test-account","expected_generation":0,"expected_revision":1}), Some(&outsider), false).await.status(), StatusCode::CONFLICT);
    // Another browser's pending OAuth must not undo Disconnect.
    let oauth_session = api.cookie(owner.user_id).await;
    let authorize = api
        .json("harvest_connect_start", json!({}), &oauth_session)
        .await;
    let authorize = openidconnect::url::Url::parse(authorize.as_str().unwrap()).unwrap();
    let nonce = authorize
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let saved_attempt: Value = api
        .client
        .get(format!("{}/test/harvest-attempt", api.base))
        .header("cookie", &oauth_session)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        saved_attempt["nonce"], nonce,
        "OAuth attempt must persist in its initiating session"
    );
    api.json("harvest_disconnect", json!({}), &admin).await;
    let callback = api
        .client
        .get(format!("{}/auth/harvest/callback", api.base))
        .header("cookie", &oauth_session)
        .query(&[("state", nonce.as_str()), ("code", "must-not-be-exchanged")])
        .send()
        .await
        .unwrap();
    assert_eq!(callback.status(), StatusCode::CONFLICT);
    assert!(
        callback
            .text()
            .await
            .unwrap()
            .contains("connection changed")
    );
    assert!(
        !crate::importers::harvest::account_switch::status(&pool, owner.org_id, true)
            .await
            .unwrap()
            .connected
    );
    crate::importers::harvest::credentials::store(
        &pool,
        owner.org_id,
        &"11".repeat(32),
        "test-account",
        "test-access",
        "test-refresh",
        None,
        None,
    )
    .await
    .unwrap();

    // Exercise a real successful change in another organization without disturbing owner jobs.
    crate::importers::harvest::credentials::store(
        &pool,
        foreign.org_id,
        &"11".repeat(32),
        "foreign-account",
        "foreign-access",
        "foreign-refresh",
        None,
        None,
    )
    .await
    .unwrap();
    let old_authorize = api
        .json("harvest_connect_start", json!({}), &outsider)
        .await;
    let old_authorize = openidconnect::url::Url::parse(old_authorize.as_str().unwrap()).unwrap();
    let old_nonce = old_authorize
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    api.json(
        "harvest_change_account",
        json!({"expected_account":"foreign-account","expected_generation":0,"expected_revision":1}),
        &outsider,
    )
    .await;
    let changed = api
        .json("harvest_connection_status", json!({}), &outsider)
        .await;
    assert_eq!(changed["account_generation"], 1);
    assert_eq!(changed["account_id"], Value::Null);
    let stale = api
        .client
        .get(format!("{}/auth/harvest/callback", api.base))
        .header("cookie", &outsider)
        .query(&[
            ("state", old_nonce.as_str()),
            ("code", "must-not-be-exchanged"),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        api.call(
            "start_harvest_api_import",
            json!({"mode":"DryRun","sync":"Full"}),
            Some(&outsider),
            false
        )
        .await
        .status(),
        StatusCode::CONFLICT
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
        assert_eq!(
            serde_json::to_value(&started).unwrap()["retry_availability"],
            "unavailable_state"
        );
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
        assert_eq!(cancelled["retry_availability"], "available");
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
    // Exercise the real download route with an archive spanning UTF-8/JSON
    // boundaries, not just a small inline fixture.
    use horae_core::importers::harvest::types::{EntityType, RowOutcome, SourceKind};
    let (lease, _stop) = crate::jobs::claim_lease_for_test(&pool).await;
    let archived_job = crate::jobs::list(&pool, owner.org_id, 20, None)
        .await
        .unwrap()
        .into_iter()
        .find(|job| job.status == "running")
        .unwrap();
    let mut report = ImportReport::new(SourceKind::Csv, ImportMode::DryRun);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV line 3".into(),
            reason: "界,\"quoted\"\n".repeat(20_000),
        },
    );
    let mut expected = report.row_errors.clone();
    let mut rolled_back = report.clone();
    let mut tx = pool.begin().await.unwrap();
    lease
        .archive_report(&mut tx, &mut rolled_back)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(
        crate::jobs::report::chunks(&pool, owner.org_id, archived_job.id, 0, 1)
            .await
            .is_err()
    );
    let mut tx = pool.begin().await.unwrap();
    lease.archive_report(&mut tx, &mut report).await.unwrap();
    assert!(report.archived_error_chunks() > 1);
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV line 4".into(),
            reason: "inline tail".into(),
        },
    );
    expected.extend(report.row_errors.clone());
    assert!(
        lease
            .complete(&mut tx, &serde_json::to_value(&report).unwrap(), 2)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    // A stale append must not change the already completed archive.
    rolled_back.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "CSV line 4".into(),
            reason: "x".repeat(20_000),
        },
    );
    let mut tx = pool.begin().await.unwrap();
    assert!(
        lease
            .archive_report(&mut tx, &mut rolled_back)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        api.errors(archived_job.id, Some(&outsider)).await.status(),
        StatusCode::NOT_FOUND
    );
    let response = api.errors(archived_job.id, Some(&admin)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "application/x-ndjson");
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = response.bytes().await.unwrap();
    let actual = serde_json::Deserializer::from_slice(&bytes)
        .into_iter::<horae_core::importers::harvest::types::RowError>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(actual, expected);
    let end = i64::try_from(report.archived_error_chunks()).unwrap();
    assert!(
        crate::jobs::report::chunks(&pool, owner.org_id, archived_job.id, end, end + 1)
            .await
            .is_err()
    );
    cli::exercise(
        &api,
        &pool,
        &owner,
        &admin,
        &[&member, &manager, &inactive, &demoted, &missing, &expired],
        &outsider,
        (archived_job.id, &bytes),
    )
    .await;
    server.abort_all();
    while server.join_next().await.is_some() {}
}
