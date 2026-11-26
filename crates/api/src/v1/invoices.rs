//! Invoices, read-only (spec 028): the Invoices page's list, an invoice's
//! detail and its PDF, with the web app's whole-invoice read grant. No OAuth
//! scope covers invoices yet, so only personal access tokens reach them.

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::{KnownFormat, SchemaFormat};
use utoipa::openapi::{ContentBuilder, ObjectBuilder, RefOr, ResponseBuilder, Type};
use utoipa::{IntoParams, ToResponse};
use uuid::Uuid;

use super::types::{Invoice, InvoiceDetail, InvoiceStatus, Page};
use super::{ProblemResponse, V1State, catalogue};
use crate::cursor::Walk;
use crate::ops::invoices::reads::{self, InvoiceQuery};
use crate::ops::{self, Caller, SnapshotLimits};
use crate::problem::{ApiPath, ApiQuery, Problem};

/// A page of the list and its filters. A cursor is bound to the filters.
#[derive(Debug, Clone, Default, Serialize, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct InvoicesQuery {
    /// Invoices per page, 1 to 1000.
    #[param(minimum = 1, maximum = 1000, default = 100)]
    limit: Option<u32>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
    /// Only invoices with this status.
    status: Option<InvoiceStatus>,
    /// Only invoices of this client.
    client_id: Option<Uuid>,
    /// Only invoices changed at or after this instant: a status change, or
    /// an edit of a draft.
    updated_since: Option<DateTime<Utc>>,
}

/// Every invoice of the organization, in `id` order. Needs the permission to
/// view all invoices.
#[utoipa::path(
    get,
    path = "/invoices",
    operation_id = "listInvoices",
    security(("token" = [])),
    tag = "invoices",
    params(InvoicesQuery),
    responses(
        (status = 200, description = "A page of invoices", body = Page<Invoice>),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn list(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiQuery(query): ApiQuery<InvoicesQuery>,
) -> Result<Json<Page<Invoice>>, Problem> {
    // The same filters give the same text, whatever the page.
    let filters = serde_json::to_string(&InvoicesQuery {
        limit: None,
        cursor: None,
        ..query.clone()
    })
    .map_err(|_| Problem::internal())?;
    let walk = Walk {
        key: &state.cursor_key,
        list: "invoices",
        filters: &filters,
    };
    let position = walk.position(query.limit, query.cursor.as_deref(), Utc::now())?;
    let page = InvoiceQuery {
        status: query.status.map(Into::into),
        client_id: query.client_id,
        updated_since: query.updated_since,
        after: position.after,
        limit: position.limit,
    };
    let mut read = ops::begin_read_snapshot(&state.pool, caller, SnapshotLimits::LockWait).await?;
    let result = reads::page(&mut read.tx, &read.actor, &page).await;
    let rows = read.finish(result).await?;
    Ok(catalogue::page(&walk, rows, position, |record| {
        record.item.invoice.id
    }))
}

/// One invoice with its lines.
#[utoipa::path(
    get,
    path = "/invoices/{id}",
    operation_id = "getInvoice",
    security(("token" = [])),
    tag = "invoices",
    params(("id" = Uuid, Path, description = "The invoice's id")),
    responses(
        (status = 200, description = "The invoice", body = InvoiceDetail),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn get(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Json<InvoiceDetail>, Problem> {
    let mut read = ops::begin_read_snapshot(&state.pool, caller, SnapshotLimits::LockWait).await?;
    let result = reads::get(&mut read.tx, &read.actor, id).await;
    Ok(Json(read.finish(result).await?.into()))
}

/// The invoice as the PDF the web app downloads, byte for byte the same for
/// the same invoice and organization details.
#[utoipa::path(
    get,
    path = "/invoices/{id}/pdf",
    operation_id = "getInvoicePdf",
    security(("token" = [])),
    tag = "invoices",
    params(("id" = Uuid, Path, description = "The invoice's id")),
    responses(
        (status = 200, response = PdfResponse),
        (status = "default", response = ProblemResponse),
    )
)]
pub async fn pdf(
    State(state): State<V1State>,
    Extension(caller): Extension<Caller>,
    ApiPath(id): ApiPath<Uuid>,
) -> Result<Response, Problem> {
    // The refusals of `get` first, in its words; the export checks again.
    let mut read = ops::begin_read_snapshot(&state.pool, caller, SnapshotLimits::LockWait).await?;
    let result = reads::require(&mut read.tx, &read.actor, id).await;
    read.finish(result).await?;
    crate::exports::invoice_pdf(&state.pool, caller, id)
        .await
        .map_err(export_problem)
}

/// The problem for an export's refusal.
fn export_problem(status: StatusCode) -> Problem {
    let (code, detail) = match status {
        StatusCode::FORBIDDEN => ("forbidden", "Current access is required"),
        StatusCode::NOT_FOUND => ("not_found", "Invoice not found"),
        StatusCode::PAYLOAD_TOO_LARGE => ("too_large", "The invoice is too large for a PDF"),
        StatusCode::SERVICE_UNAVAILABLE => (
            "unavailable",
            "Too many exports are being made; try again shortly",
        ),
        StatusCode::GATEWAY_TIMEOUT => ("timeout", "The PDF took too long to make"),
        _ => return Problem::internal(),
    };
    Problem::new(status, code, Some(detail.into()))
}

/// An invoice's PDF.
pub struct PdfResponse;

impl<'s> ToResponse<'s> for PdfResponse {
    fn response() -> (&'s str, RefOr<utoipa::openapi::Response>) {
        let binary = ObjectBuilder::new()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::Binary)))
            .build();
        let response = ResponseBuilder::new()
            .description("The invoice as a PDF, as an attachment named after its number")
            .content(
                "application/pdf",
                ContentBuilder::new().schema(Some(binary)).build(),
            )
            .build();
        ("Pdf", response.into())
    }
}

#[cfg(test)]
mod tests;
