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

/// A rate template, such as a task's default hourly rate: a positive number of
/// hundredths with no currency. It is not money until [`RateTemplate::apply`]
/// gives it a currency (Constitution I, "Rate templates"); never sum, compare
/// or invoice it as money.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateTemplate(i64);

impl RateTemplate {
    /// A typed template with at most two decimals (`"80.5"`, `"1,200.50"`).
    /// Zero and negative values are refused: zero is no rate.
    pub fn parse(s: &str) -> Result<Self, AmountError> {
        Self::from_hundredths(parse_cents(s)?).ok_or_else(|| AmountError(s.to_owned()))
    }

    /// A stored template; `None` unless it is positive.
    pub fn from_hundredths(hundredths: i64) -> Option<Self> {
        (hundredths > 0).then_some(Self(hundredths))
    }

    pub fn hundredths(self) -> i64 {
        self.0
    }

    /// Two decimals, no currency: `"80.50"`.
    pub fn format(self) -> String {
        format_cents_plain(self.0)
    }

    /// The rate this template becomes in `currency`, in that currency's
    /// hundredths. Every currency is stored in hundredths and a rate keeps two
    /// decimals in all of them, so the number is the same.
    pub fn apply(self, _currency: &str) -> i64 {
        self.0
    }
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
    "USS", "UYI", "UYU", "UYW", "UZS", "VED", "VEF", "VES", "VND", "VUV", "WST", "XAD", "XAF",
    "XAG", "XAU", "XBA", "XBB", "XBC", "XBD", "XCD", "XCG", "XDR", "XOF", "XPD", "XPF", "XPT",
    "XSU", "XUA", "YER", "ZAR", "ZMW", "ZWG", "ZWL",
];

/// Whether `currency` is an ISO 4217 code (see [`ISO_4217`]).
pub fn is_iso_currency(currency: &str) -> bool {
    ISO_4217.binary_search(&currency).is_ok()
}

/// ISO 4217 currencies with no minor unit.
pub const ZERO_DECIMAL_CURRENCIES: &[&str] = &[
    "BIF", "BYR", "CLP", "DJF", "GNF", "ISK", "JPY", "KMF", "KRW", "PYG", "RWF", "UGX", "UYI",
    "VND", "VUV", "XAF", "XOF", "XPF",
];

/// ISO 4217 currencies with three decimals.
pub const THREE_DECIMAL_CURRENCIES: &[&str] = &["BHD", "IQD", "JOD", "KWD", "LYD", "OMR", "TND"];

/// ISO 4217 minor-unit digits; every currency not listed has two. CLF and UYW
/// have four, which makes no difference: Horae keeps hundredths at most. The
/// code is trimmed and matched case-sensitively.
///
/// This and the two lists above are the only table of currency decimals in
/// Rust, and a test checks that every listed code is in [`ISO_4217`]. Its SQL
/// twin, `currency_amount_step(text)`, lists the same zero-decimal codes; a
/// parity test checks that each list holds exactly the other's zero-decimal
/// codes.
pub fn currency_decimals(currency: &str) -> u8 {
    let code = currency.trim();
    if ZERO_DECIMAL_CURRENCIES.contains(&code) {
        0
    } else if THREE_DECIMAL_CURRENCIES.contains(&code) {
        3
    } else {
        2
    }
}

// ISO 4217 minor units, the unit of `/api/v1`'s `Money`. Storage keeps
// hundredths for every currency; these two are the only conversions.

/// A stored or received amount that the other unit cannot hold exactly.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MinorUnitsError {
    /// Not a whole number of the target unit: JPY 10.50 has no yen value,
    /// and BHD 10.505 has no hundredths value.
    #[error("the amount is not a whole number of the currency's unit")]
    Inexact,
    #[error("the amount is out of range")]
    Overflow,
}

/// Stored hundredths as ISO 4217 minor units: EUR 1210 → 1210, JPY 800000 →
/// 8000, BHD 1050 → 10500. Errors rather than rounds when a zero-decimal
/// amount carries hundredths.
pub fn to_iso_minor(hundredths: i64, currency: &str) -> Result<i64, MinorUnitsError> {
    match currency_decimals(currency) {
        0 if hundredths % 100 != 0 => Err(MinorUnitsError::Inexact),
        0 => Ok(hundredths / 100),
        3 => hundredths.checked_mul(10).ok_or(MinorUnitsError::Overflow),
        _ => Ok(hundredths),
    }
}

/// ISO 4217 minor units as stored hundredths, the inverse of
/// [`to_iso_minor`]. Errors when the amount cannot be stored: a
/// three-decimal amount that is not a multiple of 10, or one too large.
pub fn from_iso_minor(minor: i64, currency: &str) -> Result<i64, MinorUnitsError> {
    match currency_decimals(currency) {
        0 => minor.checked_mul(100).ok_or(MinorUnitsError::Overflow),
        3 if minor % 10 != 0 => Err(MinorUnitsError::Inexact),
        3 => Ok(minor / 10),
        _ => Ok(minor),
    }
}

