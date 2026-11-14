//! The invoice operations behind the pool-level calls the invoice tests use,
//! each run in the transaction its server function opens.

use super::*;
use crate::ops::testing::Recorder;

pub(super) use crate::ops::invoices::{balances, defaults};

pub(super) mod editing {
    use super::*;

    pub(in crate::server_fns::invoices) async fn load(
        pool: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        id: uuid::Uuid,
    ) -> Result<InvoiceEditor, ServerFnError> {
        let mut read = begin_read(pool, org_id, actor_id).await?;
        let result = crate::ops::invoices::editing::load(&mut read.tx, &read.actor, id).await;
        Ok(read.finish(result).await?)
    }

    pub(in crate::server_fns::invoices) async fn review(
        pool: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        id: uuid::Uuid,
        edit: &InvoiceDraftEdit,
    ) -> Result<InvoiceEditReview, ServerFnError> {
        let mut read = begin_read(pool, org_id, actor_id).await?;
        let result =
            crate::ops::invoices::editing::review(&mut read.tx, &read.actor, id, edit).await;
        Ok(read.finish(result).await?)
    }

    pub(in crate::server_fns::invoices) async fn save(
        pool: &sqlx::PgPool,
        org_id: uuid::Uuid,
        actor_id: uuid::Uuid,
        id: uuid::Uuid,
        request: &InvoiceDraftSave,
    ) -> Result<InvoiceWithLines, ServerFnError> {
        let mut write = begin_write(pool, org_id, actor_id).await?;
        let result =
            crate::ops::invoices::editing::save(&mut write.tx, &write.actor, id, request).await;
        Ok(write.commit(result, &Recorder::default()).await?)
    }
}

pub(super) mod preview {
    use super::*;

    pub(in crate::server_fns::invoices) async fn prepare(
        pool: &sqlx::PgPool,
        identity: (uuid::Uuid, uuid::Uuid),
        client_id: uuid::Uuid,
        period: (chrono::NaiveDate, chrono::NaiveDate),
        selected: Option<&[uuid::Uuid]>,
        overrides: Option<&InvoiceDefaults>,
    ) -> Result<InvoicePreparation, ServerFnError> {
        prepare_with_edits(pool, identity, client_id, period, selected, overrides, None).await
    }

    pub(in crate::server_fns::invoices) async fn prepare_with_edits(
        pool: &sqlx::PgPool,
        (org_id, actor_id): (uuid::Uuid, uuid::Uuid),
        client_id: uuid::Uuid,
        period: (chrono::NaiveDate, chrono::NaiveDate),
        selected: Option<&[uuid::Uuid]>,
        overrides: Option<&InvoiceDefaults>,
        edits: Option<&[InvoiceFeeSelection]>,
    ) -> Result<InvoicePreparation, ServerFnError> {
        let mut read = begin_read(pool, org_id, actor_id).await?;
        let result = crate::ops::invoices::preview::prepare(
            &mut read.tx,
            &read.actor,
            client_id,
            period,
            selected,
            overrides,
            edits,
        )
        .await;
        Ok(read.finish(result).await?)
    }
}

pub(in crate::server_fns) async fn fetch_list(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    status: Option<InvoiceStatus>,
) -> Result<Vec<InvoiceListItem>, ServerFnError> {
    let mut read = begin_read(pool, org_id, actor_id).await?;
    let result = crate::ops::invoices::list_items(&mut read.tx, &read.actor, status).await;
    Ok(read.finish(result).await?)
}

pub(in crate::server_fns) async fn fetch_detail(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    actor_id: uuid::Uuid,
    invoice_id: uuid::Uuid,
) -> Result<InvoiceDetailView, ServerFnError> {
    let mut read = begin_read(pool, org_id, actor_id).await?;
    let result = crate::ops::invoices::detail(&mut read.tx, &read.actor, invoice_id).await;
    Ok(read.finish(result).await?)
}

pub(super) async fn transition_invoice(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    id: uuid::Uuid,
    target: InvoiceStatus,
    actor_id: uuid::Uuid,
) -> Result<Invoice, ServerFnError> {
    let mut write = begin_write(pool, org_id, actor_id).await?;
    let result = crate::ops::invoices::set_status(&mut write.tx, &write.actor, id, target).await;
    Ok(write.commit(result, &Recorder::default()).await?)
}

/// The seeded person, created before anyone a test adds: the actor of the
/// calls that do not name one.
async fn seeded_actor(pool: &sqlx::PgPool, org_id: uuid::Uuid) -> uuid::Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM users WHERE org_id=$1 AND active ORDER BY id LIMIT 1",
        org_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub(in crate::server_fns) async fn generate_invoice_for_period(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Result<InvoiceWithLines, ServerFnError> {
    generate_invoice_with_options(pool, org_id, client_id, (from, to), None, None).await
}

pub(super) async fn generate_invoice_with_options(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    project_ids: Option<&[uuid::Uuid]>,
    overrides: Option<&InvoiceDefaults>,
) -> Result<InvoiceWithLines, ServerFnError> {
    generate_invoice_with_request(
        pool,
        org_id,
        client_id,
        period,
        project_ids,
        overrides,
        None,
    )
    .await
    .map(|(invoice, _)| invoice)
}

/// Generates as the request's actor, or as the seeded person without a
/// request. Returns whether a new invoice was created.
pub(super) async fn generate_invoice_with_request(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    project_ids: Option<&[uuid::Uuid]>,
    overrides: Option<&InvoiceDefaults>,
    request: Option<(&InvoiceGenerationRequest, uuid::Uuid)>,
) -> Result<(InvoiceWithLines, bool), ServerFnError> {
    let actor_id = match request {
        Some((_, actor_id)) => actor_id,
        None => seeded_actor(pool, org_id).await,
    };
    let mut write = begin_write(pool, org_id, actor_id).await?;
    let result = crate::ops::invoices::generate(
        &mut write.tx,
        &write.actor,
        crate::ops::invoices::Generation {
            client_id,
            period,
            project_ids,
            overrides,
            request: request.map(|(request, _)| request),
        },
    )
    .await;
    let events = Recorder::default();
    let invoice = write.commit(result, &events).await?;
    Ok((invoice, !events.take().is_empty()))
}

pub(super) async fn update_invoice_defaults_in_db(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    id: uuid::Uuid,
    overrides: &InvoiceDefaults,
) -> Result<Invoice, ServerFnError> {
    // The editor decides whether the seeded person may edit.
    let actor = seeded_actor(pool, org_id).await;
    let mut editor = editing::load(pool, org_id, actor, id).await?;
    editor.edit.defaults = overrides.clone();
    let review = editing::review(pool, org_id, actor, id, &editor.edit).await?;
    editing::save(
        pool,
        org_id,
        actor,
        id,
        &InvoiceDraftSave {
            request_id: uuid::Uuid::now_v7(),
            edit: editor.edit,
            review,
            confirmed_excess: Vec::new(),
        },
    )
    .await
    .map(|saved| saved.invoice)
}
