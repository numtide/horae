// Server-only Axum handlers for CSV and XLSX export.
//
// These are plain Axum routes (not `#[server]` functions) because they
// return binary file data with custom Content-Type headers.

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use futures_util::{Stream, TryStreamExt};
use horae_core::duration::format_hours2;
use horae_core::money::format_cents_plain;
use serde::Deserialize;
use tower_sessions::Session;

mod bounded;
mod limits;
mod streaming;

#[cfg(test)]
mod privacy_tests;

/// `login_redirect_guard` lets `/api/` through, because everything else there is
/// a server function that checks its own session. These handlers must too. The
/// `active` check is what revokes a deactivated user's still-live session
/// (FR-002). Returns the caller's `(user_id, org_id)`: the org comes free from
/// the same row and is what the export queries scope on.
async fn require_session(session: &Session) -> Result<(uuid::Uuid, uuid::Uuid), StatusCode> {
    let user_id = crate::auth::session::get_session_user_id(session)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let state = crate::state::global_state().await;
    let org_id = sqlx::query_scalar!(
        "SELECT org_id FROM users WHERE id = $1 AND active = true",
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;
    Ok((user_id, org_id))
}

/// Every invoice server function gates on `require_manager`, so exporting one
/// has to as well. Returns the manager's org id so the invoice fetches below
/// can be org-scoped exactly like their server-fn counterparts.
async fn require_manager(session: &Session) -> Result<uuid::Uuid, StatusCode> {
    let (user_id, _) = require_session(session).await?;
    let state = crate::state::global_state().await;
    let row = sqlx::query!(
        r#"SELECT org_id, org_role as "org_role: horae_core::types::OrgRole"
           FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::FORBIDDEN)?;

    row.org_role
        .is_manager_or_above()
        .then_some(row.org_id)
        .ok_or(StatusCode::FORBIDDEN)
}

/// Mirrors the Reports page filters, so a download matches what is on screen.
/// Absent client/project/user/tag means "all", as on the page.
#[derive(Deserialize)]
pub struct ExportParams {
    pub from: String,
    pub to: String,
    pub client_id: Option<uuid::Uuid>,
    pub project_id: Option<uuid::Uuid>,
    pub user_id: Option<uuid::Uuid>,
    pub tag_id: Option<uuid::Uuid>,
}

/// Entity filters shared by grouped reports, detailed rows and downloads.
#[derive(Clone, Copy, Default)]
pub(crate) struct ReportFilters {
    pub client_id: Option<uuid::Uuid>,
    pub project_id: Option<uuid::Uuid>,
    pub user_id: Option<uuid::Uuid>,
    pub tag_id: Option<uuid::Uuid>,
}

impl ExportParams {
    fn filters(&self) -> ReportFilters {
        ReportFilters {
            client_id: self.client_id,
            project_id: self.project_id,
            user_id: self.user_id,
            tag_id: self.tag_id,
        }
    }
}

/// The rows behind both the CSV/XLSX exports and the manager-only
/// `report_detailed` server fn — one query, so a download always matches what
/// the Reports page shows.
pub(crate) async fn fetch_entries<'e>(
    executor: impl sqlx::PgExecutor<'e> + 'e,
    org_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    filters: ReportFilters,
) -> Result<Vec<crate::models::DetailedReportRow>, sqlx::Error> {
    stream_entries(executor, org_id, period, filters)
        .try_collect()
        .await
}

fn stream_entries<'e>(
    executor: impl sqlx::PgExecutor<'e> + 'e,
    org_id: uuid::Uuid,
    period: (chrono::NaiveDate, chrono::NaiveDate),
    filters: ReportFilters,
) -> impl Stream<Item = Result<crate::models::DetailedReportRow, sqlx::Error>> + 'e {
    sqlx::query_as!(
        crate::models::DetailedReportRow,
        r#"SELECT te.spent_date as "spent_date: chrono::NaiveDate",
                p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes,
                effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) as "rounded_minutes?",
                (te.billable AND (te.invoice_id IS NOT NULL OR (p.project_type <> 'non_billable' AND COALESCE(pt.billable, t.billable_default)))) as "billable!", te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
         JOIN users u ON te.user_id = u.id
         JOIN organizations o ON o.id = te.org_id
         WHERE te.org_id = $6
           AND te.spent_date BETWEEN $1 AND $2
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::uuid IS NULL OR te.project_id = $4)
           AND ($5::uuid IS NULL OR te.user_id = $5)
           AND ($7::uuid IS NULL OR EXISTS (
             SELECT 1 FROM project_tag_links l
             WHERE l.org_id = te.org_id AND l.project_id = te.project_id AND l.tag_id = $7
           ))
         ORDER BY te.spent_date, p.name, t.name, te.id"#,
        period.0 as chrono::NaiveDate,
        period.1 as chrono::NaiveDate,
        filters.client_id,
        filters.project_id,
        filters.user_id,
        org_id,
        filters.tag_id,
    )
    .fetch(executor)
}

