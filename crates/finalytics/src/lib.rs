//! Finalytics is a portfolio analytics engine. It keeps the books of a multi-currency portfolio
//! from its transactions, measures performance and risk, and optimises portfolios under real-world
//! constraints.
//!
//! The engine performs no input or output: callers supply instruments, transactions, prices and FX
//! rates, and receive typed results. The same API is available from Python and, through
//! WebAssembly, from JavaScript and TypeScript.

pub use finalytics_core::{
    BusinessDayConvention, Calendar, Currency, Date, Decimal, Error, Fill, Issue, Money,
    Observation, Result, Rounding, Severity, TimeSeries, ValidationReport, Weekday,
};

pub use finalytics_ledger::{
    Account, AccountId, Books, CashValue, Effect, Event, Flow, FlowKind, Holdings, Instrument,
    InstrumentId, InstrumentKind, Lot, LotId, LotMethod, LotPick, MarketData, Portfolio, Position,
    PositionValue, RealisedGain, Transaction, TransactionId, TransactionKind, Valuation,
    ValueSeries,
};

/// Numerical helpers: compensated summation and tolerance checks.
pub mod numeric {
    pub use finalytics_core::numeric::{CompensatedSum, approx_eq, compensated_sum};
}

/// The version of this release of Finalytics.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the version of this release of Finalytics, for example `"1.0.0"`.
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_the_package() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        assert!(!version().is_empty());
    }
}
