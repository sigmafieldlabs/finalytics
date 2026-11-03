//! The result of an optimisation.

use serde::{Deserialize, Serialize};

use crate::problem::Objective;

/// How an optimisation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SolveStatus {
    /// Solved to the solver's full accuracy.
    Optimal,
    /// Solved to reduced accuracy; the weights are usable but should be checked against the
    /// solution's largest constraint violation, `max_violation`.
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

/// The status in words, as it is written in JSON: `infeasible`, `iteration_limit`.
impl std::fmt::Display for SolveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SolveStatus::Optimal => "optimal",
            SolveStatus::NearlyOptimal => "nearly_optimal",
            SolveStatus::Infeasible => "infeasible",
            SolveStatus::Unbounded => "unbounded",
            SolveStatus::IterationLimit => "iteration_limit",
            SolveStatus::NumericalDifficulty => "numerical_difficulty",
        })
    }
}

/// The method that produced a solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SolveMethod {
    /// The Clarabel interior-point solver for convex conic programmes.
    InteriorPoint,
    /// Newton's method on the convex risk-budgeting problem.
    Newton,
    /// Hierarchical clustering and recursive bisection, without a solver.
    RecursiveBisection,
    /// A formula, without a solver: equal weight and inverse volatility.
    ClosedForm,
    /// The interior-point solver, followed by a heuristic for trade limits: trades below the
    /// minimum, and the smallest trades beyond the largest number allowed, are left out and the
    /// rest re-optimised, round by round. Not guaranteed to find the best portfolio within the
    /// limits.
    TradeElimination,
}

/// The result of an optimisation, with what is needed to judge it: the status, the objective,
/// how closely the constraints hold, and the work done.
///
/// Two solutions are equal when everything but the time taken is equal, so that solving the
/// same problem twice gives equal solutions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Solution {
    /// The objective optimised.
    pub objective: Objective,
    /// How the optimisation ended.
    pub status: SolveStatus,
    /// The asset labels.
    pub labels: Vec<String>,
    /// The optimal weights, one per asset, when the status is optimal or nearly optimal.
    pub weights: Option<Vec<f64>>,
    /// The value of the objective at the weights: the variance for minimum variance and target
    /// return; the utility for mean–variance; the expected return for target volatility; the
    /// Sharpe ratio; the diversification ratio; the CVaR, as a positive loss; for risk budgeting,
    /// the largest difference between an asset's share of risk and its budget. Absent for
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
    pub method: SolveMethod,
    /// Iterations of the solver or of Newton's method.
    pub iterations: u32,
    /// The time taken, in seconds. It varies from run to run and is left out of comparisons.
    pub solve_time: f64,
}

impl PartialEq for Solution {
    fn eq(&self, other: &Self) -> bool {
        // Listing every field makes a new one a compile error here until it is compared.
        let Solution {
            objective,
            status,
            labels,
            weights,
            objective_value,
            expected_return,
            volatility,
            max_violation,
            method,
            iterations,
            solve_time: _,
        } = self;
        *objective == other.objective
            && *status == other.status
            && *labels == other.labels
            && *weights == other.weights
            && *objective_value == other.objective_value
            && *expected_return == other.expected_return
            && *volatility == other.volatility
            && *max_violation == other.max_violation
            && *method == other.method
            && *iterations == other.iterations
    }
}
