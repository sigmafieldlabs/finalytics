//! Identifiers chosen by the caller.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

macro_rules! identifier {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// Identifiers are non-empty, have no leading or trailing whitespace, and are compared
        /// exactly (case matters).
        #[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates an identifier, or returns `None` if `id` is empty or has surrounding
            /// whitespace.
            pub fn new(id: impl Into<String>) -> Option<Self> {
                let id = id.into();
                (!id.is_empty() && id.trim() == id).then_some(Self(id))
            }

            /// The identifier as text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({:?})", stringify!($name), self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                $name::new(text.clone()).ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "{:?} is not a valid identifier: it must be non-empty, without leading or trailing spaces",
                        text
                    ))
                })
            }
        }
    };
}

identifier!(
    /// Identifies an instrument, for example `"AAPL"` or `"US0378331005"`.
    InstrumentId
);
identifier!(
    /// Identifies an account within a portfolio.
    AccountId
);
identifier!(
    /// Identifies a transaction. Lots opened by a transaction take their identity from it.
    TransactionId
);
identifier!(
    /// Identifies a tax lot: the transaction that opened it, followed by `/` and the corporate
    /// action that derived it, if any.
    LotId
);

impl LotId {
    /// The lot opened by transaction `tx`.
    pub(crate) fn opened_by(tx: &TransactionId) -> LotId {
        LotId(tx.as_str().to_owned())
    }

    /// The lot derived from this one by the corporate action `action`.
    pub(crate) fn derived(&self, action: &TransactionId) -> LotId {
        LotId(format!("{}/{}", self.0, action))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_and_padded_identifiers() {
        assert!(InstrumentId::new("AAPL").is_some());
        assert!(InstrumentId::new("").is_none());
        assert!(InstrumentId::new(" AAPL").is_none());
        assert!(AccountId::new("main\n").is_none());
        assert!(serde_json::from_str::<InstrumentId>("\"\"").is_err());
        assert_eq!(
            serde_json::from_str::<InstrumentId>("\"MSFT\"")
                .unwrap()
                .as_str(),
            "MSFT"
        );
    }
}