pub async fn export_csv(
    session: Session,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Same rows as the manager-only `report_detailed` server fn (every user's
    // hours and notes), so the same gate applies.
    let org_id = require_manager(&session).await?;

    let state = crate::state::global_state().await;
    streaming::entries(state.db.clone(), org_id, params).await
}

const ENTRY_EXPORT_HEADERS: [&str; 8] = [
    "Date",
    "Project",
    "Task",
    "User",
    "Hours",
    "Rounded Hours",
    "Billable",
    "Notes",
];

fn write_entry_csv(
    writer: &mut csv::Writer<Vec<u8>>,
    entry: &crate::models::DetailedReportRow,
) -> Result<(), csv::Error> {
    writer.write_record([
        entry.spent_date.to_string().as_str(),
        entry.project_name.as_str(),
        entry.task_name.as_str(),
        entry.user_name.as_str(),
        &format_hours2(entry.minutes.into()),
        &format_hours2(entry.rounded_minutes.unwrap_or(entry.minutes).into()),
        if entry.billable { "Yes" } else { "No" },
        entry.notes.as_deref().unwrap_or(""),
    ])
}

#[cfg(test)]
pub(crate) fn entries_csv(
    entries: &[crate::models::DetailedReportRow],
) -> Result<Vec<u8>, StatusCode> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(ENTRY_EXPORT_HEADERS)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for e in entries {
        write_entry_csv(&mut wtr, e).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    wtr.into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn export_xlsx(
    session: Session,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Same rows as the manager-only `report_detailed` server fn (every user's
    // hours and notes), so the same gate applies.
    let org_id = require_manager(&session).await?;
    let permit = bounded::ExportPermit::acquire()?;

    let state = crate::state::global_state().await;
    let entries = limits::entries(&state.db, org_id, &params).await?;

    let data = permit.render(move || entries_xlsx(&entries)).await?;

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"timesheet.xlsx\"",
            ),
        ],
        data,
    ))
}

fn entries_xlsx(entries: &[crate::models::DetailedReportRow]) -> Result<Vec<u8>, StatusCode> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "Date",
        "Project",
        "Task",
        "User",
        "Hours",
        "Rounded Hours",
        "Billable",
        "Notes",
    ];
    for (col, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    for (row, e) in entries.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, e.spent_date.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 1, &e.project_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 2, &e.task_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 3, &e.user_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 4, e.minutes as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 5, e.rounded_minutes.unwrap_or(e.minutes) as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 6, if e.billable { "Yes" } else { "No" })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 7, e.notes.as_deref().unwrap_or(""))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    bounded::workbook_bytes(&mut workbook)
}

// ── Projects export ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProjectsExportParams {
    /// "active" (default) | "budgeted" | "archived".
    pub scope: Option<String>,
}

struct ProjectExportRow {
    client_name: String,
    code: Option<String>,
    name: String,
    project_type: horae_core::types::ProjectType,
    currency: String,
    budget_kind: horae_core::types::BudgetKind,
    budget_amount_cents: Option<i64>,
    budget_minutes: Option<i64>,
    active: bool,
}

fn budget_cell(r: &ProjectExportRow) -> String {
    horae_core::money::format_budget(
        r.budget_kind,
        r.budget_amount_cents,
        r.budget_minutes,
        &r.currency,
    )
}

async fn fetch_projects_export<'e>(
    executor: impl sqlx::PgExecutor<'e> + 'e,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    scope: &'e str,
) -> Result<Vec<ProjectExportRow>, sqlx::Error> {
    stream_projects_export(executor, org_id, viewer_id, scope)
        .try_collect()
        .await
}

