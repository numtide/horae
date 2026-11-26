//! OAuth access tokens on the Harvest API (spec 024 US5, FR-071): for an
//! administrator, a project manager and a member, and for every scope set an
//! app may hold, a request gets through exactly when the scopes cover the
//! route, and then answers as the person's personal access token does.

use axum::body::Body;
use axum::http::{HeaderMap, Request, header};
use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::oauth::{ClientKind, Scope, ScopeSet};
use horae_core::types::EntryState;
use horae_server::config::OAuthConfig;
use serde_json::{Value, json};
use tower::ServiceExt;

use super::*;
use crate::api_tests::tokens::token;
use crate::ops::oauth::testing::{
    App, REDIRECT_URI, all_scopes, approve, exchange, pending, register,
};
use crate::pagination_tests::{api_id, limiter};
use crate::testing::{SeedIds, add_person, seed, time_entry};

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
/// seeded project and able to track its task, each with one entry.
struct Org {
    people: Vec<(BuiltInProfile, SeedIds)>,
    /// Each person's entry, by person.
    entries: Vec<(Uuid, Uuid)>,
    app: App,
}

impl Org {
    fn entry(&self, person: &SeedIds) -> Uuid {
        let found = self
            .entries
            .iter()
            .find(|(user, _)| *user == person.user_id);
        found.unwrap().1
    }
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
    let mut entries = Vec::new();
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
        entries.push((
            person.user_id,
            time_entry(pool, person, EntryState::Open).await,
        ));
    }
    let app = register(pool, &admin, "Widget", ClientKind::Public, all_scopes()).await;
    Org {
        people: vec![
            (BuiltInProfile::Administrator, admin),
            (BuiltInProfile::ProjectManager, manager),
            (BuiltInProfile::Member, member),
        ],
        entries,
        app,
    }
}

/// An access token of `person` for `app`, granted `scopes`, and its refresh
/// token.
async fn oauth_tokens(
    pool: &PgPool,
    app: &App,
    person: &SeedIds,
    scopes: ScopeSet,
) -> (String, String) {
    let (_, code) = approve(pool, person, &pending(app, REDIRECT_URI, scopes)).await;
    let tokens = exchange(pool, app, &code, REDIRECT_URI).await.unwrap();
    (
        tokens.access_token.expose().to_owned(),
        tokens.refresh_token.expose().to_owned(),
    )
}

async fn oauth_token(pool: &PgPool, app: &App, person: &SeedIds, scopes: ScopeSet) -> String {
    oauth_tokens(pool, app, person, scopes).await.0
}

fn request(method: &str, uri: &str, secret: &str, body: Option<&Value>) -> Request<Body> {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {secret}"));
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string())),
        None => builder.body(Body::empty()),
    }
    .unwrap()
}

fn get(uri: &str, secret: &str) -> Request<Body> {
    request("GET", uri, secret, None)
}

/// The status, headers and body; an empty body reads as `null`.
async fn send(app: &Router, request: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (status, headers, body)
}

fn assert_insufficient_scope(status: StatusCode, headers: &HeaderMap, body: &Value, needed: Scope) {
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["error"], "insufficient_scope", "{body}");
    assert_eq!(
        headers[header::WWW_AUTHENTICATE],
        format!(
            r#"Bearer error="insufficient_scope", scope="{}""#,
            needed.as_str()
        )
    );
}

/// The reads of the route table, each with the scope it needs (`None`: any),
/// and the key its list rows go under.
async fn reads(
    pool: &PgPool,
    ids: &SeedIds,
    entry: Uuid,
) -> Vec<(String, Option<Scope>, Option<&'static str>)> {
    use Scope::{CatalogRead, TimeRead};
    let entry = api_id(pool, "time_entry", entry).await;
    let project = api_id(pool, "project", ids.project_id).await;
    let client = api_id(pool, "client", ids.client_id).await;
    let task = api_id(pool, "task", ids.task_id).await;
    vec![
        ("/company".into(), None, None),
        ("/users/me".into(), None, None),
        (
            "/users/me/project_assignments".into(),
            Some(CatalogRead),
            Some("project_assignments"),
        ),
        ("/time_entries".into(), Some(TimeRead), Some("time_entries")),
        (format!("/time_entries/{entry}"), Some(TimeRead), None),
        ("/projects".into(), Some(CatalogRead), Some("projects")),
        (format!("/projects/{project}"), Some(CatalogRead), None),
        ("/clients".into(), Some(CatalogRead), Some("clients")),
        (format!("/clients/{client}"), Some(CatalogRead), None),
        ("/tasks".into(), Some(CatalogRead), Some("tasks")),
        (format!("/tasks/{task}"), Some(CatalogRead), None),
        ("/users".into(), Some(CatalogRead), Some("users")),
        ("/expenses".into(), Some(CatalogRead), Some("expenses")),
    ]
}

