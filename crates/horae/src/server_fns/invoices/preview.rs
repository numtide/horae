use super::*;
use crate::models::invoice::{InvoicePreviewLine, InvoiceProjectDefaults};
use std::collections::BTreeSet;

pub(super) async fn prepare(
    pool: &sqlx::PgPool,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    (from, to): (chrono::NaiveDate, chrono::NaiveDate),
    selected: Option<&[uuid::Uuid]>,
    overrides: Option<&InvoiceDefaults>,
) -> Result<InvoicePreparation, ServerFnError> {
    if from > to {
        return Err(err(BAD_REQUEST, "Invoice period ends before it starts"));
    }
    if selected.is_some_and(|ids| ids.is_empty() || ids.len() > 1000) {
        return Err(err(BAD_REQUEST, "Select between 1 and 1000 projects"));
    }
    if let Some(overrides) = overrides {
        defaults::amounts(overrides, 0)?;
    }
    let selected = selected.map(|ids| {
        ids.iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    });
    let mut tx = pool.begin().await.map_err(server_err)?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(server_err)?;
    sqlx::query_scalar!(
        "SELECT id FROM clients WHERE id = $1 AND org_id = $2",
        client_id,
        org_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(server_err)?
    .ok_or_else(|| not_found("Client not found"))?;

    let entries = entries::read(
        &mut tx,
        org_id,
        client_id,
        (from, to),
        selected.as_deref(),
        SourceRead::Preview,
    )
    .await?;
    let mut lines = Vec::with_capacity(entries.len());
    for entry in entries {
        lines.push(InvoicePreviewLine {
            project_id: entry.project_id,
            description: entry.description(),
            amount_cents: entry.amount()?,
            currency: entry.currency,
            minutes: Some(entry.minutes),
            rate_cents: Some(entry.rate_cents.unwrap_or(0)),
        });
    }
    lines.extend(
        fees::preview_fees(&mut tx, org_id, client_id, from, to, selected.as_deref()).await?,
    );
    let contributing: Vec<_> = lines
        .iter()
        .map(|line| line.project_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let project_ids = selected.as_deref().unwrap_or(&contributing);
    if project_ids.len() > 1000 {
        return Err(conflict(
            "Too many projects for one preview; select fewer projects.",
        ));
    }
    let rows = sqlx::query!(
        "SELECT p.id, p.name, COALESCE(s.terms_days, 30)::smallint as \"terms_days!\",
                COALESCE(s.po_number, '') as \"po_number!\",
                COALESCE(s.discount_bps, 0)::smallint as \"discount_bps!\",
                COALESCE(s.tax1_bps, 0)::smallint as \"tax1_bps!\", s.tax2_name, s.tax2_bps
         FROM projects p LEFT JOIN project_settings s ON s.project_id = p.id
         WHERE p.org_id = $1 AND p.client_id = $2 AND p.id = ANY($3) ORDER BY p.id",
        org_id,
        client_id,
        project_ids,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;
    if rows.len() != project_ids.len() {
        return Err(not_found(
            "Some selected projects are unavailable for this client",
        ));
    }
    let projects: Vec<_> = rows
        .into_iter()
        .map(|row| InvoiceProjectDefaults {
            project_id: row.id,
            name: row.name,
            defaults: InvoiceDefaults {
                terms_days: row.terms_days,
                po_number: row.po_number,
                discount_bps: row.discount_bps,
                tax1_bps: row.tax1_bps,
                tax2_name: row.tax2_name,
                tax2_bps: row.tax2_bps,
            },
        })
        .collect();
    let currency = lines
        .first()
        .map(|line| line.currency.trim().to_owned())
        .ok_or_else(|| {
            not_found("No billable, un-invoiced time or fees found for this client and period.")
        })?;
    if lines.iter().any(|line| line.currency.trim() != currency) {
        return Err(conflict(
            "Projects with different billing currencies cannot share an invoice.",
        ));
    }
    let subtotal_cents = lines.iter().try_fold(0_i64, |subtotal, line| {
        subtotal.checked_add(line.amount_cents).ok_or_else(|| {
            conflict("Invoice total exceeds the supported range; select a shorter period.")
        })
    })?;
    let inherited = projects
        .first()
        .map(|project| &project.defaults)
        .filter(|first| projects.iter().all(|project| &project.defaults == *first));
    let defaults = overrides.or(inherited).cloned();
    let issued_on = chrono::Utc::now().date_naive();
    let amounts = defaults
        .as_ref()
        .map(|defaults| defaults::amounts(defaults, subtotal_cents))
        .transpose()?;
    let due_on = defaults
        .as_ref()
        .map(|defaults| {
            horae_core::project::payment_due_date(issued_on, defaults.terms_days as u16)
                .map_err(|error| err(BAD_REQUEST, error.to_string()))
        })
        .transpose()?;
    tx.commit().await.map_err(server_err)?;
    Ok(InvoicePreparation {
        issued_on,
        due_on,
        currency,
        projects,
        defaults,
        lines,
        subtotal_cents,
        amounts,
    })
}
