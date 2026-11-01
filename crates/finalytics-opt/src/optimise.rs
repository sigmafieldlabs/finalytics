//! Formulations of the objectives and constraints as conic programmes.

use web_time::Instant;

use finalytics_core::Result;

use crate::problem::{Objective, Prepared, Problem, Turnover};
use crate::program::{Program, Row};
use crate::solution::{Method, Solution, SolveStatus};
use crate::{hrp, risk_budget};

/// Where the weights and auxiliary variables are in a programme.
struct Layout {
    /// The first weight (or, for the Charnes–Cooper transformation, the first scaled weight).
    weights: usize,
    /// The scale variable of the Charnes–Cooper transformation, which multiplies every constant.
    scale: Option<usize>,
    /// The absolute trades `d_i >= |w_i - current_i|`, when turnover is limited or costed.
    trades: Option<usize>,
}

impl Layout {
    /// A row whose constant is multiplied by the scale variable, if there is one.
    fn row(&self, mut coeffs: Vec<(usize, f64)>, rhs: f64) -> Row {
        match self.scale {
            Some(k) if rhs != 0.0 => {
                coeffs.push((k, -rhs));
                Row::new(coeffs, 0.0)
            }
            _ => Row::new(coeffs, rhs),
        }
    }

    /// The coefficients `values . w`.
    fn on_weights(&self, values: &[f64]) -> Vec<(usize, f64)> {
        values
            .iter()
            .enumerate()
            .filter(|(_, v)| **v != 0.0)
            .map(|(i, v)| (self.weights + i, *v))
            .collect()
    }

    /// The coefficients `-costs . d`, the trading costs as a reduction of expected return.
    fn costs(&self, costs: Option<&[f64]>) -> Vec<(usize, f64)> {
        match (costs, self.trades) {
            (Some(costs), Some(d)) => costs.iter().enumerate().map(|(i, c)| (d + i, -c)).collect(),
            _ => Vec::new(),
        }
    }
}

impl Problem {
    /// Finds the optimal portfolio for `objective`.
    ///
    /// Invalid inputs are an error, listing every problem found. A problem that is valid but has
    /// no solution, such as one whose constraints cannot all hold, returns a [`Solution`] whose
    /// status says why.
    ///
    /// ```
    /// use finalytics_opt::{Objective, Problem};
    /// use finalytics_risk::Matrix;
    ///
    /// let covariance = Matrix::new(
    ///     vec!["equity".into(), "bonds".into()],
    ///     vec![0.0400, 0.0020, 0.0020, 0.0049],
    /// )?;
    /// let solution = Problem::new(covariance).solve(&Objective::MinimumVariance)?;
    /// let weights = solution.weights.unwrap();
    /// assert!(weights[1] > weights[0]);
    /// # Ok::<(), finalytics_core::Error>(())
    /// ```
    pub fn solve(&self, objective: &Objective) -> Result<Solution> {
        let started = Instant::now();
        let prepared = self.prepare(objective)?;
        let (status, weights, method, iterations) = match objective {
            Objective::RiskBudgeting { budgets } => {
                let n = prepared.n;
                let budgets = budgets.clone().unwrap_or_else(|| vec![1.0; n]);
                let found = risk_budget::solve(prepared.covariance(), &budgets)?;
                (found.status, found.weights, found.method, found.iterations)
            }
            Objective::HierarchicalRiskParity { linkage } => {
                let weights = hrp::weights(prepared.covariance(), *linkage)?;
                (
                    SolveStatus::Optimal,
                    Some(weights),
                    Method::RecursiveBisection,
                    0,
                )
            }
            _ => {
                let (program, layout) = formulate(&prepared, objective);
                let outcome = program.solve();
                let weights = outcome
                    .status
                    .has_solution()
                    .then(|| decode(&outcome.x, &layout, prepared.n))
                    .flatten();
                let status = if outcome.status.has_solution() && weights.is_none() {
                    SolveStatus::NumericalDifficulty
                } else {
                    outcome.status
                };
                (status, weights, Method::InteriorPoint, outcome.iterations)
            }
        };
        Ok(report(
            &prepared, objective, status, weights, method, iterations, started,
        ))
    }
}

