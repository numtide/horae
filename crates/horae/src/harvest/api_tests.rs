//! Harvest API v2 compatibility through the router: integer ids, exact
//! decimals, error bodies and path aliases (specs/021-harvest-api-compat).

use super::pagination_tests::{api_id, page, request, signed_in};
use super::*;
use crate::server_fns::test_seed::{seed, time_entry};
use horae_core::types::{EntryState, OrgRole};
use serde_json::{Value, json};

#[sqlx::test(migrations = "./migrations")]
async fn responses_carry_integer_ids_for_every_reference(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let app = signed_in(&pool, ids.user_id).await;

    let body = page(&app, "/harvest/v2/time_entries").await;
    let row = &body["time_entries"][0];

    assert_eq!(row["id"], api_id(&pool, "time_entry", entry).await);
    assert_eq!(row["user"]["id"], api_id(&pool, "user", ids.user_id).await);
    assert_eq!(
        row["client"]["id"],
        api_id(&pool, "client", ids.client_id).await
    );
    assert_eq!(
        row["project"]["id"],
        api_id(&pool, "project", ids.project_id).await
    );
    assert_eq!(row["task"]["id"], api_id(&pool, "task", ids.task_id).await);
}

#[sqlx::test(migrations = "./migrations")]
async fn hours_are_exact_decimals(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("UPDATE time_entries SET minutes = 100 WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();
    let app = signed_in(&pool, ids.user_id).await;

    let (_, body) = request(&app, "/harvest/v2/time_entries").await;

    // 100 minutes is 1.666… hours: written half up, as text, not as an f64.
    assert!(body.contains(r#""hours":1.67,"#), "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn records_are_found_by_number_or_by_uuid(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;
    let number = api_id(&pool, "project", ids.project_id).await;

    for path in [
        format!("/harvest/v2/projects/{number}"),
        format!("/harvest/v2/projects/{}", ids.project_id),
        format!("/harvest/v2/projects/{number}.json"),
    ] {
        assert_eq!(page(&app, &path).await["id"], number, "{path}");
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn an_unknown_number_is_a_harvest_404(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;

    let (status, body) = request(&app, "/harvest/v2/projects/999999").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["error"], "not_found");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_filter_by_an_unknown_number_matches_nothing(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let app = signed_in(&pool, ids.user_id).await;

    let body = page(&app, "/harvest/v2/time_entries?project_id=999999").await;

    assert_eq!(body["total_entries"], 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn filters_accept_numbers(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    time_entry(&pool, &ids, EntryState::Open).await;
    let app = signed_in(&pool, ids.user_id).await;
    let project = api_id(&pool, "project", ids.project_id).await;

    let body = page(
        &app,
        &format!("/harvest/v2/time_entries?project_id={project}"),
    )
    .await;

    assert_eq!(body["total_entries"], 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_malformed_id_is_a_422_with_a_message(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;

    let (status, body) = request(&app, "/harvest/v2/projects/not-an-id").await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(serde_json::from_str::<Value>(&body).unwrap()["message"].is_string());
}

#[sqlx::test(migrations = "./migrations")]
async fn signed_out_requests_get_the_oauth_error_body(pool: PgPool) {
    let app = router(pool);

    let (status, body) = request(&app, "/harvest/v2/users/me").await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap(),
        json!({ "error": "invalid_token", "error_description": "No session" })
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn every_alias_serves_the_canonical_response(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Manager).await;
    let app = signed_in(&pool, ids.user_id).await;
    let canonical = page(&app, "/harvest/v2/users/me").await;

    for alias in [
        "/v2/users/me",
        "/api/v2/users/me",
        "/harvest/v2/users/me.json",
        "/api/v2/users/me.json",
    ] {
        assert_eq!(page(&app, alias).await, canonical, "{alias}");
    }
    let projects = page(&app, "/v2/projects.json").await;
    assert_eq!(projects["total_entries"], 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn amounts_use_the_currencys_own_decimals(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    sqlx::query!(
        "UPDATE projects SET currency = 'JPY', budget_kind = 'amount', budget_amount_cents = 100000
         WHERE id = $1",
        ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let app = signed_in(&pool, ids.user_id).await;

    let (_, body) = request(&app, &format!("/harvest/v2/projects/{}", ids.project_id)).await;

    assert!(body.contains(r#""budget":1000,"#), "{body}");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_member_filtering_by_an_unknown_person_is_refused(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Member).await;
    let app = signed_in(&pool, ids.user_id).await;

    // As for a colleague's number: a member's filter names only themselves,
    // and the answer must not tell an unknown person from a real one.
    let (status, _) = request(&app, "/harvest/v2/time_entries?user_id=999999").await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ── Numbering (migrations 0057–0059) ────────────────────────────────────────

#[sqlx::test(migrations = "./migrations")]
async fn every_organization_has_an_account_id(pool: PgPool) {
    let first = seed(&pool, OrgRole::Admin).await;
    let second = seed(&pool, OrgRole::Admin).await;

    let accounts = sqlx::query_scalar!(
        "SELECT api_account_id FROM organizations WHERE id = ANY($1) ORDER BY api_account_id",
        &[first.org_id, second.org_id],
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(accounts.len(), 2);
    assert!(accounts[0] < accounts[1]);
}

#[sqlx::test(migrations = "./migrations")]
async fn numbers_count_up_per_kind_in_creation_order(pool: PgPool) {
    let first = seed(&pool, OrgRole::Admin).await;
    let second = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &first, EntryState::Open).await;
    let later = time_entry(&pool, &second, EntryState::Open).await;

    let clients = (
        api_id(&pool, "client", first.client_id).await,
        api_id(&pool, "client", second.client_id).await,
    );
    let entries = (
        api_id(&pool, "time_entry", entry).await,
        api_id(&pool, "time_entry", later).await,
    );

    assert!(clients.0 < clients.1, "{clients:?}");
    assert!(entries.0 < entries.1, "{entries:?}");
}

#[sqlx::test(migrations = "./migrations")]
async fn a_deleted_records_number_is_not_reused(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let deleted = api_id(&pool, "time_entry", entry).await;
    sqlx::query!("DELETE FROM time_entries WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();

    let next = time_entry(&pool, &ids, EntryState::Open).await;

    assert!(api_id(&pool, "time_entry", next).await > deleted);
}

/// Two creators whose snapshots predate each other's commit still get numbers:
/// allocation must not depend on what the transaction's snapshot can see.
#[sqlx::test(migrations = "./migrations")]
async fn concurrent_creators_get_numbers_at_every_isolation_level(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    for level in ["REPEATABLE READ", "SERIALIZABLE"] {
        let mut first = pool.begin().await.unwrap();
        let mut second = pool.begin().await.unwrap();
        for tx in [&mut first, &mut second] {
            // Numbering must not wait on the other creator either: fail, not hang.
            sqlx::raw_sql(&format!(
                "SET TRANSACTION ISOLATION LEVEL {level}; SET LOCAL lock_timeout = '5s'"
            ))
            .execute(&mut **tx)
            .await
            .unwrap();
            // The snapshot is taken by the first query, before either insert.
            sqlx::query!("SELECT 1 AS one")
                .fetch_one(&mut **tx)
                .await
                .unwrap();
        }
        let mut created = Vec::new();
        for tx in [&mut first, &mut second] {
            let id = Uuid::now_v7();
            sqlx::query!(
                "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Racer', 'EUR')",
                id,
                ids.org_id
            )
            .execute(&mut **tx)
            .await
            .unwrap_or_else(|error| panic!("{level}: {error}"));
            created.push(id);
        }
        first.commit().await.unwrap();
        second.commit().await.unwrap();

        let a = api_id(&pool, "client", created[0]).await;
        let b = api_id(&pool, "client", created[1]).await;
        assert_ne!(a, b, "{level}");
    }
}

async fn insert_client(pool: &PgPool, org_id: Uuid, harvest_id: Option<i64>) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, harvest_id) VALUES ($1, $2, 'Client', 'EUR', $3)",
        id,
        org_id,
        harvest_id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn a_given_harvest_id_is_kept(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;

    let imported = insert_client(&pool, ids.org_id, Some(4_200_000)).await;
    let native = insert_client(&pool, ids.org_id, None).await;

    assert_eq!(api_id(&pool, "client", imported).await, 4_200_000);
    // Native ids come from the kind's sequence, far below Harvest's.
    assert!(api_id(&pool, "client", native).await < 4_200_000);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_native_record_skips_an_id_another_record_has(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let next = api_id(&pool, "client", ids.client_id).await + 1;
    insert_client(&pool, ids.org_id, Some(next)).await;

    let native = insert_client(&pool, ids.org_id, None).await;

    assert!(api_id(&pool, "client", native).await > next);
}

/// A number never names another record: an imported record's Harvest id stays
/// reserved after the record is deleted, so it can come back with it.
#[sqlx::test(migrations = "./migrations")]
async fn a_native_record_skips_a_harvest_id_the_import_map_holds(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let next = api_id(&pool, "client", ids.client_id).await + 1;
    crate::importers::harvest::provenance::upsert(
        &pool,
        ids.org_id,
        horae_core::importers::harvest::types::EntityType::Client,
        next,
        Uuid::now_v7(),
        None,
    )
    .await
    .unwrap();

    let native = insert_client(&pool, ids.org_id, None).await;

    assert!(api_id(&pool, "client", native).await > next);
}

#[sqlx::test(migrations = "./migrations")]
async fn ids_are_unique_per_organization_only(pool: PgPool) {
    let first = seed(&pool, OrgRole::Admin).await;
    let second = seed(&pool, OrgRole::Admin).await;
    let held = api_id(&pool, "client", first.client_id).await;

    let same = insert_client(&pool, second.org_id, Some(held)).await;
    let duplicate = sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, harvest_id) VALUES ($1, $2, 'Twin', 'EUR', $3)",
        Uuid::now_v7(),
        first.org_id,
        held
    )
    .execute(&pool)
    .await;

    assert_eq!(api_id(&pool, "client", same).await, held);
    assert!(duplicate.is_err());
}

/// Records that exist when the column is deployed: imported ones take their
/// Harvest ids, the rest are numbered in creation order, skipping those.
#[sqlx::test(migrations = false)]
async fn the_backfill_numbers_existing_records(pool: PgPool) {
    let mut before = sqlx::migrate!("./migrations");
    before.migrations = before
        .iter()
        .filter(|m| m.version < 59)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    before.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Admin).await;
    let mut natives = vec![ids.client_id];
    for _ in 0..3 {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Native', 'EUR')",
            id,
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        natives.push(id);
    }
    let imported = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Imported', 'EUR')",
        imported,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    // Harvest id 2 is a number a native client would otherwise take.
    sqlx::query!(
        "INSERT INTO harvest_import_map (org_id, harvest_entity_type, harvest_import_id, horae_id)
         VALUES ($1, 'client', 2, $2)",
        ids.org_id,
        imported
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    assert_eq!(api_id(&pool, "client", imported).await, 2);
    let mut numbers = Vec::new();
    for id in &natives {
        numbers.push(api_id(&pool, "client", *id).await);
    }
    // Creation order, except the one whose turn fell on Harvest's 2: it waits
    // for the next free number.
    assert_eq!(numbers, [1, 5, 3, 4]);
    // A record created after the backfill continues the same sequence.
    let later = insert_client(&pool, ids.org_id, None).await;
    assert_eq!(api_id(&pool, "client", later).await, 6);
}

/// Numbering existing records is not an edit: it must not move what the
/// importer and the project editor read to detect one.
#[sqlx::test(migrations = false)]
async fn the_backfill_leaves_edit_stamps_alone(pool: PgPool) {
    let mut before = sqlx::migrate!("./migrations");
    before.migrations = before
        .iter()
        .filter(|m| m.version < 59)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    before.run(&pool).await.unwrap();
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    let stamps = || async {
        sqlx::query!(
            r#"SELECT te.updated_at AS "updated_at: chrono::DateTime<chrono::Utc>", p.edit_revision
               FROM time_entries te JOIN projects p ON p.id = te.project_id WHERE te.id = $1"#,
            entry
        )
        .fetch_one(&pool)
        .await
        .map(|row| (row.updated_at, row.edit_revision))
        .unwrap()
    };
    let original = stamps().await;

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    assert_eq!(stamps().await, original);
}
