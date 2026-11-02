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
//! | Target volatility, tracking-error limit | Second-order cone programme |
//! | Maximum Sharpe ratio, maximum diversification | Quadratic programme after the Charnes–Cooper transformation |
//! | Minimum CVaR | Linear programme (Rockafellar and Uryasev) |
//! | Turnover limits and trading costs | Linear constraints on absolute trades |
//! | Factor models | Variance in factor form, `y' F y + sum d w^2` with `y = B' w`; exposure limits as linear constraints on `y` |
//! | Risk budgeting | Newton's method on Spinu's convex formulation, with an exponential-cone programme as a fallback |
//! | Hierarchical risk parity | Clustering and recursive bisection, without a solver |
//!
//! Every [`Solution`] reports the solver status, the objective value, the largest constraint
//! violation and the time taken, so that a result can be shown to be genuine.
//!
//! [`rebalance`] turns target weights into trades from an account's current holdings, choosing
//! the lots each sale closes and the currency conversions that fund purchases.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which re-exports
//! the public API of every layer.

mod frontier;
mod hrp;
mod linalg;
mod optimise;
mod problem;
mod program;
mod rebalance;
mod risk_budget;
mod solution;

pub use frontier::FrontierPoint;
pub use problem::{
    AssetBound, Bound, Constraints, FactorExposureLimit, GroupLimit, Linkage, Objective, Problem,
    TrackingErrorLimit, Turnover,
};
pub use rebalance::{
    ProposedConversion, ProposedTrade, Rebalance, RebalanceOptions, Side, rebalance,
};
pub use solution::{Method, Solution, SolveStatus};

use finalytics_core::{Error, ValidationReport};

/// An error for an invalid input.
pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}
