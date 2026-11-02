//! Formulations of the objectives and constraints as conic programmes.

use web_time::Instant;

use finalytics_core::Result;

use crate::problem::{
    AssetBound, Objective, Prepared, Problem, ReturnUncertainty, RiskModel, TRADED, TradeLimits,
    Turnover,
};
use crate::program::{Program, Row};
use crate::solution::{Solution, SolveMethod, SolveStatus};
use crate::{hrp, risk_budget};

/// Where the weights and auxiliary variables are in a programme.
struct Layout {
    /// The first weight (or, for the Charnes–Cooper transformation, the first scaled weight).
    weights: usize,
    /// The scale variable of the Charnes–Cooper transformation, which multiplies every constant.
    scale: Option<usize>,
    /// The absolute trades `d_i >= |w_i - current_i|`, when turnover is limited or costed.
    trades: Option<usize>,
    /// The portfolio's factor exposures `y = B' w`, when risk comes from a factor model.
    factors: Option<usize>,
}

/// Adds the weights and the auxiliary variables the problem needs, with the rows that define
/// them.
fn layout(program: &mut Program, prepared: &Prepared<'_>, homogeneous: bool) -> Layout {
    let n = prepared.n;
    let weights = program.variables(n);
    let scale = homogeneous.then(|| program.variables(1));
    if let Some(k) = scale {
        program.at_most(Row::new(vec![(k, -1.0)], 0.0));
    }
    let trades = prepared
        .constraints()
        .turnover
        .as_ref()
        .filter(|t| t.limit.is_some() || t.costs.is_some())
        .map(|_| program.variables(n));
    let factors = match &prepared.risk {
        Some(RiskModel::Factors { model, .. }) => {
            let k = model.factors().len();
            let y = program.variables(k);
            for j in 0..k {
                let mut coeffs = vec![(y + j, 1.0)];
                for (i, row) in model.exposures().iter().enumerate() {
                    if row[j] != 0.0 {
                        coeffs.push((weights + i, -row[j]));
                    }
                }
                program.equal(Row::new(coeffs, 0.0));
            }
            Some(y)
        }
        _ => None,
    };
    Layout {
        weights,
        scale,
        trades,
        factors,
    }
}

/// Adds `scale * w' C w` to the objective, in factor form `y' F y + sum d w^2` for a factor
/// model.
fn add_variance(program: &mut Program, layout: &Layout, prepared: &Prepared<'_>, scale: f64) {
    match (prepared.risk(), layout.factors) {
        (RiskModel::Factors { model, .. }, Some(y)) => {
            program.quadratic(y, model.factor_covariance(), scale);
            program.quadratic_diagonal(layout.weights, model.specific_variances(), scale);
        }
        (risk, _) => program.quadratic(layout.weights, &risk.dense(), scale),
    }
}

