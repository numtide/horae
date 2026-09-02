use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    pub cents: i64,
    pub currency: [u8; 3], // ISO 4217 e.g. b"USD"
}

#[derive(Debug, Error)]
#[error("currency mismatch: cannot add {a} and {b}")]
pub struct CurrencyMismatch {
    pub a: String,
    pub b: String,
}

impl Money {
    pub fn new(cents: i64, currency: [u8; 3]) -> Self {
        Self { cents, currency }
    }

    pub fn currency_str(&self) -> &str {
        std::str::from_utf8(&self.currency).unwrap_or("???")
    }
}

pub fn add(a: Money, b: Money) -> Result<Money, CurrencyMismatch> {
    if a.currency != b.currency {
        return Err(CurrencyMismatch {
            a: a.currency_str().to_owned(),
            b: b.currency_str().to_owned(),
        });
    }
    Ok(Money {
        cents: a.cents + b.cents,
        currency: a.currency,
    })
}

#[derive(Debug, Error)]
#[error("invalid amount: {0}")]
pub struct AmountError(pub String);

/// Parse a typed amount into minor units — `"1200"` and `"1,200.00"` both give
/// `120_000`. Digits are read as text rather than through a float so a value
/// like `12000.10` cannot land a cent off, and at most two decimal places are
/// accepted so a silently truncated third digit can't change the amount.
pub fn parse_cents(s: &str) -> Result<i64, AmountError> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    let cleaned = cleaned.trim_start_matches('+');
    let err = || AmountError(s.to_owned());
    if cleaned.is_empty() {
        return Err(err());
    }
    let (neg, digits) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned),
    };
    let (whole, frac) = match digits.split_once('.') {
        Some((w, f)) => (w, f),
        None => (digits, ""),
    };
    if frac.len() > 2
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return Err(err());
    }
    // An empty whole part ("​.50") is fine; an empty amount overall is not.
    if whole.is_empty() && frac.is_empty() {
        return Err(err());
    }
    let units: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| err())?
    };
    let cents: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().map_err(|_| err())? * 10,
        _ => frac.parse().map_err(|_| err())?,
    };
    let total = units
        .checked_mul(100)
        .and_then(|u| u.checked_add(cents))
        .ok_or_else(err)?;
    Ok(if neg { -total } else { total })
}

/// Format minor units for display: currency code + thousands-grouped decimal,
/// e.g. `format_cents(1_000_000, "USD")` → `"USD 10,000.00"`. A negative amount
/// keeps its sign after the code (`"USD -500.00"`).
pub fn format_cents(cents: i64, currency: &str) -> String {
    let neg = cents < 0;
    let abs = cents.unsigned_abs();
    let whole = (abs / 100).to_string();
    let len = whole.len();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!(
        "{} {}{}.{:02}",
        currency.trim(),
        if neg { "-" } else { "" },
        grouped,
        abs % 100
    )
}

/// Display a project budget in its own unit: money for an amount budget, hours
/// for an hours budget, empty for no budget.
pub fn format_budget(
    kind: crate::types::BudgetKind,
    amount_cents: Option<i64>,
    minutes: Option<i64>,
    currency: &str,
) -> String {
    use crate::types::BudgetKind;
    match kind {
        BudgetKind::Amount => amount_cents
            .map(|c| format_cents(c, currency))
            .unwrap_or_default(),
        BudgetKind::Hours => minutes
            .map(|m| format!("{}h", crate::duration::format_decimal(m.max(0) as u32)))
            .unwrap_or_default(),
        BudgetKind::None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cents_groups_thousands_and_keeps_sign() {
        assert_eq!(format_cents(1_000_000, "USD"), "USD 10,000.00");
        assert_eq!(format_cents(150_000, "USD"), "USD 1,500.00");
        assert_eq!(format_cents(99, "USD"), "USD 0.99");
        assert_eq!(format_cents(0, "EUR"), "EUR 0.00");
        assert_eq!(format_cents(-50_000, "USD"), "USD -500.00");
    }

    #[test]
    fn parse_cents_reads_whole_and_fractional_amounts() {
        assert_eq!(parse_cents("1200").unwrap(), 120_000);
        assert_eq!(parse_cents("1200.5").unwrap(), 120_050);
        assert_eq!(parse_cents("1200.05").unwrap(), 120_005);
        assert_eq!(parse_cents("0.99").unwrap(), 99);
        assert_eq!(parse_cents(".5").unwrap(), 50);
        assert_eq!(parse_cents("0").unwrap(), 0);
    }

    #[test]
    fn parse_cents_tolerates_grouping_spaces_and_signs() {
        assert_eq!(parse_cents(" 1,200.00 ").unwrap(), 120_000);
        assert_eq!(parse_cents("12 000").unwrap(), 1_200_000);
        assert_eq!(parse_cents("+50").unwrap(), 5_000);
        assert_eq!(parse_cents("-500").unwrap(), -50_000);
    }

    #[test]
    fn parse_cents_is_exact_where_a_float_would_drift() {
        // 12000.10 is not representable in binary floating point; going through
        // f64 here rounds to 1_200_009 cents.
        assert_eq!(parse_cents("12000.10").unwrap(), 1_200_010);
        assert_eq!(parse_cents("1.15").unwrap(), 115);
        assert_eq!(parse_cents("8.20").unwrap(), 820);
    }

    #[test]
    fn parse_cents_rejects_what_it_cannot_represent() {
        // A third decimal would have to be dropped, changing the amount.
        assert!(parse_cents("1.005").is_err());
        assert!(parse_cents("").is_err());
        assert!(parse_cents("   ").is_err());
        assert!(parse_cents("abc").is_err());
        assert!(parse_cents("1.2.3").is_err());
        assert!(parse_cents("1e3").is_err());
        assert!(parse_cents("-").is_err());
    }
}