fn stream_projects_export<'e>(
    executor: impl sqlx::PgExecutor<'e> + 'e,
    org_id: uuid::Uuid,
    viewer_id: uuid::Uuid,
    scope: &'e str,
) -> impl Stream<Item = Result<ProjectExportRow, sqlx::Error>> + 'e {
    sqlx::query_as!(
        ProjectExportRow,
        r#"SELECT c.name as client_name, p.code, p.name,
                  p.project_type as "project_type: horae_core::types::ProjectType",
                  p.currency,
                  p.budget_kind as "budget_kind: horae_core::types::BudgetKind",
                  p.budget_amount_cents, p.budget_minutes, p.active
           FROM projects p
           JOIN clients c ON c.id = p.client_id
           JOIN project_read_access access ON access.project_id = p.id AND access.org_id = p.org_id
           WHERE p.org_id = $1 AND access.user_id = $3 AND access.can_view_progress AND CASE $2
             WHEN 'budgeted' THEN p.active AND p.budget_kind <> 'none'
             WHEN 'archived' THEN NOT p.active ELSE p.active END
           ORDER BY c.name, p.name, p.id"#,
        org_id,
        scope,
        viewer_id,
    )
    .fetch(executor)
}

const PROJECT_EXPORT_HEADERS: [&str; 7] = [
    "Client", "Code", "Project", "Type", "Currency", "Budget", "Status",
];

pub async fn export_projects_csv(
    session: Session,
    Query(params): Query<ProjectsExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (viewer_id, org_id) = require_session(&session).await?;

    let state = crate::state::global_state().await;
    streaming::projects(state.db.clone(), org_id, viewer_id, params).await
}

pub async fn export_projects_xlsx(
    session: Session,
    Query(params): Query<ProjectsExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (viewer_id, org_id) = require_session(&session).await?;
    let permit = bounded::ExportPermit::acquire()?;

    let scope = params.scope.as_deref().unwrap_or("active");
    let state = crate::state::global_state().await;
    let rows = limits::projects(&state.db, org_id, viewer_id, scope).await?;

    let data = permit.render(move || projects_xlsx(&rows)).await?;

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"projects.xlsx\"",
            ),
        ],
        data,
    ))
}

