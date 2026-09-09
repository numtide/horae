// Server-only Axum handlers for CSV and XLSX export.
//
// These are plain Axum routes (not `#[server]` functions) because they
// return binary file data with custom Content-Type headers.

use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use horae_core::duration::format_hours2;
use horae_core::money::format_cents_plain;
use serde::Deserialize;
use tower_sessions::Session;

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
/// Absent client/project/user means "all", as on the page.
#[derive(Deserialize)]
pub struct ExportParams {
    pub from: String,
    pub to: String,
    pub client_id: Option<uuid::Uuid>,
    pub project_id: Option<uuid::Uuid>,
    pub user_id: Option<uuid::Uuid>,
}

/// The rows behind both the CSV/XLSX exports and the manager-only
/// `report_detailed` server fn — one query, so a download always matches what
/// the Reports page shows.
pub(crate) async fn fetch_entries(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
    client_id: Option<uuid::Uuid>,
    project_id: Option<uuid::Uuid>,
    user_id: Option<uuid::Uuid>,
) -> Result<Vec<crate::models::DetailedReportRow>, sqlx::Error> {
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
         ORDER BY te.spent_date, p.name, t.name, te.id"#,
        from as chrono::NaiveDate,
        to as chrono::NaiveDate,
        client_id,
        project_id,
        user_id,
        org_id,
    )
    .fetch_all(pool)
    .await
}

pub async fn export_csv(
    session: Session,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    // Same rows as the manager-only `report_detailed` server fn (every user's
    // hours and notes), so the same gate applies.
    let org_id = require_manager(&session).await?;

    let from: chrono::NaiveDate = params.from.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let to: chrono::NaiveDate = params.to.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let state = crate::state::global_state().await;
    let entries = fetch_entries(
        &state.db,
        org_id,
        from,
        to,
        params.client_id,
        params.project_id,
        params.user_id,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data = entries_csv(&entries)?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"timesheet.csv\"",
            ),
        ],
        data,
    ))
}

pub(crate) fn entries_csv(
    entries: &[crate::models::DetailedReportRow],
) -> Result<Vec<u8>, StatusCode> {
    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "Date",
        "Project",
        "Task",
        "User",
        "Hours",
        "Rounded Hours",
        "Billable",
        "Notes",
    ])
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for e in entries {
        wtr.write_record(&[
            e.spent_date.to_string(),
            e.project_name.clone(),
            e.task_name.clone(),
            e.user_name.clone(),
            format_hours2(e.minutes.into()),
            format_hours2(e.rounded_minutes.unwrap_or(e.minutes).into()),
            if e.billable { "Yes" } else { "No" }.into(),
            e.notes.clone().unwrap_or_default(),
        ])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

    let from: chrono::NaiveDate = params.from.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let to: chrono::NaiveDate = params.to.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let state = crate::state::global_state().await;
    let entries = fetch_entries(
        &state.db,
        org_id,
        from,
        to,
        params.client_id,
        params.project_id,
        params.user_id,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    let data = workbook
        .save_to_buffer()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

async fn fetch_projects_export(
    org_id: uuid::Uuid,
    scope: &str,
) -> Result<Vec<ProjectExportRow>, sqlx::Error> {
    let state = crate::state::global_state().await;
    let rows = sqlx::query_as!(
        ProjectExportRow,
        r#"SELECT c.name as client_name, p.code, p.name,
                  p.project_type as "project_type: horae_core::types::ProjectType",
                  p.currency,
                  p.budget_kind as "budget_kind: horae_core::types::BudgetKind",
                  p.budget_amount_cents, p.budget_minutes, p.active
           FROM projects p
           JOIN clients c ON c.id = p.client_id
           WHERE p.org_id = $1
           ORDER BY c.name, p.name"#,
        org_id,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(rows
        .into_iter()
        .filter(|r| match scope {
            "budgeted" => r.active && r.budget_kind != horae_core::types::BudgetKind::None,
            "archived" => !r.active,
            _ => r.active,
        })
        .collect())
}

const PROJECT_EXPORT_HEADERS: [&str; 7] = [
    "Client", "Code", "Project", "Type", "Currency", "Budget", "Status",
];

pub async fn export_projects_csv(
    session: Session,
    Query(params): Query<ProjectsExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (_, org_id) = require_session(&session).await?;

    let scope = params.scope.as_deref().unwrap_or("active");
    let rows = fetch_projects_export(org_id, scope)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(PROJECT_EXPORT_HEADERS)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for r in &rows {
        wtr.write_record(&[
            r.client_name.clone(),
            r.code.clone().unwrap_or_default(),
            r.name.clone(),
            r.project_type.label().to_string(),
            r.currency.trim().to_string(),
            budget_cell(r),
            if r.active { "Active" } else { "Archived" }.to_string(),
        ])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let data = wtr
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"projects.csv\"",
            ),
        ],
        data,
    ))
}