/// Builds the programme for one of the solver-based objectives.
fn formulate(prepared: &Prepared<'_>, objective: &Objective) -> (Program, Layout) {
    let n = prepared.n;
    let mut program = Program::default();
    let weights = program.variables(n);
    let homogeneous = matches!(
        objective,
        Objective::MaximumSharpe { .. } | Objective::MaximumDiversification
    );
    let scale = homogeneous.then(|| program.variables(1));
    if let Some(k) = scale {
        program.at_most(Row::new(vec![(k, -1.0)], 0.0));
    }
    let constraints = prepared.constraints();
    let trades = constraints
        .turnover
        .as_ref()
        .filter(|t| t.limit.is_some() || t.costs.is_some())
        .map(|_| program.variables(n));
    let layout = Layout {
        weights,
        scale,
        trades,
    };
    add_constraints(&mut program, &layout, prepared);

    let mu = prepared.expected_returns();
    let costs = prepared.costs();
    match objective {
        Objective::MinimumVariance | Objective::TargetReturn { .. } => {
            program.quadratic(weights, prepared.covariance(), 1.0);
            if let Objective::TargetReturn { target } = objective {
                let mut coeffs = layout.on_weights(&mu.iter().map(|m| -m).collect::<Vec<_>>());
                coeffs.extend(layout.costs(costs).into_iter().map(|(i, c)| (i, -c)));
                program.at_most(Row::new(coeffs, -target));
            }
        }
        Objective::MeanVariance { risk_aversion } => {
            program.quadratic(weights, prepared.covariance(), *risk_aversion);
            add_return_to_objective(&mut program, &layout, mu, costs);
        }
        Objective::TargetVolatility { target } => {
            add_return_to_objective(&mut program, &layout, mu, costs);
            let factor = prepared
                .factor
                .as_ref()
                .map(|f| f.rows.as_slice())
                .unwrap_or(&[]);
            let mut rows = vec![Row::new(Vec::new(), *target)];
            for r in factor {
                rows.push(Row::new(
                    layout.on_weights(&r.iter().map(|v| -v).collect::<Vec<_>>()),
                    0.0,
                ));
            }
            program.second_order_cone(rows);
        }
        Objective::MaximumSharpe { risk_free_rate } => {
            program.quadratic(weights, prepared.covariance(), 1.0);
            let mut coeffs = layout.on_weights(mu);
            coeffs.extend(layout.costs(costs));
            if let Some(k) = scale {
                coeffs.push((k, -risk_free_rate));
            }
            program.equal(Row::new(coeffs, 1.0));
        }
        Objective::MaximumDiversification => {
            let covariance = prepared.covariance();
            program.quadratic(weights, covariance, 1.0);
            let volatilities: Vec<f64> = covariance.diagonal().iter().map(|v| v.sqrt()).collect();
            program.equal(Row::new(layout.on_weights(&volatilities), 1.0));
        }
        Objective::MinimumCvar { confidence } => {
            // The scenarios are present: `prepare` checks it for this objective.
            if let Some(scenarios) = &prepared.problem.scenarios {
                let t = scenarios.observations();
                let alpha = 1.0 - confidence;
                let threshold = program.variables(1);
                let excess = program.variables(t);
                program.linear[threshold] = 1.0;
                for s in 0..t {
                    program.linear[excess + s] = 1.0 / (alpha * t as f64);
                    let returns: Vec<f64> = scenarios.columns().iter().map(|c| -c[s]).collect();
                    let mut coeffs = layout.on_weights(&returns);
                    coeffs.push((threshold, -1.0));
                    coeffs.push((excess + s, -1.0));
                    program.at_most(Row::new(coeffs, 0.0));
                    program.at_most(Row::new(vec![(excess + s, -1.0)], 0.0));
                }
            }
        }
        Objective::RiskBudgeting { .. } | Objective::HierarchicalRiskParity { .. } => {}
    }
    (program, layout)
}

/// Adds `-(mu . w - costs . d)` to the linear objective.
fn add_return_to_objective(
    program: &mut Program,
    layout: &Layout,
    mu: &[f64],
    costs: Option<&[f64]>,
) {
    for (i, m) in mu.iter().enumerate() {
        program.linear[layout.weights + i] -= m;
    }
    for (i, c) in layout.costs(costs) {
        program.linear[i] -= c;
    }
}

