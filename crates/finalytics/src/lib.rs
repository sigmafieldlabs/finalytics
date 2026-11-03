//! Finalytics is a portfolio analytics engine. It compares portfolio optimisation methods on
//! returns they have not seen, and keeps the books of a real portfolio from its transactions to
//! report its performance, risk and attribution.
//!
//! The engine itself performs no input or output: callers supply prices and transactions, and
//! receive typed results. The default `io` feature adds reading files by path and showing
//! reports in a browser. The same API is available from Python and, through WebAssembly, from
//! JavaScript and TypeScript.
//!
//! # Where to start
//!
//! Every front door is a builder that ends in `build()`, and [`prelude`] imports them:
//!
//! - [`Assets`], the prices or returns of things one might hold: their statistics and
//!   covariance; [`Assets::optimise`] for weights by a [`Method`] ([`Optimisation`]);
//!   [`Assets::compare`] to backtest several methods side by side ([`Comparison`]); and
//!   [`Assets::backtest`] for one method or one set of weights, in sample, on a holdout or
//!   walk-forward ([`Backtest`]).
//! - [`Portfolio`], a portfolio's transactions with the market data that values them: its
//!   valuation, performance and risk; [`Portfolio::report`] for every report; and
//!   [`Portfolio::rebalance`] for the trades that reach target weights.
//!
//! ```
//! use finalytics::prelude::*;
//!
//! let assets = Assets::builder()
//!     .prices(datasets::asset_class_prices())
//!     .risk_free_asset("T-Bills")
//!     .build()?;
//! let result = assets.optimise(Method::MaximumSharpe).max_weight(0.4).build()?;
//! assert!(result.results.sharpe_ratio.is_some());
//!
//! let report = datasets::portfolio()
//!     .report()
//!     .benchmark("BENCH")
//!     .group_by(["asset_class", "region"])
//!     .build()?;
//! assert!(report.risk.provenance.source.transactions > 0);
//! # Ok::<(), finalytics::Error>(())
//! ```
//!
//! The modules beneath them, such as [`risk`], [`opt`], [`performance`] and [`backtest`], give
//! each method on its own, for analysis the front doors do not cover. [`datasets`] has data to
//! practise on: S&P 500 stocks, a household portfolio and asset-class indices.
//!
//! Every result of [`Assets`] has the same shape: the report in `report`, the raw results in
//! `results`, the report as text when printed, and the report as a page with
//! [`Show::show`]:
//!
//! ```no_run
//! use finalytics::prelude::*;
//!
//! let assets = Assets::builder()
//!     .prices(datasets::asset_class_prices())
//!     .risk_free_asset("T-Bills")
//!     .build()?;
//! let result = assets
//!     .compare([Method::EqualWeight, Method::MinimumVariance])
//!     .max_weight(0.40)
//!     .walk_forward(Years(5))
//!     .build()?; // likewise optimise(...) and backtest(...)
//!
//! let report = &result.report; // the report
//! let results = &result.results; // the raw results
//! println!("{result}"); // the report as text
//! result.show()?; // the report as a page
//! # Ok::<(), finalytics::Error>(())
//! ```
//!
//! # Options and results
//!
//! Option types, such as [`opt::Constraints`] and [`risk::StatisticsOptions`], and the results
//! the engine returns are non-exhaustive, so that later 1.x releases can add fields to them
//! without breaking code that uses them. Start options from their defaults and set the fields
//! you need:
//!
//! ```
//! use finalytics::opt::{Bound, Constraints};
//!
//! let mut constraints = Constraints::default();
//! constraints.bounds = Bound { min: 0.0, max: 0.4 };
//! ```
//!
//! Records that are complete as they stand, such as an [`opt::Bound`] or an [`opt::GroupLimit`],
//! are built directly. Enumerations that may gain variants, such as [`opt::Objective`], are
//! non-exhaustive too, so a `match` on one needs a wildcard arm.

