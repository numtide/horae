//! "Invoiced?" Yes in a CSV marks the entry billed with no invoice (Q5).

use super::super::csv_source::{csv_body, import_body};
use super::*;

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn csv_billed_rows_are_invoiced_with_no_invoice(pool: PgPool) {
    let org = seed_org(&pool).await;
    seed_user(&pool, org, "dev@acme.com").await;
    let csv = "Date,Client,Project,Task,Hours,Email,Notes,Invoiced?\n\
               2026-01-15,Acme,Website,Design,1,dev@acme.com,billed,Yes\n\
               2026-01-16,Acme,Website,Design,1,dev@acme.com,open,No\n";
    for (mode, marked) in [
        (ImportMode::DryRun, 1),
        (ImportMode::Commit, 1),
        (ImportMode::Commit, 0),
    ] {
        let report = import_body(&pool, org, "USD", csv_body(csv), mode)
            .await
            .unwrap();
        assert_eq!(report.error_count(), 0, "{report:?}");
        assert_eq!(report.summary.links.billed_without_invoice, marked);
        assert_eq!(report.summary.invoices.processed(), 0);
    }
    let states = sqlx::query!(
        r#"SELECT notes AS "notes!", state::text AS "state!", invoice_id
           FROM time_entries WHERE org_id = $1 ORDER BY spent_date"#,
        org
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        states
            .iter()
            .map(|row| (row.notes.as_str(), row.state.as_str(), row.invoice_id))
            .collect::<Vec<_>>(),
        [("billed", "invoiced", None), ("open", "open", None)]
    );
}
