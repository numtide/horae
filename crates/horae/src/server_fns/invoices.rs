//! Invoice server functions: thin adapters over `ops::invoices`.

use super::*;
use crate::models::invoice::{
    InvoiceDefaults, InvoiceDetailView, InvoiceDraftEdit, InvoiceDraftSave, InvoiceEditReview,
    InvoiceEditor, InvoiceFeeSelection, InvoiceGenerationRequest, InvoiceListItem,
    InvoicePreparation,
};

/// A snapshot read: every statement sees the state the actor's authority was
/// loaded in.
#[cfg(feature = "server")]
async fn begin_read(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
) -> Result<crate::ops::ReadTx, ServerFnError> {
    crate::ops::begin_read_snapshot(
        pool,
        crate::ops::Caller::session(org_id, actor_id),
        crate::ops::SnapshotLimits::LockWait,
    )
    .await
    .map_err(op_error)
}

/// User revocation holds the organization before updating this writer's
/// actor, so the shared gate comes before the actor's own locks and the
/// operation's invoice lock.
#[cfg(feature = "server")]
async fn begin_write(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
) -> Result<crate::ops::WriteTx, ServerFnError> {
    crate::ops::begin_gated(
        pool,
        crate::ops::Caller::session(org_id, actor_id),
        crate::db::OrganizationLock::Shared,
        crate::ops::WriteIsolation::ReadCommitted,
    )
    .await
    .map_err(op_error)
}

/// Parses an optional project selection; the operation bounds it.
#[cfg(feature = "server")]
fn parse_projects(
    project_ids: Option<Vec<String>>,
) -> Result<Option<Vec<uuid::Uuid>>, ServerFnError> {
    project_ids
        .map(|ids| {
            ids.iter()
                .map(|id| parse_uuid(id, "project_id"))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
}

#[server]
pub async fn get_invoice_editor(invoice_id: String) -> Result<InvoiceEditor, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    let mut read = begin_read(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::editing::load(&mut read.tx, &read.actor, id).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn review_invoice_edit(
    invoice_id: String,
    edit: InvoiceDraftEdit,
) -> Result<InvoiceEditReview, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    let mut read = begin_read(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::editing::review(&mut read.tx, &read.actor, id, &edit).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn save_invoice_draft(
    invoice_id: String,
    request: InvoiceDraftSave,
) -> Result<InvoiceWithLines, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    let mut write = begin_write(&state.db, caller.org_id, caller.user_id).await?;
    let result =
        crate::ops::invoices::editing::save(&mut write.tx, &write.actor, id, &request).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}

#[server]
pub async fn prepare_invoice(
    client_id: String,
    period_from: String,
    period_to: String,
    project_ids: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
    fees: Option<Vec<InvoiceFeeSelection>>,
) -> Result<InvoicePreparation, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let selected = parse_projects(project_ids)?;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let period = (
        parse_date(&period_from, "period_from")?,
        parse_date(&period_to, "period_to")?,
    );
    let mut read = begin_read(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::preview::prepare(
        &mut read.tx,
        &read.actor,
        client_id,
        period,
        selected.as_deref(),
        overrides.as_ref(),
        fees.as_deref(),
    )
    .await;
    read.finish(result).await.map_err(op_error)
}

#[cfg(all(test, feature = "server"))]
mod test_support;
#[cfg(all(test, feature = "server"))]
use crate::models::invoice::InvoiceSource;
#[cfg(all(test, feature = "server"))]
use test_support::{
    balances, defaults, editing, generate_invoice_with_options, generate_invoice_with_request,
    preview, transition_invoice, update_invoice_defaults_in_db,
};
#[cfg(all(test, feature = "server"))]
pub(super) use test_support::{fetch_detail, fetch_list, generate_invoice_for_period};

#[cfg(all(test, feature = "server"))]
mod tests;

#[cfg(all(test, feature = "server"))]
mod fee_tests;

#[cfg(all(test, feature = "server"))]
mod snapshot_tests;

#[cfg(all(test, feature = "server"))]
mod client_label_tests;

// ── Invoices ──────────────────────────────────────────────────────────────────

#[server]
pub async fn list_invoices(status: Option<String>) -> Result<Vec<InvoiceListItem>, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;

    let status_filter: Option<InvoiceStatus> = status
        .as_deref()
        .map(|s| parse_enum(s, "status"))
        .transpose()?;

    let mut read = begin_read(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::list_items(&mut read.tx, &read.actor, status_filter).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn get_invoice(invoice_id: String) -> Result<InvoiceDetailView, ServerFnError> {
    let caller = session_caller().await?;
    let id = parse_uuid(&invoice_id, "invoice_id")?;

    let state = crate::state::global_state().await;
    let mut read = begin_read(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::detail(&mut read.tx, &read.actor, id).await;
    read.finish(result).await.map_err(op_error)
}

#[server]
pub async fn generate_invoice(
    client_id: String,
    period_from: String,
    period_to: String,
    project_ids: Option<Vec<String>>,
    overrides: Option<InvoiceDefaults>,
    request: InvoiceGenerationRequest,
) -> Result<InvoiceWithLines, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let client_id = parse_uuid(&client_id, "client_id")?;
    let from = parse_date(&period_from, "period_from")?;
    let to = parse_date(&period_to, "period_to")?;
    let project_ids = parse_projects(project_ids)?;
    let mut write = begin_write(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::generate(
        &mut write.tx,
        &write.actor,
        crate::ops::invoices::Generation {
            client_id,
            period: (from, to),
            project_ids: project_ids.as_deref(),
            overrides: overrides.as_ref(),
            request: Some(&request),
        },
    )
    .await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}

#[server]
pub async fn update_invoice_status(
    invoice_id: String,
    new_status: String,
) -> Result<Invoice, ServerFnError> {
    let caller = session_caller().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&invoice_id, "invoice_id")?;
    let target: InvoiceStatus = parse_enum(&new_status, "status")?;

    let mut write = begin_write(&state.db, caller.org_id, caller.user_id).await?;
    let result = crate::ops::invoices::set_status(&mut write.tx, &write.actor, id, target).await;
    write
        .commit(result, &*state.plugins)
        .await
        .map_err(op_error)
}