/// The rows `R (w - shift)` of a second-order cone whose norm is the volatility of `w - shift`,
/// as `rhs - coeffs . x`.
fn risk_rows(prepared: &Prepared<'_>, layout: &Layout, shift: Option<&[f64]>) -> Vec<Row> {
    let w = layout.weights;
    let negated = |values: &[f64], offset: usize| -> Vec<(usize, f64)> {
        values
            .iter()
            .enumerate()
            .filter(|(_, v)| **v != 0.0)
            .map(|(i, v)| (offset + i, -v))
            .collect()
    };
    match (prepared.risk(), layout.factors) {
        (RiskModel::Factors { model, root }, Some(y)) => {
            let shifted = shift.map(|b| model.portfolio_exposures(b).unwrap_or_default());
            let mut rows: Vec<Row> = root
                .rows
                .iter()
                .map(|r| {
                    let constant = shifted
                        .as_ref()
                        .map_or(0.0, |x| r.iter().zip(x).map(|(a, b)| a * b).sum());
                    layout.row(negated(r, y), -constant)
                })
                .collect();
            for (i, d) in model.specific_variances().iter().enumerate() {
                if *d > 0.0 {
                    let s = d.sqrt();
                    let constant = shift.map_or(0.0, |b| s * b[i]);
                    rows.push(layout.row(vec![(w + i, -s)], -constant));
                }
            }
            rows
        }
        (
            RiskModel::Covariance {
                root: Some(root), ..
            },
            _,
        ) => {
            let shifted = shift.map(|b| root.times(b));
            root.rows
                .iter()
                .enumerate()
                .map(|(k, r)| {
                    let constant = shifted.as_ref().map_or(0.0, |x| x[k]);
                    layout.row(negated(r, w), -constant)
                })
                .collect()
        }
        _ => Vec::new(),
    }
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
    /// # mod finalytics {
    /// #     pub use finalytics_core::*;
    /// #     pub mod risk { pub use finalytics_risk::*; }
    /// #     pub mod opt { pub use finalytics_opt::*; }
    /// # }
    /// use finalytics::opt::{Objective, Problem};
    /// use finalytics::risk::Matrix;
    ///
    /// let covariance = Matrix::new(
    ///     vec!["equity".into(), "bonds".into()],
    ///     vec![0.0400, 0.0020, 0.0020, 0.0049],
    /// )?;
    /// let solution = Problem::from_covariance(covariance).solve(&Objective::MinimumVariance)?;
    /// let weights = solution.weights.unwrap();
    /// assert!(weights[1] > weights[0]);
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn solve(&self, objective: &Objective) -> Result<Solution> {
        let started = Instant::now();
        let prepared = self.prepare(objective)?;
        let (status, weights, method, iterations) = match objective {
            Objective::RiskBudgeting { budgets } => {
                let n = prepared.n;
                let budgets = budgets.clone().unwrap_or_else(|| vec![1.0; n]);
                let found = risk_budget::solve(&prepared.risk().dense(), &budgets)?;
                (found.status, found.weights, found.method, found.iterations)
            }
            Objective::HierarchicalRiskParity { linkage } => {
                let weights = hrp::weights(&prepared.risk().dense(), *linkage)?;
                (
                    SolveStatus::Optimal,
                    Some(weights),
                    SolveMethod::RecursiveBisection,
                    0,
                )
            }
            _ => match &self.constraints.trades {
                Some(limits) => self.eliminate_trades(objective, limits)?,
                None => {
                    let (status, weights, iterations) = solve_conic(&prepared, objective);
                    (status, weights, SolveMethod::InteriorPoint, iterations)
                }
            },
        };
        Ok(report(
            &prepared, objective, status, weights, method, iterations, started,
        ))
    }
}

/// Solves one of the solver-based objectives: the status, the weights and the iterations.
fn solve_conic(
    prepared: &Prepared<'_>,
    objective: &Objective,
) -> (SolveStatus, Option<Vec<f64>>, u32) {
    let (program, layout) = formulate(prepared, objective);
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
    (status, weights, outcome.iterations)
}

impl Problem {
    /// The trade-limit heuristic: solve without the limits, then leave out every trade below
    /// the minimum and, beyond the largest number of trades, the smallest of the rest, by fixing
    /// those assets at their current weights; re-optimise, and repeat until the limits hold.
    fn eliminate_trades(
        &self,
        objective: &Objective,
        limits: &TradeLimits,
    ) -> Result<(SolveStatus, Option<Vec<f64>>, SolveMethod, u32)> {
        // `prepare` checks that trade limits come with the current weights.
        let current = self
            .constraints
            .turnover
            .as_ref()
            .map(|t| t.current.clone())
            .unwrap_or_default();
        let mut fixed = vec![false; self.labels.len()];
        let mut problem = self.clone();
        problem.constraints.trades = None;
        let mut iterations = 0;
        loop {
            let prepared = problem.prepare(objective)?;
            let (status, weights, used) = solve_conic(&prepared, objective);
            iterations += used;
            let Some(mut w) = weights.filter(|_| status.has_solution()) else {
                return Ok((status, None, SolveMethod::TradeElimination, iterations));
            };
            // The solver holds a fixed asset at its current weight only to its tolerance.
            for (i, w) in w.iter_mut().enumerate() {
                if fixed[i] {
                    *w = current[i];
                }
            }
            let mut trades: Vec<(usize, f64)> = w
                .iter()
                .zip(&current)
                .enumerate()
                .filter(|(i, _)| !fixed[*i])
                .map(|(i, (w, c))| (i, (w - c).abs()))
                .filter(|(_, d)| *d > TRADED)
                .collect();
            trades.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let mut leave_out: Vec<usize> = trades
                .iter()
                .filter(|(_, d)| limits.minimum.is_some_and(|m| *d < m))
                .map(|(i, _)| *i)
                .collect();
            let kept = trades.len() - leave_out.len();
            if let Some(most) = limits.maximum_count
                && kept > most
            {
                let excess = kept - most;
                let extra: Vec<usize> = trades
                    .iter()
                    .filter(|(i, _)| !leave_out.contains(i))
                    .take(excess)
                    .map(|(i, _)| *i)
                    .collect();
                leave_out.extend(extra);
            }
            if leave_out.is_empty() {
                return Ok((status, Some(w), SolveMethod::TradeElimination, iterations));
            }
            for i in leave_out {
                fixed[i] = true;
                problem.constraints.asset_bounds.push(AssetBound {
                    asset: self.labels[i].clone(),
                    min: current[i],
                    max: current[i],
                });
            }
        }
    }
}

