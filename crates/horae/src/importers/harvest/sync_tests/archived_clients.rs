//! specs/012-clients-design/increments/lifecycle LC-006, derived from 004
//! FR-011/014/015/017–021 and 015 FR-036: a new active project under an
//! archived local client is a visible record failure, and its time fails as a
//! dependent. The client is never restored. An incoming archived project and an
//! existing match import as before, and a re-run after restoring creates it once.

use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_new_active_project_of_an_archived_client_fails_as_a_record(pool: PgPool) {
    let org = setup(&pool).await;
    let client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency, active) VALUES ($1, $2, 'Client', 'USD', false)",
        client,
        org,
    )
    .execute(&pool)
    .await
    .unwrap();
    // Allowed before the archive prerequisite existed; a match is left unchanged.
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, 'Existing', 'USD')",
        Uuid::now_v7(),
        org,
        client,
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut source = valid_data();
    source.projects = serde_json::from_value(json!([
        {"id":2,"name":"Project","client":{"id":1}},
        {"id":6,"name":"Old project","client":{"id":1},"is_active":false},
        {"id":7,"name":"Existing","client":{"id":1}}
    ]))
    .unwrap();

    let preview = apply_api_data(&pool, org, "USD", ImportMode::DryRun, &source, day(4))
        .await
        .unwrap();
    let report = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(
        (&preview.summary, &preview.row_errors),
        (&report.summary, &report.row_errors)
    );
    assert!(report.reconciles(), "{report:?}");
    assert_eq!(
        (
            report.summary.projects.created,
            report.summary.projects.skipped,
            report.summary.projects.errored,
            report.summary.time_entries.errored
        ),
        (1, 1, 1, 1),
        "{report:?}"
    );
    assert!(report.row_errors.iter().any(|error| {
        error.source_location == "project 2" && error.reason.contains("client is archived")
    }));
    assert!(
        report
            .row_errors
            .iter()
            .any(|error| error.reason.contains("project 2 failed to import"))
    );
    let projects: Vec<(String, bool)> = sqlx::query!(
        "SELECT name, active FROM projects WHERE client_id = $1 ORDER BY name",
        client
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|project| (project.name, project.active))
    .collect();
    assert_eq!(
        projects,
        [
            ("Existing".to_owned(), true),
            ("Old project".to_owned(), false)
        ]
    );
    let active = sqlx::query_scalar!("SELECT active FROM clients WHERE id = $1", client)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!active, "an import must never restore the client");

    sqlx::query!("UPDATE clients SET active = true WHERE id = $1", client)
        .execute(&pool)
        .await
        .unwrap();
    let retry = apply_api_data(&pool, org, "USD", ImportMode::Commit, &source, day(4))
        .await
        .unwrap();
    assert_eq!(retry.error_count(), 0, "{retry:?}");
    assert_eq!(
        (
            retry.summary.projects.created,
            retry.summary.time_entries.created
        ),
        (1, 1),
        "{retry:?}"
    );
    let created = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM projects WHERE client_id = $1 AND name = 'Project'"#,
        client
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(created, 1);
}
