//! Shared validation for client identity and billing defaults.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ClientValidationError {
    #[error("Client name must contain 1–200 characters and no null characters")]
    Name,
    #[error("Choose a supported client currency")]
    Currency,
    #[error("Billing address must not contain null characters")]
    Address,
    #[error("Tax ID must not contain null characters")]
    TaxId,
    #[error(
        "Default rate must be a nonnegative amount with at most two decimal places within the supported range"
    )]
    DefaultRate,
}

pub fn validate_profile(
    name: &str,
    currency: &str,
    address: Option<&str>,
    tax_id: Option<&str>,
) -> Result<(), ClientValidationError> {
    if !(1..=200).contains(&name.trim().chars().count()) || name.contains('\0') {
        return Err(ClientValidationError::Name);
    }
    if !crate::project::PROJECT_CURRENCIES.contains(&currency) {
        return Err(ClientValidationError::Currency);
    }
    if address.is_some_and(|value| value.contains('\0')) {
        return Err(ClientValidationError::Address);
    }
    if tax_id.is_some_and(|value| value.contains('\0')) {
        return Err(ClientValidationError::TaxId);
    }
    Ok(())
}

pub fn parse_default_rate(value: &str) -> Result<Option<i64>, ClientValidationError> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    let cents = crate::money::parse_cents(value).map_err(|_| ClientValidationError::DefaultRate)?;
    if cents < 0 {
        return Err(ClientValidationError::DefaultRate);
    }
    Ok(Some(cents))
}

/// Empty optional fields are absent; nonempty imported/user text is retained.
pub fn optional_text(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_accepts_supported_currencies_and_unicode_character_limit() {
        for currency in crate::project::PROJECT_CURRENCIES {
            assert_eq!(
                validate_profile(
                    &format!("  {}  ", "界".repeat(200)),
                    currency,
                    Some("Street\nFloor 2"),
                    Some("VAT-123")
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn profile_rejects_blank_overlong_and_null_names() {
        for name in [
            String::new(),
            " \n\t ".into(),
            "界".repeat(201),
            "Company\0Name".into(),
        ] {
            assert_eq!(
                validate_profile(&name, "EUR", None, None),
                Err(ClientValidationError::Name)
            );
        }
    }

    #[test]
    fn profile_rejects_unsupported_currency_and_null_billing_fields() {
        for currency in ["", "eur", " EUR ", "JPY", "EUR\0"] {
            assert_eq!(
                validate_profile("Client", currency, None, None),
                Err(ClientValidationError::Currency)
            );
        }
        assert_eq!(
            validate_profile("Client", "EUR", Some("a\0b"), None),
            Err(ClientValidationError::Address)
        );
        assert_eq!(
            validate_profile("Client", "EUR", None, Some("a\0b")),
            Err(ClientValidationError::TaxId)
        );
    }

    #[test]
    fn default_rate_preserves_absence_zero_and_exact_large_amounts() {
        for (input, expected) in [
            ("", None),
            (" \t ", None),
            ("0", Some(0)),
            ("0.00", Some(0)),
            ("1,234.56", Some(123456)),
            ("92233720368547758.07", Some(i64::MAX)),
        ] {
            assert_eq!(parse_default_rate(input), Ok(expected), "{input}");
        }
    }

    #[test]
    fn default_rate_rejects_negative_fractional_cent_overflow_and_invalid_text() {
        for input in [
            "-0.01",
            "1.001",
            "92233720368547758.08",
            "NaN",
            "1e2",
            "€10",
            "10\0",
        ] {
            assert_eq!(
                parse_default_rate(input),
                Err(ClientValidationError::DefaultRate),
                "{input}"
            );
        }
    }

    #[test]
    fn optional_text_normalizes_only_blank_fields() {
        assert_eq!(optional_text(" \n\t"), None);
        assert_eq!(
            optional_text("  Street\nFloor 2  "),
            Some("  Street\nFloor 2  ")
        );
        assert_eq!(
            optional_text("<Client & Company>"),
            Some("<Client & Company>")
        );
    }
}
