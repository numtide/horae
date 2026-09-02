//! Organization-branding server functions.

use super::*;

/// The organization's display name — for the admin shell's workspace header.
/// Any signed-in user may read it (it is not sensitive).
#[server]
pub async fn get_org_name() -> Result<String, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let name = sqlx::query_scalar!(
        r#"SELECT o.name
             FROM organizations o
             JOIN users u ON u.org_id = o.id
            WHERE u.id = $1"#,
        user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(name)
}

// ── Organization branding ─────────────────────────────────────────────────────

#[server]
pub async fn get_org_branding() -> Result<OrgBranding, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let branding = sqlx::query_as!(
        OrgBranding,
        r#"SELECT provider_name, provider_address, provider_tax_id,
                  provider_email, provider_phone,
                  bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                  invoice_notes, invoice_payment_terms
           FROM organizations WHERE id = $1"#,
        manager.org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(branding)
}

#[server]
pub async fn update_org_branding(branding: OrgBranding) -> Result<OrgBranding, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let branding = sqlx::query_as!(
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
           RETURNING provider_name, provider_address, provider_tax_id,
                     provider_email, provider_phone,
                     bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                     invoice_notes, invoice_payment_terms"#,
        manager.org_id,
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
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::OrgBrandingUpdated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            org: crate::plugin::event::OrgBrandingPayload {
                org_id: manager.org_id,
                provider_name: branding.provider_name.clone(),
            },
        });
    Ok(branding)
}
