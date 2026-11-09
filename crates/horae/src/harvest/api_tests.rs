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

// ── Numbering (migration 0056) ──────────────────────────────────────────────

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
async fn numbers_count_up_per_organization_and_kind(pool: PgPool) {
    let first = seed(&pool, OrgRole::Admin).await;
    let second = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &first, EntryState::Open).await;
    let later = time_entry(&pool, &first, EntryState::Open).await;

    assert_eq!(api_id(&pool, "client", first.client_id).await, 1);
    assert_eq!(api_id(&pool, "client", second.client_id).await, 1);
    assert_eq!(api_id(&pool, "time_entry", entry).await, 1);
    assert_eq!(api_id(&pool, "time_entry", later).await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_deleted_records_number_is_not_reused(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!("DELETE FROM time_entries WHERE id = $1", entry)
        .execute(&pool)
        .await
        .unwrap();

    let next = time_entry(&pool, &ids, EntryState::Open).await;

    assert_eq!(api_id(&pool, "time_entry", next).await, 2);
}

/// The importer creates a record and maps its Harvest id in one transaction.
async fn import_client(pool: &PgPool, org_id: Uuid, harvest_id: i64) -> Uuid {
    let id = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Imported', 'EUR')",
        id,
        org_id
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    crate::importers::harvest::provenance::upsert(
        &mut *tx,
        org_id,
        horae_core::importers::harvest::types::EntityType::Client,
        harvest_id,
        id,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn an_imported_record_keeps_its_harvest_id(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;

    let imported = import_client(&pool, ids.org_id, 4_200_000).await;
    let native = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Native', 'EUR')",
        native,
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(api_id(&pool, "client", imported).await, 4_200_000);
    assert_eq!(api_id(&pool, "client", native).await, 4_200_001);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_harvest_id_held_by_another_record_gets_a_fresh_number(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    // The seeded client holds number 1.

    let imported = import_client(&pool, ids.org_id, 1).await;

    assert_eq!(api_id(&pool, "client", imported).await, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_record_clients_already_know_keeps_its_number_when_matched(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;

    // A later import matches the existing client instead of creating one.
    crate::importers::harvest::provenance::upsert(
        &pool,
        ids.org_id,
        horae_core::importers::harvest::types::EntityType::Client,
        4_200_000,
        ids.client_id,
        None,
    )
    .await
    .unwrap();

    assert_eq!(api_id(&pool, "client", ids.client_id).await, 1);
}
