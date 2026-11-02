//! Portfolio accounting: instruments, accounts, transactions and corporate actions, tax lots,
//! positions, cash in several currencies, and valuation with caller-supplied prices and FX rates.
//!
//! A [`Portfolio`] holds the static data and the transactions. [`Books::build`] applies the
//! transactions in order and keeps every lot, cash balance and realised gain; [`Books::value_on`]
//! and [`Books::value_series`] value the result with [`MarketData`].
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which
//! re-exports the public API of every layer.

mod books;
mod ids;
mod instrument;
mod lots;
mod market;
mod portfolio;
mod transaction;
mod valuation;

pub use books::{
    Allocation, Books, CashBalance, Effect, Event, Flow, FlowKind, HoldingKey, Holdings, LotSale,
    Position, RealisedGain,
};
pub use ids::{AccountId, InstrumentId, LotId, TransactionId};
pub use instrument::{Account, Instrument, InstrumentKind, LotMethod};
pub use lots::Lot;
pub use market::MarketData;
pub use portfolio::Portfolio;
pub use transaction::{LotPick, Transaction, TransactionKind};
pub use valuation::{
    AccountLine, AccountSeries, CashValue, HoldingLine, HoldingSeries, PositionValue, Valuation,
    ValueSeries,
};
