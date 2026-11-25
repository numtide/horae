//! OAuth access tokens on every `/api/v1` operation (spec 024 SC-003): for a
//! member, a project manager and an administrator, and for every scope set
//! an app may hold, a request gets through exactly when the scopes cover the
//! operation, and then answers as the person's personal access token does
//! (FR-064). Every answer is checked against the document by `send`.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ops::oauth::testing::{REDIRECT_URI, all_scopes, approve, exchange, pending, register};
use crate::testing::{SeedIds, add_person, seed};
use crate::v1::testing::{call, get, json, router, send, token};

/// Every scope set an app may be granted: non-empty, and `time:write`
/// always with `time:read`.
fn scope_sets() -> Vec<ScopeSet> {
    (1..8u8)
        .map(|bits| {
            let scopes: Vec<Scope> = Scope::ALL
                .into_iter()
                .enumerate()
                .filter(|(index, _)| bits & (1 << index) != 0)
                .map(|(_, scope)| scope)
                .collect();
            ScopeSet::of(&scopes)
        })
        .filter(|scopes| *scopes == scopes.normalize())
        .collect()
}

/// An administrator, a project manager and a member, all assigned to the
/// seeded project and able to track its task, and each with one entry.
struct Org {
    people: Vec<(BuiltInProfile, SeedIds)>,
    app: crate::ops::oauth::testing::App,
}

async fn org(pool: &PgPool) -> Org {
    let admin = seed(pool, BuiltInProfile::Administrator).await;
    let manager = add_person(pool, &admin, BuiltInProfile::ProjectManager).await;
    let member = add_person(pool, &admin, BuiltInProfile::Member).await;
    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable) VALUES ($1, $2, true)",
        admin.project_id,
        admin.task_id
    )
    .execute(pool)
    .await
    .unwrap();
    for person in [&admin, &manager, &member] {
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            admin.project_id,
            person.user_id
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let app = register(pool, &admin, "Widget", ClientKind::Public, all_scopes()).await;
    Org {
        people: vec![
            (BuiltInProfile::Administrator, admin),
            (BuiltInProfile::ProjectManager, manager),
            (BuiltInProfile::Member, member),
        ],
        app,
    }
}

/// An access token of `person` for `app`, granted `scopes`.
async fn oauth_token(
    pool: &PgPool,
    app: &crate::ops::oauth::testing::App,
    person: &SeedIds,
    scopes: ScopeSet,
) -> String {
    let (_, code) = approve(pool, person, &pending(app, REDIRECT_URI, scopes)).await;
    let tokens = exchange(pool, app, &code, REDIRECT_URI).await.unwrap();
    tokens.access_token.expose().to_owned()
}

fn entry_body(ids: &SeedIds) -> Value {
    json!({
        "project_id": ids.project_id,
        "task_id": ids.task_id,
        "spent_date": "2026-09-08",
        "minutes": 90,
    })
}

/// The reads, by operation, each with the scope it declares (`None`: any).
fn reads(ids: &SeedIds, entry: Uuid) -> Vec<(&'static str, String, Option<Scope>)> {
    use Scope::{CatalogRead, TimeRead};
    vec![
        ("getMe", "/api/v1/me".into(), None),
        (
            "listTrackable",
            "/api/v1/me/trackable".into(),
            Some(CatalogRead),
        ),
        ("getRunningTimer", "/api/v1/me/timer".into(), Some(TimeRead)),
        (
            "listTimeEntries",
            "/api/v1/time_entries".into(),
            Some(TimeRead),
        ),
        (
            "getTimeEntry",
            format!("/api/v1/time_entries/{entry}"),
            Some(TimeRead),
        ),
        ("listProjects", "/api/v1/projects".into(), Some(CatalogRead)),
        (
            "getProject",
            format!("/api/v1/projects/{}", ids.project_id),
            Some(CatalogRead),
        ),
        ("listTasks", "/api/v1/tasks".into(), Some(CatalogRead)),
        (
            "getTask",
            format!("/api/v1/tasks/{}", ids.task_id),
            Some(CatalogRead),
        ),
        ("listClients", "/api/v1/clients".into(), Some(CatalogRead)),
        (
            "getClient",
            format!("/api/v1/clients/{}", ids.client_id),
            Some(CatalogRead),
        ),
        ("listPeople", "/api/v1/people".into(), Some(CatalogRead)),
        (
            "getPerson",
            format!("/api/v1/people/{}", ids.user_id),
            Some(CatalogRead),
        ),
    ]
}

