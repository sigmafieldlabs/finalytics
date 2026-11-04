//! Optimisation problems: assets, their risk and return estimates, constraints and objectives.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use finalytics_core::{Error, Result, ValidationReport};
use finalytics_risk::{FactorModel, Matrix, ReturnsTable};

use crate::linalg::Factor;

/// Lower and upper limits on a weight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bound {
    /// The smallest weight allowed; negative for a short position.
    pub min: f64,
    /// The largest weight allowed.
    pub max: f64,
}

/// Limits on one asset's weight, in place of the default bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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

/// Trading from the current portfolio: a limit on turnover, costs of trading, or both.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Turnover {
    /// The current weights, one per asset.
    pub current: Vec<f64>,
    /// The largest one-way turnover, half the absolute weight changes, `sum |w - current| / 2`:
    /// 0.2 allows 20% of the portfolio to be bought (and, equally, sold).
    #[serde(default)]
    pub limit: Option<f64>,
    /// Costs per unit of weight traded, one per asset, in the units of the expected returns.
    /// They reduce the expected return of objectives that use one.
    #[serde(default)]
    pub costs: Option<Vec<f64>>,
    /// Costs that grow with the square of the weight traded, one per asset, such as the market
    /// impact of large trades: `sum k_i (w_i - current_i)^2`, in the units of the expected
    /// returns. They reduce the expected return of mean–variance, robust mean–variance and
    /// target volatility objectives.
    #[serde(default)]
    pub quadratic_costs: Option<Vec<f64>>,
}

impl Turnover {
    /// Trading from the `current` weights, one per asset, with no limit and no costs yet.
    pub fn new(current: Vec<f64>) -> Self {
        Turnover {
            current,
            limit: None,
            costs: None,
            quadratic_costs: None,
        }
    }
}

/// Limits on the trades from the current portfolio (the turnover's current weights). They are
/// not convex, so the solver's portfolio is adjusted by a heuristic, reported as the method
/// `trade_elimination`, which is not guaranteed to find the best portfolio within the limits.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TradeLimits {
    /// The smallest trade allowed, as a weight: each asset is either not traded or traded by at
    /// least this much.
    #[serde(default)]
    pub minimum: Option<f64>,
    /// The largest number of assets traded.
    #[serde(default)]
    pub maximum_count: Option<usize>,
}

/// A limit on ex-ante tracking error against a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TrackingErrorLimit {
    /// The benchmark's weights, one per asset.
    pub benchmark: Vec<f64>,
    /// The largest tracking error allowed, `sqrt((w - b)' C (w - b))`, in the units of the
    /// covariance matrix's volatility.
    pub limit: f64,
}

/// Limits on a portfolio's exposure to one factor of the problem's factor model, such as a beta
/// between 0.9 and 1.1 to the market factor or a neutral exposure to an industry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FactorExposureLimit {
    /// The factor's name, as in the factor model.
    pub factor: String,
    /// The smallest exposure allowed.
    pub min: f64,
    /// The largest exposure allowed.
    pub max: f64,
    /// Benchmark weights, one per asset. When given, the limits apply to the active exposure,
    /// that of `weights - benchmark`.
    #[serde(default)]
    pub benchmark: Option<Vec<f64>>,
}

/// The constraints on a portfolio. The default is fully invested and long-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
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
    /// Limits on factor exposures; they need a factor model.
    pub factor_exposures: Vec<FactorExposureLimit>,
    /// Limits on the size and number of trades; they need the turnover's current weights.
    pub trades: Option<TradeLimits>,
    /// The smallest expected return, net of trading costs, in the units of the expected
    /// returns: a floor for the objectives that do not use expected returns, such as minimum
    /// CVaR.
    pub min_return: Option<f64>,
    /// The smallest effective number of assets, `1 / sum w_i^2`, from 1 to the number of
    /// assets: the constraint `sum w_i^2 <= 1 / k` of DeMiguel, Garlappi, Nogales and Uppal
    /// (2009), which keeps the weights from concentrating in a few assets.
    pub min_effective_assets: Option<f64>,
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
            factor_exposures: Vec::new(),
            trades: None,
            min_return: None,
            min_effective_assets: None,
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
            && self.factor_exposures.is_empty()
            && self.trades.is_none()
            && self.min_return.is_none()
            && self.min_effective_assets.is_none()
    }
}

