//! Currencies.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Error, Result};

/// A currency, identified by its ISO 4217 code, with the number of minor units used when amounts
/// are rounded (2 for USD, 0 for JPY, 3 for KWD).
///
/// Codes that are not in the built-in table default to two minor units; use
/// [`Currency::with_minor_units`] to set another number. A currency is serialised as its code.
///
/// ```
/// use finalytics_core::Currency;
///
/// let yen: Currency = "JPY".parse()?;
/// assert_eq!(yen.minor_units(), 0);
/// assert_eq!(Currency::USD.code(), "USD");
/// # Ok::<(), finalytics_core::Error>(())
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Currency {
    code: [u8; 3],
    minor_units: u8,
}

// ISO 4217 currencies whose minor units differ from two.
const MINOR_UNITS: &[(&[u8; 3], u8)] = &[
    (b"BHD", 3),
    (b"BIF", 0),
    (b"CLP", 0),
    (b"DJF", 0),
    (b"GNF", 0),
    (b"IQD", 3),
    (b"ISK", 0),
    (b"JOD", 3),
    (b"JPY", 0),
    (b"KMF", 0),
    (b"KRW", 0),
    (b"KWD", 3),
    (b"LYD", 3),
    (b"OMR", 3),
    (b"PYG", 0),
    (b"RWF", 0),
    (b"TND", 3),
    (b"UGX", 0),
    (b"UYI", 0),
    (b"VND", 0),
    (b"VUV", 0),
    (b"XAF", 0),
    (b"XOF", 0),
    (b"XPF", 0),
];

impl Currency {
    /// United States dollar.
    pub const USD: Currency = Currency::known(*b"USD", 2);
    /// Euro.
    pub const EUR: Currency = Currency::known(*b"EUR", 2);
    /// Pound sterling.
    pub const GBP: Currency = Currency::known(*b"GBP", 2);
    /// Japanese yen.
    pub const JPY: Currency = Currency::known(*b"JPY", 0);
    /// Swiss franc.
    pub const CHF: Currency = Currency::known(*b"CHF", 2);
    /// Canadian dollar.
    pub const CAD: Currency = Currency::known(*b"CAD", 2);
    /// Australian dollar.
    pub const AUD: Currency = Currency::known(*b"AUD", 2);
    /// Hong Kong dollar.
    pub const HKD: Currency = Currency::known(*b"HKD", 2);
    /// Chinese yuan renminbi.
    pub const CNY: Currency = Currency::known(*b"CNY", 2);
    /// Singapore dollar.
    pub const SGD: Currency = Currency::known(*b"SGD", 2);

    const fn known(code: [u8; 3], minor_units: u8) -> Currency {
        Currency { code, minor_units }
    }

    /// The currency with ISO 4217 code `code`, which must be three upper-case ASCII letters.
    pub fn new(code: &str) -> Result<Currency> {
        let bytes = parse_code(code)?;
        let minor_units = MINOR_UNITS
            .iter()
            .find(|(c, _)| **c == bytes)
            .map_or(2, |&(_, units)| units);
        Ok(Currency {
            code: bytes,
            minor_units,
        })
    }

    /// The currency `code` with an explicit number of minor units (at most 8).
    pub fn with_minor_units(code: &str, minor_units: u8) -> Result<Currency> {
        let bytes = parse_code(code)?;
        if minor_units > 8 {
            return Err(Error::InvalidCurrency(format!(
                "{code} with {minor_units} minor units"
            )));
        }
        Ok(Currency {
            code: bytes,
            minor_units,
        })
    }

    /// The three-letter code, for example `"USD"`.
    pub fn code(&self) -> &str {
        // The code is validated as ASCII upper-case letters on construction.
        std::str::from_utf8(&self.code).unwrap_or("???")
    }

    /// The number of decimal places amounts in this currency are rounded to.
    pub fn minor_units(&self) -> u32 {
        u32::from(self.minor_units)
    }
}

fn parse_code(code: &str) -> Result<[u8; 3]> {
    match code.as_bytes() {
        &[a, b, c] if [a, b, c].iter().all(u8::is_ascii_uppercase) => Ok([a, b, c]),
        _ => Err(Error::InvalidCurrency(code.to_owned())),
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl fmt::Debug for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Currency({})", self.code())
    }
}

impl FromStr for Currency {
    type Err = Error;

    fn from_str(s: &str) -> Result<Currency> {
        Currency::new(s)
    }
}

impl Serialize for Currency {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        Currency::new(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Currency {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Currency".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[A-Z]{3}$",
            "description": "An ISO 4217 currency code, for example \"USD\"."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_default_minor_units() {
        assert_eq!(Currency::new("USD").unwrap(), Currency::USD);
        assert_eq!(Currency::new("JPY").unwrap().minor_units(), 0);
        assert_eq!(Currency::new("KWD").unwrap().minor_units(), 3);
        assert_eq!(Currency::new("NGN").unwrap().minor_units(), 2);
        assert_eq!(
            Currency::with_minor_units("XAU", 4).unwrap().minor_units(),
            4
        );
    }

    #[test]
    fn rejects_malformed_codes() {
        for bad in ["usd", "US", "USDT", "U$D", "", "ÜSD"] {
            assert!(Currency::new(bad).is_err(), "{bad} should be rejected");
        }
        assert!(Currency::with_minor_units("USD", 9).is_err());
    }

    #[test]
    fn minor_unit_table_is_sorted_and_unique() {
        assert!(MINOR_UNITS.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn serialises_as_its_code() {
        assert_eq!(serde_json::to_string(&Currency::EUR).unwrap(), "\"EUR\"");
        assert_eq!(
            serde_json::from_str::<Currency>("\"GBP\"").unwrap(),
            Currency::GBP
        );
        assert!(serde_json::from_str::<Currency>("\"gbp\"").is_err());
    }
}
