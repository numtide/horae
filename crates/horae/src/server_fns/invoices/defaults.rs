use super::*;
use crate::models::invoice::InvoiceDefaults;
use horae_core::project::Percentage;

pub(super) fn amounts(
    defaults: &InvoiceDefaults,
    subtotal: i64,
) -> Result<horae_core::invoice::InvoiceAmounts, ServerFnError> {
    if !(0..=365).contains(&defaults.terms_days) {
        return Err(err(
            BAD_REQUEST,
            "Payment terms must be between 0 and 365 days",
        ));
    }
    if defaults.po_number.chars().count() > 200 || defaults.po_number.contains('\0') {
        return Err(err(
            BAD_REQUEST,
            "Purchase order must contain at most 200 characters",
        ));
    }
    if defaults.tax2_name.is_some() != defaults.tax2_bps.is_some()
        || defaults.tax2_name.as_ref().is_some_and(|name| {
            name.trim().is_empty() || name.chars().count() > 100 || name.contains('\0')
        })
    {
        return Err(err(
            BAD_REQUEST,
            "A second tax requires a name of 1–100 characters and a percentage",
        ));
    }
    let percentage = |bps: i16| -> Result<Percentage, ServerFnError> {
        u16::try_from(bps)
            .ok()
            .and_then(|bps| Percentage::try_from(bps).ok())
            .ok_or_else(|| err(BAD_REQUEST, "Percentages must be between 0 and 100"))
    };
    horae_core::invoice::invoice_amounts(
        subtotal,
        percentage(defaults.discount_bps)?,
        percentage(defaults.tax1_bps)?,
        percentage(defaults.tax2_bps.unwrap_or(0))?,
    )
    .map_err(|error| conflict(error.to_string()))
}

pub(super) async fn resolve(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    project_ids: &[uuid::Uuid],
    overrides: Option<&InvoiceDefaults>,
) -> Result<InvoiceDefaults, ServerFnError> {
    // Projects without settings participate with the original Net 30 defaults.
    // Lock existing settings so a concurrent edit cannot change the snapshot.
    let rows = sqlx::query_as!(
        InvoiceDefaults,
        "SELECT terms_days,po_number,discount_bps,tax1_bps,tax2_name,tax2_bps
         FROM project_settings WHERE org_id = $1 AND project_id = ANY($2)
         ORDER BY project_id FOR SHARE",
        org_id,
        project_ids,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(server_err)?;
    let first = rows.first().cloned().unwrap_or_default();
    let different = rows.iter().any(|row| row != &first)
        || (rows.len() < project_ids.len() && first != InvoiceDefaults::default());
    let defaults = match overrides {
        Some(overrides) => overrides.clone(),
        None if different => {
            return Err(conflict(
                "Selected projects have different invoice defaults. Choose explicit invoice values before generating.",
            ));
        }
        None => first,
    };
    amounts(&defaults, 0)?;
    Ok(defaults)
}
