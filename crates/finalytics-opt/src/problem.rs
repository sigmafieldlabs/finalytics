//! Optimisation problems: assets, their risk and return estimates, constraints and objectives.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};
use finalytics_risk::{Matrix, ReturnsTable};

use crate::linalg::Factor;

/// Lower and upper limits on a weight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bound {
    /// The smallest weight allowed; negative for a short position.
    pub min: f64,
    /// The largest weight allowed.
    pub max: f64,
}

/// Limits on one asset's weight, in place of the default bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetBound {
    /// The asset's label.
    pub asset: String,
    /// The smallest weight allowed.
    pub min: f64,
    /// The largest weight allowed.
    pub max: f64,
}

/// Limits on the total weight of a group of assets, such as an asset class or a region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupLimit {
    /// The group's name, used in messages.
    pub name: String,
    /// The labels of the assets in the group.
    pub assets: Vec<String>,
    /// The smallest total weight allowed.
    pub min: f64,
    /// The largest total weight allowed.
    pub max: f64,
}

/// Trading from the current portfolio: a limit on turnover, costs per unit traded, or both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turnover {
    /// The current weights, one per asset.
    pub current: Vec<f64>,
    /// The largest total of absolute weight changes, `sum |w - current|`; 0.2 allows 20% of the
    /// portfolio to be bought and sold in total.
    #[serde(default)]
    pub limit: Option<f64>,
    /// Costs per unit of weight traded, one per asset, in the units of the expected returns.
    /// They reduce the expected return of objectives that use one.
    #[serde(default)]
    pub costs: Option<Vec<f64>>,
}

/// A limit on ex-ante tracking error against a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackingErrorLimit {
    /// The benchmark's weights, one per asset.
    pub benchmark: Vec<f64>,
    /// The largest tracking error allowed, `sqrt((w - b)' C (w - b))`, in the units of the
    /// covariance matrix's volatility.
    pub limit: f64,
}

/// The constraints on a portfolio. The default is fully invested and long-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Constraints {
    /// The sum of the weights; 1 by default.
    pub budget: f64,
    /// Bounds on every weight; `[0, 1]` by default.
    pub bounds: Bound,
    /// Bounds on particular assets, in place of `bounds`.
    pub asset_bounds: Vec<AssetBound>,
    /// Limits on groups of assets.
    pub groups: Vec<GroupLimit>,
    /// The largest gross exposure, `sum |w|`, for portfolios with short positions.
    pub leverage: Option<f64>,
    /// A turnover limit and trading costs.
    pub turnover: Option<Turnover>,
    /// A tracking-error limit.
    pub tracking_error: Option<TrackingErrorLimit>,
}

impl Default for Constraints {
    fn default() -> Self {
        Constraints {
            budget: 1.0,
            bounds: Bound { min: 0.0, max: 1.0 },
            asset_bounds: Vec::new(),
            groups: Vec::new(),
            leverage: None,
            turnover: None,
            tracking_error: None,
        }
    }
}

impl Constraints {
    /// Whether these are the default constraints, or differ only in bounds that the default
    /// constraints imply anyway.
    pub(crate) fn only_fully_invested_long_only(&self) -> bool {
        self.budget == 1.0
            && self.bounds.min <= 0.0
            && self.bounds.max >= 1.0
            && self.asset_bounds.is_empty()
            && self.groups.is_empty()
            && self.leverage.is_none()
            && self.turnover.is_none()
            && self.tracking_error.is_none()
    }
}

/// How clusters are joined in hierarchical risk parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Linkage {
    /// The distance between the closest members (the method of López de Prado, 2016).
    #[default]
    Single,
    /// The distance between the furthest members.
    Complete,
    /// The mean distance between members.
    Average,
    /// The increase in within-cluster variance (Ward's method).
    Ward,
}

/// What an optimisation seeks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "objective", rename_all = "snake_case")]
pub enum Objective {
    /// The smallest variance.
    MinimumVariance,
    /// The largest `expected return - risk_aversion / 2 * variance`.
    MeanVariance {
        /// The weight on variance, greater than zero.
        risk_aversion: f64,
    },
    /// The smallest variance with an expected return of at least `target`.
    TargetReturn {
        /// The expected return required.
        target: f64,
    },
    /// The largest expected return with a volatility of at most `target`.
    TargetVolatility {
        /// The volatility allowed, greater than zero.
        target: f64,
    },
    /// The largest Sharpe ratio, `(expected return - risk_free_rate) / volatility`.
    MaximumSharpe {
        /// The risk-free rate, in the units of the expected returns.
        risk_free_rate: f64,
    },
    /// The largest diversification ratio: the weighted average of the assets' volatilities over
    /// the portfolio's volatility (Choueifaty and Coignard, 2008).
    MaximumDiversification,
    /// The smallest conditional value at risk (expected shortfall) of the scenario returns, by
    /// the linear programme of Rockafellar and Uryasev (2000).
    MinimumCvar {
        /// The confidence level, for example 0.95.
        confidence: f64,
    },
    /// Weights at which each asset's share of the portfolio's volatility equals its budget
    /// (Spinu, 2013). Without budgets every asset contributes equally (risk parity). Only the
    /// default constraints apply.
    RiskBudgeting {
        /// The risk budgets, one per asset, each greater than zero; they are scaled to sum to one.
        #[serde(default)]
        budgets: Option<Vec<f64>>,
    },
    /// Hierarchical risk parity (López de Prado, 2016): assets are ordered by hierarchical
    /// clustering of their correlations and weighted by recursive bisection. Uses no solver;
    /// only the default constraints apply.
    HierarchicalRiskParity {
        /// How clusters are joined.
        #[serde(default)]
        linkage: Linkage,
    },
}

