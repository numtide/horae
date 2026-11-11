//! Committing a finished preview from its run page, and grouping a run's
//! errors (spec 019 US2).

use super::*;
use crate::models::CommitAvailability;
use horae_core::importers::harvest::types::{
    EntityType, ImportReport, PersonError, PersonErrorKind, RowError, RowOutcome, SourceKind,
};

/// Mark a job as a preview that finished with `report`.
async fn finish(pool: &PgPool, job: Uuid, report: &ImportReport) {
    sqlx::query!(
        "UPDATE horae_jobs SET status = 'succeeded', finished_at = now(), report = $2 WHERE id = $1",
        job,
        serde_json::to_value(report).unwrap(),
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn api_preview(pool: &PgPool, ids: &SeedIds, key: &str) -> Uuid {
    let preview = start_api(
        pool,
        ids.org_id,
        ids.user_id,
        &payload(),
        key,
        JobPolicy::default(),
        0,
    )
    .await
    .unwrap();
    let report = ImportReport::new(SourceKind::HarvestApi, ImportMode::DryRun);
    finish(pool, preview.id, &report).await;
    preview.id
}

async fn csv_preview(pool: &PgPool, ids: &SeedIds, key: &str) -> Uuid {
    let preview = start_csv(
        pool,
        ids.org_id,
        ids.user_id,
        ImportMode::DryRun,
        CSV.to_vec(),
        key,
        JobPolicy::default(),
    )
    .await
    .unwrap();
    let report = ImportReport::new(SourceKind::Csv, ImportMode::DryRun);
    finish(pool, preview.id, &report).await;
    preview.id
}

async fn commit(pool: &PgPool, ids: &SeedIds, preview: Uuid) -> Result<JobStatus, ServerFnError> {
    commit_preview(pool, ids.org_id, ids.user_id, preview, JobPolicy::default()).await
}

async fn availability(pool: &PgPool, ids: &SeedIds, job: Uuid) -> CommitAvailability {
    status(pool, ids.org_id, ids.user_id, job)
        .await
        .unwrap()
        .unwrap()
        .commit_availability
}

fn refused_with(result: Result<JobStatus, ServerFnError>, code: u16, message: &str) {
    match result {
        Err(ServerFnError::ServerError {
            code: actual,
            message: actual_message,
            ..
        }) if actual == code && actual_message.contains(message) => {}
        other => panic!("expected {code} {message:?}, got {other:?}"),
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn an_api_preview_commits_once_with_its_scope_and_account(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let preview = api_preview(&pool, &ids, "preview").await;
    let shown = status(&pool, ids.org_id, ids.user_id, preview)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(shown.mode, Some(ImportMode::DryRun));
    assert_eq!(shown.source, Some(SourceKind::HarvestApi));
    assert_eq!(shown.commit_availability, CommitAvailability::Available);

    let first = commit(&pool, &ids, preview).await.unwrap();
    let again = commit(&pool, &ids, preview).await.unwrap();
    assert_eq!(first.id, again.id, "a second click returns the same commit");
    assert_eq!(first.mode, Some(ImportMode::Commit));
    assert_eq!(first.kind, "harvest_api_import");
    assert_eq!(first.commit_availability, CommitAvailability::NotPreview);
    let payload = sqlx::query_scalar!("SELECT payload FROM horae_jobs WHERE id = $1", first.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(payload["payload"]["value"]["sync"], "Full", "{payload}");
    // The preview points at its own commit, not at "a newer import".
    let shown = status(&pool, ids.org_id, ids.user_id, preview)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(shown.commit_availability, CommitAvailability::Committed);
    assert_eq!(shown.committed_as, Some(first.id));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_preview_of_a_previous_account_is_refused(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let preview = api_preview(&pool, &ids, "preview").await;
    sqlx::query!(
        "UPDATE harvest_connection_generations SET account_generation = 1 WHERE org_id = $1",
        ids.org_id,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        availability(&pool, &ids, preview).await,
        CommitAvailability::PreviousAccount
    );
    refused_with(
        commit(&pool, &ids, preview).await,
        CONFLICT,
        "account changed",
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_csv_preview_commits_its_retained_upload_until_it_expires(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let preview = csv_preview(&pool, &ids, "first").await;
    let committed = commit(&pool, &ids, preview).await.unwrap();
    assert_eq!(committed.mode, Some(ImportMode::Commit));
    assert_eq!(committed.source, Some(SourceKind::Csv));
    let body = sqlx::query_scalar!(
        "SELECT body FROM horae_job_uploads WHERE job_id = $1",
        committed.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(body, CSV);

    let expired = csv_preview(&pool, &ids, "second").await;
    sqlx::query!("DELETE FROM horae_job_uploads WHERE job_id = $1", expired)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        availability(&pool, &ids, expired).await,
        CommitAvailability::MissingUpload
    );
    refused_with(
        commit(&pool, &ids, expired).await,
        CONFLICT,
        "upload the file again",
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn only_the_latest_finished_preview_of_its_source_commits(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let old = api_preview(&pool, &ids, "old").await;
    // Another source's later run does not replace an API preview.
    csv_preview(&pool, &ids, "csv").await;
    assert_eq!(
        availability(&pool, &ids, old).await,
        CommitAvailability::Available
    );
    // Nor does a later run that failed or was cancelled.
    for (key, state) in [("failed", "failed"), ("cancelled", "cancelled")] {
        let dead = api_preview(&pool, &ids, key).await;
        sqlx::query!(
            "UPDATE horae_jobs SET status = $2 WHERE id = $1",
            dead,
            state
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            availability(&pool, &ids, old).await,
            CommitAvailability::Available,
            "{state}"
        );
    }
    let newer = api_preview(&pool, &ids, "newer").await;
    assert_eq!(
        availability(&pool, &ids, old).await,
        CommitAvailability::NewerImport
    );
    refused_with(commit(&pool, &ids, old).await, CONFLICT, "newer import");
    commit(&pool, &ids, newer).await.unwrap();

    // A run still going, or one that already wrote data, is not a preview to
    // commit.
    let running = start_api(
        &pool,
        ids.org_id,
        ids.user_id,
        &payload(),
        "queued",
        JobPolicy::default(),
        0,
    )
    .await
    .unwrap();
    refused_with(
        commit(&pool, &ids, running.id).await,
        CONFLICT,
        "Only a preview that finished",
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn committing_a_preview_needs_an_administrator_of_its_organization(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let preview = api_preview(&pool, &ids, "preview").await;
    let foreign = administrator(&pool).await;
    refused_with(
        commit_preview(
            &pool,
            foreign.org_id,
            foreign.user_id,
            preview,
            JobPolicy::default(),
        )
        .await,
        NOT_FOUND,
        "not found",
    );
    assign_profile(&pool, ids.org_id, ids.user_id, BuiltInProfile::Member).await;
    assert!(matches!(
        commit(&pool, &ids, preview).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
    assert_eq!(
        sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM horae_jobs"#)
            .fetch_one(&pool)
            .await
            .unwrap(),
        // The fixture's job and the preview.
        2
    );
}

fn unmatched(location: &str, email: &str) -> RowError {
    RowError {
        source_location: location.into(),
        entity: EntityType::TimeEntry,
        reason: format!("no Horae user matches email {email:?}"),
        person: Some(PersonError {
            kind: PersonErrorKind::UnmatchedEmail,
            email: Some(email.into()),
            name: Some("Dana Dev".into()),
        }),
        entry_kind: None,
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_summary_groups_archived_and_inline_errors_for_administrators(pool: PgPool) {
    let (ids, _) = fixture(&pool).await;
    let job = api_preview(&pool, &ids, "preview").await;
    let mut report = ImportReport::new(SourceKind::HarvestApi, ImportMode::DryRun);
    let archived = [
        unmatched("row 1", "dana@x.com"),
        unmatched("row 2", "dana@x.com"),
    ];
    let mut chunk = Vec::new();
    for error in &archived {
        report.record(
            error.entity,
            &RowOutcome::Errored {
                source_location: error.source_location.clone(),
                reason: error.reason.clone(),
                person: error.person.clone(),
                entry_kind: error.entry_kind,
            },
        );
        serde_json::to_writer(&mut chunk, error).unwrap();
        chunk.push(b'\n');
    }
    report.archive_errors(1).unwrap();
    report.record(
        EntityType::TimeEntry,
        &RowOutcome::Errored {
            source_location: "row 3".into(),
            reason: "bad hours".into(),
            person: None,
            entry_kind: None,
        },
    );
    finish(&pool, job, &report).await;
    sqlx::query!(
        "INSERT INTO horae_job_report_error_chunks (id, job_id, org_id, sequence, body)
         VALUES ($1, $2, $3, 0, $4)",
        Uuid::now_v7(),
        job,
        ids.org_id,
        chunk,
    )
    .execute(&pool)
    .await
    .unwrap();

    // An import or access change holding the organization does not delay it.
    let mut holder = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        ids.org_id
    )
    .fetch_one(&mut *holder)
    .await
    .unwrap();
    let summary = tokio::time::timeout(
        Duration::from_secs(5),
        error_summary(&pool, ids.org_id, ids.user_id, job),
    )
    .await
    .expect("the summary must not wait for the organization lock")
    .unwrap();
    holder.rollback().await.unwrap();
    let groups: Vec<_> = summary
        .groups
        .iter()
        .map(|group| {
            let locations: Vec<_> = group
                .samples
                .iter()
                .map(|sample| sample.location.clone())
                .collect();
            (group.kind, group.count, locations)
        })
        .collect();
    assert_eq!(
        groups,
        [
            (
                Some(PersonErrorKind::UnmatchedEmail),
                2,
                vec!["row 1".to_owned(), "row 2".to_owned()]
            ),
            (None, 1, vec!["row 3".to_owned()]),
        ]
    );
    assert_eq!(summary.people.len(), 1);
    assert_eq!(summary.people[0].email.as_deref(), Some("dana@x.com"));
    assert_eq!(summary.people[0].name.as_deref(), Some("Dana Dev"));
    assert_eq!(summary.people[0].entries, 2);

    let foreign = administrator(&pool).await;
    assert!(matches!(
        error_summary(&pool, foreign.org_id, foreign.user_id, job).await,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            ..
        })
    ));
    assign_profile(&pool, ids.org_id, ids.user_id, BuiltInProfile::Member).await;
    assert!(matches!(
        error_summary(&pool, ids.org_id, ids.user_id, job).await,
        Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        })
    ));
}