fn add_constraints(program: &mut Program, layout: &Layout, prepared: &Prepared<'_>) {
    let n = prepared.n;
    let c = prepared.constraints();
    let w = layout.weights;

    program.equal(layout.row(layout.on_weights(&vec![1.0; n]), c.budget));
    for (i, b) in prepared.bounds.iter().enumerate() {
        program.at_most(layout.row(vec![(w + i, 1.0)], b.max));
        program.at_most(layout.row(vec![(w + i, -1.0)], -b.min));
    }
    for (members, g) in prepared.groups.iter().zip(&c.groups) {
        let up: Vec<_> = members.iter().map(|&i| (w + i, 1.0)).collect();
        let down: Vec<_> = members.iter().map(|&i| (w + i, -1.0)).collect();
        program.at_most(layout.row(up, g.max));
        program.at_most(layout.row(down, -g.min));
    }
    if let Some(limit) = c.leverage {
        let gross = program.variables(n);
        for i in 0..n {
            program.at_most(Row::new(vec![(w + i, 1.0), (gross + i, -1.0)], 0.0));
            program.at_most(Row::new(vec![(w + i, -1.0), (gross + i, -1.0)], 0.0));
        }
        program.at_most(layout.row((0..n).map(|i| (gross + i, 1.0)).collect(), limit));
    }
    if let (Some(Turnover { current, limit, .. }), Some(d)) = (&c.turnover, layout.trades) {
        for (i, &now) in current.iter().enumerate() {
            program.at_most(layout.row(vec![(w + i, 1.0), (d + i, -1.0)], now));
            program.at_most(layout.row(vec![(w + i, -1.0), (d + i, -1.0)], -now));
        }
        if let Some(limit) = limit {
            program.at_most(layout.row((0..n).map(|i| (d + i, 1.0)).collect(), *limit));
        }
    }
    if let (Some(te), Some(factor)) = (&c.tracking_error, &prepared.factor) {
        let mut rows = vec![layout.row(Vec::new(), te.limit)];
        let shift = factor.times(&te.benchmark);
        for (r, s) in factor.rows.iter().zip(shift) {
            let coeffs = layout.on_weights(&r.iter().map(|v| -v).collect::<Vec<_>>());
            rows.push(layout.row(coeffs, -s));
        }
        program.second_order_cone(rows);
    }
}

/// The weights from the solver's variables, undoing the Charnes–Cooper scaling.
fn decode(x: &[f64], layout: &Layout, n: usize) -> Option<Vec<f64>> {
    let weights = x.get(layout.weights..layout.weights + n)?;
    match layout.scale {
        Some(k) => {
            let scale = *x.get(k)?;
            (scale > 1e-12).then(|| weights.iter().map(|y| y / scale).collect())
        }
        None => Some(weights.to_vec()),
    }
}

