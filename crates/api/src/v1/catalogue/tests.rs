//! `/projects`, `/tasks`, `/clients` and `/people` and their `/{id}`: every
//! answer checked against the document, cursor walks, refused cursors, and
//! what a member's token may see.

use axum::Router;
use axum::http::StatusCode;
use chrono::{SecondsFormat, TimeDelta, Utc};
use horae_core::permissions::catalog::BuiltInProfile;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::CursorKey;
use crate::cursor::Walk;
use crate::testing::{SeedIds, add_person, seed};
use crate::v1::testing::{get, router, send, token};

async fn add_project(pool: &PgPool, ids: &SeedIds, name: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, rate_cents)
         VALUES ($1, $2, $3, $4, 'EUR', 9000)",
        id,
        ids.org_id,
        ids.client_id,
        name,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// A project in `currency` with a rate and a money budget, stored in
/// hundredths as every amount is.
async fn add_priced_project(
    pool: &PgPool,
    ids: &SeedIds,
    currency: &str,
    rate_hundredths: i64,
    budget_hundredths: i64,
) -> Uuid {
    let id = add_project(pool, ids, currency).await;
    sqlx::query!(
        "UPDATE projects SET currency = $2, rate_cents = $3, budget_kind = 'amount',
                budget_amount_cents = $4
         WHERE id = $1",
        id,
        currency,
        rate_hundredths,
        budget_hundredths,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Amounts are served in the currency's ISO 4217 minor unit: a two-decimal
/// currency's hundredths unchanged, a zero-decimal one's whole units, a
/// three-decimal one's thousandths.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn project_amounts_are_in_the_currencys_minor_unit(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);
    for (currency, rate, budget, rate_minor, budget_minor) in [
        ("EUR", 1_210, 500_000, 1_210, 500_000),
        ("JPY", 800_000, 50_000_000, 8_000, 500_000),
        ("BHD", 1_050, 500_000, 10_500, 5_000_000),
    ] {
        let id = add_priced_project(&pool, &admin, currency, rate, budget).await;
        let (status, project) = read(
            &router,
            "getProject",
            &format!("/api/v1/projects/{id}"),
            &secret,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{currency}: {project}");
        assert_eq!(
            project["hourly_rate"],
            json!({ "amount_minor": rate_minor, "currency": currency })
        );
        assert_eq!(
            project["budget_amount"],
            json!({ "amount_minor": budget_minor, "currency": currency })
        );
    }
}

/// A stored zero-decimal amount with hundredths has no exact value in the
/// currency's minor unit: the read fails with a 500 rather than round it.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_inexact_stored_amount_is_refused_not_rounded(pool: PgPool) {
    let admin = seed(&pool, BuiltInProfile::Administrator).await;
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);
    let id = add_priced_project(&pool, &admin, "JPY", 800_050, 0).await;
    for (operation, uri) in [
        ("getProject", format!("/api/v1/projects/{id}")),
        ("listProjects", "/api/v1/projects".to_owned()),
    ] {
        let (status, problem) = read(&router, operation, &uri, &secret).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{operation}");
        assert_eq!(problem["code"], "internal", "{operation}: {problem}");
    }
}

