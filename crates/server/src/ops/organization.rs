//! Organization settings: the invoice branding (issuer, payment details and
//! invoice defaults) and the workspace time zone.
//!
//! Branding needs `InvoiceWriteAll`, to read as well as to save: it is the
//! invoice settings page, which has no read-only form. The time zone, and
//! reading the workspace settings pages (Plugins), need `CompanyWrite`.
//!
//! [`super::begin`] holds the organization row `FOR SHARE` for the whole
//! transaction, so the row read here cannot change before the write. Two saves
//! racing for that row can deadlock on the upgrade; PostgreSQL aborts one,
//! which reaches the caller as a conflict.

use horae_core::permissions::catalog::Permission;

use super::{Actor, Done, OpError, Tx};
use crate::models::OrgBranding;
use crate::plugin::AppEvent;
use crate::plugin::event::OrgBrandingPayload;

const SETTINGS_REQUIRED: &str = "Invoice settings need permission to manage all invoices";
const COMPANY_REQUIRED: &str = "Workspace settings need permission to manage the company";
const NOT_FOUND: &str = "Organization not found";

fn authorize(actor: &Actor, permission: Permission, message: &str) -> Result<(), OpError> {
    if actor.has(permission) {
        Ok(())
    } else {
        Err(OpError::forbidden(message))
    }
}

/// The organization's invoice branding, as the settings page edits it.
pub async fn branding(tx: &mut Tx, actor: &Actor) -> Result<OrgBranding, OpError> {
    authorize(actor, Permission::InvoiceWriteAll, SETTINGS_REQUIRED)?;
    current_branding(tx, actor).await
}

async fn current_branding(tx: &mut Tx, actor: &Actor) -> Result<OrgBranding, OpError> {
    match fetch_branding_from(&mut **tx, actor.org_id()).await {
        Ok(branding) => Ok(branding),
        Err(sqlx::Error::RowNotFound) => Err(OpError::not_found(NOT_FOUND)),
        Err(error) => Err(error.into()),
    }
}

/// The org's invoice branding block, shared with the invoice PDF
/// (`http::reports::limits::pdf`) so the PDF carries the identity [`branding`] edits.
pub async fn fetch_branding_from(
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

/// Saves the branding edited from `expected`. The request carries every
/// column, so one edited from an older snapshot would silently restore values
/// someone else saved since: that is a conflict. An unchanged save writes
/// nothing and announces nothing.
pub async fn update_branding(
    tx: &mut Tx,
    actor: &Actor,
    expected: &OrgBranding,
    branding: &OrgBranding,
) -> Result<Done<OrgBranding>, OpError> {
    authorize(actor, Permission::InvoiceWriteAll, SETTINGS_REQUIRED)?;
    // Only edited values need checking: the rest must equal the stored row
    // (enforced below), even one stored before values were checked.
    for (value, previous) in branding_values(branding)
        .into_iter()
        .zip(branding_values(expected))
    {
        if let Some(message) = value
            .as_deref()
            .filter(|_| value != previous)
            .and_then(crate::models::branding_value_error)
        {
            return Err(OpError::bad_input(message));
        }
    }
    let org_id = actor.org_id();
    let before = current_branding(tx, actor).await?;
    if before != *expected {
        return Err(OpError::conflict(
            "Someone changed these settings after you opened this page. Reload the page to see the current values; your unsaved edits will be lost.",
        ));
    }
    let updated = sqlx::query_as!(
        OrgBranding,
        r#"UPDATE organizations SET
             provider_name = $2,
             provider_address = $3,
             provider_tax_id = $4,
             provider_email = $5,
             provider_phone = $6,
             bank_name = $7,
             bank_iban = $8,
             bank_bic = $9,
             bank_routing = $10,
             bank_account = $11,
             invoice_notes = $12,
             invoice_payment_terms = $13
           WHERE id = $1
             AND (provider_name, provider_address, provider_tax_id,
                  provider_email, provider_phone,
                  bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                  invoice_notes, invoice_payment_terms)
                 IS DISTINCT FROM ($2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
           RETURNING provider_name, provider_address, provider_tax_id,
                     provider_email, provider_phone,
                     bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                     invoice_notes, invoice_payment_terms"#,
        org_id,
        branding.provider_name,
        branding.provider_address,
        branding.provider_tax_id,
        branding.provider_email,
        branding.provider_phone,
        branding.bank_name,
        branding.bank_iban,
        branding.bank_bic,
        branding.bank_routing,
        branding.bank_account,
        branding.invoice_notes,
        branding.invoice_payment_terms,
    )
    .fetch_optional(&mut **tx)
    .await?;
    Ok(match updated {
        Some(updated) => {
            let event = AppEvent::OrgBrandingUpdated {
                occurred_at: chrono::Utc::now(),
                org_id,
                org: OrgBrandingPayload {
                    org_id,
                    provider_name: updated.provider_name.clone(),
                },
            };
            Done::new(updated).with(event)
        }
        None => Done::new(before),
    })
}

fn branding_values(branding: &OrgBranding) -> [&Option<String>; 12] {
    [
        &branding.provider_name,
        &branding.provider_address,
        &branding.provider_tax_id,
        &branding.provider_email,
        &branding.provider_phone,
        &branding.bank_name,
        &branding.bank_iban,
        &branding.bank_bic,
        &branding.bank_routing,
        &branding.bank_account,
        &branding.invoice_notes,
        &branding.invoice_payment_terms,
    ]
}

/// Allows the workspace settings pages, Plugins included, to someone with
/// `CompanyWrite`; the plugin list is read through this check.
pub fn require_settings(actor: &Actor) -> Result<(), OpError> {
    authorize(actor, Permission::CompanyWrite, COMPANY_REQUIRED)
}

/// Changes the time zone every person without their own follows.
pub async fn set_timezone(tx: &mut Tx, actor: &Actor, timezone: &str) -> Result<Done<()>, OpError> {
    authorize(actor, Permission::CompanyWrite, COMPANY_REQUIRED)?;
    let timezone = crate::timezone::parse(timezone)
        .ok_or_else(|| OpError::invalid("timezone", "Choose a time zone from the list"))?
        .name();
    crate::timezone::replan_for_org_zone(tx, actor.org_id(), timezone).await?;
    sqlx::query!(
        "UPDATE organizations SET timezone = $2 WHERE id = $1",
        actor.org_id(),
        timezone,
    )
    .execute(&mut **tx)
    .await?;
    Ok(Done::new(()))
}

#[cfg(test)]
mod tests;
