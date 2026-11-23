use thiserror::Error;

/// Some amount in the set is not visible to the viewer.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("an amount in the total is withheld")]
pub struct Withheld;

/// Exact per-currency totals, ordered by currency code. Amounts in different
/// currencies are never combined, and codes are compared exactly as given.
/// Any withheld (`None`) amount makes the whole result `Err(Withheld)`,
/// whatever else the set contains, so no partial total can leak. Within a
/// currency, `None` marks a final sum that does not fit in `i64`; summing in
/// `i128` makes that independent of input order.
pub fn sum_by_currency<'a>(
    amounts: impl IntoIterator<Item = (&'a str, Option<i64>)>,
) -> Result<Vec<(&'a str, Option<i64>)>, Withheld> {
    let mut sums = std::collections::BTreeMap::<&str, i128>::new();
    for (currency, cents) in amounts {
        *sums.entry(currency).or_default() += i128::from(cents.ok_or(Withheld)?);
    }
    Ok(sums
        .into_iter()
        .map(|(currency, sum)| (currency, i64::try_from(sum).ok()))
        .collect())
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
    let units: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().map_err(|_| err())?
    };
    let cents: i128 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i128>().map_err(|_| err())? * 10,
        _ => frac.parse().map_err(|_| err())?,
    };
    // Signed in i128 so `i64::MIN`, whose magnitude exceeds `i64::MAX`, reads back.
    let total = units
        .checked_mul(100)
        .and_then(|u| u.checked_add(cents))
        .ok_or_else(err)?;
    i64::try_from(if neg { -total } else { total }).map_err(|_| err())
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

/// Two-decimal amount with no currency code and no thousands grouping,
/// e.g. `-50` → `"-0.50"`. Integer math, so the sign survives amounts under
/// one currency unit (where `cents / 100` would truncate to `0`).
pub fn format_cents_plain(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    format!(
        "{}{}.{:02}",
        if cents < 0 { "-" } else { "" },
        abs / 100,
        abs % 100
    )
}

/// An amount in major units with the currency's own number of decimals (ISO
/// 4217), for wire formats such as the Harvest API: `JPY 1000`, `EUR 10.00`,
/// `BHD 10.000`. Horae stores every currency in hundredths, so a zero-decimal
/// currency keeps its two decimals when the stored amount has hundredths:
/// exactness comes first.
pub fn format_major_units(cents: i64, currency: &str) -> String {
    let plain = format_cents_plain(cents);
    match currency_decimals(currency) {
        0 if cents % 100 == 0 => plain[..plain.len() - 3].to_owned(),
        3 => plain + "0",
        _ => plain,
    }
}

/// Pads a plain decimal with zeros to the places Horae shows amounts in for
/// `currency` (its minor unit, at most two), so Harvest's `1.0` or `100`
/// reads `1.00` and `100.00` beside the amounts. It never removes a digit:
/// `95.2381` stays as written.
pub fn pad_to_currency(decimal: &str, currency: &str) -> String {
    let places = usize::from(currency_decimals(currency).min(2));
    let written = decimal
        .split_once('.')
        .map_or(0, |(_, fraction)| fraction.len());
    if written >= places {
        return decimal.to_owned();
    }
    let point = if written == 0 { "." } else { "" };
    format!("{decimal}{point}{}", "0".repeat(places - written))
}

