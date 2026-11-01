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
    Account, AccountId, Allocation, Books, CashBalance, CashValue, Effect, Event, Flow, FlowKind,
    HoldingKey, HoldingLine, HoldingSeries, Holdings, Instrument, InstrumentId, InstrumentKind,
    Lot, LotId, LotMethod, LotPick, LotSale, MarketData, Portfolio, Position, PositionValue,
    RealisedGain, Transaction, TransactionId, TransactionKind, Valuation, ValueSeries,
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
        CsvOptions, FORMAT, FORMAT_VERSION, FxSeries, MarketDocument, PortfolioDocument,
        fx_from_csv, instruments_from_csv, portfolio_from_csv, prices_from_csv,
        transactions_from_csv,
    };
}

/// Risk analytics: statistics, drawdowns, value at risk and expected shortfall, covariance
/// estimation, and the decomposition of a portfolio's risk by holding and by group.
///
/// ```
/// let returns = [0.010, -0.004, 0.007, -0.012, 0.003];
/// let stats = finalytics::risk::statistics(&returns, &Default::default())?;
/// assert!(stats.volatility > 0.0);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub mod risk {
    pub use finalytics_risk::{
        CovarianceEstimate, CovarianceMethod, Drawdown, GroupRisk, HoldingReturns, Matrix,
        RelativeStatistics, ReturnsTable, RiskContribution, RiskDecomposition, Statistics,
        StatisticsOptions, TailMethod, TailRisk, covariance, drawdown_series, drawdowns,
        group_risk, holding_returns, holding_weights, max_drawdown, normal_cdf, normal_pdf,
        normal_quantile, relative_statistics, risk_decomposition, rolling,
        rolling_relative_statistics, rolling_statistics, rolling_tail_risk, statistics, tail_risk,
        total_return, tracking_error_decomposition,
    };
}

/// Portfolio optimisation: minimum variance, mean–variance, target return and volatility,
/// maximum Sharpe ratio and diversification, minimum CVaR, risk budgeting and hierarchical risk
/// parity, under budget, bound, group, leverage, turnover and tracking-error constraints; the
/// efficient frontier; and the trades, with lot selection, that move an account to target
/// weights.
///
/// ```
/// use finalytics::opt::{Objective, Problem};
/// use finalytics::risk::Matrix;
///
/// let covariance = Matrix::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![0.0400, 0.0020, 0.0020, 0.0049],
/// )?;
/// let problem = Problem::new(covariance).with_expected_returns(vec![0.07, 0.03]);
/// let solution = problem.solve(&Objective::MaximumSharpe { risk_free_rate: 0.02 })?;
/// assert!(solution.status.has_solution());
/// # Ok::<(), finalytics::Error>(())
/// ```
pub mod opt {
    pub use finalytics_opt::{
        AssetBound, Bound, Constraints, FrontierPoint, GroupLimit, Linkage, Method, Objective,
        Problem, ProposedConversion, ProposedTrade, Rebalance, RebalanceOptions, Side, Solution,
        SolveStatus, TrackingErrorLimit, Turnover, rebalance,
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