async fn create_entry(router: &Router, secret: &str, ids: &SeedIds) -> Uuid {
    let request = json("POST", "/api/v1/time_entries", secret, &entry_body(ids));
    let (status, _, body) = send(router, "createTimeEntry", request).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["id"].as_str().unwrap().parse().unwrap()
}

fn assert_insufficient_scope(
    status: StatusCode,
    headers: &axum::http::HeaderMap,
    body: &Value,
    needed: Scope,
) {
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["code"], "insufficient_scope");
    assert_eq!(
        headers[header::WWW_AUTHENTICATE],
        format!(
            r#"Bearer error="insufficient_scope", scope="{}""#,
            needed.as_str()
        )
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reads_answer_as_the_personal_token_when_the_scopes_cover_them(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 100_000);
    for (profile, person) in &org.people {
        let (personal, _) = token(&pool, person, "script").await;
        let entry = create_entry(&router, &personal, person).await;
        let mut expected = Vec::new();
        for (operation, uri, _) in reads(person, entry) {
            let (status, _, body) = send(&router, operation, get(&uri, Some(&personal))).await;
            expected.push((status, body));
        }
        for scopes in scope_sets() {
            let secret = oauth_token(&pool, &org.app, person, scopes).await;
            for ((operation, uri, needed), (personal_status, personal_body)) in
                reads(person, entry).into_iter().zip(&expected)
            {
                let (status, headers, body) =
                    send(&router, operation, get(&uri, Some(&secret))).await;
                let context = format!("{profile:?} {scopes:?} {operation}: {body}");
                match needed {
                    Some(scope) if !scopes.normalize().contains(scope) => {
                        assert_insufficient_scope(status, &headers, &body, scope);
                    }
                    _ => {
                        assert_eq!(status, *personal_status, "{context}");
                        assert_ne!(body["code"], "insufficient_scope", "{context}");
                        // Scopes never let more through than the person may see.
                        if let (Some(mine), Some(theirs)) =
                            (body["data"].as_array(), personal_body["data"].as_array())
                        {
                            assert!(mine.len() <= theirs.len(), "{context}");
                        }
                    }
                }
            }
        }
    }
}

/// Every write needs `time:write`. Without it nothing changes; with it, the
/// same sequence answers as it does for the person's personal token.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn writes_need_time_write_and_then_answer_as_the_personal_token(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 100_000);
    for (profile, person) in &org.people {
        let (personal, _) = token(&pool, person, "script").await;
        let entry = create_entry(&router, &personal, person).await;
        for scopes in scope_sets()
            .into_iter()
            .filter(|scopes| !scopes.contains(Scope::TimeWrite))
        {
            let secret = oauth_token(&pool, &org.app, person, scopes).await;
            for (operation, request) in writes(&secret, person, entry) {
                let (status, headers, body) = send(&router, operation, request).await;
                assert_insufficient_scope(status, &headers, &body, Scope::TimeWrite);
            }
        }
        let expected = write_sequence(&router, &personal, person).await;
        let time = ScopeSet::of(&[Scope::TimeWrite]).normalize();
        let secret = oauth_token(&pool, &org.app, person, time).await;
        assert_eq!(
            write_sequence(&router, &secret, person).await,
            expected,
            "{profile:?}"
        );
    }
}

