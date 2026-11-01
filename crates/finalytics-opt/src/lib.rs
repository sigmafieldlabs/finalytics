//! Portfolio optimisation: convex formulations of the standard objectives and constraints, risk
//! budgeting and hierarchical risk parity, and the efficient frontier.
//!
//! A [`Problem`] holds the assets' covariance matrix, expected returns or scenario returns, and
//! [`Constraints`]; [`Problem::solve`] finds the optimum for an [`Objective`]. The convex
//! problems are solved by Clarabel, an interior-point solver for conic programmes:
//!
//! | Objective | Formulation |
//! | --- | --- |
//! | Minimum variance, mean–variance, target return | Quadratic programme |
//! | Target volatility, tracking-error limit | Second-order cone programme |
//! | Maximum Sharpe ratio, maximum diversification | Quadratic programme after the Charnes–Cooper transformation |
//! | Minimum CVaR | Linear programme (Rockafellar and Uryasev) |
//! | Turnover limits and trading costs | Linear constraints on absolute trades |
//! | Risk budgeting | Newton's method on Spinu's convex formulation, with an exponential-cone programme as a fallback |
//! | Hierarchical risk parity | Clustering and recursive bisection, without a solver |
//!
//! Every [`Solution`] reports the solver status, the objective value, the largest constraint
//! violation and the time taken, so that a result can be shown to be genuine.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which re-exports
//! the public API of every layer.

mod frontier;
mod hrp;
mod linalg;
mod optimise;
mod problem;
mod program;
mod risk_budget;
mod solution;

pub use frontier::FrontierPoint;
pub use problem::{
    AssetBound, Bound, Constraints, GroupLimit, Linkage, Objective, Problem, TrackingErrorLimit,
    Turnover,
};
pub use solution::{Method, Solution, SolveStatus};

use finalytics_core::{Error, ValidationReport};

/// An error for an invalid input.
pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}
