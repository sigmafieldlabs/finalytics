//! Portfolio optimisation: convex formulations of the standard objectives and constraints, risk
//! budgeting and hierarchical risk parity, the efficient frontier, and the trades that move an
//! account to target weights.
//!
//! A [`Problem`] holds the assets' covariance matrix or a factor model, expected returns or
//! scenario returns, and [`Constraints`]; [`Problem::solve`] finds the optimum for an
//! [`Objective`]. The convex
//! problems are solved by Clarabel, an interior-point solver for conic programmes:
//!
//! | Objective | Formulation |
//! | --- | --- |
//! | Minimum variance, mean–variance, target return | Quadratic programme |
//! | Robust mean–variance | Second-order cone (ellipsoid) or quadratic programme (box) for the worst-case return |
//! | Target volatility, tracking-error limit | Second-order cone programme |
//! | Maximum Sharpe ratio, maximum diversification | Quadratic programme after the Charnes–Cooper transformation |
//! | Minimum CVaR | Linear programme (Rockafellar and Uryasev) |
//! | Turnover limits and trading costs | Linear constraints on absolute trades; quadratic costs in the objective |
//! | Minimum trade sizes, number of trades | A heuristic: trades are left out and the rest re-optimised, round by round |
//! | Factor models | Variance in factor form, `y' F y + sum d w^2` with `y = B' w`; exposure limits as linear constraints on `y` |
//! | Risk budgeting | Newton's method on Spinu's convex formulation, with an exponential-cone programme as a fallback |
//! | Hierarchical risk parity | Clustering and recursive bisection, without a solver |
//!
//! Every [`Solution`] reports the solver status, the objective value, the largest constraint
//! violation and the time taken, so that a result can be shown to be genuine.
//!
//! [`BlackLitterman`] blends equilibrium returns implied by market weights with views, for the
//! expected returns and covariance of a problem. [`Problem::resampled`] averages the optimal
//! portfolios for estimates re-made from simulated histories, which makes them less sensitive to
//! estimation error.
//!
//! [`rebalance`] turns target weights into trades from an account's current holdings, choosing
//! the lots each sale closes (by tax, when tax rates are given), the fees it pays and the currency
//! conversions that fund purchases; [`rebalance_accounts`] does the same for several accounts
//! together.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod black_litterman;
mod frontier;
mod hrp;
mod linalg;
mod optimise;
mod problem;
mod program;
mod rebalance;
mod resample;
mod risk_budget;
mod solution;

pub use black_litterman::{
    BlackLitterman, BlackLittermanEstimate, Prior, View, ViewUncertainty, implied_risk_aversion,
};
pub use frontier::FrontierPoint;
pub use problem::{
    AssetBound, Bound, Constraints, FactorExposureLimit, GroupLimit, Linkage, Objective, Problem,
    ReturnUncertainty, TrackingErrorLimit, TradeLimits, Turnover,
};
pub use rebalance::{
    CombinedRebalance, Fees, ProposedConversion, ProposedTrade, Rebalance, RebalanceOptions, Side,
    TaxRates, rebalance, rebalance_accounts,
};
pub use resample::{ResampledSolution, Resampling};
pub use solution::{Solution, SolveMethod, SolveStatus};

use finalytics_core::{Error, ValidationReport};

/// An error for an invalid input.
pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}
