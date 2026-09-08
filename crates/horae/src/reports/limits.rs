use axum::http::StatusCode;
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{DetailedReportRow, Invoice, InvoiceLine, OrgBranding};

use super::{ExportParams, ProjectExportRow};

const MAX_FIELD_BYTES: i32 = 32_767;

#[derive(Clone, Copy)]
struct Limits {
    rows: i64,
    bytes: i64,
}

const XLSX: Limits = Limits {
    rows: 10_000,
    bytes: 8 * 1024 * 1024,
};
const PDF: Limits = Limits {
    rows: 1_000,
    bytes: 1024 * 1024,
};

fn check(rows: i64, bytes: i64, field_bytes: i32, limits: Limits) -> Result<(), StatusCode> {
    if rows > limits.rows || bytes > limits.bytes || field_bytes > MAX_FIELD_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    Ok(())
}

fn database_error(error: sqlx::Error) -> StatusCode {
    tracing::error!("Export query failed: {error}");
    if error
        .as_database_error()
        .is_some_and(|error| error.code().as_deref() == Some("57014"))
    {
        StatusCode::GATEWAY_TIMEOUT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn begin(pool: &PgPool) -> Result<Transaction<'_, Postgres>, StatusCode> {
    let mut tx = pool.begin().await.map_err(database_error)?;
    // Size checks and payload reads must see exactly the same rows and text.
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    sqlx::query!("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    sqlx::query!("SET LOCAL idle_in_transaction_session_timeout = '10s'")
        .execute(&mut *tx)
        .await
        .map_err(database_error)?;
    Ok(tx)
}

pub(super) async fn entries(
    pool: &PgPool,
    org_id: Uuid,
    params: &ExportParams,
) -> Result<Vec<DetailedReportRow>, StatusCode> {
    let from: chrono::NaiveDate = params.from.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let to: chrono::NaiveDate = params.to.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut tx = begin(pool).await?;
    let size = sqlx::query!(
        r#"SELECT COUNT(*) as "rows!",
                  COALESCE(SUM(octet_length(project_name)::bigint + octet_length(task_name)
                    + octet_length(user_name) + COALESCE(octet_length(notes), 0)), 0)::bigint as "bytes!",
                  COALESCE(MAX(GREATEST(octet_length(project_name), octet_length(task_name),
                    octet_length(user_name), COALESCE(octet_length(notes), 0))), 0) as "field_bytes!"
           FROM (SELECT p.name project_name, t.name task_name, u.name user_name, te.notes
                 FROM time_entries te
                 JOIN projects p ON p.id = te.project_id
                 JOIN tasks t ON t.id = te.task_id
                 JOIN users u ON u.id = te.user_id
                 WHERE te.org_id = $6 AND te.spent_date BETWEEN $1 AND $2
                   AND ($3::uuid IS NULL OR p.client_id = $3)
                   AND ($4::uuid IS NULL OR te.project_id = $4)
                   AND ($5::uuid IS NULL OR te.user_id = $5)
                 LIMIT $7) bounded"#,
        from as chrono::NaiveDate, to as chrono::NaiveDate,
        params.client_id, params.project_id, params.user_id, org_id, XLSX.rows + 1,
    ).fetch_one(&mut *tx).await.map_err(database_error)?;
    check(size.rows, size.bytes, size.field_bytes, XLSX)?;
    let rows = super::fetch_entries(
        &mut *tx,
        org_id,
        from,
        to,
        params.client_id,
        params.project_id,
        params.user_id,
    )
    .await
    .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(rows)
}

pub(super) async fn projects(
    pool: &PgPool,
    org_id: Uuid,
    scope: &str,
) -> Result<Vec<ProjectExportRow>, StatusCode> {
    let mut tx = begin(pool).await?;
    let size = sqlx::query!(
        r#"SELECT COUNT(*) as "rows!",
                  COALESCE(SUM(octet_length(client_name)::bigint + COALESCE(octet_length(code), 0)
                    + octet_length(name) + octet_length(currency)), 0)::bigint as "bytes!",
                  COALESCE(MAX(GREATEST(octet_length(client_name), COALESCE(octet_length(code), 0),
                    octet_length(name), octet_length(currency))), 0) as "field_bytes!"
           FROM (SELECT c.name client_name, p.code, p.name, p.currency
                 FROM projects p JOIN clients c ON c.id = p.client_id
                 WHERE p.org_id = $1 AND CASE $2
                   WHEN 'budgeted' THEN p.active AND p.budget_kind <> 'none'
                   WHEN 'archived' THEN NOT p.active ELSE p.active END
                 LIMIT $3) bounded"#,
        org_id,
        scope,
        XLSX.rows + 1,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(database_error)?;
    check(size.rows, size.bytes, size.field_bytes, XLSX)?;
    let rows = super::fetch_projects_export(&mut *tx, org_id, scope)
        .await
        .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(rows)
}

async fn read_invoice(
    connection: &mut PgConnection,
    org_id: Uuid,
    invoice_id: Uuid,
    limits: Limits,
) -> Result<(Invoice, Vec<InvoiceLine>), StatusCode> {
    let size = sqlx::query!(
        r#"SELECT (octet_length(number)::bigint + octet_length(currency) + COALESCE(octet_length(notes), 0)) as "bytes!",
                  GREATEST(octet_length(number), octet_length(currency), COALESCE(octet_length(notes), 0)) as "field_bytes!"
           FROM invoices WHERE id = $1 AND org_id = $2"#,
        invoice_id, org_id,
    ).fetch_optional(&mut *connection).await.map_err(database_error)?.ok_or(StatusCode::NOT_FOUND)?;
    check(0, size.bytes, size.field_bytes, limits)?;
    let remaining = Limits {
        bytes: limits.bytes - size.bytes,
        ..limits
    };
    let size = sqlx::query!(
        r#"SELECT COUNT(*) as "rows!", COALESCE(SUM(octet_length(description)::bigint), 0)::bigint as "bytes!",
                  COALESCE(MAX(octet_length(description)), 0) as "field_bytes!"
           FROM (SELECT description FROM invoice_line_items WHERE invoice_id = $1 LIMIT $2) bounded"#,
        invoice_id, limits.rows + 1,
    ).fetch_one(&mut *connection).await.map_err(database_error)?;
    check(size.rows, size.bytes, size.field_bytes, remaining)?;
    super::fetch_invoice_from(connection, invoice_id, org_id)
        .await
        .map_err(database_error)?
        .ok_or(StatusCode::NOT_FOUND)
}

pub(super) async fn invoice(
    pool: &PgPool,
    org_id: Uuid,
    invoice_id: Uuid,
) -> Result<(Invoice, Vec<InvoiceLine>), StatusCode> {
    let mut tx = begin(pool).await?;
    let result = read_invoice(&mut tx, org_id, invoice_id, XLSX).await?;
    tx.commit().await.map_err(database_error)?;
    Ok(result)
}

pub(super) struct PdfInvoice {
    pub invoice: Invoice,
    pub lines: Vec<InvoiceLine>,
    pub client_name: String,
    pub client_address: Option<String>,
    pub client_tax_id: Option<String>,
    pub branding: OrgBranding,
}

pub(super) async fn pdf(
    pool: &PgPool,
    org_id: Uuid,
    invoice_id: Uuid,
) -> Result<PdfInvoice, StatusCode> {
    let mut tx = begin(pool).await?;
    let size = sqlx::query!(
        r#"SELECT COALESCE(SUM(octet_length(value)::bigint), 0)::bigint as "bytes!",
                  COALESCE(MAX(octet_length(value)), 0) as "field_bytes!"
           FROM (SELECT unnest(ARRAY[c.name, c.address, c.tax_id, o.provider_name,
                    o.provider_address, o.provider_tax_id, o.provider_email, o.provider_phone,
                    o.bank_name, o.bank_iban, o.bank_bic, o.bank_routing, o.bank_account,
                    o.invoice_notes, o.invoice_payment_terms]) value
                 FROM invoices i JOIN clients c ON c.id = i.client_id
                 JOIN organizations o ON o.id = i.org_id
                 WHERE i.id = $1 AND i.org_id = $2) fields"#,
        invoice_id,
        org_id,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(database_error)?;
    check(0, size.bytes, size.field_bytes, PDF)?;
    let remaining = Limits {
        bytes: PDF.bytes - size.bytes,
        ..PDF
    };
    let (invoice, lines) = read_invoice(&mut tx, org_id, invoice_id, remaining).await?;
    let client = sqlx::query!(
        "SELECT name, address, tax_id FROM clients WHERE id = $1",
        invoice.client_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(database_error)?;
    let branding = super::fetch_branding_from(&mut *tx, org_id)
        .await
        .map_err(database_error)?;
    tx.commit().await.map_err(database_error)?;
    Ok(PdfInvoice {
        invoice,
        lines,
        client_name: client.name,
        client_address: client.address,
        client_tax_id: client.tax_id,
        branding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_fns::test_seed::{SeedIds, seed};
    use horae_core::types::OrgRole;
    use std::io::{Cursor, Read};

    fn xlsx_part(bytes: &[u8], path: &str) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut content = String::new();
        zip.by_name(path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        content
    }

    fn assert_cell(sheet: &str, cell: &str, value: &str) {
        let (_, after) = sheet.split_once(&format!("r=\"{cell}\"")).unwrap();
        let (element, _) = after.split_once("</c>").unwrap();
        assert!(
            element.contains(&format!("<v>{value}</v>")),
            "{cell}: {element}"
        );
    }

    fn params() -> ExportParams {
        ExportParams {
            from: "2026-09-07".to_owned(),
            to: "2026-09-07".to_owned(),
            client_id: None,
            project_id: None,
            user_id: None,
        }
    }

    async fn add_entries(pool: &PgPool, ids: &SeedIds, count: usize) -> Vec<Uuid> {
        let keys: Vec<_> = (0..count).map(|_| Uuid::now_v7()).collect();
        sqlx::query!(
            "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable)
             SELECT id, $2, $3, $4, $5, '2026-09-07', 60, true FROM unnest($1::uuid[]) id",
            &keys, ids.org_id, ids.user_id, ids.project_id, ids.task_id,
        ).execute(pool).await.unwrap();
        keys
    }

    async fn add_invoice(pool: &PgPool, ids: &SeedIds, count: usize) -> (Uuid, Vec<Uuid>) {
        let entries = add_entries(pool, ids, count).await;
        let invoice_id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO invoices (id, org_id, client_id, number, issued_on, due_on, currency, total_cents)
             VALUES ($1, $2, $3, 'EXPORT-1', '2026-09-07', '2026-10-07', 'EUR', $4)",
            invoice_id, ids.org_id, ids.client_id, count as i64 * 1234,
        ).execute(pool).await.unwrap();
        sqlx::query!(
            "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
             SELECT id, $2, id, 'Development', 60, 1234, 1234 FROM unnest($1::uuid[]) id",
            &entries, invoice_id,
        ).execute(pool).await.unwrap();
        (invoice_id, entries)
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn bounded_entries_enforce_the_row_limit_after_all_filters(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let other = seed(&pool, OrgRole::Manager).await;
        add_entries(&pool, &other, 1).await;
        add_entries(&pool, &ids, XLSX.rows as usize).await;
        assert_eq!(
            entries(&pool, ids.org_id, &params()).await.unwrap().len(),
            XLSX.rows as usize
        );
        add_entries(&pool, &ids, 1).await;
        assert!(matches!(
            entries(&pool, ids.org_id, &params()).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
        for filter in [
            ExportParams {
                client_id: Some(other.client_id),
                ..params()
            },
            ExportParams {
                project_id: Some(other.project_id),
                ..params()
            },
            ExportParams {
                user_id: Some(other.user_id),
                ..params()
            },
            ExportParams {
                from: "2026-09-08".to_owned(),
                to: "2026-09-08".to_owned(),
                ..params()
            },
        ] {
            assert!(
                entries(&pool, ids.org_id, &filter)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn bounded_entries_reject_large_fields_and_combined_text(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        add_entries(&pool, &ids, 1).await;
        sqlx::query!(
            "UPDATE time_entries SET notes = repeat('界', 10923) WHERE org_id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            entries(&pool, ids.org_id, &params()).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
        sqlx::query!(
            "UPDATE time_entries SET notes = repeat('n', 32767) WHERE org_id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            entries(&pool, ids.org_id, &params()).await.unwrap()[0]
                .notes
                .as_ref()
                .unwrap()
                .len(),
            32767
        );
        add_entries(&pool, &ids, 299).await;
        sqlx::query!(
            "UPDATE time_entries SET notes = repeat('n', 32767) WHERE org_id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            entries(&pool, ids.org_id, &params()).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn bounded_projects_filter_archives_before_size_checks(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let _other = seed(&pool, OrgRole::Manager).await;
        let keys: Vec<_> = (0..=XLSX.rows).map(|_| Uuid::now_v7()).collect();
        sqlx::query!("INSERT INTO projects (id, org_id, client_id, name, currency, active) SELECT id, $2, $3, 'Archived', 'EUR', false FROM unnest($1::uuid[]) id", &keys, ids.org_id, ids.client_id).execute(&pool).await.unwrap();
        for scope in ["active", "unknown"] {
            let rows = projects(&pool, ids.org_id, scope).await.unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].name, "Widget");
        }
        assert!(
            projects(&pool, ids.org_id, "budgeted")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            projects(&pool, ids.org_id, "archived").await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
        sqlx::query!(
            "UPDATE projects SET budget_kind = 'hours', budget_minutes = 60 WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            projects(&pool, ids.org_id, "budgeted").await.unwrap().len(),
            1
        );
        sqlx::query!(
            "UPDATE projects SET name = repeat('x', 32768) WHERE id = $1",
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            projects(&pool, ids.org_id, "active").await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn bounded_invoice_limits_include_branding_and_preserve_tenant_scope(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let other = seed(&pool, OrgRole::Manager).await;
        let (id, lines) = add_invoice(&pool, &ids, PDF.rows as usize + 1).await;
        assert_eq!(
            invoice(&pool, ids.org_id, id).await.unwrap().1.len(),
            PDF.rows as usize + 1
        );
        assert!(matches!(
            pdf(&pool, ids.org_id, id).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
        sqlx::query!("DELETE FROM invoice_line_items WHERE id = $1", lines[0])
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            pdf(&pool, ids.org_id, id).await.unwrap().lines.len(),
            PDF.rows as usize
        );
        assert!(matches!(
            invoice(&pool, other.org_id, id).await,
            Err(StatusCode::NOT_FOUND)
        ));
        assert!(matches!(
            pdf(&pool, other.org_id, id).await,
            Err(StatusCode::NOT_FOUND)
        ));
        sqlx::query!(
            "UPDATE organizations SET provider_address = repeat('x', 32768) WHERE id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(matches!(
            pdf(&pool, ids.org_id, id).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
        assert!(invoice(&pool, ids.org_id, id).await.is_ok());
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn export_size_checks_and_reads_share_a_repeatable_read_snapshot(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let (id, _) = add_invoice(&pool, &ids, 1).await;
        let mut tx = begin(&pool).await.unwrap();
        let before = read_invoice(&mut tx, ids.org_id, id, XLSX).await.unwrap();
        sqlx::query!(
            "UPDATE invoices SET notes = repeat('x', 32768) WHERE id = $1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        let during = read_invoice(&mut tx, ids.org_id, id, XLSX).await.unwrap();
        assert_eq!(before, during);
        tx.commit().await.unwrap();
        assert!(matches!(
            invoice(&pool, ids.org_id, id).await,
            Err(StatusCode::PAYLOAD_TOO_LARGE)
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn export_queries_are_read_only_and_have_a_database_deadline(pool: PgPool) {
        let mut tx = begin(&pool).await.unwrap();
        let settings = sqlx::query!(r#"SELECT current_setting('transaction_isolation') as "isolation!", current_setting('statement_timeout') as "timeout!""#).fetch_one(&mut *tx).await.unwrap();
        assert_eq!(settings.isolation, "repeatable read");
        assert_eq!(settings.timeout, "5s");
        let write = sqlx::query!("DELETE FROM time_entries WHERE false")
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(
            write.as_database_error().unwrap().code().as_deref(),
            Some("25006")
        );
        tx.rollback().await.unwrap();
        let mut tx = begin(&pool).await.unwrap();
        let slow = sqlx::query!("DO $$ BEGIN PERFORM pg_sleep(10); END $$")
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(database_error(slow), StatusCode::GATEWAY_TIMEOUT);
        tx.rollback().await.unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    async fn bounded_exports_preserve_real_workbooks_and_deterministic_pdf(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let (id, _) = add_invoice(&pool, &ids, 1).await;
        sqlx::query!(
            "UPDATE time_entries SET rounded_minutes = 0, notes = 'Café <&>' WHERE org_id = $1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let rows = entries(&pool, ids.org_id, &params()).await.unwrap();
        let sheet = super::super::entries_xlsx(&rows).unwrap();
        let xml = xlsx_part(&sheet, "xl/worksheets/sheet1.xml");
        assert_cell(&xml, "E2", "1");
        assert_cell(&xml, "F2", "0");
        assert!(xlsx_part(&sheet, "xl/sharedStrings.xml").contains("Café &lt;&amp;&gt;"));
        let rows = projects(&pool, ids.org_id, "active").await.unwrap();
        let sheet = super::super::projects_xlsx(&rows).unwrap();
        assert!(xlsx_part(&sheet, "xl/sharedStrings.xml").contains("Widget"));
        let (invoice, lines) = invoice(&pool, ids.org_id, id).await.unwrap();
        let sheet = super::super::invoice_xlsx(&invoice, &lines).unwrap();
        let xml = xlsx_part(&sheet, "xl/worksheets/sheet1.xml");
        assert_cell(&xml, "C2", "12.34");
        assert_cell(&xml, "D2", "12.34");
        assert_cell(&xml, "D3", "12.34");
        let document = pdf(&pool, ids.org_id, id).await.unwrap();
        tokio::task::spawn_blocking(move || {
            let render = || {
                crate::render::render_invoice_pdf(
                    &document.invoice,
                    &document.lines,
                    &document.client_name,
                    document.client_address.as_deref(),
                    document.client_tax_id.as_deref(),
                    &document.branding,
                )
                .unwrap()
            };
            let first = render();
            assert!(first.starts_with(b"%PDF-"));
            assert_eq!(first, render());
        })
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "./migrations")]
    #[serial_test::serial]
    #[ignore = "manual release export measurement; set HORAE_EXPORT_PROBE_DIR for artifacts"]
    async fn measure_bounded_export_limits(pool: PgPool) {
        let ids = seed(&pool, OrgRole::Manager).await;
        let (id, _) = add_invoice(&pool, &ids, PDF.rows as usize).await;
        add_entries(&pool, &ids, (XLSX.rows - PDF.rows) as usize).await;
        let start = std::time::Instant::now();
        let rows = entries(&pool, ids.org_id, &params()).await.unwrap();
        assert_eq!(rows.len(), XLSX.rows as usize);
        let loaded = start.elapsed();
        let bytes = tokio::task::spawn_blocking(move || super::super::entries_xlsx(&rows).unwrap())
            .await
            .unwrap();
        eprintln!(
            "XLSX rows={}, load={loaded:?}, total={:?}, bytes={}",
            XLSX.rows,
            start.elapsed(),
            bytes.len()
        );
        if let Ok(directory) = std::env::var("HORAE_EXPORT_PROBE_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join("bounded-timesheet.xlsx"),
                &bytes,
            )
            .unwrap();
        }
        drop(bytes);
        let start = std::time::Instant::now();
        let document = pdf(&pool, ids.org_id, id).await.unwrap();
        let loaded = start.elapsed();
        let bytes = tokio::task::spawn_blocking(move || {
            crate::render::render_invoice_pdf(
                &document.invoice,
                &document.lines,
                &document.client_name,
                document.client_address.as_deref(),
                document.client_tax_id.as_deref(),
                &document.branding,
            )
            .unwrap()
        })
        .await
        .unwrap();
        eprintln!(
            "PDF rows={}, load={loaded:?}, total={:?}, bytes={}",
            PDF.rows,
            start.elapsed(),
            bytes.len()
        );
        if let Ok(directory) = std::env::var("HORAE_EXPORT_PROBE_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join("bounded-invoice.pdf"),
                &bytes,
            )
            .unwrap();
        }
        #[cfg(target_os = "linux")]
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status
                .lines()
                .filter(|line| line.starts_with("VmHWM:") || line.starts_with("VmRSS:"))
            {
                eprintln!("{line}");
            }
        }
    }
}
