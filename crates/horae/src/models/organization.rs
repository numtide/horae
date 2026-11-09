use serde::{Deserialize, Serialize};

/// Branding-only DTO for invoice rendering and settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub struct OrgBranding {
    pub provider_name: Option<String>,
    pub provider_address: Option<String>,
    pub provider_tax_id: Option<String>,
    pub provider_email: Option<String>,
    pub provider_phone: Option<String>,
    pub bank_name: Option<String>,
    pub bank_iban: Option<String>,
    pub bank_bic: Option<String>,
    pub bank_routing: Option<String>,
    pub bank_account: Option<String>,
    pub invoice_notes: Option<String>,
    pub invoice_payment_terms: Option<String>,
}

/// The longest branding value the invoice PDF export accepts (see `reports::limits`).
pub const MAX_BRANDING_VALUE_BYTES: usize = 32_767;

/// Why a branding value cannot be saved, if it cannot: the PDF export refuses
/// longer values, and PostgreSQL text cannot hold a NUL character.
pub fn branding_value_error(value: &str) -> Option<&'static str> {
    if value.len() > MAX_BRANDING_VALUE_BYTES {
        Some("Too long to print on an invoice: use at most 32,767 bytes.")
    } else if value.contains('\0') {
        Some("Remove the NUL character.")
    } else {
        None
    }
}