/// Where the true expected returns may lie around the estimates, for robust mean–variance
/// optimisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReturnUncertainty {
    /// An ellipsoid around the estimates, `(mu - m)' S^-1 (mu - m) <= size^2`: the worst case
    /// lowers a portfolio's expected return by `size * sqrt(w' S w)`. For estimates that are
    /// sample means of `T` periods, `S` is the covariance divided by `T`, and `size^2` a
    /// chi-squared quantile with as many degrees of freedom as assets.
    Ellipsoid {
        /// The size of the ellipsoid, not negative.
        size: f64,
        /// The covariance `S` of the estimation error, labelled like the problem; without one,
        /// the problem's covariance matrix or factor model.
        #[serde(default)]
        covariance: Option<Matrix>,
    },
    /// A box around the estimates, `|mu_i - m_i| <= half_widths_i`: the worst case lowers a
    /// portfolio's expected return by `sum half_widths_i |w_i|`.
    Box {
        /// The half-width for each asset, not negative.
        half_widths: Vec<f64>,
    },
}

/// How clusters are joined in hierarchical risk parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Objective {
    /// The smallest variance.
    MinimumVariance,
    /// The largest `expected return - risk_aversion / 2 * variance`.
    MeanVariance {
        /// The weight on variance, greater than zero.
        risk_aversion: f64,
    },
    /// The largest worst-case `expected return - risk_aversion / 2 * variance` over a set of
    /// possible expected returns around the estimates (robust optimisation), which guards against
    /// error in the estimates of expected return.
    RobustMeanVariance {
        /// The weight on variance, greater than zero.
        risk_aversion: f64,
        /// The set of possible expected returns.
        uncertainty: ReturnUncertainty,
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
    /// The smallest semivariance of the scenario returns below zero, `mean(min(r, 0)^2)` over
    /// every scenario: the square of the downside deviation.
    MinimumSemivariance,
    /// The smallest conditional drawdown at risk of the scenario returns (Chekhlov, Uryasev and
    /// Zabarankin, 2005): the Rockafellar–Uryasev tail mean of the drawdowns after each
    /// scenario, measured on the cumulative sum of the returns from a start of zero, as a
    /// linear programme.
    MinimumCdar {
        /// The confidence level, for example 0.95.
        confidence: f64,
    },
    /// Every asset with the same weight, `1 / n`: the naive portfolio that estimated ones must
    /// beat out of sample (DeMiguel, Garlappi and Uppal, 2009). Uses no estimates and no solver;
    /// only the default constraints apply.
    EqualWeight,
    /// Weights in proportion to the inverse of each asset's volatility, scaled to sum to one:
    /// risk parity when the assets are uncorrelated. Uses only the variances and no solver; only
    /// the default constraints apply.
    InverseVolatility,
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
                | Objective::RobustMeanVariance { .. }
                | Objective::TargetReturn { .. }
                | Objective::TargetVolatility { .. }
                | Objective::MaximumSharpe { .. }
        )
    }

    fn needs_covariance(&self) -> bool {
        !(self.uses_scenarios() || matches!(self, Objective::EqualWeight))
    }

    /// Whether the objective is measured on the scenario returns: minimum CVaR, semivariance
    /// and CDaR.
    pub(crate) fn uses_scenarios(&self) -> bool {
        matches!(
            self,
            Objective::MinimumCvar { .. }
                | Objective::MinimumSemivariance
                | Objective::MinimumCdar { .. }
        )
    }
}