fn report(
    prepared: &Prepared<'_>,
    objective: &Objective,
    status: SolveStatus,
    weights: Option<Vec<f64>>,
    method: Method,
    iterations: u32,
    started: Instant,
) -> Solution {
    let problem = prepared.problem;
    let mu = problem.expected_returns.as_deref();
    let covariance = problem.covariance.as_ref();
    let expected_return = weights
        .as_ref()
        .zip(mu)
        .map(|(w, mu)| w.iter().zip(mu).map(|(a, b)| a * b).sum::<f64>());
    let variance = weights
        .as_ref()
        .zip(covariance)
        .and_then(|(w, c)| c.quadratic_form(w).ok())
        .map(|v| v.max(0.0));
    let volatility = variance.map(f64::sqrt);
    let objective_value = weights.as_ref().and_then(|w| match objective {
        Objective::MinimumVariance | Objective::TargetReturn { .. } => variance,
        Objective::MeanVariance { risk_aversion } => {
            Some(expected_return? - risk_aversion / 2.0 * variance?)
        }
        Objective::TargetVolatility { .. } => expected_return,
        Objective::MaximumSharpe { risk_free_rate } => {
            Some((expected_return? - risk_free_rate) / volatility?)
        }
        Objective::MaximumDiversification => {
            let c = covariance?;
            let weighted: f64 = w.iter().zip(c.diagonal()).map(|(w, v)| w * v.sqrt()).sum();
            Some(weighted / volatility?)
        }
        Objective::MinimumCvar { confidence } => {
            let scenarios = problem.scenarios.as_ref()?;
            let returns: Vec<f64> = (0..scenarios.observations())
                .map(|s| {
                    scenarios
                        .columns()
                        .iter()
                        .zip(w)
                        .map(|(c, w)| c[s] * w)
                        .sum()
                })
                .collect();
            Some(cvar(&returns, 1.0 - confidence))
        }
        Objective::RiskBudgeting { budgets } => {
            risk_budget::largest_gap(covariance?, w, budgets.as_deref())
        }
        Objective::HierarchicalRiskParity { .. } => None,
    });
    let mut max_violation = weights.as_ref().map(|w| prepared.violation(w));
    if let (Some(v), Some(w)) = (&mut max_violation, &weights) {
        match objective {
            Objective::TargetReturn { target } => {
                let traded = traded_costs(prepared, w);
                *v = v.max(target - (expected_return.unwrap_or(0.0) - traded));
            }
            Objective::TargetVolatility { target } => {
                *v = v.max(volatility.unwrap_or(0.0) - target);
            }
            _ => {}
        }
    }
    Solution {
        objective: objective.clone(),
        status,
        labels: problem.labels.clone(),
        weights,
        objective_value,
        expected_return,
        volatility,
        max_violation,
        method,
        iterations,
        solve_time: started.elapsed().as_secs_f64(),
    }
}

/// The trading costs of moving from the current weights to `w`.
fn traded_costs(prepared: &Prepared<'_>, w: &[f64]) -> f64 {
    match (&prepared.constraints().turnover, prepared.costs()) {
        (Some(t), Some(costs)) => w
            .iter()
            .zip(&t.current)
            .zip(costs)
            .map(|((w, c), k)| (w - c).abs() * k)
            .sum(),
        _ => 0.0,
    }
}

/// The Rockafellar–Uryasev CVaR of scenario returns at tail probability `alpha`, as a positive
/// loss: the minimum over `z` of `z + mean(max(-r - z, 0)) / alpha`, attained at a quantile of the
/// losses.
pub(crate) fn cvar(returns: &[f64], alpha: f64) -> f64 {
    let mut losses: Vec<f64> = returns.iter().map(|r| -r).collect();
    losses.sort_by(f64::total_cmp);
    let t = losses.len();
    // With z at the k-th smallest loss, the losses above it exceed it by
    // (sum of the larger losses) - (their number) * z.
    let mut above = 0.0;
    let mut best = f64::INFINITY;
    for k in (0..t).rev() {
        let z = losses[k];
        let excess = above - (t - 1 - k) as f64 * z;
        best = best.min(z + excess / (alpha * t as f64));
        above += z;
    }
    best
}

/// The largest expected return any portfolio satisfying the constraints can have, net of trading
/// costs, or `None` if the solver finds none.
pub(crate) fn maximum_return(prepared: &Prepared<'_>) -> Option<f64> {
    let n = prepared.n;
    let mut program = Program::default();
    let weights = program.variables(n);
    let constraints = prepared.constraints();
    let trades = constraints
        .turnover
        .as_ref()
        .filter(|t| t.limit.is_some() || t.costs.is_some())
        .map(|_| program.variables(n));
    let layout = Layout {
        weights,
        scale: None,
        trades,
    };
    add_constraints(&mut program, &layout, prepared);
    add_return_to_objective(
        &mut program,
        &layout,
        prepared.expected_returns(),
        prepared.costs(),
    );
    let outcome = program.solve();
    let w = outcome
        .status
        .has_solution()
        .then(|| decode(&outcome.x, &layout, n))??;
    let gross: f64 = w
        .iter()
        .zip(prepared.expected_returns())
        .map(|(a, b)| a * b)
        .sum();
    Some(gross - traded_costs(prepared, &w))
}