impl Objective {
    fn needs_expected_returns(&self) -> bool {
        matches!(
            self,
            Objective::MeanVariance { .. }
                | Objective::TargetReturn { .. }
                | Objective::TargetVolatility { .. }
                | Objective::MaximumSharpe { .. }
        )
    }

    fn needs_covariance(&self) -> bool {
        !matches!(self, Objective::MinimumCvar { .. })
    }
}

/// A portfolio optimisation problem: the assets, estimates of their risk and return, and the
/// constraints.
///
/// The covariance matrix and expected returns should be in the same units, for example both
/// annualised. Scenario returns, for minimum CVaR, are per period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    /// The asset labels.
    pub labels: Vec<String>,
    /// The covariance matrix of the assets' returns.
    #[serde(default)]
    pub covariance: Option<Matrix>,
    /// The expected return of each asset.
    #[serde(default)]
    pub expected_returns: Option<Vec<f64>>,
    /// Scenario returns, one column per asset, for minimum CVaR.
    #[serde(default)]
    pub scenarios: Option<ReturnsTable>,
    /// The constraints.
    #[serde(default)]
    pub constraints: Constraints,
}

impl Problem {
    /// A problem over the assets of a covariance matrix, with the default constraints.
    pub fn new(covariance: Matrix) -> Problem {
        Problem {
            labels: covariance.labels().to_vec(),
            covariance: Some(covariance),
            expected_returns: None,
            scenarios: None,
            constraints: Constraints::default(),
        }
    }

    /// A problem over the assets of a table of scenario returns, for minimum CVaR.
    pub fn from_scenarios(scenarios: ReturnsTable) -> Problem {
        Problem {
            labels: scenarios.labels().to_vec(),
            covariance: None,
            expected_returns: None,
            scenarios: Some(scenarios),
            constraints: Constraints::default(),
        }
    }

    /// Sets the expected returns, one per asset.
    pub fn with_expected_returns(mut self, expected_returns: Vec<f64>) -> Problem {
        self.expected_returns = Some(expected_returns);
        self
    }

    /// Sets the covariance matrix.
    pub fn with_covariance(mut self, covariance: Matrix) -> Problem {
        self.covariance = Some(covariance);
        self
    }

    /// Sets the scenario returns.
    pub fn with_scenarios(mut self, scenarios: ReturnsTable) -> Problem {
        self.scenarios = Some(scenarios);
        self
    }

    /// Sets the constraints.
    pub fn with_constraints(mut self, constraints: Constraints) -> Problem {
        self.constraints = constraints;
        self
    }

    /// Checks the problem for `objective` and resolves labels to positions.
    pub(crate) fn prepare(&self, objective: &Objective) -> Result<Prepared<'_>> {
        let mut report = ValidationReport::new();
        let n = self.labels.len();
        if n == 0 {
            report.error("labels", "needs at least one asset");
        }
        let mut index = HashMap::with_capacity(n);
        for (i, label) in self.labels.iter().enumerate() {
            if index.insert(label.as_str(), i).is_some() {
                report.error(format!("labels[{i}]"), format!("duplicate asset {label}"));
            }
        }

        if let Some(covariance) = &self.covariance {
            if covariance.labels() != self.labels.as_slice() {
                report.error(
                    "covariance.labels",
                    "must be the problem's labels, in order",
                );
            } else {
                check_symmetric(covariance, &mut report);
            }
        } else if objective.needs_covariance() || self.constraints.tracking_error.is_some() {
            report.error(
                "covariance",
                "is needed for this objective and these constraints",
            );
        }
        match &self.expected_returns {
            Some(mu) => check_vector(mu, n, "expected_returns", &mut report),
            None if objective.needs_expected_returns() => {
                report.error("expected_returns", "are needed for this objective");
            }
            None => {}
        }
        match &self.scenarios {
            Some(s) if s.labels() != self.labels.as_slice() => {
                report.error("scenarios.labels", "must be the problem's labels, in order");
            }
            None if matches!(objective, Objective::MinimumCvar { .. }) => {
                report.error("scenarios", "are needed for minimum CVaR");
            }
            _ => {}
        }

