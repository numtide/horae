//! A CSV creates missing people from rows with an email, never from a name.

use axum::body::Body;
use horae_core::importers::harvest::types::{PersonErrorKind, PersonOutcome};

use super::super::csv_source::{import_body, import_body_with_lease};
use super::*;

const HEADER: &str = "Date,Client,Project,Task,Hours,First Name,Last Name,Email\n";

fn row(hours: &str, first: &str, last: &str, email: &str) -> String {
    format!("2026-01-15,Acme,Website,Design,{hours},{first},{last},{email}\n")
}

async fn account(pool: &PgPool, org: Uuid, email: &str) -> Option<(String, String, bool)> {
    sqlx::query!(
        r#"SELECT name, org_role::text AS "role!", active FROM users
           WHERE org_id = $1 AND email = $2 AND cost_rate_cents IS NULL
             AND billable_rate_cents IS NULL AND oidc_subject IS NULL"#,
        org,
        email,
    )
    .fetch_optional(pool)
    .await
    .unwrap()
    .map(|r| (r.name, r.role, r.active))
}

async fn import(
    pool: &PgPool,
    org: Uuid,
    csv: &str,
    mode: ImportMode,
) -> super::super::report::ImportReport {
    import_body(pool, org, "USD", Body::from(csv.to_owned()), mode)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn email_rows_create_one_deactivated_person_and_names_never_do(pool: PgPool) {
    let org = seed_org(&pool).await;
    let csv = format!(
        "{HEADER}{}{}{}",
        row("0.5", "New", "Person", "new@acme.com"),
        row("1", "New", "Person", "NEW@acme.com"),
        row("1", "Ghost", "Person", ""),
    );
    let preview = import(&pool, org, &csv, ImportMode::DryRun).await;
    assert_eq!(account(&pool, org, "new@acme.com").await, None);

    let commit = import(&pool, org, &csv, ImportMode::Commit).await;
    assert_eq!(preview.summary, commit.summary);
    assert_eq!(
        (commit.summary.people.created, commit.summary.people.skipped),
        (1, 0)
    );
    assert_eq!(commit.summary.time_entries.created, 2);
    assert_eq!(
        commit.row_errors[0].person.as_ref().unwrap().kind,
        PersonErrorKind::UnmatchedName
    );
    assert_eq!(
        account(&pool, org, "new@acme.com").await,
        Some(("New Person".into(), "member".into(), false))
    );
    assert_eq!(commit.people.len(), 1);
    assert_eq!(commit.people[0].outcome, PersonOutcome::Created);
    assert_eq!(
        super::super::sync_tests::people::queued_user_events(&pool, org).await,
        1
    );

    let again = import(&pool, org, &csv, ImportMode::Commit).await;
    assert_eq!(
        (again.summary.people.created, again.summary.people.skipped),
        (0, 1)
    );
    assert_eq!(again.summary.time_entries.skipped, 2);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_person_reached_by_name_and_by_email_counts_once(pool: PgPool) {
    let org = seed_org(&pool).await;
    let id = seed_user(&pool, org, "dev@acme.com").await;
    let csv = format!(
        "{HEADER}{}{}",
        row("1", "Dana", "Dev", ""),
        row("2", "Dana", "Dev", "dev@acme.com"),
    );
    let report = import(&pool, org, &csv, ImportMode::Commit).await;
    assert_eq!(report.summary.time_entries.created, 2);
    assert_eq!(
        (report.summary.people.skipped, report.summary.people.created),
        (1, 0)
    );
    assert_eq!(report.people.len(), 1);
    assert_eq!(report.people[0].horae_id, id);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_rolled_back_row_leaves_no_person_behind(pool: PgPool) {
    let org = seed_org(&pool).await;
    let csv = format!(
        "{HEADER}{}",
        row("not-hours", "Lost", "Row", "lost@acme.com")
    );
    let report = import(&pool, org, &csv, ImportMode::Commit).await;
    assert_eq!(report.summary.time_entries.errored, 1);
    assert_eq!(report.summary.people.processed(), 0);
    assert!(report.people.is_empty());
    assert_eq!(account(&pool, org, "lost@acme.com").await, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_durable_preview_keeps_a_simulated_person_across_batches(pool: PgPool) {
    use crate::jobs;

    let org = seed_org(&pool).await;
    // One minute each keeps 501 rows inside the person's daily limit.
    let csv = format!(
        "{HEADER}{}",
        row("0.02", "New", "Person", "new@acme.com").repeat(501)
    );
    jobs::enqueue_csv(
        &pool,
        org,
        ImportMode::DryRun,
        csv.clone().into_bytes(),
        "people-preview",
        Default::default(),
    )
    .await
    .unwrap();
    let (lease, stop) = jobs::claim_lease_for_test(&pool).await;
    jobs::run_claimed(&pool, &lease, stop, async {
        let report = import_body_with_lease(
            &pool,
            org,
            "USD",
            Body::from(csv),
            ImportMode::DryRun,
            Some(&lease),
        )
        .await?;
        assert_eq!(report.error_count(), 0, "{:?}", report.row_errors);
        assert_eq!(report.summary.people.created, 1);
        assert_eq!(report.summary.time_entries.created, 501);
        super::super::job_report(&report)
    })
    .await
    .unwrap();
    assert_eq!(account(&pool, org, "new@acme.com").await, None);
}