/// The raw results of an optimisation, a comparison, a backtest or a rebalancing, given as JSON,
/// as text. The Python and JavaScript packages use it; from Rust, display the results.
#[doc(hidden)]
pub fn results_text(results: &str) -> Result<String> {
    let invalid = |message: String| {
        let mut report = ValidationReport::new();
        report.error("results", message);
        Error::Validation(report)
    };
    let value: serde_json::Value =
        serde_json::from_str(results).map_err(|e| invalid(format!("is not JSON: {e}")))?;
    if value.get("solution").is_some() {
        let result: OptimisationResult =
            serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
        Ok(result.to_string())
    } else if value.get("cash").is_some() {
        let result: BacktestResults =
            serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
        Ok(result.to_string())
    } else if value.get("weights_before").is_some() {
        let result: opt::RebalanceResults =
            serde_json::from_value(value).map_err(|e| invalid(e.to_string()))?;
        Ok(result.to_string())
    } else {
        Err(invalid(
            "are not the results of an optimisation, a backtest or a rebalancing".to_owned(),
        ))
    }
}

// The README's examples run as documentation tests.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct ReadmeExamples;

pub use finalytics_core::{
    BusinessDayConvention, Calendar, Currency, Date, Decimal, Error, Fill, Issue, Money,
    Observation, Result, Rounding, Severity, TimeSeries, ValidationReport, Weekday,
};

pub use finalytics_ledger::{
    Account, AccountId, AccountLine, AccountSeries, Allocation, Books, CashBalance, CashValue,
    Effect, Event, Flow, FlowKind, HoldingKey, HoldingLine, HoldingSeries, Holdings, Instrument,
    InstrumentId, InstrumentKind, Lot, LotId, LotMethod, LotPick, LotSale, MarketData,
    PortfolioData, Position, PositionValue, RealisedGain, Transaction, TransactionId,
    TransactionKind, Valuation, ValueSeries,
};

pub use finalytics_formats::PortfolioDocument;

mod assets;
pub mod backtest;
pub mod datasets;
mod portfolio;
#[cfg(feature = "io")]
mod show;

/// What most programs need, in one import: `use finalytics::prelude::*;`.
pub mod prelude {
    pub use crate::backtest::{Months, Periods, Rebalance, Years};
    #[cfg(feature = "io")]
    pub use crate::show::Show;
    pub use crate::{Assets, Error, Method, Portfolio, PortfolioCsv, Result, datasets};
}

pub use assets::{
    AssetGroup, AssetStatistics, Assets, AssetsBuilder, Method, Optimisation, OptimisationResult,
    Optimise, OptimiseOptions,
};
pub use backtest::{Backtest, BacktestResults, Comparison};
pub use finalytics_reports::{
    BacktestReport, ComparisonReport, OptimisationReport, PortfolioReport, RebalanceReport,
};
pub use finalytics_risk::PriceTable;
#[cfg(feature = "io")]
pub use portfolio::PortfolioBuilder;
#[doc(hidden)]
pub use portfolio::targets_from_json;
pub use portfolio::{
    Portfolio, PortfolioCsv, RebalanceBuilder, Rebalancing, ReportBuilder, Targets,
};
#[cfg(feature = "io")]
pub use show::Show;

/// Performance measurement: time- and money-weighted returns over reporting periods,
/// contribution by holding and by group, households of accounts, and attribution against a
/// benchmark (Brinson–Fachler, linked over periods, and currency attribution by Karnosky and
/// Singer).
pub mod performance {
    pub use finalytics_perf::{
        AccountsPerformance, Attribution, AttributionPeriod, Contribution, CurrencyAttribution,
        CurrencyAttributionPeriod, CurrencyEffect, CurrencyPeriodAttribution, CurrencyReturn,
        CurrencySegment, ExcessReturn, FlowTiming, GroupContribution, Household, HouseholdOptions,
        Linking, Performance, Period, PeriodAttribution, PeriodReturn, SchemePerformance, Segment,
        SegmentEffects, brinson_fachler, group_contributions, household, karnosky_singer,
    };
}