fn projects_xlsx(rows: &[ProjectExportRow]) -> Result<Vec<u8>, StatusCode> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    for (col, h) in PROJECT_EXPORT_HEADERS.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    for (row, r) in rows.iter().enumerate() {
        let cells = [
            r.client_name.clone(),
            r.code.clone().unwrap_or_default(),
            r.name.clone(),
            r.project_type.label().to_string(),
            r.currency.trim().to_string(),
            budget_cell(r),
            if r.active { "Active" } else { "Archived" }.to_string(),
        ];
        for (col, v) in cells.iter().enumerate() {
            worksheet
                .write_string((row + 1) as u32, col as u16, v)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    bounded::workbook_bytes(&mut workbook)
}

// ── Invoice export ────────────────────────────────────────────────────────────

/// An invoice and its line items, org-scoped — shared with the `get_invoice`
/// server fn so exports render exactly what the app serves. `None` when the
/// org has no such invoice; each caller maps that to its own not-found error.
pub(crate) async fn fetch_invoice_with_lines(
    invoice_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<(crate::models::Invoice, Vec<crate::models::InvoiceLine>)>, sqlx::Error> {
    let state = crate::state::global_state().await;
    let mut connection = state.db.acquire().await?;
    fetch_invoice_from(&mut connection, invoice_id, org_id).await
}

pub(crate) async fn fetch_invoice_from(
    connection: &mut sqlx::PgConnection,
    invoice_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<(crate::models::Invoice, Vec<crate::models::InvoiceLine>)>, sqlx::Error> {
    let Some(invoice) = fetch_invoice_metadata(&mut *connection, invoice_id, org_id).await? else {
        return Ok(None);
    };
    let lines = stream_invoice_lines(&mut *connection, invoice_id)
        .try_collect()
        .await?;
    Ok(Some((invoice, lines)))
}

pub(crate) async fn fetch_invoice_metadata(
    executor: impl sqlx::PgExecutor<'_>,
    invoice_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<crate::models::Invoice>, sqlx::Error> {
    use horae_core::types::InvoiceStatus;
    sqlx::query_as!(
        crate::models::Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes, terms_days, po_number,
                  discount_bps, tax1_bps, tax2_name, tax2_bps,
                  subtotal_cents, discount_cents, tax1_cents, tax2_cents,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE id = $1 AND org_id = $2"#,
        invoice_id,
        org_id,
    )
    .fetch_optional(executor)
    .await
}

fn stream_invoice_lines<'e>(
    executor: impl sqlx::PgExecutor<'e> + 'e,
    invoice_id: uuid::Uuid,
) -> impl Stream<Item = Result<crate::models::InvoiceLine, sqlx::Error>> + 'e {
    sqlx::query_as!(
        crate::models::InvoiceLine,
        r#"SELECT id, invoice_id, time_entry_id, fee_occurrence_id, description,
                  minutes, rate_cents, amount_cents
           FROM invoice_line_items
           WHERE invoice_id = $1
           ORDER BY id"#,
        invoice_id,
    )
    .fetch(executor)
}

/// The org's invoice branding block — shared with the `get_org_branding`
/// server fn so the PDF carries the same identity the settings page edits.
pub(crate) async fn fetch_org_branding(
    org_id: uuid::Uuid,
) -> Result<crate::models::OrgBranding, sqlx::Error> {
    let state = crate::state::global_state().await;
    fetch_branding_from(&state.db, org_id).await
}

async fn fetch_branding_from(
    executor: impl sqlx::PgExecutor<'_>,
    org_id: uuid::Uuid,
) -> Result<crate::models::OrgBranding, sqlx::Error> {
    sqlx::query_as!(
        crate::models::OrgBranding,
        r#"SELECT provider_name, provider_address, provider_tax_id,
                  provider_email, provider_phone,
                  bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                  invoice_notes, invoice_payment_terms
           FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(executor)
    .await
}

pub async fn export_invoice_csv(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;

    let state = crate::state::global_state().await;
    streaming::invoice(state.db.clone(), org_id, invoice_id).await
}

pub async fn export_invoice_xlsx(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;
    let permit = bounded::ExportPermit::acquire()?;

    let state = crate::state::global_state().await;
    let (invoice, lines) = limits::invoice(&state.db, org_id, invoice_id).await?;

    let filename = format!("invoice-{}.xlsx", invoice.number);
    let data = permit
        .render(move || invoice_xlsx(&invoice, &lines))
        .await?;

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        data,
    ))
}

const INVOICE_HEADERS: [&str; 9] = [
    "Description",
    "Hours",
    "Rate",
    "Amount",
    "Currency",
    "Issued on",
    "Due on",
    "Payment terms (days)",
    "Purchase order",
];

fn invoice_export_metadata(invoice: &crate::models::Invoice) -> [String; 5] {
    [
        invoice.currency.trim().into(),
        invoice.issued_on.to_string(),
        invoice.due_on.to_string(),
        invoice.terms_days.to_string(),
        invoice.po_number.clone(),
    ]
}

fn write_invoice_amount(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    column: u16,
    cents: i64,
) -> Result<(), StatusCode> {
    // Excel keeps 15 significant digits. Larger amounts must be text to retain cents.
    let result = if cents.unsigned_abs() <= 999_999_999_999_999 {
        sheet.write_number(row, column, cents as f64 / 100.0)
    } else {
        sheet.write_string(row, column, format_cents_plain(cents))
    };
    result
        .map(|_| ())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn invoice_xlsx(
    invoice: &crate::models::Invoice,
    lines: &[crate::models::InvoiceLine],
) -> Result<Vec<u8>, StatusCode> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (col, h) in INVOICE_HEADERS.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    for (row, line) in lines.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, &line.description)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(minutes) = line.minutes {
            worksheet
                .write_number(r, 1, minutes as f64 / 60.0)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        if let Some(rate) = line.rate_cents {
            write_invoice_amount(worksheet, r, 2, rate)?;
        }
        write_invoice_amount(worksheet, r, 3, line.amount_cents)?;
    }

    let breakdown = invoice.breakdown();
    for (index, (label, cents)) in breakdown.iter().enumerate() {
        let row = (lines.len() + index + 1) as u32;
        worksheet
            .write_string(row, 0, label)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        write_invoice_amount(worksheet, row, 3, *cents)?;
    }
    let metadata = invoice_export_metadata(invoice);
    for row in 1..=(lines.len() + breakdown.len()) as u32 {
        for (column, value) in metadata.iter().enumerate() {
            worksheet
                .write_string(row, column as u16 + 4, value)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    bounded::workbook_bytes(&mut workbook)
}

pub async fn export_invoice_pdf(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;
    let permit = bounded::ExportPermit::acquire()?;

    let state = crate::state::global_state().await;
    let document = limits::pdf(&state.db, org_id, invoice_id).await?;

    let filename = format!("invoice-{}.pdf", document.invoice.number);
    let pdf_bytes = permit
        .render(move || {
            crate::render::render_invoice_pdf(
                &document.invoice,
                &document.lines,
                &document.client_name,
                document.client_address.as_deref(),
                document.client_tax_id.as_deref(),
                &document.branding,
            )
            .map_err(|e| {
                tracing::error!("PDF rendering failed: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })
        })
        .await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/pdf".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        pdf_bytes,
    ))
}