fn row_ids(body: &Value, key: &str) -> Vec<Value> {
    body[key]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["id"].clone())
        .collect()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn reads_answer_as_the_personal_token_when_the_scopes_cover_them(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    for (profile, person) in &org.people {
        let (personal, _) = token(&pool, person).await;
        let reads = reads(&pool, person, org.entry(person)).await;
        let mut expected = Vec::new();
        for (path, _, _) in &reads {
            let (status, _, body) = send(&app, get(&format!("/harvest/v2{path}"), &personal)).await;
            expected.push((status, body));
        }
        for scopes in scope_sets() {
            let secret = oauth_token(&pool, &org.app, person, scopes).await;
            for ((path, needed, rows), (personal_status, personal_body)) in
                reads.iter().zip(&expected)
            {
                let (status, headers, body) = send(&app, get(&format!("/v2{path}"), &secret)).await;
                let context = format!("{profile:?} {scopes:?} {path}: {body}");
                match needed {
                    Some(scope) if !scopes.normalize().contains(*scope) => {
                        assert_insufficient_scope(status, &headers, &body, *scope);
                    }
                    _ => {
                        assert_eq!(status, *personal_status, "{context}");
                        // Scopes never let through a row the person may not see.
                        // A subset, not equality: an administrator's app token is
                        // not an administrator (FR-061), so it can see fewer rows.
                        if let Some(key) = rows {
                            let theirs = row_ids(personal_body, key);
                            for id in row_ids(&body, key) {
                                assert!(theirs.contains(&id), "{context}: {id}");
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A member's token with `time:read` lists the member's own entry, as the
/// personal token does, and the identity reads take any scope.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_time_read_token_lists_the_persons_time(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let (personal, _) = token(&pool, member).await;
    let secret = oauth_token(&pool, &org.app, member, ScopeSet::of(&[Scope::TimeRead])).await;

    let (_, _, expected) = send(&app, get("/harvest/v2/time_entries", &personal)).await;
    let (status, _, listed) = send(&app, get("/harvest/v2/time_entries", &secret)).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed, expected);
    assert_eq!(listed["total_entries"], 1);

    let (status, _, me) = send(&app, get("/harvest/v2/users/me", &secret)).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    assert_eq!(me["id"], api_id(&pool, "user", member.user_id).await);
}

/// FR-060: a member's `catalog:read` token sees the projects and tasks they
/// track on, as their personal token does, though it cannot write time.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_catalog_read_token_lists_the_persons_trackable_pairs(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let (personal, _) = token(&pool, member).await;
    let secret = oauth_token(&pool, &org.app, member, ScopeSet::of(&[Scope::CatalogRead])).await;
    let uri = "/harvest/v2/users/me/project_assignments";

    let (_, _, expected) = send(&app, get(uri, &personal)).await;
    let (status, _, listed) = send(&app, get(uri, &secret)).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(listed, expected);
    let project = api_id(&pool, "project", member.project_id).await;
    let task = api_id(&pool, "task", member.task_id).await;
    let assignment = &listed["project_assignments"][0];
    assert_eq!(assignment["project"]["id"], project, "{listed}");
    assert_eq!(
        assignment["task_assignments"][0]["task"]["id"], task,
        "{listed}"
    );
}

/// A HEAD is a read and a `.json` path is its bare path, for scopes as for
/// everything else.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn head_and_json_paths_need_the_same_scopes(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let catalog = oauth_token(&pool, &org.app, member, ScopeSet::of(&[Scope::CatalogRead])).await;
    let time = oauth_token(&pool, &org.app, member, ScopeSet::of(&[Scope::TimeRead])).await;
    let body = entry_body(&pool, member, json!({ "hours": 1 })).await;

    let (status, headers, _) =
        send(&app, request("HEAD", "/v2/time_entries", &catalog, None)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        headers[header::WWW_AUTHENTICATE],
        r#"Bearer error="insufficient_scope", scope="time:read""#
    );
    assert_eq!(
        send(&app, request("HEAD", "/v2/time_entries.json", &time, None))
            .await
            .0,
        StatusCode::OK
    );
    let (status, headers, body) = send(
        &app,
        request("POST", "/v2/time_entries.json", &time, Some(&body)),
    )
    .await;
    assert_insufficient_scope(status, &headers, &body, Scope::TimeWrite);
    let (status, headers, body) = send(&app, get("/v2/projects.json", &time)).await;
    assert_insufficient_scope(status, &headers, &body, Scope::CatalogRead);
}

/// An administrator's app token never makes an administrator (FR-061).
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrators_app_token_is_not_an_administrator(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, admin) = &org.people[0];
    let (personal, _) = token(&pool, admin).await;
    let secret = oauth_token(&pool, &org.app, admin, all_scopes()).await;

    let (_, _, mine) = send(&app, get("/harvest/v2/users/me", &personal)).await;
    let (_, _, apps) = send(&app, get("/harvest/v2/users/me", &secret)).await;
    assert_eq!(
        (&mine["is_admin"], &apps["is_admin"]),
        (&json!(true), &json!(false))
    );
}

/// The seeded pair on a fixed day, plus `fields`.
async fn entry_body(pool: &PgPool, ids: &SeedIds, fields: Value) -> Value {
    let mut body = json!({
        "project_id": api_id(pool, "project", ids.project_id).await,
        "task_id": api_id(pool, "task", ids.task_id).await,
        "spent_date": "2026-09-08",
    });
    body.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    body
}

/// Creates a timer, stops, restarts, changes and deletes it, and answers each
/// step's status.
async fn write_sequence(
    pool: &PgPool,
    app: &Router,
    secret: &str,
    ids: &SeedIds,
) -> Vec<StatusCode> {
    let timer = entry_body(pool, ids, json!({ "notes": "Call" })).await;
    let (status, _, created) = send(
        app,
        request("POST", "/harvest/v2/time_entries", secret, Some(&timer)),
    )
    .await;
    let uri = format!("/harvest/v2/time_entries/{}", created["id"]);
    let mut statuses = vec![status];
    for (method, uri, body) in [
        ("PATCH", format!("{uri}/stop"), None),
        ("PATCH", format!("{uri}/restart"), None),
        ("PATCH", uri.clone(), Some(json!({ "notes": "Review" }))),
        ("DELETE", uri.clone(), None),
    ] {
        statuses.push(
            send(app, request(method, &uri, secret, body.as_ref()))
                .await
                .0,
        );
    }
    statuses
}

async fn entry_count(pool: &PgPool) -> Option<i64> {
    sqlx::query_scalar!("SELECT count(*) FROM time_entries")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Every write needs `time:write`. Without it nothing changes; with it, the
/// same sequence answers as it does for the person's personal token.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn writes_need_time_write_and_then_answer_as_the_personal_token(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    for (profile, person) in &org.people {
        let entry = api_id(&pool, "time_entry", org.entry(person)).await;
        let uri = format!("/harvest/v2/time_entries/{entry}");
        let body = entry_body(&pool, person, json!({ "hours": 1.5 })).await;
        let before = entry_count(&pool).await;
        for scopes in scope_sets()
            .into_iter()
            .filter(|scopes| !scopes.contains(Scope::TimeWrite))
        {
            let secret = oauth_token(&pool, &org.app, person, scopes).await;
            for request in [
                request("POST", "/harvest/v2/time_entries", &secret, Some(&body)),
                request("PATCH", &uri, &secret, Some(&json!({ "hours": 2 }))),
                request("DELETE", &uri, &secret, None),
                request("PATCH", &format!("{uri}/stop"), &secret, None),
                request("PATCH", &format!("{uri}/restart"), &secret, None),
            ] {
                let (status, headers, body) = send(&app, request).await;
                assert_insufficient_scope(status, &headers, &body, Scope::TimeWrite);
            }
        }
        assert_eq!(entry_count(&pool).await, before, "{profile:?}");

        let (personal, _) = token(&pool, person).await;
        let expected = write_sequence(&pool, &app, &personal, person).await;
        assert!(
            expected.iter().all(StatusCode::is_success),
            "{profile:?} {expected:?}"
        );
        let time = ScopeSet::of(&[Scope::TimeWrite]).normalize();
        let secret = oauth_token(&pool, &org.app, person, time).await;
        assert_eq!(
            write_sequence(&pool, &app, &secret, person).await,
            expected,
            "{profile:?}"
        );
    }
}

/// D-5: an OAuth token in the `access_token` query parameter is refused,
/// whether or not it is valid, while a personal token there still works.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_query_parameter_refuses_oauth_tokens(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let secret = oauth_token(&pool, &org.app, member, all_scopes()).await;
    let (personal, _) = token(&pool, member).await;

    // Refused alone, and beside a valid header token of either kind.
    let tokens = [secret.as_str(), "horae_oat_unknown"];
    let headers = [None, Some(secret.as_str()), Some(personal.as_str())];
    for (token, header_token) in tokens.into_iter().flat_map(|t| headers.map(|h| (t, h))) {
        let uri = format!("/harvest/v2/users/me?access_token={token}");
        let mut request = Request::builder().uri(&uri);
        if let Some(header_token) = header_token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {header_token}"));
        }
        let request = request.body(Body::empty()).unwrap();
        let (status, headers, body) = send(&app, request).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(
            body,
            json!({
                "error": "invalid_token",
                "error_description": "OAuth access tokens go in the Authorization header",
            })
        );
        assert_eq!(
            headers[header::WWW_AUTHENTICATE],
            r#"Bearer error="invalid_token""#
        );
    }
    let uri = format!("/harvest/v2/users/me?access_token={personal}");
    let request = Request::builder().uri(&uri).body(Body::empty()).unwrap();
    assert_eq!(send(&app, request).await.0, StatusCode::OK);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_refused_token_gets_an_invalid_token_challenge(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let secret = oauth_token(&pool, &org.app, member, all_scopes()).await;
    let mut tx = ops::oauth::tokens::begin(&pool).await.unwrap();
    let result = ops::oauth::tokens::revoke_token(&mut tx, &org.app.auth(), &secret).await;
    ops::oauth::tokens::finish(tx, result).await.unwrap();

    for token in [secret.as_str(), "horae_oat_unknown", "horae_pat_unknown"] {
        let (status, headers, body) = send(&app, get("/harvest/v2/users/me", token)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
        assert_eq!(body["error"], "invalid_token");
        assert_eq!(
            headers[header::WWW_AUTHENTICATE],
            r#"Bearer error="invalid_token""#
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_account_id_must_name_the_oauth_tokens_organization(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, member) = &org.people[2];
    let secret = oauth_token(&pool, &org.app, member, all_scopes()).await;
    let account = ops::api_tokens::fetch_harvest_account_id(&pool, member.org_id)
        .await
        .unwrap();

    for (named, expected) in [
        (account, StatusCode::OK),
        (account + 1, StatusCode::FORBIDDEN),
    ] {
        let mut request = get("/harvest/v2/users/me", &secret);
        request
            .headers_mut()
            .insert("harvest-account-id", named.to_string().parse().unwrap());
        let (status, _, body) = send(&app, request).await;
        assert_eq!(status, expected, "{named}: {body}");
    }
}

/// An app's requests count against its grant: a refreshed access token does
/// not start a new window (spec 024 R-14).
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_limit_counts_an_apps_requests_by_grant(pool: PgPool) {
    let org = org(&pool).await;
    let limit = horae_server::config::RateLimit {
        requests: 1,
        window: std::time::Duration::from_secs(60),
    };
    let app = router(pool.clone(), Arc::new(RateLimiter::new(limit)));
    let (_, member) = &org.people[2];
    let (first, refresh) = oauth_tokens(&pool, &org.app, member, all_scopes()).await;
    let mut tx = ops::oauth::tokens::begin(&pool).await.unwrap();
    let result = ops::oauth::tokens::refresh(
        &mut tx,
        &OAuthConfig::default(),
        &org.app.auth(),
        &refresh,
        None,
    )
    .await;
    let second = ops::oauth::tokens::finish(tx, result).await.unwrap();

    assert_eq!(
        send(&app, get("/harvest/v2/users/me", &first)).await.0,
        StatusCode::OK
    );
    let (status, _, body) = send(
        &app,
        get("/harvest/v2/users/me", second.access_token.expose()),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{body}");
}

/// The switch off answers 404 to an OAuth token as to any request.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn switched_off_an_oauth_token_gets_an_empty_404(pool: PgPool) {
    let org = org(&pool).await;
    let (_, member) = &org.people[2];
    let secret = oauth_token(&pool, &org.app, member, all_scopes()).await;
    let app = mount(false, test_state(pool.clone(), limiter()));

    for uri in ["/harvest/v2/users/me", "/v2/time_entries"] {
        let (status, _, body) = send(&app, get(uri, &secret)).await;
        assert_eq!(
            (status, body),
            (StatusCode::NOT_FOUND, Value::Null),
            "{uri}"
        );
    }
}

/// Every route of the table names a scope: an OAuth token holding all of
/// them reaches each route's handler, never an `insufficient_scope` or the
/// 500 of a route without one.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_route_names_a_scope(pool: PgPool) {
    let org = org(&pool).await;
    let app = router(pool.clone(), limiter());
    let (_, admin) = &org.people[0];
    let secret = oauth_token(&pool, &org.app, admin, all_scopes()).await;

    let paths = routes().flat_map(|(path, _, _)| {
        let path = path.replace("{id}", "999999");
        // The `.json` twins `serve` adds beside fixed paths.
        let twin = (!path.ends_with("999999")).then(|| format!("/v2{path}.json"));
        [Some(format!("/harvest/v2{path}")), twin]
            .into_iter()
            .flatten()
    });
    for path in paths {
        for method in ["GET", "HEAD", "POST", "PATCH", "DELETE"] {
            let (status, _, body) = send(&app, request(method, &path, &secret, None)).await;
            assert_ne!(body["error"], "insufficient_scope", "{method} {path}");
            assert_ne!(
                status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "{method} {path}: {body}"
            );
        }
    }
}
