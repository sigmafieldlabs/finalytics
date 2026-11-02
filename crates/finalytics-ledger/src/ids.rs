//! Identifiers chosen by the caller.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde::{Deserialize, Deserializer, Serialize};

macro_rules! identifier {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// Identifiers are non-empty, have no leading or trailing whitespace, and are compared
        /// exactly (case matters).
        // The text is shared between copies: the books clone identifiers into every lot, event
        // and lookup key, so a clone is a reference count rather than an allocation.
        #[derive(Clone, Eq, PartialOrd, Ord, Serialize)]
        #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
        #[serde(transparent)]
        pub struct $name(Arc<str>);

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
            }
        }

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        impl $name {
            /// Creates an identifier, or returns `None` if `id` is empty or has surrounding
            /// whitespace.
            pub fn new(id: impl Into<String>) -> Option<Self> {
                let id: String = id.into();
                (!id.is_empty() && id.trim() == id).then(|| Self(Arc::from(id)))
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

        impl std::str::FromStr for $name {
            type Err = finalytics_core::Error;

            /// Parses an identifier: non-empty, without leading or trailing spaces.
            fn from_str(text: &str) -> finalytics_core::Result<Self> {
                $name::new(text).ok_or_else(|| {
                    let mut report = finalytics_core::ValidationReport::new();
                    report.error(
                        "id",
                        format!(
                            "{text:?} is not a valid identifier: it must be non-empty, without \
                             leading or trailing spaces"
                        ),
                    );
                    finalytics_core::Error::Validation(report)
                })
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                $name::new(text.as_str()).ok_or_else(|| {
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
        LotId(Arc::from(tx.as_str()))
    }

    /// The lot derived from this one by the corporate action `action`.
    pub(crate) fn derived(&self, action: &TransactionId) -> LotId {
        LotId(Arc::from(format!("{}/{}", self.0, action)))
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