pub async fn export_projects_xlsx(
    session: Session,
    Query(params): Query<ProjectsExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (_, org_id) = require_session(&session).await?;

    let scope = params.scope.as_deref().unwrap_or("active");
    let rows = fetch_projects_export(org_id, scope)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    let data = workbook
        .save_to_buffer()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

// ── Invoice export ────────────────────────────────────────────────────────────

/// An invoice and its line items, org-scoped — shared with the `get_invoice`
/// server fn so exports render exactly what the app serves. `None` when the
/// org has no such invoice; each caller maps that to its own not-found error.
pub(crate) async fn fetch_invoice_with_lines(
    invoice_id: uuid::Uuid,
    org_id: uuid::Uuid,
) -> Result<Option<(crate::models::Invoice, Vec<crate::models::InvoiceLine>)>, sqlx::Error> {
    use horae_core::types::InvoiceStatus;

    let state = crate::state::global_state().await;
    let Some(invoice) = sqlx::query_as!(
        crate::models::Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE id = $1 AND org_id = $2"#,
        invoice_id,
        org_id,
    )
    .fetch_optional(&state.db)
    .await?
    else {
        return Ok(None);
    };

    let lines = sqlx::query_as!(
        crate::models::InvoiceLine,
        r#"SELECT id, invoice_id, time_entry_id, description,
                  minutes, rate_cents, amount_cents
           FROM invoice_line_items
           WHERE invoice_id = $1
           ORDER BY id"#,
        invoice_id,
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Some((invoice, lines)))
}

/// The org's invoice branding block — shared with the `get_org_branding`
/// server fn so the PDF carries the same identity the settings page edits.
pub(crate) async fn fetch_org_branding(
    org_id: uuid::Uuid,
) -> Result<crate::models::OrgBranding, sqlx::Error> {
    let state = crate::state::global_state().await;
    sqlx::query_as!(
        crate::models::OrgBranding,
        r#"SELECT provider_name, provider_address, provider_tax_id,
                  provider_email, provider_phone,
                  bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                  invoice_notes, invoice_payment_terms
           FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(&state.db)
    .await
}

pub async fn export_invoice_csv(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;

    let (invoice, lines) = fetch_invoice_with_lines(invoice_id, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(["Description", "Hours", "Rate", "Amount"])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for line in &lines {
        wtr.write_record(&[
            line.description.clone(),
            format_hours2(line.minutes.into()),
            format_cents_plain(line.rate_cents),
            format_cents_plain(line.amount_cents),
        ])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Total row
    wtr.write_record(&[
        "Total".to_string(),
        String::new(),
        String::new(),
        format_cents_plain(invoice.total_cents),
    ])
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data = wtr
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("invoice-{}.csv", invoice.number);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        data,
    ))
}

pub async fn export_invoice_xlsx(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;

    let (invoice, lines) = fetch_invoice_with_lines(invoice_id, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = ["Description", "Hours", "Rate", "Amount"];
    for (col, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    for (row, line) in lines.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, &line.description)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 1, line.minutes as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 2, line.rate_cents as f64 / 100.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 3, line.amount_cents as f64 / 100.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Total row
    let total_row = (lines.len() + 1) as u32;
    worksheet
        .write_string(total_row, 0, "Total")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    worksheet
        .write_number(total_row, 3, invoice.total_cents as f64 / 100.0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data = workbook
        .save_to_buffer()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("invoice-{}.xlsx", invoice.number);
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

pub async fn export_invoice_pdf(
    session: Session,
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let org_id = require_manager(&session).await?;

    let (invoice, lines) = fetch_invoice_with_lines(invoice_id, org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let state = crate::state::global_state().await;

    // Fetch client name and address.
    let client = sqlx::query!(
        "SELECT name, address, tax_id FROM clients WHERE id = $1",
        invoice.client_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let branding = fetch_org_branding(invoice.org_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let pdf_bytes = crate::render::render_invoice_pdf(
        &invoice,
        &lines,
        &client.name,
        client.address.as_deref(),
        client.tax_id.as_deref(),
        &branding,
    )
    .map_err(|e| {
        tracing::error!("PDF rendering failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let filename = format!("invoice-{}.pdf", invoice.number);
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
