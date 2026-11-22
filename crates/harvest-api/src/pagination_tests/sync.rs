//! Full and incremental sync (spec 021 US3, T048): page sizes, absolute
//! links, cursor walks, `updated_since`, the time-entry filters and the
//! kinds Horae lacks.

use super::*;
use crate::testing::add_person;

async fn get_with(app: &Router, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, Value) {
    let mut request = Request::builder().uri(uri);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn big_page(app: &Router, uri: &str) -> Value {
    let (status, body) = get_with(app, uri, &[]).await;
    assert_eq!(status, StatusCode::OK, "{uri}: {body}");
    body
}

fn ids_of(body: &Value, kind: &str) -> Vec<i64> {
    body[kind]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_i64().unwrap())
        .collect()
}

/// Adds `count` clients to the organization, one update time apart each.
async fn add_clients(pool: &PgPool, org_id: Uuid, count: i32) {
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, created_at, updated_at)
         SELECT gen_random_uuid(), $1, 'Client ' || n, 'EUR',
                '2026-01-01'::timestamptz + n * interval '1 second',
                '2026-01-01'::timestamptz + n * interval '1 second'
         FROM generate_series(1, $2) n",
        org_id,
        count,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn per_page_runs_from_1_to_2000_and_defaults_to_2000(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_clients(&pool, ids.org_id, 2001).await;
    let app = signed_in(&pool, ids.user_id).await;

    let default = big_page(&app, "/harvest/v2/clients").await;
    assert_eq!(default["per_page"], 2000);
    assert_eq!(default["clients"].as_array().unwrap().len(), 2000);
    assert_eq!(default["total_entries"], 2002);
    assert_eq!(default["total_pages"], 2);
    assert_eq!(default["next_page"], 2);

    let full = big_page(&app, "/harvest/v2/clients?per_page=2000&page=2").await;
    assert_eq!(full["clients"].as_array().unwrap().len(), 2);
    assert!(full["next_page"].is_null());

    let one = big_page(&app, "/harvest/v2/clients?per_page=1").await;
    assert_eq!(one["clients"].as_array().unwrap().len(), 1);
    assert_eq!(one["total_pages"], 2002);

    for resource in [
        "clients",
        "projects",
        "tasks",
        "users",
        "time_entries",
        "users/me/project_assignments",
        "contacts",
    ] {
        let (status, body) =
            get_with(&app, &format!("/harvest/v2/{resource}?per_page=2001"), &[]).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{resource}: {body}"
        );
        assert!(body["message"].as_str().unwrap().contains("2000"));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn links_are_absolute_on_the_request_host(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_clients(&pool, ids.org_id, 3).await;
    let app = signed_in(&pool, ids.user_id).await;
    for (path, headers, prefix) in [
        (
            "/v2/clients.json?per_page=2&page=2&is_active=true",
            vec![
                ("host", "acme.example.test"),
                ("x-forwarded-proto", "https"),
            ],
            "https://acme.example.test/v2/clients.json?",
        ),
        (
            "/harvest/v2/time_entries?per_page=1",
            vec![("host", "127.0.0.1:8080")],
            "http://127.0.0.1:8080/harvest/v2/time_entries?",
        ),
        (
            "/harvest/v2/roles",
            vec![("host", "h.test"), ("x-forwarded-proto", "https, http")],
            "https://h.test/harvest/v2/roles?",
        ),
    ] {
        let (status, body) = get_with(&app, path, &headers).await;
        assert_eq!(status, StatusCode::OK, "{path}: {body}");
        for key in ["first", "next", "previous", "last"] {
            if let Some(link) = body["links"][key].as_str() {
                assert!(link.starts_with(prefix), "{path} {key}: {link}");
            }
        }
    }
    // The filters ride along on the cursor links too.
    let (_, body) = get_with(
        &app,
        "/v2/clients?per_page=2&page=2&is_active=true",
        &[("host", "acme.example.test")],
    )
    .await;
    let previous = body["links"]["previous"].as_str().unwrap();
    assert!(previous.contains("cursor=") && previous.ends_with("&is_active=true"));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_cursor_walk_visits_the_same_rows_as_a_page_walk(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_rows_of_every_kind(&pool, &ids, 5).await;
    let app = signed_in(&pool, ids.user_id).await;
    for resource in LISTS {
        let mut by_page = Vec::new();
        let first = big_page(&app, &format!("/harvest/v2/{resource}?per_page=2")).await;
        let total_pages = first["total_pages"].as_i64().unwrap();
        for page_number in 1..=total_pages {
            let body = big_page(
                &app,
                &format!("/harvest/v2/{resource}?per_page=2&page={page_number}"),
            )
            .await;
            by_page.extend(ids_of(&body, resource));
        }

        let mut by_link = ids_of(&first, resource);
        let mut pages = vec![first];
        while let Some(next) = pages.last().unwrap()["links"]["next"].as_str() {
            assert!(next.contains("cursor="), "{next}");
            let body = big_page(&app, next).await;
            assert_eq!(body["page"], pages.len() + 1, "{resource}");
            by_link.extend(ids_of(&body, resource));
            pages.push(body);
        }
        assert_eq!(by_link, by_page, "{resource}");
        assert_eq!(pages.len() as i64, total_pages);

        // Going back by cursor lands on the page before.
        for (index, body) in pages.iter().enumerate().skip(1) {
            let previous = big_page(&app, body["links"]["previous"].as_str().unwrap()).await;
            assert_eq!(previous["page"], index);
            assert_eq!(
                ids_of(&previous, resource),
                ids_of(&pages[index - 1], resource)
            );
        }
    }
    // Catalogue lists run newest first, as Harvest's do.
    let clients = big_page(&app, "/harvest/v2/clients").await;
    let created: Vec<&str> = clients["clients"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["created_at"].as_str().unwrap())
        .collect();
    assert!(created.is_sorted_by(|a, b| a >= b), "{created:?}");
}

const LISTS: [&str; 5] = ["clients", "projects", "tasks", "users", "time_entries"];

/// Adds `count` records of every listed kind, each kind's newest last, and
/// time entries over several days.
async fn add_rows_of_every_kind(pool: &PgPool, ids: &crate::testing::SeedIds, count: i32) {
    for n in 0..count {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, $3, 'EUR')",
            id,
            ids.org_id,
            format!("Client {n}")
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, $4, 'EUR')",
            Uuid::now_v7(),
            ids.org_id,
            id,
            format!("Project {n}")
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, $3)",
            Uuid::now_v7(),
            ids.org_id,
            format!("Task {n}")
        )
        .execute(pool)
        .await
        .unwrap();
        add_person(pool, ids, BuiltInProfile::Member).await;
        let entry = time_entry(pool, ids, EntryState::Open).await;
        sqlx::query!(
            "UPDATE time_entries SET spent_date = spent_date - $2::int WHERE id = $1",
            entry,
            n % 3
        )
        .execute(pool)
        .await
        .unwrap();
    }
}