/// ISO 4217 codes, sorted: those in use, and those withdrawn since 2014 that
/// an old invoice can still carry (HRK, LTL, MRO, …). The codes for testing
/// (XTS) and for no currency (XXX) are left out.
const ISO_4217: &[&str] = &[
    "AED", "AFN", "ALL", "AMD", "ANG", "AOA", "ARS", "AUD", "AWG", "AZN", "BAM", "BBD", "BDT",
    "BGN", "BHD", "BIF", "BMD", "BND", "BOB", "BOV", "BRL", "BSD", "BTN", "BWP", "BYN", "BYR",
    "BZD", "CAD", "CDF", "CHE", "CHF", "CHW", "CLF", "CLP", "CNY", "COP", "COU", "CRC", "CUC",
    "CUP", "CVE", "CZK", "DJF", "DKK", "DOP", "DZD", "EGP", "ERN", "ETB", "EUR", "FJD", "FKP",
    "GBP", "GEL", "GHS", "GIP", "GMD", "GNF", "GTQ", "GYD", "HKD", "HNL", "HRK", "HTG", "HUF",
    "IDR", "ILS", "INR", "IQD", "IRR", "ISK", "JMD", "JOD", "JPY", "KES", "KGS", "KHR", "KMF",
    "KPW", "KRW", "KWD", "KYD", "KZT", "LAK", "LBP", "LKR", "LRD", "LSL", "LTL", "LVL", "LYD",
    "MAD", "MDL", "MGA", "MKD", "MMK", "MNT", "MOP", "MRO", "MRU", "MUR", "MVR", "MWK", "MXN",
    "MXV", "MYR", "MZN", "NAD", "NGN", "NIO", "NOK", "NPR", "NZD", "OMR", "PAB", "PEN", "PGK",
    "PHP", "PKR", "PLN", "PYG", "QAR", "RON", "RSD", "RUB", "RWF", "SAR", "SBD", "SCR", "SDG",
    "SEK", "SGD", "SHP", "SLE", "SLL", "SOS", "SRD", "SSP", "STD", "STN", "SVC", "SYP", "SZL",
    "THB", "TJS", "TMT", "TND", "TOP", "TRY", "TTD", "TWD", "TZS", "UAH", "UGX", "USD", "USN",
    "UYI", "UYU", "UYW", "UZS", "VED", "VEF", "VES", "VND", "VUV", "WST", "XAF", "XAG", "XAU",
    "XBA", "XBB", "XBC", "XBD", "XCD", "XCG", "XDR", "XOF", "XPD", "XPF", "XPT", "XSU", "XUA",
    "YER", "ZAR", "ZMW", "ZWG", "ZWL",
];

/// Whether `currency` is an ISO 4217 code (see [`ISO_4217`]).
pub fn is_iso_currency(currency: &str) -> bool {
    ISO_4217.binary_search(&currency).is_ok()
}

