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
    Account, AccountId, Allocation, Books, CashValue, Effect, Event, Flow, FlowKind, HoldingKey,
    HoldingLine, HoldingSeries, Holdings, Instrument, InstrumentId, InstrumentKind, Lot, LotId,
    LotMethod, LotPick, MarketData, Portfolio, Position, PositionValue, RealisedGain, Transaction,
    TransactionId, TransactionKind, Valuation, ValueSeries,
};

pub use finalytics_formats::PortfolioDocument;
pub use finalytics_perf::{
    Contribution, ExcessReturn, FlowTiming, GroupContribution, Performance, Period, PeriodReturn,
    group_contributions,
};

/// Reading and writing portfolios: CSV import with validation reports, and the versioned
/// portfolio document in JSON.
pub mod formats {
    pub use finalytics_formats::{
        CsvOptions, FORMAT, FORMAT_VERSION, FxSeries, PortfolioDocument, fx_from_csv,
        instruments_from_csv, portfolio_from_csv, prices_from_csv, transactions_from_csv,
    };
}

/// Risk analytics on returns series: statistics, drawdowns, value at risk and expected shortfall,
/// and rolling versions of each.
///
/// ```
/// let returns = [0.010, -0.004, 0.007, -0.012, 0.003];
/// let stats = finalytics::risk::statistics(&returns, &Default::default())?;
/// assert!(stats.volatility > 0.0);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub mod risk {
    pub use finalytics_risk::{
        Drawdown, RelativeStatistics, Statistics, StatisticsOptions, TailMethod, TailRisk,
        drawdown_series, drawdowns, max_drawdown, normal_cdf, normal_pdf, normal_quantile,
        relative_statistics, rolling, rolling_relative_statistics, rolling_statistics,
        rolling_tail_risk, statistics, tail_risk, total_return,
    };
}

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
