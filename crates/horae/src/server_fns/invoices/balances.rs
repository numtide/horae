use super::*;
use horae_core::project::Percentage;
use uuid::Uuid;

pub(super) async fn lock_invoices(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
) -> Result<(), ServerFnError> {
    sqlx::query_scalar!(
        r#"SELECT pg_advisory_xact_lock(hashtextextended($1, 0)) as "lock!: ()""#,
        org_id.to_string(),
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(server_err)?;
    Ok(())
}

/// Replace an invoice's contribution, never add it to its previous value.
/// The caller holds the organization invoice lock before any row lock.
pub(super) async fn replace_contributions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    invoice_id: Uuid,
    discount_bps: i16,
) -> Result<(), ServerFnError> {
    let rows = sqlx::query!(
        r#"SELECT l.id, l.amount_cents, f.id AS "fee_id?",
          (f.amount_cents::numeric - COALESCE((
            SELECT sum(other.net_before_tax_cents::numeric)
            FROM invoice_line_items other JOIN invoices i ON i.id = other.invoice_id
            WHERE other.fee_occurrence_id = f.id AND i.org_id = $1
              AND i.status <> 'void' AND i.id <> $2
          ), 0))::bigint AS remaining
        FROM invoice_line_items l JOIN invoices own ON own.id = l.invoice_id
        LEFT JOIN project_fee_occurrences f ON f.id = l.fee_occurrence_id AND f.org_id = $1
        WHERE own.org_id = $1 AND l.invoice_id = $2
        ORDER BY l.time_entry_id NULLS LAST, f.project_id, f.period_key COLLATE "C", l.id"#,
        org_id,
        invoice_id,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(server_err)?;
    let discount = u16::try_from(discount_bps)
        .ok()
        .and_then(|bps| Percentage::try_from(bps).ok())
        .ok_or_else(|| err(BAD_REQUEST, "Discount must be between 0 and 100"))?;
    let gross: Vec<_> = rows.iter().map(|row| row.amount_cents).collect();
    let net = horae_core::invoice::allocate_invoice_discount(&gross, discount)
        .map_err(|error| conflict(error.to_string()))?;
    for (row, &contribution) in rows.iter().zip(&net) {
        if row.fee_id.is_some()
            && row
                .remaining
                .is_none_or(|remaining| contribution > remaining)
        {
            return Err(conflict(
                "This invoice exceeds the remaining fee balance. Review the other drafts before changing its discount.",
            ));
        }
    }
    let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
    let discounts: Vec<_> = gross
        .iter()
        .zip(net)
        .map(|(gross, net)| gross - net)
        .collect();
    sqlx::query!(
        "UPDATE invoice_line_items l SET allocated_discount_cents = billed.discount
         FROM unnest($1::uuid[], $2::bigint[]) AS billed(id, discount)
         WHERE l.id = billed.id AND l.invoice_id = $3",
        &ids,
        &discounts,
        invoice_id,
    )
    .execute(&mut **tx)
    .await
    .map_err(server_err)?;
    Ok(())
}
