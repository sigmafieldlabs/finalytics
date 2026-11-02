//! The result of an optimisation.

use serde::{Deserialize, Serialize};

use crate::problem::Objective;

/// How an optimisation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SolveStatus {
    /// Solved to the solver's full accuracy.
    Optimal,
    /// Solved to reduced accuracy; the weights are usable but should be checked against
    /// [`Solution::max_violation`].
    NearlyOptimal,
    /// No portfolio satisfies the constraints.
    Infeasible,
    /// The objective can be improved without limit; add bounds or a leverage limit.
    Unbounded,
    /// The solver stopped at its iteration limit.
    IterationLimit,
    /// The solver could not make progress, usually because the inputs are badly scaled or
    /// nearly degenerate.
    NumericalDifficulty,
}

impl SolveStatus {
    /// Whether the optimisation produced weights.
    pub fn has_solution(self) -> bool {
        matches!(self, SolveStatus::Optimal | SolveStatus::NearlyOptimal)
    }
}

/// The method that produced a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// The Clarabel interior-point solver for convex conic programmes.
    InteriorPoint,
    /// Newton's method on the convex risk-budgeting problem.
    Newton,
    /// Hierarchical clustering and recursive bisection, without a solver.
    RecursiveBisection,
}

/// The result of an optimisation, with what is needed to judge it: the status, the objective,
/// how closely the constraints hold, and the work done.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Solution {
    /// The objective optimised.
    pub objective: Objective,
    /// How the optimisation ended.
    pub status: SolveStatus,
    /// The asset labels.
    pub labels: Vec<String>,
    /// The optimal weights, one per asset, when [`SolveStatus::has_solution`].
    pub weights: Option<Vec<f64>>,
    /// The value of the objective at the weights: the variance for minimum variance and target
    /// return; the utility for mean–variance; the expected return for target volatility; the
    /// Sharpe ratio; the diversification ratio; the CVaR, as a positive loss; for risk budgeting,
    /// the largest difference between an asset's share of risk and its budget. `None` for
    /// hierarchical risk parity, which optimises no objective.
    pub objective_value: Option<f64>,
    /// The expected return at the weights, when expected returns are given.
    pub expected_return: Option<f64>,
    /// The volatility at the weights, when a covariance matrix or a factor model is given.
    pub volatility: Option<f64>,
    /// The largest amount by which the weights break a constraint (0 when every constraint
    /// holds exactly).
    pub max_violation: Option<f64>,
    /// The method used.
    pub method: Method,
    /// Iterations of the solver or of Newton's method.
    pub iterations: u32,
    /// The time taken, in seconds.
    pub solve_time: f64,
}