/// ISO 4217 minor-unit digits; every currency not listed has two.
pub(crate) fn currency_decimals(currency: &str) -> u8 {
    match currency.trim() {
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "BIF" | "BYR" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF"
        | "UGX" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        _ => 2,
    }
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
            .map(|m| format!("{}h", crate::duration::format_decimal(m)))
            .unwrap_or_default(),
        BudgetKind::None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_currencies_are_listed_in_order() {
        assert!(ISO_4217.windows(2).all(|pair| pair[0] < pair[1]));
        for known in ["EUR", "USD", "JPY", "HRK", "XCG"] {
            assert!(is_iso_currency(known), "{known}");
        }
        for unknown in ["XYZ", "XTS", "XXX", "eur", "EURO", ""] {
            assert!(!is_iso_currency(unknown), "{unknown}");
        }
    }

    #[test]
    fn unit_prices_pad_to_the_currency_without_losing_digits() {
        assert_eq!(pad_to_currency("1.0", "USD"), "1.00");
        assert_eq!(pad_to_currency("0", "USD"), "0.00");
        assert_eq!(pad_to_currency("100", "EUR"), "100.00");
        assert_eq!(pad_to_currency("95.2381", "USD"), "95.2381");
        assert_eq!(pad_to_currency("80.50", "JPY"), "80.50");
        assert_eq!(pad_to_currency("80", "JPY"), "80");
        assert_eq!(pad_to_currency("1.5", "KWD"), "1.50");
    }

    #[test]
    fn major_units_use_the_currencys_own_decimals() {
        assert_eq!(format_major_units(123_456, "EUR"), "1234.56");
        assert_eq!(format_major_units(100_000, "JPY"), "1000");
        assert_eq!(format_major_units(-100, "KRW"), "-1");
        assert_eq!(format_major_units(1_230, "BHD"), "12.300");
        assert_eq!(format_major_units(-5, "KWD"), "-0.050");
    }

    #[test]
    fn a_zero_decimal_currency_keeps_stored_hundredths() {
        assert_eq!(format_major_units(100_050, "JPY"), "1000.50");
    }

    #[test]
    fn major_units_parse_back_to_the_same_hundredths() {
        for (cents, currency) in [(i64::MAX, "BHD"), (i64::MIN, "JPY"), (7, "USD")] {
            let text = format_major_units(cents, currency);
            let two_places = match text.split_once('.') {
                Some((whole, frac)) => format!("{whole}.{}", &frac[..2]),
                None => text.clone(),
            };
            assert_eq!(parse_cents(&two_places).unwrap(), cents, "{text}");
        }
    }

    #[test]
    fn currency_totals_are_exact_per_currency_and_ordered_by_code() {
        let amounts = [
            ("USD", Some(i64::MAX)),
            ("EUR", Some(100)),
            ("eur", Some(1)),
            ("EUR", Some(i64::MAX - 100)),
        ];
        let expected = vec![
            ("EUR", Some(i64::MAX)),
            ("USD", Some(i64::MAX)),
            ("eur", Some(1)),
        ];
        assert_eq!(sum_by_currency(amounts), Ok(expected.clone()));
        assert_eq!(sum_by_currency(amounts.into_iter().rev()), Ok(expected));
        assert_eq!(sum_by_currency([]), Ok(vec![]));
    }

    #[test]
    fn currency_totals_report_only_the_final_sum_out_of_range() {
        for edge in [i64::MAX, i64::MIN] {
            let amounts = [("EUR", Some(edge)), ("EUR", Some(edge)), ("USD", Some(5))];
            assert_eq!(
                sum_by_currency(amounts),
                Ok(vec![("EUR", None), ("USD", Some(5))])
            );
        }
        // Leaves the range part-way through but ends inside it.
        let amounts = [("EUR", Some(i64::MAX)), ("EUR", Some(1)), ("EUR", Some(-2))];
        assert_eq!(
            sum_by_currency(amounts),
            Ok(vec![("EUR", Some(i64::MAX - 1))])
        );
    }

    #[test]
    fn currency_totals_keep_zero_and_signed_sums() {
        let amounts = [
            ("EUR", Some(500)),
            ("USD", Some(0)),
            ("EUR", Some(-500)),
            ("CHF", Some(-1)),
        ];
        assert_eq!(
            sum_by_currency(amounts),
            Ok(vec![("CHF", Some(-1)), ("EUR", Some(0)), ("USD", Some(0))])
        );
    }

    #[test]
    fn a_withheld_amount_hides_every_currency_total() {
        let late = [
            ("EUR", Some(i64::MAX)),
            ("EUR", Some(i64::MAX)),
            ("USD", Some(5)),
            ("EUR", None),
        ];
        assert_eq!(sum_by_currency(late), Err(Withheld));
        let early = [("USD", None), ("EUR", Some(1))];
        assert_eq!(sum_by_currency(early), Err(Withheld));
    }

    #[test]
    fn format_cents_groups_thousands_and_keeps_sign() {
        assert_eq!(format_cents(1_000_000, "USD"), "USD 10,000.00");
        assert_eq!(format_cents(150_000, "USD"), "USD 1,500.00");
        assert_eq!(format_cents(99, "USD"), "USD 0.99");
        assert_eq!(format_cents(0, "EUR"), "EUR 0.00");
        assert_eq!(format_cents(-50_000, "USD"), "USD -500.00");
    }

    #[test]
    fn format_cents_plain_keeps_sign_below_one_unit() {
        assert_eq!(format_cents_plain(0), "0.00");
        assert_eq!(format_cents_plain(99), "0.99");
        assert_eq!(format_cents_plain(123_456), "1234.56");
        assert_eq!(format_cents_plain(-50), "-0.50");
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