/// A portfolio optimisation problem: the assets, estimates of their risk and return, and the
/// constraints.
///
/// Risk comes from a covariance matrix or from a factor model, which the solver uses in factor
/// form, so that large problems solve faster. The risk estimates and expected returns should be
/// in the same units, for example both annualised. Scenario returns, for minimum CVaR, are per
/// period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Problem {
    /// The asset labels.
    pub labels: Vec<String>,
    /// The covariance matrix of the assets' returns.
    #[serde(default)]
    pub covariance: Option<Matrix>,
    /// A factor model of the assets' returns, in place of a covariance matrix. Its assets must
    /// be the problem's labels, in order.
    #[serde(default)]
    pub factor_model: Option<FactorModel>,
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
    pub fn from_covariance(covariance: Matrix) -> Problem {
        Problem {
            labels: covariance.labels().to_vec(),
            covariance: Some(covariance),
            factor_model: None,
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
            factor_model: None,
            expected_returns: None,
            scenarios: Some(scenarios),
            constraints: Constraints::default(),
        }
    }

    /// A problem over the assets of a factor model, with the default constraints.
    pub fn from_factor_model(model: FactorModel) -> Problem {
        Problem {
            labels: model.assets().to_vec(),
            covariance: None,
            factor_model: Some(model),
            expected_returns: None,
            scenarios: None,
            constraints: Constraints::default(),
        }
    }

    /// Sets the factor model.
    pub fn with_factor_model(mut self, model: FactorModel) -> Problem {
        self.factor_model = Some(model);
        self
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

        match (&self.covariance, &self.factor_model) {
            (Some(_), Some(_)) => report.error(
                "factor_model",
                "give a covariance matrix or a factor model, not both",
            ),
            (Some(covariance), None) => {
                if covariance.labels() != self.labels.as_slice() {
                    report.error(
                        "covariance.labels",
                        "must be the problem's labels, in order",
                    );
                } else {
                    check_symmetric(covariance, &mut report);
                }
            }
            (None, Some(model)) => {
                if model.assets() != self.labels.as_slice() {
                    report.error(
                        "factor_model.assets",
                        "must be the problem's labels, in order",
                    );
                }
            }
            (None, None) => {
                if objective.needs_covariance() || self.constraints.tracking_error.is_some() {
                    report.error(
                        "covariance",
                        "a covariance matrix or a factor model is needed for this objective and \
                         these constraints",
                    );
                }
            }
        }
        match &self.expected_returns {
            Some(mu) => check_vector(mu, n, "expected_returns", &mut report),
            None if objective.needs_expected_returns() => {
                report.error("expected_returns", "are needed for this objective");
            }
            None if self.constraints.min_return.is_some() => {
                report.error("expected_returns", "are needed for constraints.min_return");
            }
            None => {}
        }
        match &self.scenarios {
            Some(s) if s.labels() != self.labels.as_slice() => {
                report.error("scenarios.labels", "must be the problem's labels, in order");
            }
            None if objective.uses_scenarios() => {
                report.error("scenarios", "are needed for this objective");
            }
            _ => {}
        }

        let bounds = self.check_constraints(&index, objective, &mut report);
        check_objective(objective, &self.labels, &mut report);
        if report.has_errors() {
            return Err(Error::Validation(report));
        }

        let needs_root = self.constraints.tracking_error.is_some()
            || matches!(
                objective,
                Objective::TargetVolatility { .. }
                    | Objective::RobustMeanVariance {
                        uncertainty: ReturnUncertainty::Ellipsoid {
                            covariance: None,
                            ..
                        },
                        ..
                    }
            );
        let uncertainty_root = match objective {
            Objective::RobustMeanVariance {
                uncertainty:
                    ReturnUncertainty::Ellipsoid {
                        covariance: Some(s),
                        ..
                    },
                ..
            } => Some(Factor::of(s).map_err(|_| {
                crate::invalid(
                    "objective.uncertainty.covariance",
                    "is not positive semidefinite",
                )
            })?),
            _ => None,
        };
        let risk = match (&self.covariance, &self.factor_model) {
            (Some(matrix), _) => {
                // Factorising also checks that the matrix is positive semidefinite.
                let root = Factor::of(matrix)?;
                Some(RiskModel::Covariance {
                    matrix,
                    root: needs_root.then_some(root),
                })
            }
            (None, Some(model)) => Some(RiskModel::Factors {
                model,
                root: Factor::of(model.factor_covariance())?,
            }),
            (None, None) => None,
        };
        let groups = self
            .constraints
            .groups
            .iter()
            .map(|g| g.assets.iter().map(|a| index[a.as_str()]).collect())
            .collect();
        let exposure_limits = match &self.factor_model {
            Some(model) => self
                .constraints
                .factor_exposures
                .iter()
                .filter_map(|limit| {
                    let k = model.factors().iter().position(|f| *f == limit.factor)?;
                    let shift = limit.benchmark.as_ref().map(|b| {
                        model
                            .portfolio_exposures(b)
                            .map_or(0.0, |x| x.get(k).copied().unwrap_or(0.0))
                    });
                    Some((k, shift.unwrap_or(0.0)))
                })
                .collect(),
            None => Vec::new(),
        };
        Ok(Prepared {
            problem: self,
            n,
            bounds,
            groups,
            risk,
            exposure_limits,
            uncertainty_root,
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
            if let Some(costs) = &t.quadratic_costs {
                check_vector(costs, n, "constraints.turnover.quadratic_costs", report);
                if costs.iter().any(|x| *x < 0.0) {
                    report.error(
                        "constraints.turnover.quadratic_costs",
                        "must not be negative",
                    );
                }
                if !matches!(
                    objective,
                    Objective::MeanVariance { .. }
                        | Objective::RobustMeanVariance { .. }
                        | Objective::TargetVolatility { .. }
                ) {
                    report.error(
                        "constraints.turnover.quadratic_costs",
                        "apply only to mean–variance, robust mean–variance and target volatility",
                    );
                }
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
        if let Some(limits) = &c.trades {
            if c.turnover.is_none() {
                report.error(
                    "constraints.trades",
                    "need the current weights, given as constraints.turnover.current",
                );
            }
            if let Some(m) = limits.minimum
                && !(m.is_finite() && m >= 0.0)
            {
                report.error(
                    "constraints.trades.minimum",
                    "must be a finite number, not negative",
                );
            }
            if objective.uses_scenarios() {
                report.error(
                    "constraints.trades",
                    "are not supported for the objectives of scenario returns, whose optimum \
                     need not be unique",
                );
            }
        }
        for (k, limit) in c.factor_exposures.iter().enumerate() {
            let path = format!("constraints.factor_exposures[{k}]");
            check_bound(limit.min, limit.max, &path, report);
            match &self.factor_model {
                Some(model) => {
                    if !model.factors().contains(&limit.factor) {
                        report.error(
                            format!("{path}.factor"),
                            format!("the factor model has no factor {}", limit.factor),
                        );
                    }
                }
                None => report.error(&path, "factor exposure limits need a factor model"),
            }
            if let Some(b) = &limit.benchmark {
                check_vector(b, n, &format!("{path}.benchmark"), report);
            }
        }
        if let Some(r) = c.min_return
            && !r.is_finite()
        {
            report.error("constraints.min_return", "must be a finite number");
        }
        if let Some(k) = c.min_effective_assets
            && !(k.is_finite() && k >= 1.0 && k <= n as f64)
        {
            report.error(
                "constraints.min_effective_assets",
                format!("must be from 1 to the number of assets, {n}"),
            );
        }
        let fixed_form = matches!(
            objective,
            Objective::EqualWeight
                | Objective::InverseVolatility
                | Objective::RiskBudgeting { .. }
                | Objective::HierarchicalRiskParity { .. }
        );
        if fixed_form && !c.only_fully_invested_long_only() {
            report.error(
                "constraints",
                "equal weight, inverse volatility, risk budgeting and hierarchical risk parity \
                 give a fully invested, long-only portfolio and take no other constraints",
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
    /// The risk model, when the problem has one.
    pub(crate) risk: Option<RiskModel<'a>>,
    /// For each factor exposure limit, the factor's position and the benchmark's exposure to it
    /// (0 without a benchmark).
    pub(crate) exposure_limits: Vec<(usize, f64)>,
    /// A factor of the robust objective's own uncertainty covariance, when it has one.
    pub(crate) uncertainty_root: Option<Factor>,
}

/// Where a problem's risk comes from.
pub(crate) enum RiskModel<'a> {
    /// A covariance matrix `C`, with a factor `R` (`R'R = C`) when a second-order cone needs one.
    Covariance {
        matrix: &'a Matrix,
        root: Option<Factor>,
    },
    /// A factor model, with a factor `R` of its factor covariance (`R'R = F`).
    Factors {
        model: &'a FactorModel,
        root: Factor,
    },
}

impl RiskModel<'_> {
    /// The variance of a portfolio with weights `w`.
    pub(crate) fn variance(&self, w: &[f64]) -> f64 {
        let v = match self {
            RiskModel::Covariance { matrix, .. } => matrix.quadratic_form(w).ok(),
            RiskModel::Factors { model, .. } => model.variance(w).ok(),
        };
        v.unwrap_or(f64::NAN).max(0.0)
    }

    /// Each asset's volatility.
    pub(crate) fn volatilities(&self) -> Vec<f64> {
        match self {
            RiskModel::Covariance { matrix, .. } => {
                matrix.diagonal().iter().map(|v| v.sqrt()).collect()
            }
            RiskModel::Factors { model, .. } => {
                let f = model.factor_covariance();
                model
                    .exposures()
                    .iter()
                    .zip(model.specific_variances())
                    .map(|(b, d)| {
                        let fb = f.times(b).unwrap_or_default();
                        let v: f64 = b.iter().zip(&fb).map(|(x, y)| x * y).sum();
                        (v + d).max(0.0).sqrt()
                    })
                    .collect()
            }
        }
    }

    /// The covariance matrix, built from the factor model if need be.
    pub(crate) fn dense(&self) -> Cow<'_, Matrix> {
        match self {
            RiskModel::Covariance { matrix, .. } => Cow::Borrowed(*matrix),
            RiskModel::Factors { model, .. } => Cow::Owned(model.covariance()),
        }
    }
}

impl Prepared<'_> {
    /// The risk model. `prepare` checks that an objective or constraint that needs one has it.
    pub(crate) fn risk(&self) -> &RiskModel<'_> {
        self.risk
            .as_ref()
            .expect("a validated problem has a risk model where it needs one")
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

    pub(crate) fn quadratic_costs(&self) -> Option<(&[f64], &[f64])> {
        let t = self.constraints().turnover.as_ref()?;
        Some((t.quadratic_costs.as_deref()?, &t.current))
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
            // One-way turnover, as the limit is stated.
            let turnover: f64 = weights
                .iter()
                .zip(current)
                .map(|(w, c)| (w - c).abs())
                .sum::<f64>()
                / 2.0;
            worst = worst.max(turnover - limit);
        }
        if let (
            Some(TradeLimits {
                minimum: Some(m), ..
            }),
            Some(t),
        ) = (&c.trades, &c.turnover)
        {
            for (w, now) in weights.iter().zip(&t.current) {
                let traded = (w - now).abs();
                if traded > TRADED {
                    worst = worst.max(m - traded);
                }
            }
        }
        if let Some(te) = &c.tracking_error {
            let active: Vec<f64> = weights
                .iter()
                .zip(&te.benchmark)
                .map(|(w, b)| w - b)
                .collect();
            worst = worst.max(self.risk().variance(&active).sqrt() - te.limit);
        }
        if let Some(floor) = c.min_return {
            let expected: f64 = weights
                .iter()
                .zip(self.expected_returns())
                .map(|(w, m)| w * m)
                .sum();
            worst = worst.max(floor - (expected - crate::optimise::traded_costs(self, weights)));
        }
        if let Some(k) = c.min_effective_assets {
            worst = worst.max(weights.iter().map(|w| w * w).sum::<f64>() - 1.0 / k);
        }
        if let Some(RiskModel::Factors { model, .. }) = &self.risk {
            let x = model.portfolio_exposures(weights).unwrap_or_default();
            for (&(k, shift), limit) in self.exposure_limits.iter().zip(&c.factor_exposures) {
                let exposure = x.get(k).copied().unwrap_or(f64::NAN) - shift;
                worst = worst.max(limit.min - exposure).max(exposure - limit.max);
            }
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

/// A change in weight below this counts as no trade: well above the solver's tolerance, so that
/// an asset held at its current weight is not mistaken for a small trade.
pub(crate) const TRADED: f64 = 1e-7;

pub(crate) fn check_vector(values: &[f64], n: usize, path: &str, report: &mut ValidationReport) {
    if values.len() != n {
        report.error(path, format!("has {} entries for {n} assets", values.len()));
    }
    for (k, v) in values.iter().enumerate() {
        if !v.is_finite() {
            report.error(format!("{path}[{k}]"), "must be a finite number");
        }
    }
}

pub(crate) fn check_symmetric(covariance: &Matrix, report: &mut ValidationReport) {
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

fn check_objective(objective: &Objective, labels: &[String], report: &mut ValidationReport) {
    let n = labels.len();
    let positive = |value: f64, path: &str, report: &mut ValidationReport| {
        if !(value.is_finite() && value > 0.0) {
            report.error(path, "must be greater than zero");
        }
    };
    match objective {
        Objective::MeanVariance { risk_aversion } => {
            positive(*risk_aversion, "objective.risk_aversion", report);
        }
        Objective::RobustMeanVariance {
            risk_aversion,
            uncertainty,
        } => {
            positive(*risk_aversion, "objective.risk_aversion", report);
            match uncertainty {
                ReturnUncertainty::Ellipsoid { size, covariance } => {
                    if !(size.is_finite() && *size >= 0.0) {
                        report.error(
                            "objective.uncertainty.size",
                            "must be a finite number, not negative",
                        );
                    }
                    if let Some(s) = covariance {
                        if s.labels() != labels {
                            report.error(
                                "objective.uncertainty.covariance.labels",
                                "must be the problem's labels, in order",
                            );
                        } else {
                            check_symmetric(s, report);
                        }
                    }
                }
                ReturnUncertainty::Box { half_widths } => {
                    check_vector(half_widths, n, "objective.uncertainty.half_widths", report);
                    if half_widths.iter().any(|h| *h < 0.0) {
                        report.error("objective.uncertainty.half_widths", "must not be negative");
                    }
                }
            }
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
        Objective::MinimumCvar { confidence } | Objective::MinimumCdar { confidence } => {
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