fn writes(secret: &str, ids: &SeedIds, entry: Uuid) -> Vec<(&'static str, Request<Body>)> {
    let uri = format!("/api/v1/time_entries/{entry}");
    let timer = json!({ "project_id": ids.project_id, "task_id": ids.task_id });
    vec![
        (
            "createTimeEntry",
            json("POST", "/api/v1/time_entries", secret, &entry_body(ids)),
        ),
        (
            "updateTimeEntry",
            json("PATCH", &uri, secret, &json!({ "minutes": 30 })),
        ),
        ("deleteTimeEntry", call("DELETE", &uri, secret)),
        ("stopTimer", call("POST", &format!("{uri}/stop"), secret)),
        (
            "restartTimer",
            call("POST", &format!("{uri}/restart"), secret),
        ),
        ("startTimer", json("POST", "/api/v1/timers", secret, &timer)),
    ]
}

/// Creates, changes, times and deletes entries of `ids`' person, and
/// answers each step's status.
async fn write_sequence(router: &Router, secret: &str, ids: &SeedIds) -> Vec<StatusCode> {
    let mut statuses = Vec::new();
    let mut step = async |operation: &str, request: Request<Body>| {
        let (status, _, body) = send(router, operation, request).await;
        statuses.push(status);
        body
    };
    let created = step(
        "createTimeEntry",
        json("POST", "/api/v1/time_entries", secret, &entry_body(ids)),
    )
    .await;
    let entry = format!("/api/v1/time_entries/{}", created["id"].as_str().unwrap());
    step(
        "updateTimeEntry",
        json("PATCH", &entry, secret, &json!({ "minutes": 30 })),
    )
    .await;
    let timer = json!({ "project_id": ids.project_id, "task_id": ids.task_id });
    let started = step("startTimer", json("POST", "/api/v1/timers", secret, &timer)).await;
    let running = format!("/api/v1/time_entries/{}", started["id"].as_str().unwrap());
    step(
        "stopTimer",
        call("POST", &format!("{running}/stop"), secret),
    )
    .await;
    step(
        "restartTimer",
        call("POST", &format!("{running}/restart"), secret),
    )
    .await;
    step(
        "stopTimer",
        call("POST", &format!("{running}/stop"), secret),
    )
    .await;
    step("deleteTimeEntry", call("DELETE", &running, secret)).await;
    step("deleteTimeEntry", call("DELETE", &entry, secret)).await;
    statuses
}

