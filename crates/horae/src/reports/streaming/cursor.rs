use chrono::NaiveDate;
use sqlx::PgConnection;

use super::*;

pub(super) mod groups;
mod project;
mod time;
pub(super) use project::{declare_projects, projects};
pub(super) use time::{declare_entries, entries};

pub(super) struct InvoiceRow {
    pub number: String,
    pub currency: String,
    pub issued_on: NaiveDate,
    pub due_on: NaiveDate,
    pub terms_days: i32,
    pub po_number: String,
    pub subtotal_cents: i64,
    pub discount_cents: i64,
    pub tax1_cents: i64,
    pub tax2_cents: i64,
    pub total_cents: i64,
    pub discount_bps: i16,
    pub tax1_bps: i16,
    pub tax2_name: Option<String>,
    pub tax2_bps: Option<i16>,
    pub line_id: Option<Uuid>,
    pub description: Option<String>,
    pub minutes: Option<i32>,
    pub rate_cents: Option<i64>,
    pub amount_cents: Option<i64>,
}

impl InvoiceRow {
    pub fn metadata(&self) -> [String; 5] {
        crate::reports::invoice_metadata(
            &self.currency,
            self.issued_on,
            self.due_on,
            self.terms_days,
            &self.po_number,
        )
    }

    pub fn breakdown(&self) -> Vec<(String, i64)> {
        crate::models::invoice::adjustment_breakdown(
            &horae_core::invoice::InvoiceAmounts {
                subtotal_cents: self.subtotal_cents,
                discount_cents: self.discount_cents,
                tax1_cents: self.tax1_cents,
                tax2_cents: self.tax2_cents,
                total_cents: self.total_cents,
            },
            self.discount_bps,
            self.tax1_bps,
            self.tax2_name.as_deref().zip(self.tax2_bps),
        )
    }

    pub fn write(
        &self,
        writer: &mut csv::Writer<Vec<u8>>,
        metadata: &[String; 5],
    ) -> Result<(), StatusCode> {
        writer
            .write_record(
                [
                    self.description
                        .as_deref()
                        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    &self
                        .minutes
                        .map(|minutes| crate::reports::format_hours2(minutes.into()))
                        .unwrap_or_default(),
                    &self
                        .rate_cents
                        .map(crate::reports::format_cents_plain)
                        .unwrap_or_default(),
                    &crate::reports::format_cents_plain(
                        self.amount_cents.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?,
                    ),
                ]
                .into_iter()
                .chain(metadata.iter().map(String::as_str)),
            )
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub(super) async fn declare_invoice(
    connection: &mut PgConnection,
    org_id: Uuid,
    invoice_id: Uuid,
) -> Result<(), StatusCode> {
    sqlx::query!(
        "DECLARE horae_csv NO SCROLL CURSOR FOR
         SELECT i.number,i.currency::text,i.issued_on,i.due_on,i.terms_days,i.po_number,
                i.subtotal_cents,i.discount_cents,i.tax1_cents,i.tax2_cents,i.total_cents,
                i.discount_bps,i.tax1_bps,i.tax2_name,i.tax2_bps,
                l.id,l.description,l.minutes,l.rate_cents,l.amount_cents,
                256::bigint + octet_length(i.number)::bigint + octet_length(i.currency::text)::bigint
                  + octet_length(i.po_number)::bigint + COALESCE(octet_length(i.tax2_name),0)::bigint
                  + COALESCE(octet_length(l.description),0)::bigint AS export_bytes
         FROM invoices i LEFT JOIN invoice_line_items l ON l.invoice_id=i.id
         WHERE i.id=$1 AND i.org_id=$2 ORDER BY l.id",
        invoice_id, org_id,
    ).execute(connection).await.map_err(database_error)?;
    Ok(())
}

pub(super) async fn invoice(
    connection: &mut PgConnection,
    limit: i32,
) -> Result<Vec<InvoiceRow>, StatusCode> {
    sqlx::query_as!(InvoiceRow,
        r#"SELECT number AS "number!",currency AS "currency!",issued_on AS "issued_on!: NaiveDate",
            due_on AS "due_on!: NaiveDate",terms_days AS "terms_days!",po_number AS "po_number!",
            subtotal_cents AS "subtotal_cents!",discount_cents AS "discount_cents!",
            tax1_cents AS "tax1_cents!",tax2_cents AS "tax2_cents!",total_cents AS "total_cents!",
            discount_bps AS "discount_bps!",tax1_bps AS "tax1_bps!",tax2_name,tax2_bps,
            line_id,description,minutes,rate_cents,amount_cents
         FROM fetch_csv_export_rows($1) AS source(number text,currency text,issued_on date,due_on date,
            terms_days integer,po_number text,subtotal_cents bigint,discount_cents bigint,tax1_cents bigint,
            tax2_cents bigint,total_cents bigint,discount_bps smallint,tax1_bps smallint,tax2_name text,
            tax2_bps smallint,line_id uuid,description text,minutes integer,rate_cents bigint,
            amount_cents bigint,export_bytes bigint)"#,
        limit,
    ).fetch_all(connection).await.map_err(database_error)
}