/// A per-hour price (a rate) as `/api/v1` serves it: a decimal in major units
/// with exactly the two decimals Horae stores for every currency, trailing
/// zeros included. JPY 8050 hundredths is `"80.50"`, JPY 800000 is
/// `"8000.00"`, USD 12000 is `"120.00"` and BHD 1050 is `"10.50"`. A rate
/// multiplies time before rounding, so it keeps its stored precision even
/// where the currency's minor unit is coarser, and never fails.
pub fn rate_decimal(hundredths: i64) -> String {
    format_cents_plain(hundredths)
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
        for known in ["EUR", "USD", "JPY", "HRK", "XCG", "XAD", "USS"] {
            assert!(is_iso_currency(known), "{known}");
        }
        for unknown in ["XYZ", "XTS", "XXX", "eur", "EURO", ""] {
            assert!(!is_iso_currency(unknown), "{unknown}");
        }
        assert_eq!(currency_decimals("UYI"), 0);
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
    fn rate_templates_parse_two_decimals_and_refuse_zero() {
        for (typed, hundredths) in [
            ("80", 8_000),
            ("80.5", 8_050),
            ("0.01", 1),
            ("1,200.50", 120_050),
        ] {
            assert_eq!(
                RateTemplate::parse(typed)
                    .map(RateTemplate::hundredths)
                    .ok(),
                Some(hundredths),
                "{typed}"
            );
        }
        assert_eq!(
            RateTemplate::parse("92233720368547758.07")
                .map(RateTemplate::hundredths)
                .ok(),
            Some(i64::MAX)
        );
        for refused in ["80.505", "-1", "", "0", "0.00", "92233720368547758.08"] {
            assert!(RateTemplate::parse(refused).is_err(), "{refused}");
        }
        assert_eq!(RateTemplate::from_hundredths(0), None);
    }

    #[test]
    fn rate_templates_format_and_apply_the_same_number() {
        let template = RateTemplate::parse("80.5").unwrap();
        assert_eq!(template.format(), "80.50");
        for currency in ["EUR", "JPY", "BHD"] {
            assert_eq!(template.apply(currency), 8_050, "{currency}");
        }
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
    fn iso_minor_units_follow_the_currencys_decimals() {
        assert_eq!(to_iso_minor(1_210, "EUR"), Ok(1_210));
        assert_eq!(to_iso_minor(800_000, "JPY"), Ok(8_000));
        assert_eq!(to_iso_minor(1_050, "BHD"), Ok(10_500));
        assert_eq!(to_iso_minor(-100, "KRW"), Ok(-1));
        assert_eq!(from_iso_minor(1_210, "EUR"), Ok(1_210));
        assert_eq!(from_iso_minor(8_000, "JPY"), Ok(800_000));
        assert_eq!(from_iso_minor(10_500, "BHD"), Ok(1_050));
    }

    #[test]
    fn rates_keep_their_two_stored_decimals_in_every_currency() {
        assert_eq!(rate_decimal(8_050), "80.50");
        assert_eq!(rate_decimal(800_000), "8000.00");
        assert_eq!(rate_decimal(12_000), "120.00");
        assert_eq!(rate_decimal(1_050), "10.50");
        assert_eq!(rate_decimal(5), "0.05");
    }

    #[test]
    fn iso_minor_units_refuse_what_they_cannot_hold() {
        assert_eq!(to_iso_minor(800_050, "JPY"), Err(MinorUnitsError::Inexact));
        assert_eq!(
            to_iso_minor(i64::MAX, "BHD"),
            Err(MinorUnitsError::Overflow)
        );
        assert_eq!(from_iso_minor(10_505, "BHD"), Err(MinorUnitsError::Inexact));
        assert_eq!(
            from_iso_minor(i64::MIN, "JPY"),
            Err(MinorUnitsError::Overflow)
        );
    }

    #[test]
    fn currency_decimals_follow_iso_4217() {
        assert_eq!(currency_decimals("JPY"), 0);
        assert_eq!(currency_decimals(" JPY"), 0);
        assert_eq!(currency_decimals("EUR"), 2);
        assert_eq!(currency_decimals("KWD"), 3);
    }

    #[test]
    fn every_currency_with_other_decimals_is_an_iso_code() {
        for code in ZERO_DECIMAL_CURRENCIES
            .iter()
            .chain(THREE_DECIMAL_CURRENCIES)
        {
            assert!(is_iso_currency(code), "{code}");
        }
    }

    #[test]
    fn currency_decimals_are_two_for_unknown_and_lowercase_codes() {
        assert_eq!(currency_decimals("XYZ"), 2);
        assert_eq!(currency_decimals("jpy"), 2);
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