/// An administrator, and a member who works on the seeded project and may
/// track its task. The client has an address and tax id, the project a rate
/// and a money budget, and a second project belongs to nobody.
async fn organization(pool: &PgPool) -> (SeedIds, SeedIds, Uuid) {
    let admin = seed(pool, BuiltInProfile::Administrator).await;
    let member = add_person(pool, &admin, BuiltInProfile::Member).await;
    sqlx::query!(
        "UPDATE clients SET address = 'Street', tax_id = 'VAT' WHERE id = $1",
        admin.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET rate_cents = 12000, budget_kind = 'amount', budget_amount_cents = 500000
         WHERE id = $1",
        admin.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        admin.project_id,
        admin.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
        Uuid::now_v7(),
        admin.project_id,
        member.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let elsewhere = add_project(pool, &admin, "Elsewhere").await;
    (admin, member, elsewhere)
}

async fn read(router: &Router, operation: &str, uri: &str, secret: &str) -> (StatusCode, Value) {
    let (status, _, body) = send(router, operation, get(uri, Some(secret))).await;
    (status, body)
}

fn ids(page: &Value) -> Vec<Value> {
    page["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["id"].clone())
        .collect()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_catalogue_read_answers_as_documented(pool: PgPool) {
    let (admin, _, _) = organization(&pool).await;
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);
    for (list, one, uri, id) in [
        (
            "listProjects",
            "getProject",
            "/api/v1/projects",
            admin.project_id,
        ),
        ("listTasks", "getTask", "/api/v1/tasks", admin.task_id),
        (
            "listClients",
            "getClient",
            "/api/v1/clients",
            admin.client_id,
        ),
        ("listPeople", "getPerson", "/api/v1/people", admin.user_id),
    ] {
        let (status, page) = read(&router, list, uri, &secret).await;
        assert_eq!(status, StatusCode::OK, "{list}: {page}");
        assert!(ids(&page).contains(&json!(id)), "{list}: {page}");
        assert_eq!(page["next_cursor"], Value::Null);

        let (status, record) = read(&router, one, &format!("{uri}/{id}"), &secret).await;
        assert_eq!(status, StatusCode::OK, "{one}: {record}");
        assert_eq!(record["id"], json!(id));
    }

    let (_, project) = read(
        &router,
        "getProject",
        &format!("/api/v1/projects/{}", admin.project_id),
        &secret,
    )
    .await;
    assert_eq!(
        project,
        json!({
            "id": admin.project_id, "client_id": admin.client_id, "name": "Widget", "code": null,
            "currency": "EUR", "project_type": "time_and_materials", "budget_kind": "amount",
            "budget_amount": { "amount_minor": 500000, "currency": "EUR" },
            "hourly_rate": { "amount_minor": 12000, "currency": "EUR" },
            "starts_on": null, "ends_on": null, "active": true,
            "created_at": project["created_at"], "updated_at": project["updated_at"],
        })
    );
    let (_, client) = read(
        &router,
        "getClient",
        &format!("/api/v1/clients/{}", admin.client_id),
        &secret,
    )
    .await;
    assert_eq!(
        (&client["address"], &client["tax_id"]),
        (&json!("Street"), &json!("VAT"))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_cursor_walks_every_record_once(pool: PgPool) {
    let (admin, _, elsewhere) = organization(&pool).await;
    let mut expected = vec![json!(admin.project_id), json!(elsewhere)];
    for name in ["C", "D", "E"] {
        expected.push(json!(add_project(&pool, &admin, name).await));
    }
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);

    let mut seen = Vec::new();
    let mut pages = 0;
    let mut uri = "/api/v1/projects?limit=2".to_owned();
    loop {
        let (status, page) = read(&router, "listProjects", &uri, &secret).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        pages += 1;
        seen.extend(ids(&page));
        match page["next_cursor"].as_str() {
            Some(cursor) => uri = format!("/api/v1/projects?limit=2&cursor={cursor}"),
            None => break,
        }
    }
    assert_eq!((seen, pages), (expected, 3));
}

fn assert_invalid_cursor(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_cursor");
    assert_eq!(body["field"], "cursor");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_cursor_is_refused_when_tampered_expired_or_used_elsewhere(pool: PgPool) {
    let (admin, _, _) = organization(&pool).await;
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);
    let (_, page) = read(&router, "listProjects", "/api/v1/projects?limit=1", &secret).await;
    let cursor = page["next_cursor"].as_str().unwrap().to_owned();

    let mut tampered = cursor.clone().into_bytes();
    let last = tampered.len() - 1;
    tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(tampered).unwrap();
    let key = CursorKey::new([1; 32]);
    let expired = Walk {
        key: &key,
        list: "projects",
        filters: "client_id=None;active=true;updated_since=None",
    }
    .encode(admin.project_id, Utc::now() - TimeDelta::hours(25));
    for (operation, uri) in [
        (
            "listProjects",
            format!("/api/v1/projects?limit=1&cursor={tampered}"),
        ),
        (
            "listProjects",
            format!("/api/v1/projects?limit=1&cursor={expired}"),
        ),
        (
            "listProjects",
            format!("/api/v1/projects?limit=1&active=false&cursor={cursor}"),
        ),
        (
            "listClients",
            format!("/api/v1/clients?limit=1&cursor={cursor}"),
        ),
    ] {
        let (status, body) = read(&router, operation, &uri, &secret).await;
        assert_invalid_cursor(status, &body);
    }
    let fresh = Walk {
        key: &key,
        list: "projects",
        filters: "client_id=None;active=true;updated_since=None",
    }
    .encode(admin.project_id, Utc::now());
    let (status, body) = read(
        &router,
        "listProjects",
        &format!("/api/v1/projects?limit=1&cursor={fresh}"),
        &secret,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "the walk's filters are as signed: {body}"
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_members_token_sees_what_the_member_sees_in_the_web_app(pool: PgPool) {
    let (admin, member, elsewhere) = organization(&pool).await;
    let untracked = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Untracked')",
        untracked,
        admin.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (secret, _) = token(&pool, &member, "sync").await;
    let router = router(&pool, 100);

    let (_, projects) = read(&router, "listProjects", "/api/v1/projects", &secret).await;
    assert_eq!(ids(&projects), [json!(admin.project_id)]);
    let project = &projects["data"][0];
    for hidden in ["hourly_rate", "budget_amount"] {
        assert!(project.get(hidden).is_none(), "{hidden} in {project}");
    }
    let (_, tasks) = read(&router, "listTasks", "/api/v1/tasks", &secret).await;
    assert_eq!(ids(&tasks), [json!(admin.task_id)]);
    let (_, clients) = read(&router, "listClients", "/api/v1/clients", &secret).await;
    assert_eq!(ids(&clients), [json!(admin.client_id)]);
    let client = &clients["data"][0];
    for hidden in ["address", "tax_id"] {
        assert!(client.get(hidden).is_none(), "{hidden} in {client}");
    }

    for (operation, uri) in [
        ("getProject", format!("/api/v1/projects/{elsewhere}")),
        ("getTask", format!("/api/v1/tasks/{untracked}")),
        ("getProject", format!("/api/v1/projects/{}", Uuid::now_v7())),
    ] {
        let (status, body) = read(&router, operation, &uri, &secret).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{uri}: {body}");
        assert_eq!(body["code"], "not_found");
    }
    let (status, body) = read(
        &router,
        "listPeople",
        "/api/v1/people?active=false",
        &secret,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "forbidden");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn updated_since_returns_only_changed_records(pool: PgPool) {
    let (admin, member, _) = organization(&pool).await;
    let since = Utc::now();
    sqlx::query!(
        "UPDATE users SET name = 'Renamed' WHERE id = $1",
        member.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (secret, _) = token(&pool, &admin, "sync").await;
    let since = since.to_rfc3339_opts(SecondsFormat::Micros, true);
    let (status, page) = read(
        &router(&pool, 100),
        "listPeople",
        &format!("/api/v1/people?updated_since={since}"),
        &secret,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(ids(&page), [json!(member.user_id)]);
    assert_eq!(page["data"][0]["name"], "Renamed");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn malformed_parameters_are_problems(pool: PgPool) {
    let (admin, _, _) = organization(&pool).await;
    let (secret, _) = token(&pool, &admin, "sync").await;
    let router = router(&pool, 100);
    for (operation, uri, status, code) in [
        (
            "getProject",
            "/api/v1/projects/nope",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            "listTasks",
            "/api/v1/tasks?updated_since=yesterday",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            "listClients",
            "/api/v1/clients?active=maybe",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            "listPeople",
            "/api/v1/people?limit=0",
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
        ),
        (
            "listPeople",
            "/api/v1/people?limit=1001",
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid",
        ),
    ] {
        let (answered, body) = read(&router, operation, uri, &secret).await;
        assert_eq!(
            (answered, body["code"].as_str()),
            (status, Some(code)),
            "{uri}: {body}"
        );
    }
    let (status, _) = read(
        &router,
        "listProjects",
        "/api/v1/projects",
        "horae_pat_nope",
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