/// The ids a link-following client collects from `resource` in pages of two,
/// running `between` after the first page.
async fn follow_links(app: &Router, resource: &str, between: impl AsyncFnOnce()) -> Vec<i64> {
    let mut body = big_page(app, &format!("/harvest/v2/{resource}?per_page=2")).await;
    let mut seen = ids_of(&body, resource);
    between().await;
    while let Some(next) = body["links"]["next"].as_str() {
        body = big_page(app, next).await;
        seen.extend(ids_of(&body, resource));
    }
    seen
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn edits_and_deletes_during_a_cursor_walk_skip_nothing(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_rows_of_every_kind(&pool, &ids, 5).await;
    let app = signed_in(&pool, ids.user_id).await;
    for resource in LISTS {
        let before = follow_links(&app, resource, async || {}).await;
        // Edit every record and delete one the client has already read: on
        // an offset walk the rest would shift and one would be skipped.
        let walked = follow_links(&app, resource, async || {
            let first = before[0];
            match resource {
                "clients" => {
                    sqlx::query!(
                        "UPDATE clients SET name = name || '!' WHERE org_id = $1",
                        ids.org_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                "projects" => {
                    sqlx::query!(
                        "UPDATE projects SET name = name || '!' WHERE org_id = $1",
                        ids.org_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                "tasks" => {
                    sqlx::query!(
                        "UPDATE tasks SET name = name || '!' WHERE org_id = $1",
                        ids.org_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                "users" => {
                    sqlx::query!(
                        "UPDATE users SET name = name || '!' WHERE org_id = $1",
                        ids.org_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
                _ => {
                    sqlx::query!(
                        "UPDATE time_entries SET notes = 'edited' WHERE org_id = $1",
                        ids.org_id
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                    sqlx::query!(
                        "DELETE FROM time_entries WHERE org_id = $1 AND harvest_id = $2",
                        ids.org_id,
                        first
                    )
                    .execute(&pool)
                    .await
                    .unwrap();
                }
            }
        })
        .await;
        assert_eq!(walked, before, "{resource}");
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_changed_or_foreign_cursor_is_a_bad_request(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_clients(&pool, ids.org_id, 3).await;
    let app = signed_in(&pool, ids.user_id).await;
    let first = big_page(&app, "/harvest/v2/clients?per_page=1").await;
    let next = first["links"]["next"].as_str().unwrap();
    let cursor = form_urlencoded::parse(next.split_once('?').unwrap().1.as_bytes())
        .find(|(key, _)| key == "cursor")
        .unwrap()
        .1
        .into_owned();
    let mut changed = cursor.clone().into_bytes();
    changed[4] = if changed[4] == b'A' { b'B' } else { b'A' };
    let changed = String::from_utf8(changed).unwrap();
    for list in ["clients", "time_entries", "roles"] {
        for bad in [changed.as_str(), "nope", ""] {
            let (status, body) =
                get_with(&app, &format!("/harvest/v2/{list}?cursor={bad}"), &[]).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{list} {bad}: {body}");
        }
    }
    // My project assignments have no cursor, so one is refused, not ignored.
    let (status, _) = get_with(
        &app,
        &format!("/harvest/v2/users/me/project_assignments?cursor={cursor}"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    // The cursor wins over page.
    let both = big_page(
        &app,
        &format!("/harvest/v2/clients?per_page=1&page=4&cursor={cursor}"),
    )
    .await;
    assert_eq!(both["page"], 2);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn updated_since_returns_only_what_changed_on_every_kind(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other = add_person(&pool, &ids, BuiltInProfile::Member).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    time_entry(&pool, &other, EntryState::Open).await;
    add_clients(&pool, ids.org_id, 2).await;
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Other', 'EUR')",
        Uuid::now_v7(),
        ids.org_id,
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Other')",
        Uuid::now_v7(),
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    // Every row so far is older than the mark.
    let mark =
        sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!: chrono::DateTime<chrono::Utc>""#)
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
    let mark: String = form_urlencoded::byte_serialize(mark.as_bytes()).collect();
    let app = signed_in(&pool, ids.user_id).await;
    for resource in ["clients", "projects", "tasks", "users", "time_entries"] {
        let body = big_page(
            &app,
            &format!("/harvest/v2/{resource}?updated_since={mark}"),
        )
        .await;
        assert_eq!(body["total_entries"], 0, "{resource}: {body}");
    }

    // A no-op save is not a change.
    sqlx::query!(
        "UPDATE clients SET name = name WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE clients SET name = 'Renamed' WHERE id = $1",
        ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET code = 'P-1' WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE tasks SET name = 'Renamed' WHERE id = $1",
        ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET name = 'Renamed Person' WHERE id = $1",
        other.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET notes = 'Changed' WHERE id = $1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    for (resource, kind, id) in [
        ("clients", "client", ids.client_id),
        ("projects", "project", ids.project_id),
        ("tasks", "task", ids.task_id),
        ("users", "user", other.user_id),
        ("time_entries", "time_entry", entry),
    ] {
        let body = big_page(
            &app,
            &format!("/harvest/v2/{resource}?updated_since={mark}"),
        )
        .await;
        assert_eq!(
            ids_of(&body, resource),
            [api_id(&pool, kind, id).await],
            "{resource}"
        );
    }

    // An administrator change shows on the person, through their stored state.
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator = true WHERE user_id = $1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let users = big_page(&app, &format!("/harvest/v2/users?updated_since={mark}")).await;
    assert_eq!(users["total_entries"], 2);

    // Assignments have no update time, so the filter is refused, not ignored.
    let (status, _) = get_with(
        &app,
        &format!("/harvest/v2/users/me/project_assignments?updated_since={mark}"),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn time_entries_take_every_documented_filter(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let open = time_entry(&pool, &ids, EntryState::Open).await;
    let submitted = time_entry(&pool, &ids, EntryState::Submitted).await;
    let approved = time_entry(&pool, &ids, EntryState::Approved).await;
    let (client, project, task) = (Uuid::now_v7(), Uuid::now_v7(), Uuid::now_v7());
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Second', 'EUR')",
        client,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Second', 'EUR')",
        project,
        ids.org_id,
        client
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Second')",
        task,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let elsewhere = time_entry(
        &pool,
        &crate::testing::SeedIds {
            client_id: client,
            project_id: project,
            task_id: task,
            ..ids
        },
        EntryState::Open,
    )
    .await;
    let app = signed_in(&pool, ids.user_id).await;
    let numbers = async |entries: &[Uuid]| {
        let mut numbers = Vec::new();
        for entry in entries {
            numbers.push(api_id(&pool, "time_entry", *entry).await);
        }
        numbers.sort();
        numbers
    };
    let client_number = api_id(&pool, "client", client).await;
    let task_number = api_id(&pool, "task", task).await;
    for (query, expected) in [
        (format!("client_id={client_number}"), vec![elsewhere]),
        (format!("task_id={task_number}"), vec![elsewhere]),
        (
            format!("client_id={}", ids.client_id),
            vec![open, submitted, approved],
        ),
        (
            "approval_status=unsubmitted".to_owned(),
            vec![open, elsewhere],
        ),
        ("approval_status=submitted".to_owned(), vec![submitted]),
        ("approval_status=approved".to_owned(), vec![approved]),
        ("is_billed=true".to_owned(), vec![]),
        (
            "is_billed=false".to_owned(),
            vec![open, submitted, approved, elsewhere],
        ),
        ("external_reference_id=abc".to_owned(), vec![]),
    ] {
        let body = big_page(&app, &format!("/harvest/v2/time_entries?{query}")).await;
        let mut got = ids_of(&body, "time_entries");
        got.sort();
        assert_eq!(got, numbers(&expected).await, "{query}");
        assert_eq!(body["total_entries"], expected.len(), "{query}");
    }
    let (status, _) = get_with(&app, "/harvest/v2/time_entries?approval_status=maybe", &[]).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    for (entry, status) in [
        (open, "unsubmitted"),
        (submitted, "submitted"),
        (approved, "approved"),
    ] {
        let row = big_page(&app, &format!("/harvest/v2/time_entries/{entry}")).await;
        assert_eq!(row["approval_status"], status);
    }

    // A running timer's hours include the time since it last started.
    sqlx::query!(
        "UPDATE time_entries SET is_running = true, started_at = now() - interval '90 minutes' WHERE id = $1",
        elsewhere
    )
    .execute(&pool)
    .await
    .unwrap();
    let running = big_page(&app, &format!("/harvest/v2/time_entries/{elsewhere}")).await;
    assert_eq!(running["hours_without_timer"], 1.0);
    assert_eq!(running["hours"], 2.5);
    let listed = big_page(&app, "/harvest/v2/time_entries?is_running=true").await;
    assert_eq!(listed["time_entries"][0]["hours"], 2.5);

    let row = &big_page(&app, &format!("/harvest/v2/time_entries/{open}")).await;
    assert_eq!(row["hours_without_timer"], 1.0);
    for field in [
        "user_assignment",
        "task_assignment",
        "invoice",
        "external_reference",
    ] {
        assert!(
            row[field].is_null() && row.get(field).is_some(),
            "{field}: {row}"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn kinds_horae_lacks_list_empty_and_find_nothing(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Member).await;
    let app = signed_in(&pool, ids.user_id).await;
    for kind in [
        "expenses",
        "expense_categories",
        "estimates",
        "estimate_item_categories",
        "contacts",
        "invoice_item_categories",
        "roles",
    ] {
        for path in [
            format!("/harvest/v2/{kind}"),
            format!("/v2/{kind}.json?page=2"),
        ] {
            let body = big_page(&app, &path).await;
            assert_eq!(body[kind], Value::Array(Vec::new()), "{path}");
            assert_eq!(body["total_entries"], 0);
            assert_eq!(body["per_page"], 2000);
            assert!(
                body["links"]["first"]
                    .as_str()
                    .unwrap()
                    .starts_with("http://localhost/")
            );
        }
        let (status, _) = get_with(&app, &format!("/harvest/v2/{kind}/1"), &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let unsigned = router(pool.clone(), limiter());
    let (status, _) = get_with(&unsigned, "/harvest/v2/expenses", &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

/// The ids and the latest `updated_at` a sync client sees on `/clients` since
/// `watermark`.
async fn changed_clients(app: &Router, watermark: &str) -> (Vec<i64>, Option<String>) {
    let since: String = form_urlencoded::byte_serialize(watermark.as_bytes()).collect();
    let body = big_page(app, &format!("/harvest/v2/clients?updated_since={since}")).await;
    let latest = body["clients"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["updated_at"].as_str())
        .max()
        .map(str::to_owned);
    (ids_of(&body, "clients"), latest)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_late_commit_is_not_skipped_by_an_updated_since_watermark(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let early = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Early', 'EUR')",
        early,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;
    let mut watermark =
        sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!: chrono::DateTime<chrono::Utc>""#)
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_rfc3339();

    // A slow transaction edits the seeded client and stays open while a
    // quick one edits another client and commits.
    let mut slow = pool.begin().await.unwrap();
    sqlx::query!(
        "UPDATE clients SET name = 'Late' WHERE id = $1",
        ids.client_id
    )
    .execute(&mut *slow)
    .await
    .unwrap();
    sqlx::query!("UPDATE clients SET name = 'Quick' WHERE id = $1", early)
        .execute(&pool)
        .await
        .unwrap();

    // The client syncs, keeping the latest time it saw as its watermark,
    // then syncs again after the slow transaction commits.
    let mut seen = std::collections::BTreeSet::new();
    let (changed, latest) = changed_clients(&app, &watermark).await;
    assert!(
        changed.is_empty(),
        "a time past an uncommitted write was served"
    );
    seen.extend(changed);
    if let Some(latest) = latest {
        watermark = latest;
    }
    slow.commit().await.unwrap();
    let (changed, _) = changed_clients(&app, &watermark).await;
    seen.extend(changed);

    let expected = std::collections::BTreeSet::from([
        api_id(&pool, "client", ids.client_id).await,
        api_id(&pool, "client", early).await,
    ]);
    assert_eq!(seen, expected);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_token_walks_the_lists_by_link_and_nothing_is_cached(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    add_rows_of_every_kind(&pool, &ids, 3).await;
    let mut write = ops::begin(&pool, ops::Caller::session(ids.org_id, ids.user_id))
        .await
        .unwrap();
    let result = ops::api_tokens::create(&mut write.tx, &write.actor, "Sync", None).await;
    let (secret, _) = write.commit(result, &ops::NoEvents).await.unwrap();
    let bearer = format!("Bearer {}", secret.expose());
    let app = router(pool.clone(), limiter());
    for resource in LISTS {
        let mut uri = format!("/v2/{resource}?per_page=2");
        let mut count = 0;
        loop {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(&uri)
                        .header("authorization", &bearer)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{uri}");
            assert_eq!(response.headers()["cache-control"], "private, no-store");
            let body: Value = serde_json::from_slice(
                &axum::body::to_bytes(response.into_body(), 1024 * 1024)
                    .await
                    .unwrap(),
            )
            .unwrap();
            count += body[resource].as_array().unwrap().len();
            match body["links"]["next"].as_str() {
                Some(next) => uri = next.to_owned(),
                None => {
                    assert_eq!(body["total_entries"], count, "{resource}");
                    break;
                }
            }
        }
    }
}
