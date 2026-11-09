//! Hours and money on the Harvest wire: JSON numbers written from integer
//! minutes and minor units as exact decimal text, never through `f64`
//! (Constitution I, decimal wire formats). The `server` feature enables
//! serde_json's `arbitrary_precision`, so a `Number` parsed from text is
//! written back verbatim.

use horae_core::duration::format_hours2;
use horae_core::money::format_cents_plain;
use serde::{Serialize, Serializer};

/// Integer minutes, written as hours rounded half up to two decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hours(pub i64);

/// Integer minor units, written in major units. Horae stores every currency
/// in hundredths, so every amount has two decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(pub i64);

fn number<S: Serializer>(text: String, serializer: S) -> Result<S::Ok, S::Error> {
    text.parse::<serde_json::Number>()
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

impl Serialize for Hours {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        number(format_hours2(self.0), serializer)
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        number(format_cents_plain(self.0), serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_and_amounts_are_exact_json_numbers() {
        let json = serde_json::to_string(&(Hours(1), Hours(90), Amount(1234), Amount(-5))).unwrap();
        assert_eq!(json, "[0.02,1.50,12.34,-0.05]");
    }

    #[test]
    fn large_amounts_keep_every_digit() {
        let json = serde_json::to_string(&Amount(i64::MAX)).unwrap();
        assert_eq!(json, "92233720368547758.07");
    }
}