        let bounds = self.check_constraints(&index, objective, &mut report);
        check_objective(objective, n, &mut report);
        if report.has_errors() {
            return Err(Error::Validation(report));
        }

        let needs_factor = self.constraints.tracking_error.is_some()
            || matches!(objective, Objective::TargetVolatility { .. });
        let factor = match (&self.covariance, needs_factor) {
            (Some(c), true) => Some(Factor::of(c)?),
            (Some(c), false) => {
                // Checks that the matrix is positive semidefinite.
                Factor::of(c)?;
                None
            }
            (None, _) => None,
        };
        let groups = self
            .constraints
            .groups
            .iter()
            .map(|g| g.assets.iter().map(|a| index[a.as_str()]).collect())
            .collect();
        Ok(Prepared {
            problem: self,
            n,
            bounds,
            groups,
            factor,
        })
    }

    fn check_constraints(
        &self,
        index: &HashMap<&str, usize>,
        objective: &Objective,
        report: &mut ValidationReport,
    ) -> Vec<Bound> {
        let n = self.labels.len();
        let c = &self.constraints;
        if !c.budget.is_finite() {
            report.error("constraints.budget", "must be a finite number");
        }
        check_bound(c.bounds.min, c.bounds.max, "constraints.bounds", report);
        let mut bounds = vec![c.bounds; n];
        for (k, b) in c.asset_bounds.iter().enumerate() {
            let path = format!("constraints.asset_bounds[{k}]");
            check_bound(b.min, b.max, &path, report);
            match index.get(b.asset.as_str()) {
                Some(&i) => {
                    bounds[i] = Bound {
                        min: b.min,
                        max: b.max,
                    }
                }
                None => report.error(
                    format!("{path}.asset"),
                    format!("unknown asset {}", b.asset),
                ),
            }
        }
        for (k, g) in c.groups.iter().enumerate() {
            let path = format!("constraints.groups[{k}]");
            check_bound(g.min, g.max, &path, report);
            let mut seen = BTreeSet::new();
            for (j, asset) in g.assets.iter().enumerate() {
                if !index.contains_key(asset.as_str()) {
                    report.error(
                        format!("{path}.assets[{j}]"),
                        format!("unknown asset {asset}"),
                    );
                } else if !seen.insert(asset) {
                    report.error(
                        format!("{path}.assets[{j}]"),
                        format!("{asset} is listed twice"),
                    );
                }
            }
        }
        if let Some(l) = c.leverage
            && !(l.is_finite() && l > 0.0)
        {
            report.error("constraints.leverage", "must be greater than zero");
        }
        if let Some(t) = &c.turnover {
            check_vector(&t.current, n, "constraints.turnover.current", report);
            if let Some(limit) = t.limit
                && !(limit.is_finite() && limit >= 0.0)
            {
                report.error("constraints.turnover.limit", "must not be negative");
            }
            if let Some(costs) = &t.costs {
                check_vector(costs, n, "constraints.turnover.costs", report);
                if costs.iter().any(|x| *x < 0.0) {
                    report.error("constraints.turnover.costs", "must not be negative");
                }
                if !objective.needs_expected_returns() {
                    report.error(
                        "constraints.turnover.costs",
                        "apply only to objectives with expected returns",
                    );
                }
            }
        }
        if let Some(te) = &c.tracking_error {
            check_vector(
                &te.benchmark,
                n,
                "constraints.tracking_error.benchmark",
                report,
            );
            if !(te.limit.is_finite() && te.limit > 0.0) {
                report.error(
                    "constraints.tracking_error.limit",
                    "must be greater than zero",
                );
            }
        }
        let fixed_form = matches!(
            objective,
            Objective::RiskBudgeting { .. } | Objective::HierarchicalRiskParity { .. }
        );
        if fixed_form && !c.only_fully_invested_long_only() {
            report.error(
                "constraints",
                "risk budgeting and hierarchical risk parity give a fully invested, long-only \
                 portfolio and take no other constraints",
            );
        }
        bounds
    }
}

/// A validated problem, with labels resolved to positions.
pub(crate) struct Prepared<'a> {
    pub(crate) problem: &'a Problem,
    pub(crate) n: usize,
    /// The bounds on each asset.
    pub(crate) bounds: Vec<Bound>,
    /// The members of each group, by position.
    pub(crate) groups: Vec<Vec<usize>>,
    /// A factor `R` of the covariance matrix with `R'R = C`, when a second-order cone needs one.
    pub(crate) factor: Option<Factor>,
}