/// A picker needs no write access: `catalog:read` alone shows a member the
/// pairs and tasks they track on, as their personal token does, and refuses
/// every time write. A stored state without `TimeWriteOwn` (which the Member
/// floor makes invalid) is refused rather than listed.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn catalog_read_shows_a_member_what_they_track_on_and_writes_nothing(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 1000);
    let member = &org.people[2].1;
    let (personal, _) = token(&pool, member, "script").await;
    let entry = create_entry(&router, &personal, member).await;
    let secret = oauth_token(&pool, &org.app, member, ScopeSet::of(&[Scope::CatalogRead])).await;
    let task = format!("/api/v1/tasks/{}", member.task_id);
    for (operation, uri) in [
        ("listTrackable", "/api/v1/me/trackable"),
        ("listTasks", "/api/v1/tasks"),
        ("getTask", task.as_str()),
    ] {
        let (_, _, expected) = send(&router, operation, get(uri, Some(&personal))).await;
        let (status, _, body) = send(&router, operation, get(uri, Some(&secret))).await;
        assert_eq!(status, StatusCode::OK, "{operation}: {body}");
        assert_eq!(body, expected, "{operation}");
    }
    let (_, _, trackable) = send(
        &router,
        "listTrackable",
        get("/api/v1/me/trackable", Some(&secret)),
    )
    .await;
    assert_eq!(trackable["data"].as_array().unwrap().len(), 1);
    for (operation, request) in writes(&secret, member, entry) {
        let (status, headers, body) = send(&router, operation, request).await;
        assert_insufficient_scope(status, &headers, &body, Scope::TimeWrite);
    }

    let write_own = horae_core::permissions::catalog::Permission::TimeWriteOwn;
    crate::ops::testing::drop_grant(&pool, member, write_own).await;
    for (operation, uri) in [
        ("listTrackable", "/api/v1/me/trackable"),
        ("getTask", task.as_str()),
    ] {
        let (status, _, body) = send(&router, operation, get(uri, Some(&secret))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{operation}: {body}");
        assert_eq!(body["code"], "forbidden");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn me_names_the_app_and_its_scopes_for_any_scope(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 1000);
    let (profile, admin) = &org.people[0];
    assert_eq!(*profile, BuiltInProfile::Administrator);
    for scopes in scope_sets() {
        let secret = oauth_token(&pool, &org.app, admin, scopes).await;
        let (status, _, me) = send(&router, "getMe", get("/api/v1/me", Some(&secret))).await;
        assert_eq!(status, StatusCode::OK, "{me}");
        assert_eq!(me["person"]["id"], json!(admin.user_id));
        assert_eq!(
            me["administrator"], false,
            "an app is never an administrator"
        );
        assert_eq!(me["token"]["kind"], "oauth");
        assert_eq!(me["token"]["name"], "Widget");
        assert_eq!(me["token"]["oauth"]["app_name"], "Widget");
        assert_eq!(me["token"]["oauth"]["client_id"], json!(org.app.client_id));
        let granted: Vec<&str> = me["token"]["oauth"]["scopes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|scope| scope.as_str().unwrap())
            .collect();
        assert_eq!(ScopeSet::parse_each(granted).unwrap(), scopes);
    }

    let (personal, _) = token(&pool, admin, "script").await;
    let (_, _, me) = send(&router, "getMe", get("/api/v1/me", Some(&personal))).await;
    assert_eq!(me["administrator"], true);
    assert_eq!(me["token"]["kind"], "personal");
    assert_eq!(me["token"]["oauth"], Value::Null);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refused_oauth_token_gets_an_invalid_token_challenge(pool: PgPool) {
    let router = router(&pool, 1000);
    for authorization in [Some("Bearer horae_oat_unknown"), None] {
        let mut request = Request::get("/api/v1/me");
        if let Some(value) = authorization {
            request = request.header(header::AUTHORIZATION, value);
        }
        let (status, headers, body) =
            send(&router, "getMe", request.body(Body::empty()).unwrap()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "unauthenticated");
        let expected = match authorization {
            Some(_) => r#"Bearer error="invalid_token""#,
            None => "Bearer",
        };
        assert_eq!(headers[header::WWW_AUTHENTICATE], expected);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_revoked_grant_stops_its_access_token(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 1000);
    let member = &org.people[2].1;
    let secret = oauth_token(&pool, &org.app, member, all_scopes()).await;
    let me = || get("/api/v1/me", Some(&secret));
    assert_eq!(send(&router, "getMe", me()).await.0, StatusCode::OK);
    sqlx::query!(
        "UPDATE oauth_grants SET revoked_at = now(), revoke_reason = 'person' WHERE user_id = $1",
        member.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let (status, headers, _) = send(&router, "getMe", me()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        headers[header::WWW_AUTHENTICATE],
        r#"Bearer error="invalid_token""#
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_oauth_tokens_requests_count_against_its_grant(pool: PgPool) {
    let org = org(&pool).await;
    let router = router(&pool, 2);
    let member = &org.people[2].1;
    let request = pending(&org.app, REDIRECT_URI, all_scopes());
    let (_, code) = approve(&pool, member, &request).await;
    let first = exchange(&pool, &org.app, &code, REDIRECT_URI)
        .await
        .unwrap();
    let me = |secret: &str| get("/api/v1/me", Some(secret));
    let first_access = first.access_token.expose().to_owned();
    assert_eq!(
        send(&router, "getMe", me(&first_access)).await.0,
        StatusCode::OK
    );

    let mut tx = crate::ops::oauth::tokens::begin(&pool).await.unwrap();
    let result = crate::ops::oauth::tokens::refresh(
        &mut tx,
        &Default::default(),
        &org.app.auth(),
        first.refresh_token.expose(),
        None,
    )
    .await;
    let refreshed = crate::ops::oauth::tokens::finish(tx, result).await.unwrap();
    let second_access = refreshed.access_token.expose().to_owned();
    assert_eq!(
        send(&router, "getMe", me(&second_access)).await.0,
        StatusCode::OK
    );
    let (status, _, body) = send(&router, "getMe", me(&second_access)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
}
