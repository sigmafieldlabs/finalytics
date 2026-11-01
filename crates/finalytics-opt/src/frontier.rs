//! The efficient frontier.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;

use crate::invalid;
use crate::optimise::maximum_return;
use crate::problem::{Objective, Problem};

/// A portfolio on the efficient frontier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrontierPoint {
    /// The expected return.
    pub expected_return: f64,
    /// The volatility.
    pub volatility: f64,
    /// The weights, one per asset.
    pub weights: Vec<f64>,
}

impl Problem {
    /// The efficient frontier under the problem's constraints: `points` portfolios (at least 2)
    /// with target returns evenly spaced from the minimum-variance portfolio's expected return
    /// to the largest attainable, each with the smallest variance for its return.
    ///
    /// Needs a covariance matrix and expected returns. Points the solver cannot solve are left
    /// out.
    pub fn frontier(&self, points: usize) -> Result<Vec<FrontierPoint>> {
        if points < 2 {
            return Err(invalid("points", "must be at least 2"));
        }
        let anchor = Objective::TargetReturn { target: 0.0 };
        let prepared = self.prepare(&anchor)?;
        let start = self.solve(&Objective::MinimumVariance)?;
        let (Some(low), Some(_)) = (start.expected_return, start.weights.as_ref()) else {
            return Err(invalid(
                "constraints",
                "no portfolio satisfies the constraints, so there is no frontier",
            ));
        };
        let high = maximum_return(&prepared).unwrap_or(low).max(low);
        let mut frontier = Vec::with_capacity(points);
        for k in 0..points {
            let target = if k + 1 == points {
                high
            } else {
                low + (high - low) * k as f64 / (points - 1) as f64
            };
            // Back the end points off by a hair, so that rounding cannot make them infeasible.
            let target = target - 1e-10 * target.abs().max(1.0);
            let solution = self.solve(&Objective::TargetReturn { target })?;
            if let (Some(weights), Some(expected_return), Some(volatility)) = (
                solution.weights,
                solution.expected_return,
                solution.volatility,
            ) {
                frontier.push(FrontierPoint {
                    expected_return,
                    volatility,
                    weights,
                });
            }
        }
        Ok(frontier)
    }
}
