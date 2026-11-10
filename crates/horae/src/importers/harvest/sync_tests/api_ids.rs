//! A real API import gives every record it creates its Harvest id as the
//! Harvest API's integer id (FR-016). The claim only works while each record
//! is created and mapped in one transaction, which this pins.

use super::*;

/// Every kind with an id far from the sequences' range, plus a person Horae
/// already has, whom the import only matches.
fn data_with_ids() -> HarvestData {
    HarvestData {
        clients: serde_json::from_value(json!([{"id":70001,"name":"Client","currency":"USD"}]))
            .unwrap(),
        projects: serde_json::from_value(json!([
            {"id":70002,"name":"Project","client":{"id":70001}}
        ]))
        .unwrap(),
        tasks: serde_json::from_value(json!([{"id":70003,"name":"Task"}])).unwrap(),
        users: serde_json::from_value(json!([
            {"id":70004,"email":"known@example.com"},
            {"id":70005,"email":"new@example.com","first_name":"New","last_name":"Person"}
        ]))
        .unwrap(),
        time_entries: serde_json::from_value(json!([
            {"id":70010,"spent_date":"2026-01-01","hours":1,"project":{"id":70002},
             "task":{"id":70003},"user":{"id":70005},"updated_at":day(3)}
        ]))
        .unwrap(),
    }
}

async fn number(pool: &PgPool, kind: &str, harvest_id: i64) -> Option<(i64, bool)> {
    sqlx::query!(
        r#"SELECT a.number, a.from_harvest
           FROM harvest_import_map m
           JOIN api_ids a ON a.org_id = m.org_id AND a.horae_id = m.horae_id
             AND a.kind::text = $2
           WHERE m.harvest_import_id = $1"#,
        harvest_id,
        kind,
    )
    .fetch_optional(pool)
    .await
    .unwrap()
    .map(|row| (row.number, row.from_harvest))
}

#[sqlx::test(migrations = "./migrations")]
async fn an_api_import_numbers_created_records_with_their_harvest_ids(pool: PgPool) {
    let org = setup(&pool).await;

    let report = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::Commit,
        &data_with_ids(),
        day(4),
    )
    .await
    .unwrap();

    assert!(report.row_errors.is_empty(), "{:?}", report.row_errors);
    for (kind, harvest_id) in [
        ("client", 70001),
        ("project", 70002),
        ("task", 70003),
        ("user", 70005),
        ("time_entry", 70010),
    ] {
        assert_eq!(
            number(&pool, kind, harvest_id).await,
            Some((harvest_id, true)),
            "{kind}"
        );
    }
    // The person who existed before keeps the number clients may know.
    assert_eq!(
        number(&pool, "user", 70004).await.map(|(_, from)| from),
        Some(false)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_preview_leaves_no_numbers_behind(pool: PgPool) {
    let org = setup(&pool).await;

    apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::DryRun,
        &data_with_ids(),
        day(4),
    )
    .await
    .unwrap();

    let numbers = sqlx::query_scalar!("SELECT count(*) FROM api_ids WHERE from_harvest")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(numbers, Some(0));
}
