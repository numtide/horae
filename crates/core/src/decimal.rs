//! Exact non-negative decimal conversion shared by time input and imports.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecimalError {
    #[error("not a valid decimal number: {0:?}")]
    NotANumber(String),
    #[error("value must not be negative: {0:?}")]
    Negative(String),
    #[error("decimal value or precision out of range: {0:?}")]
    OutOfRange(String),
}

/// Parse `s` as a non-negative decimal and return `round(value * scale)` (ties
/// away from zero, which for a non-negative value is half-up) using exact integer
/// arithmetic — no `f64` rounds through the middle.
pub(crate) fn scale_decimal(s: &str, scale: i128) -> Result<i64, DecimalError> {
    let (numerator, denominator) = parse_decimal(s)?;
    let out_of_range = || DecimalError::OutOfRange(s.to_owned());
    let whole = (numerator / denominator)
        .checked_mul(scale)
        .ok_or_else(out_of_range)?;
    let fraction = numerator % denominator;
    let mut scaled_fraction = 0;
    let mut remainder = 0;
    // At most 100 additions (the cents scale). Reducing each addition avoids
    // overflowing fraction * scale even for a 38-digit decimal fraction.
    for _ in 0..scale {
        if remainder >= denominator - fraction {
            remainder -= denominator - fraction;
            scaled_fraction += 1;
        } else {
            remainder += fraction;
        }
    }
    let rounded_fraction = scaled_fraction + i128::from(remainder >= denominator - remainder);
    let rounded = whole
        .checked_add(rounded_fraction)
        .ok_or_else(out_of_range)?;
    i64::try_from(rounded).map_err(|_| out_of_range())
}

/// Parse a non-negative decimal string into `(numerator, 10^fractional_digits)`
/// so `numerator / denominator` is its exact value. Accepts an optional leading
/// `+`, digits, an optional single `.`, and digits; rejects everything else.
/// Supports up to 38 fractional digits and an unscaled numerator up to
/// `i128::MAX`; exceeding either bound returns `OutOfRange`.
fn parse_decimal(s: &str) -> Result<(i128, i128), DecimalError> {
    let trimmed = s.trim();
    let body = trimmed.strip_prefix('+').unwrap_or(trimmed);
    if body.is_empty() {
        return Err(DecimalError::NotANumber(s.to_owned()));
    }
    if body.starts_with('-') {
        return Err(DecimalError::Negative(s.to_owned()));
    }

    let (int_part, frac_part) = match body.split_once('.') {
        Some((i, f)) => (i, f),
        None => (body, ""),
    };
    // An empty integer part is fine (".5"); an empty fractional part is fine
    // ("5."). But at least one digit must be present overall.
    if int_part.is_empty() && frac_part.is_empty() {
        return Err(DecimalError::NotANumber(s.to_owned()));
    }
    if !int_part.bytes().all(|b| b.is_ascii_digit())
        || !frac_part.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(DecimalError::NotANumber(s.to_owned()));
    }

    let mut numerator: i128 = 0;
    for b in int_part.bytes().chain(frac_part.bytes()) {
        numerator = numerator
            .checked_mul(10)
            .and_then(|n| n.checked_add((b - b'0') as i128))
            .ok_or_else(|| DecimalError::OutOfRange(s.to_owned()))?;
    }
    let denominator = 10i128
        .checked_pow(
            u32::try_from(frac_part.len()).map_err(|_| DecimalError::OutOfRange(s.to_owned()))?,
        )
        .ok_or_else(|| DecimalError::OutOfRange(s.to_owned()))?;
    Ok((numerator, denominator))
}
