//! Read-only project attribution of stored invoice contributions.

use super::*;
use crate::models::project::ProjectInvoices;

#[server]
pub async fn get_project_invoices(project_id: String) -> Result<ProjectInvoices, ServerFnError> {
    let viewer = require_user().await?;
    let state = crate::state::global_state().await;
    fetch_project_invoices(
        &state.db,
        viewer.org_id,
        viewer.id,
        parse_uuid(&project_id, "project_id")?,
    )
    .await
}

#[cfg(feature = "server")]
async fn fetch_project_invoices(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    project_id: uuid::Uuid,
) -> Result<ProjectInvoices, ServerFnError> {
    use crate::models::project::ProjectInvoice;
    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query_scalar!(
        "SELECT project_id FROM project_read_access
         WHERE org_id=$1 AND project_id=$2 AND user_id=$3
           AND can_view_rates AND can_view_progress",
        org_id,
        project_id,
        viewer_id,
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Project not found"))?;
    // Lines, not the entry's current invoice_id, preserve void/reissue history.
    let invoices = sqlx::query_as!(ProjectInvoice,
        r#"WITH contributions AS (
             SELECT l.invoice_id, l.net_before_tax_cents
             FROM invoice_line_items l JOIN time_entries e ON e.id=l.time_entry_id
             WHERE e.org_id=$1 AND e.project_id=$2
             UNION ALL
             SELECT l.invoice_id, l.net_before_tax_cents
             FROM invoice_line_items l JOIN project_fee_occurrences f ON f.id=l.fee_occurrence_id
             WHERE f.org_id=$1 AND f.project_id=$2
           ), totals AS (
             SELECT invoice_id, SUM(net_before_tax_cents::numeric)::bigint AS amount
             FROM contributions GROUP BY invoice_id
           )
           SELECT i.id, i.number, i.status AS "status: InvoiceStatus", i.issued_on AS "issued_on: chrono::NaiveDate",
                  i.currency, t.amount AS "net_before_tax_cents!"
           FROM invoices i JOIN totals t ON t.invoice_id=i.id
           WHERE i.org_id=$1 ORDER BY i.issued_on DESC, i.created_at DESC, i.id DESC
           LIMIT 5001"#,
        org_id, project_id,
    ).fetch_all(&mut *tx).await.map_err(server_err)?;
    if invoices.len() > 5_000 {
        return Err(err(
            BAD_REQUEST,
            "More than 5000 project invoices. No partial history or totals are shown.",
        ));
    }
    let totals = horae_core::invoice::invoice_ledger_totals(
        invoices
            .iter()
            .map(|row| (row.currency.as_str(), row.status, row.net_before_tax_cents)),
    )
    .map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;
    Ok(ProjectInvoices { invoices, totals })
}

#[cfg(all(test, feature = "server"))]
mod tests;
