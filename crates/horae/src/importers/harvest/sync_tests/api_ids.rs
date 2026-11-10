//! A record an API import creates takes its Harvest id as its id in the
//! Harvest-compatible API (`harvest_id`), unless another record of its kind
//! already has that id: then it gets a fresh one and the report says so
//! (FR-016).

use super::*;
use horae_core::importers::harvest::types::IdNotice;

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

const CREATED: [(EntityType, i64); 5] = [
    (EntityType::Client, 70001),
    (EntityType::Project, 70002),
    (EntityType::Task, 70003),
    (EntityType::Person, 70005),
    (EntityType::TimeEntry, 70010),
];

/// The API id of the record a Harvest record was imported into.
async fn api_id(
    pool: &PgPool,
    org: Uuid,
    entity: EntityType,
    harvest_import_id: i64,
) -> Option<i64> {
    let horae_id = provenance::lookup(pool, org, entity, harvest_import_id)
        .await
        .unwrap()?;
    Some(crate::harvest::ids::number(pool, entity.into(), horae_id).await)
}

/// Native records that already have each of the import's Harvest ids.
async fn take_every_harvest_id(pool: &PgPool, org: Uuid) {
    let (client, project, task, user) = (
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    );
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, harvest_id)
         VALUES ($1, $2, 'Other', 'USD', 70001)",
        client,
        org
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency, harvest_id)
         VALUES ($1, $2, $3, 'Other', 'USD', 70002)",
        project,
        org,
        client
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, harvest_id) VALUES ($1, $2, 'Other', 70003)",
        task,
        org
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, harvest_id)
         VALUES ($1, $2, 'other@example.com', 'Other', 70005)",
        user,
        org
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, harvest_id)
         VALUES ($1, $2, $3, $4, $5, '2026-01-02', 30, true, 70010)",
        Uuid::now_v7(),
        org,
        user,
        project,
        task
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn an_api_import_gives_created_records_their_harvest_ids(pool: PgPool) {
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
    assert!(report.id_notices.is_empty(), "{:?}", report.id_notices);
    for (entity, harvest_id) in CREATED {
        assert_eq!(
            api_id(&pool, org, entity, harvest_id).await,
            Some(harvest_id),
            "{entity:?}"
        );
    }
    // The person who existed before keeps the id clients may know.
    assert_ne!(
        api_id(&pool, org, EntityType::Person, 70004).await,
        Some(70004)
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn a_harvest_id_another_record_has_gets_a_fresh_id_and_a_report_line(pool: PgPool) {
    let org = setup(&pool).await;
    take_every_harvest_id(&pool, org).await;

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
    let mut expected = Vec::new();
    for (entity, harvest_id) in CREATED {
        let fresh = api_id(&pool, org, entity, harvest_id).await.unwrap();
        assert_ne!(fresh, harvest_id, "{entity:?}");
        expected.push(IdNotice {
            entity,
            harvest_id,
            api_id: fresh,
        });
    }
    assert_eq!(report.id_notices, expected);
}

#[sqlx::test(migrations = "./migrations")]
async fn a_preview_reports_the_ids_it_cannot_keep_and_writes_nothing(pool: PgPool) {
    let org = setup(&pool).await;
    take_every_harvest_id(&pool, org).await;

    let report = apply_api_data(
        &pool,
        org,
        "USD",
        ImportMode::DryRun,
        &data_with_ids(),
        day(4),
    )
    .await
    .unwrap();

    let noticed: Vec<_> = report
        .id_notices
        .iter()
        .map(|notice| (notice.entity, notice.harvest_id))
        .collect();
    assert_eq!(noticed, CREATED);
    let clients = sqlx::query_scalar!("SELECT count(*) FROM clients WHERE org_id = $1", org)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(clients, Some(1));
}

/// A native record that takes the number between the importer's check and its
/// insert: the import waits for it, then numbers its own record afresh instead
/// of failing it.
#[sqlx::test(migrations = "./migrations")]
async fn a_harvest_id_taken_during_the_insert_gets_a_fresh_id(pool: PgPool) {
    let org = setup(&pool).await;
    let mut racer = pool.begin().await.unwrap();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, harvest_id)
         VALUES ($1, $2, 'Racer', 'USD', 70001)",
        Uuid::now_v7(),
        org
    )
    .execute(&mut *racer)
    .await
    .unwrap();

    let import = tokio::spawn({
        let pool = pool.clone();
        async move {
            let data = data_with_ids();
            apply_api_data(&pool, org, "USD", ImportMode::Commit, &data, day(4)).await
        }
    });
    // The import's insert waits on the racer's uncommitted number.
    while sqlx::query_scalar!(r#"SELECT count(*) AS "waiting!" FROM pg_locks WHERE NOT granted"#)
        .fetch_one(&pool)
        .await
        .unwrap()
        == 0
    {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    racer.commit().await.unwrap();
    let report = import.await.unwrap().unwrap();

    assert!(report.row_errors.is_empty(), "{:?}", report.row_errors);
    let fresh = api_id(&pool, org, EntityType::Client, 70001).await.unwrap();
    assert_ne!(fresh, 70001);
    assert_eq!(
        report.id_notices,
        [IdNotice {
            entity: EntityType::Client,
            harvest_id: 70001,
            api_id: fresh,
        }]
    );
}
