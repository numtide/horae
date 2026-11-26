use crate::models::invoice::InvoiceDefaults;
use horae_core::invoice::Rounding;
use horae_core::project::Percentage;
use horae_core::types::RoundingMethod;

use crate::ops::OpError;

/// Checks the values alone; with no subtotal, rounding makes no difference.
pub fn validate(defaults: &InvoiceDefaults) -> Result<(), OpError> {
    amounts(defaults, 0, Rounding::new("", RoundingMethod::default())).map(drop)
}

/// The rounding method a new invoice is computed with: the organization's
/// current setting. An existing invoice keeps the method it stores.
pub(super) async fn org_rounding(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
) -> Result<RoundingMethod, OpError> {
    Ok(sqlx::query_scalar!(
        r#"SELECT invoice_rounding AS "rounding: RoundingMethod" FROM organizations WHERE id = $1"#,
        org_id
    )
    .fetch_one(&mut **tx)
    .await?)
}

pub fn amounts(
    defaults: &InvoiceDefaults,
    subtotal: i64,
    rounding: Rounding,
) -> Result<horae_core::invoice::InvoiceAmounts, OpError> {
    if !(0..=365).contains(&defaults.terms_days) {
        return Err(OpError::bad_input(
            "Payment terms must be between 0 and 365 days",
        ));
    }
    if defaults.po_number.chars().count() > 200 || defaults.po_number.contains('\0') {
        return Err(OpError::bad_input(
            "Purchase order must contain at most 200 characters",
        ));
    }
    if defaults.tax2_name.is_some() != defaults.tax2_bps.is_some()
        || defaults.tax2_name.as_ref().is_some_and(|name| {
            name.trim().is_empty() || name.chars().count() > 100 || name.contains('\0')
        })
    {
        return Err(OpError::bad_input(
            "A second tax requires a name of 1–100 characters and a percentage",
        ));
    }
    let percentage = |bps: i16| -> Result<Percentage, OpError> {
        u16::try_from(bps)
            .ok()
            .and_then(|bps| Percentage::try_from(bps).ok())
            .ok_or_else(|| OpError::bad_input("Percentages must be between 0 and 100"))
    };
    horae_core::invoice::invoice_amounts(
        subtotal,
        percentage(defaults.discount_bps)?,
        percentage(defaults.tax1_bps)?,
        percentage(defaults.tax2_bps.unwrap_or(0))?,
        rounding,
    )
    .map_err(|error| OpError::conflict(error.to_string()))
}

pub(super) async fn resolve(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    project_ids: &[uuid::Uuid],
    overrides: Option<&InvoiceDefaults>,
) -> Result<InvoiceDefaults, OpError> {
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
    .await?;
    let first = rows.first().cloned().unwrap_or_default();
    let different = rows.iter().any(|row| row != &first)
        || (rows.len() < project_ids.len() && first != InvoiceDefaults::default());
    let defaults = match overrides {
        Some(overrides) => overrides.clone(),
        None if different => {
            return Err(OpError::conflict(
                "Selected projects have different invoice defaults. Choose explicit invoice values before generating.",
            ));
        }
        None => first,
    };
    validate(&defaults)?;
    Ok(defaults)
}
