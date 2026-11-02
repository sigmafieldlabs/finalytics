//! Risk analytics: return statistics, drawdowns, value at risk and expected shortfall,
//! covariance estimation, factor models, the decomposition of a portfolio's risk by holding, by
//! group and by factor, and scenarios: shocks and historical replays, with their effect on value
//! and risk.
//!
//! The statistics take returns as plain slices, one per period, so they serve both a portfolio's
//! returns from [`finalytics_perf`](https://docs.rs/finalytics-perf) and returns supplied
//! directly. Inputs are validated: a non-finite value or a return below -100% is reported by
//! position. [`holding_weights`] and [`holding_returns`] prepare the inputs to risk
//! decomposition from a portfolio's books.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod covariance;
mod decomposition;
mod distribution;
mod drawdown;
mod factor;
mod holdings;
mod input;
mod linalg;
mod matrix;
mod prices;
mod rolling;
mod scenario;
mod statistics;
mod tail;

pub use covariance::{CovarianceEstimate, CovarianceMethod, covariance};
pub use decomposition::{
    GroupRisk, RiskContribution, RiskDecomposition, group_risk, risk_decomposition,
    tracking_error_decomposition,
};
pub use distribution::{normal_cdf, normal_pdf, normal_quantile};
pub use drawdown::{Drawdown, drawdown_series, drawdowns};
pub use factor::{
    AssetFit, FactorContribution, FactorMethod, FactorModel, FactorModelEstimate, FactorRisk,
    factor_risk, factor_tracking_error, fundamental_model, regression_model, statistical_model,
};
pub use holdings::{HoldingReturns, holding_returns, holding_weights};
pub use matrix::{Matrix, ReturnsTable};
pub use prices::{PriceReturns, PriceTable, periods_per_year};
pub use rolling::{rolling, rolling_relative_statistics, rolling_statistics, rolling_tail_risk};
pub use scenario::{
    CorrelationStress, FactorShocks, GroupImpact, GroupShock, HistoricalWindow, HoldingImpact,
    ScenarioOptions, ScenarioPath, ScenarioResult, ScenarioRisk, ScenarioRiskModel, SchemeImpact,
    SchemeRisk, ShockSource, Shocks, historical_scenario, shock_scenario, stressed_covariance,
};
pub use statistics::{
    RelativeStatistics, Statistics, StatisticsOptions, max_drawdown, relative_statistics,
    statistics, total_return,
};
pub use tail::{TailMethod, TailRisk, tail_risk};
