//! Hours and money on the Harvest wire: JSON numbers written from integer
//! minutes and minor units as exact decimal text, never through `f64`
//! (Constitution I, decimal wire formats). The `server` feature enables
//! serde_json's `arbitrary_precision`, so a `Number` parsed from text is
//! written back verbatim.

use horae_core::duration::format_hours2;
use horae_core::money::format_major_units;
use serde::{Serialize, Serializer};

/// Integer minutes, written as hours rounded half up to two decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hours(pub i64);

/// Integer minor units, written in major units with the currency's own
/// decimals (`money::format_major_units`). An unknown currency gets two.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    pub cents: i64,
    pub currency: String,
}

impl Amount {
    pub fn new(cents: i64, currency: &str) -> Self {
        Self {
            cents,
            currency: currency.to_owned(),
        }
    }
}

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
        number(format_major_units(self.cents, &self.currency), serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hours_and_amounts_are_exact_json_numbers() {
        let json = serde_json::to_string(&(
            Hours(1),
            Hours(90),
            Amount::new(1234, "EUR"),
            Amount::new(-5, "USD"),
            Amount::new(100_000, "JPY"),
            Amount::new(1234, "BHD"),
        ))
        .unwrap();
        assert_eq!(json, "[0.02,1.50,12.34,-0.05,1000,12.340]");
    }

    #[test]
    fn large_amounts_keep_every_digit() {
        let json = serde_json::to_string(&Amount::new(i64::MAX, "EUR")).unwrap();
        assert_eq!(json, "92233720368547758.07");
    }
}