/// Builds the programme for one of the solver-based objectives.
fn formulate(prepared: &Prepared<'_>, objective: &Objective) -> (Program, Layout) {
    let mut program = Program::default();
    let homogeneous = matches!(
        objective,
        Objective::MaximumSharpe { .. } | Objective::MaximumDiversification
    );
    let layout = layout(&mut program, prepared, homogeneous);
    let scale = layout.scale;
    add_constraints(&mut program, &layout, prepared);

    let mu = prepared.expected_returns();
    let costs = prepared.costs();
    match objective {
        Objective::MinimumVariance | Objective::TargetReturn { .. } => {
            add_variance(&mut program, &layout, prepared, 1.0);
            if let Objective::TargetReturn { target } = objective {
                let mut coeffs = layout.on_weights(&mu.iter().map(|m| -m).collect::<Vec<_>>());
                coeffs.extend(layout.costs(costs).into_iter().map(|(i, c)| (i, -c)));
                program.at_most(Row::new(coeffs, -target));
            }
        }
        Objective::MeanVariance { risk_aversion } => {
            add_variance(&mut program, &layout, prepared, *risk_aversion);
            add_return_to_objective(&mut program, &layout, mu, costs);
            add_quadratic_costs(&mut program, &layout, prepared);
        }
        Objective::RobustMeanVariance {
            risk_aversion,
            uncertainty,
        } => {
            add_variance(&mut program, &layout, prepared, *risk_aversion);
            add_return_to_objective(&mut program, &layout, mu, costs);
            add_quadratic_costs(&mut program, &layout, prepared);
            add_worst_case(&mut program, &layout, prepared, uncertainty);
        }
        Objective::TargetVolatility { target } => {
            add_return_to_objective(&mut program, &layout, mu, costs);
            add_quadratic_costs(&mut program, &layout, prepared);
            let mut rows = vec![Row::new(Vec::new(), *target)];
            rows.extend(risk_rows(prepared, &layout, None));
            program.second_order_cone(rows);
        }
        Objective::MaximumSharpe { risk_free_rate } => {
            add_variance(&mut program, &layout, prepared, 1.0);
            let mut coeffs = layout.on_weights(mu);
            coeffs.extend(layout.costs(costs));
            if let Some(k) = scale {
                coeffs.push((k, -risk_free_rate));
            }
            program.equal(Row::new(coeffs, 1.0));
        }
        Objective::MaximumDiversification => {
            add_variance(&mut program, &layout, prepared, 1.0);
            let volatilities = prepared.risk().volatilities();
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

/// Adds the worst-case shortfall of the expected return to the objective: `size * t` with
/// `t >= |R w|` for an ellipsoid, or `sum h_i u_i` with `u_i >= |w_i|` for a box.
fn add_worst_case(
    program: &mut Program,
    layout: &Layout,
    prepared: &Prepared<'_>,
    uncertainty: &ReturnUncertainty,
) {
    let w = layout.weights;
    match uncertainty {
        ReturnUncertainty::Ellipsoid { size, .. } => {
            let t = program.variables(1);
            program.linear[t] = *size;
            let mut rows = vec![Row::new(vec![(t, -1.0)], 0.0)];
            match &prepared.uncertainty_root {
                Some(root) => rows.extend(root.rows.iter().map(|r| {
                    Row::new(
                        r.iter()
                            .enumerate()
                            .filter(|(_, v)| **v != 0.0)
                            .map(|(i, v)| (w + i, -v))
                            .collect(),
                        0.0,
                    )
                })),
                None => rows.extend(risk_rows(prepared, layout, None)),
            }
            program.second_order_cone(rows);
        }
        ReturnUncertainty::Box { half_widths } => {
            let u = program.variables(half_widths.len());
            for (i, h) in half_widths.iter().enumerate() {
                program.linear[u + i] = *h;
                program.at_most(Row::new(vec![(w + i, 1.0), (u + i, -1.0)], 0.0));
                program.at_most(Row::new(vec![(w + i, -1.0), (u + i, -1.0)], 0.0));
            }
        }
    }
}

/// How much the worst case in `uncertainty` lowers the expected return of `w`.
fn worst_case_shortfall(
    prepared: &Prepared<'_>,
    uncertainty: &ReturnUncertainty,
    w: &[f64],
) -> Option<f64> {
    match uncertainty {
        ReturnUncertainty::Ellipsoid {
            size,
            covariance: Some(s),
        } => Some(size * s.quadratic_form(w).ok()?.max(0.0).sqrt()),
        ReturnUncertainty::Ellipsoid {
            size,
            covariance: None,
        } => Some(size * prepared.risk.as_ref()?.variance(w).sqrt()),
        ReturnUncertainty::Box { half_widths } => {
            Some(half_widths.iter().zip(w).map(|(h, w)| h * w.abs()).sum())
        }
    }
}

/// Adds `sum k_i (w_i - c_i)^2` to the objective, as `1/2 w' (2 diag k) w - (2 k c)' w` plus a
/// constant.
fn add_quadratic_costs(program: &mut Program, layout: &Layout, prepared: &Prepared<'_>) {
    if let Some((k, current)) = prepared.quadratic_costs() {
        program.quadratic_diagonal(layout.weights, k, 2.0);
        for (i, (k, c)) in k.iter().zip(current).enumerate() {
            program.linear[layout.weights + i] -= 2.0 * k * c;
        }
    }
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
    if let Some(te) = &c.tracking_error {
        let mut rows = vec![layout.row(Vec::new(), te.limit)];
        rows.extend(risk_rows(prepared, layout, Some(&te.benchmark)));
        program.second_order_cone(rows);
    }
    if let Some(y) = layout.factors {
        for (&(k, shift), limit) in prepared.exposure_limits.iter().zip(&c.factor_exposures) {
            program.at_most(layout.row(vec![(y + k, 1.0)], limit.max + shift));
            program.at_most(layout.row(vec![(y + k, -1.0)], -(limit.min + shift)));
        }
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
    method: SolveMethod,
    iterations: u32,
    started: Instant,
) -> Solution {
    let problem = prepared.problem;
    let mu = problem.expected_returns.as_deref();
    let risk = prepared.risk.as_ref();
    let expected_return = weights
        .as_ref()
        .zip(mu)
        .map(|(w, mu)| w.iter().zip(mu).map(|(a, b)| a * b).sum::<f64>());
    let variance = weights.as_ref().zip(risk).map(|(w, r)| r.variance(w));
    let volatility = variance.map(f64::sqrt);
    let objective_value = weights.as_ref().and_then(|w| match objective {
        Objective::MinimumVariance | Objective::TargetReturn { .. } => variance,
        Objective::MeanVariance { risk_aversion } => {
            Some(expected_return? - risk_aversion / 2.0 * variance?)
        }
        Objective::RobustMeanVariance {
            risk_aversion,
            uncertainty,
        } => Some(
            expected_return?
                - worst_case_shortfall(prepared, uncertainty, w)?
                - risk_aversion / 2.0 * variance?,
        ),
        Objective::TargetVolatility { .. } => expected_return,
        Objective::MaximumSharpe { risk_free_rate } => {
            Some((expected_return? - risk_free_rate) / volatility?)
        }
        Objective::MaximumDiversification => {
            let volatilities = risk?.volatilities();
            let weighted: f64 = w.iter().zip(volatilities).map(|(w, v)| w * v).sum();
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
            risk_budget::largest_gap(&risk?.dense(), w, budgets.as_deref())
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
    let layout = layout(&mut program, prepared, false);
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