/// Reading and writing portfolios: CSV import with validation reports, the versioned portfolio
/// document in JSON, and a JSON reader for any of the engine's types.
pub mod formats {
    pub use finalytics_formats::{
        CsvOptions, FORMAT, FORMAT_VERSION, FxSeries, MarketDocument, PortfolioDocument,
        accounts_from_csv, from_json, fx_from_csv, instruments_from_csv, market_from_csv,
        portfolio_from_csv, price_columns_from_csv, prices_from_csv, transactions_from_csv,
    };
}

/// Risk analytics: statistics, drawdowns, value at risk and expected shortfall, covariance
/// estimation, factor models, the decomposition of a portfolio's risk by holding, by group and
/// by factor, and scenarios: shocks and historical replays, with their effect on value and risk.
///
/// ```
/// let returns = [0.010, -0.004, 0.007, -0.012, 0.003];
/// let stats = finalytics::risk::statistics(&returns, &Default::default())?;
/// assert!(stats.volatility > 0.0);
/// # Ok::<(), finalytics::Error>(())
/// ```
pub mod risk {
    pub use finalytics_risk::{
        AssetFit, CorrelationStress, CovarianceEstimate, CovarianceMethod, Drawdown,
        FactorContribution, FactorMethod, FactorModel, FactorModelEstimate, FactorRisk,
        FactorShocks, GroupImpact, GroupRisk, GroupShock, HistoricalWindow, HoldingImpact,
        HoldingReturns, Matrix, PriceReturns, PriceTable, RelativeStatistics, ReturnsTable,
        RiskContribution, RiskDecomposition, ScenarioOptions, ScenarioPath, ScenarioResult,
        ScenarioRisk, ScenarioRiskModel, SchemeImpact, SchemeRisk, ShockSource, Shocks, Statistics,
        StatisticsOptions, TailMethod, TailRisk, covariance, drawdown_series, drawdowns,
        factor_risk, factor_tracking_error, fundamental_model, group_risk, historical_scenario,
        holding_returns, holding_weights, max_drawdown, normal_cdf, normal_pdf, normal_quantile,
        periods_per_year, regression_model, relative_statistics, risk_decomposition, rolling,
        rolling_relative_statistics, rolling_statistics, rolling_tail_risk, shock_scenario,
        statistical_model, statistics, stressed_covariance, tail_risk, total_return,
        tracking_error_decomposition,
    };
}

/// Report data for each workstation screen: holdings, overview, performance, risk, attribution
/// and scenarios, with chart-ready series, provenance and tables for export.
pub mod reports {
    pub use finalytics_reports::*;
}

/// Portfolio optimisation: minimum variance, mean–variance and robust mean–variance, target
/// return and volatility, maximum Sharpe ratio and diversification, minimum CVaR, risk budgeting
/// and hierarchical risk parity, with a covariance matrix or a factor model, under budget, bound,
/// group, leverage, turnover, tracking-error and factor exposure constraints; Black–Litterman
/// expected returns; resampled optimisation; the efficient frontier; and the trades, with lot
/// selection, that move an account to target weights.
///
/// ```
/// use finalytics::opt::{Objective, Problem};
/// use finalytics::risk::Matrix;
///
/// let covariance = Matrix::new(
///     vec!["equity".into(), "bonds".into()],
///     vec![0.0400, 0.0020, 0.0020, 0.0049],
/// )?;
/// let problem = Problem::from_covariance(covariance).with_expected_returns(vec![0.07, 0.03]);
/// let solution = problem.solve(&Objective::MaximumSharpe { risk_free_rate: 0.02 })?;
/// assert!(solution.status.has_solution());
/// # Ok::<(), finalytics::Error>(())
/// ```
pub mod opt {
    pub use finalytics_opt::{
        AssetBound, BlackLitterman, BlackLittermanEstimate, Bound, Constraints,
        FactorExposureLimit, Fees, FrontierPoint, GroupLimit, Linkage, Objective, Prior, Problem,
        ProposedConversion, ProposedTrade, Rebalance, RebalanceOptions, RebalanceResults,
        ResampledSolution, Resampling, ReturnUncertainty, Side, Solution, SolveMethod, SolveStatus,
        TaxRates, TrackingErrorLimit, TradeLimits, Turnover, View, ViewUncertainty,
        implied_risk_aversion, rebalance, rebalance_accounts,
    };
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