impl Prepared<'_> {
    pub(crate) fn covariance(&self) -> &Matrix {
        // `prepare` checks that the objective's covariance matrix is present.
        self.problem
            .covariance
            .as_ref()
            .expect("a validated problem has its covariance matrix")
    }

    pub(crate) fn expected_returns(&self) -> &[f64] {
        self.problem.expected_returns.as_deref().unwrap_or(&[])
    }

    pub(crate) fn constraints(&self) -> &Constraints {
        &self.problem.constraints
    }

    pub(crate) fn costs(&self) -> Option<&[f64]> {
        self.constraints().turnover.as_ref()?.costs.as_deref()
    }

    /// The largest amount by which `weights` break a constraint, 0 if they satisfy all of them.
    pub(crate) fn violation(&self, weights: &[f64]) -> f64 {
        let c = self.constraints();
        let mut worst: f64 = (weights.iter().sum::<f64>() - c.budget).abs();
        for (w, b) in weights.iter().zip(&self.bounds) {
            worst = worst.max(b.min - w).max(w - b.max);
        }
        for (members, g) in self.groups.iter().zip(&c.groups) {
            let total: f64 = members.iter().map(|&i| weights[i]).sum();
            worst = worst.max(g.min - total).max(total - g.max);
        }
        if let Some(l) = c.leverage {
            worst = worst.max(weights.iter().map(|w| w.abs()).sum::<f64>() - l);
        }
        if let Some(Turnover {
            current,
            limit: Some(limit),
            ..
        }) = &c.turnover
        {
            let traded: f64 = weights
                .iter()
                .zip(current)
                .map(|(w, c)| (w - c).abs())
                .sum();
            worst = worst.max(traded - limit);
        }
        if let Some(te) = &c.tracking_error {
            let active: Vec<f64> = weights
                .iter()
                .zip(&te.benchmark)
                .map(|(w, b)| w - b)
                .collect();
            let variance = self
                .covariance()
                .quadratic_form(&active)
                .unwrap_or(f64::NAN);
            worst = worst.max(variance.max(0.0).sqrt() - te.limit);
        }
        worst.max(0.0)
    }
}

fn check_bound(min: f64, max: f64, path: &str, report: &mut ValidationReport) {
    if !(min.is_finite() && max.is_finite()) {
        report.error(path, "the limits must be finite numbers");
    } else if min > max {
        report.error(path, format!("min {min} is above max {max}"));
    }
}

fn check_vector(values: &[f64], n: usize, path: &str, report: &mut ValidationReport) {
    if values.len() != n {
        report.error(path, format!("has {} entries for {n} assets", values.len()));
    }
    for (k, v) in values.iter().enumerate() {
        if !v.is_finite() {
            report.error(format!("{path}[{k}]"), "must be a finite number");
        }
    }
}

fn check_symmetric(covariance: &Matrix, report: &mut ValidationReport) {
    let n = covariance.dimension();
    let scale = covariance
        .values()
        .iter()
        .fold(0.0f64, |m, v| m.max(v.abs()));
    for i in 0..n {
        for j in i + 1..n {
            if (covariance.get(i, j) - covariance.get(j, i)).abs() > 1e-12 * scale {
                report.error(
                    "covariance",
                    format!("is not symmetric: entries ({i}, {j}) and ({j}, {i}) differ"),
                );
                return;
            }
        }
    }
}

fn check_objective(objective: &Objective, n: usize, report: &mut ValidationReport) {
    let positive = |value: f64, path: &str, report: &mut ValidationReport| {
        if !(value.is_finite() && value > 0.0) {
            report.error(path, "must be greater than zero");
        }
    };
    match objective {
        Objective::MeanVariance { risk_aversion } => {
            positive(*risk_aversion, "objective.risk_aversion", report);
        }
        Objective::TargetReturn { target } => {
            if !target.is_finite() {
                report.error("objective.target", "must be a finite number");
            }
        }
        Objective::TargetVolatility { target } => positive(*target, "objective.target", report),
        Objective::MaximumSharpe { risk_free_rate } => {
            if !risk_free_rate.is_finite() {
                report.error("objective.risk_free_rate", "must be a finite number");
            }
        }
        Objective::MinimumCvar { confidence } => {
            if !(*confidence > 0.0 && *confidence < 1.0) {
                report.error(
                    "objective.confidence",
                    "must be between 0 and 1, for example 0.95",
                );
            }
        }
        Objective::RiskBudgeting {
            budgets: Some(budgets),
        } => {
            check_vector(budgets, n, "objective.budgets", report);
            if budgets.iter().any(|b| b.is_nan() || *b <= 0.0) {
                report.error("objective.budgets", "must all be greater than zero");
            }
        }
        _ => {}
    }
}
