//! Instruments and accounts.

use std::collections::BTreeMap;

use finalytics_core::{Currency, Decimal};
use serde::{Deserialize, Serialize};

use crate::{AccountId, InstrumentId};

/// What kind of instrument something is.
///
/// Every kind is held and valued the same way, as a quantity times a price times a multiplier.
/// Derivatives and bonds are supplied already priced; Finalytics does not price them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InstrumentKind {
    /// A share in a company.
    Equity,
    /// A mutual fund, exchange-traded fund or similar pooled vehicle.
    Fund,
    /// A bond, priced per unit (supply clean or dirty prices consistently).
    Bond,
    /// A futures contract, priced per unit of the underlying with a contract multiplier.
    Future,
    /// An option, priced per unit of the underlying with a contract multiplier.
    Option,
    /// An FX forward, priced as its value per unit of notional.
    FxForward,
    /// Anything else.
    Other,
}

/// Something a portfolio can hold, other than cash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Instrument {
    /// The caller's identifier.
    pub id: InstrumentId,
    /// A display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The kind of instrument.
    pub kind: InstrumentKind,
    /// The currency it is priced and traded in.
    pub currency: Currency,
    /// The value of one unit at a price of one: 1 for shares, the contract size for futures
    /// and options.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "schema", schemars(extend("default" = "1")))]
    pub multiplier: Decimal,
    /// Classifications by scheme, for example `asset_class: Equity`, `region: Europe`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub classifications: BTreeMap<String, String>,
    /// Other identifiers by scheme, for example `isin` or `ticker`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub identifiers: BTreeMap<String, String>,
}

impl Instrument {
    /// An instrument with a multiplier of one and no classifications.
    pub fn new(id: InstrumentId, kind: InstrumentKind, currency: Currency) -> Instrument {
        Instrument {
            id,
            name: None,
            kind,
            currency,
            multiplier: Decimal::ONE,
            classifications: BTreeMap::new(),
            identifiers: BTreeMap::new(),
        }
    }

    /// This instrument with a classification added.
    pub fn classified(mut self, scheme: &str, value: &str) -> Instrument {
        self.classifications
            .insert(scheme.to_owned(), value.to_owned());
        self
    }
}

fn one() -> Decimal {
    Decimal::ONE
}

fn is_one(value: &Decimal) -> bool {
    *value == Decimal::ONE
}

/// How sales choose which tax lots to close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LotMethod {
    /// First in, first out: the oldest lots first.
    #[default]
    Fifo,
    /// Last in, first out: the newest lots first.
    Lifo,
    /// The lots with the highest cost per unit first.
    HighestCost,
    /// The lots with the lowest cost per unit first.
    LowestCost,
    /// Every lot reduced in proportion, so each sale carries the average cost.
    AverageCost,
    /// The lots named by the sale.
    SpecificId,
}

/// An account in a portfolio, such as a brokerage or custody account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Account {
    /// The caller's identifier.
    pub id: AccountId,
    /// A display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// How sales in this account choose lots, unless a sale says otherwise.
    #[serde(default)]
    pub lot_method: LotMethod,
    /// Classifications by scheme, for grouping accounts: for example `owner: Alice` or
    /// `wrapper: ISA`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub classifications: BTreeMap<String, String>,
}

impl Account {
    /// An account using first-in, first-out lot relief, with no classifications.
    pub fn new(id: AccountId) -> Account {
        Account {
            id,
            name: None,
            lot_method: LotMethod::Fifo,
            classifications: BTreeMap::new(),
        }
    }

    /// This account with a classification added.
    pub fn classified(mut self, scheme: &str, value: &str) -> Account {
        self.classifications
            .insert(scheme.to_owned(), value.to_owned());
        self
    }
}
