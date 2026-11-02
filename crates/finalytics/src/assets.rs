//! Assets: the prices or returns of things one might hold, and what can be learnt from them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Decimal, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, price_columns_from_csv};
use finalytics_opt::{
    AssetBound, Bound, Constraints, FrontierPoint, GroupLimit, Linkage, Objective, Problem,
    ReturnUncertainty, Solution, SolveStatus,
};
use finalytics_risk::{
    CovarianceEstimate, CovarianceMethod, Matrix, PriceTable, ReturnsTable, Statistics,
    StatisticsOptions, TailMethod, TailRisk, covariance, periods_per_year, statistics, tail_risk,
};

/// The prices or returns of several assets over the same periods, with the settings that turn
/// them into annual figures: the number of periods in a year (detected from the dates), the
/// annual risk-free rate (zero by default) and the covariance estimator (Ledoit–Wolf by
/// default).
///
/// ```
/// use finalytics::{Assets, datasets};
///
/// let assets = Assets::from_prices(&datasets::asset_class_prices())?
///     .with_risk_free_rate(0.02);
/// let result = assets
///     .optimiser()
///     .maximum_sharpe()
///     .bounds(0.0, 0.4)
///     .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
///     .solve()?;
/// let weights = result.weights.unwrap_or_default();
/// assert!(weights.values().all(|w| *w <= 0.4 + 1e-6));
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Assets {
    dates: Vec<Date>,
    returns: ReturnsTable,
    periods_per_year: f64,
    risk_free_rate: f64,
    risk_free_asset: Option<usize>,
    covariance_method: CovarianceMethod,
    warnings: ValidationReport,
}

/// One asset's statistics, with its value at risk and expected shortfall at 95% (historical).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct AssetStatistics {
    /// The asset.
    pub asset: String,
    /// Its statistics, annualised with the assets' periods per year and risk-free rate.
    pub statistics: Statistics,
    /// Its value at risk and expected shortfall over one period at 95%, from history.
    pub tail_risk: TailRisk,
}

/// What an optimisation seeks, by name. The parameters some objectives need are given beside
/// it, in [`OptimiseOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ObjectiveName {
    /// The smallest variance.
    MinimumVariance,
    /// The largest expected return less `risk_aversion / 2` times the variance.
    MeanVariance,
    /// Mean–variance against the worst expected returns within `uncertainty`.
    RobustMeanVariance,
    /// The smallest variance with an expected return of at least `target`.
    TargetReturn,
    /// The largest expected return with a volatility of at most `target`.
    TargetVolatility,
    /// The largest Sharpe ratio.
    #[default]
    MaximumSharpe,
    /// The largest diversification ratio.
    MaximumDiversification,
    /// The smallest conditional value at risk of the periods' returns, at `confidence`.
    MinimumCvar,
    /// Each asset's share of the risk equal to its budget (equal shares without `budgets`).
    RiskBudgeting,
    /// Hierarchical risk parity.
    HierarchicalRiskParity,
}

/// A group of assets whose total weight is limited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct AssetGroup {
    /// The assets in the group.
    pub assets: Vec<String>,
    /// The smallest total weight.
    pub min: f64,
    /// The largest total weight.
    pub max: f64,
}

/// What to optimise, and the limits on the weights. Everything is optional: by default the
/// portfolio with the largest Sharpe ratio, fully invested and long only.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct OptimiseOptions {
    /// What to optimise: the largest Sharpe ratio by default.
    pub objective: ObjectiveName,
    /// The annual risk-free rate for the Sharpe ratio; by default the assets' own.
    pub risk_free_rate: Option<f64>,
    /// The annual expected return (`target_return`) or volatility (`target_volatility`) to aim
    /// for.
    pub target: Option<f64>,
    /// The weight on variance, for mean–variance objectives.
    pub risk_aversion: Option<f64>,
    /// The uncertainty of the expected returns, for robust mean–variance.
    pub uncertainty: Option<ReturnUncertainty>,
    /// The confidence of the conditional value at risk: 0.95 by default.
    pub confidence: Option<f64>,
    /// The risk budgets, by asset, for risk budgeting; equal without them.
    pub budgets: Option<BTreeMap<String, f64>>,
    /// How hierarchical risk parity joins clusters: single linkage by default.
    pub linkage: Option<Linkage>,
    /// The smallest and largest weight of every asset: 0 and 1 by default.
    pub bounds: Option<Bound>,
    /// Bounds for particular assets, which take the place of `bounds` for them.
    pub asset_bounds: BTreeMap<String, Bound>,
    /// Limits on the total weight of groups of assets, by group name.
    pub groups: BTreeMap<String, AssetGroup>,
    /// Annual expected returns by asset, in place of the historical means.
    pub expected_returns: BTreeMap<String, f64>,
}

/// The result of an optimisation over assets: the weights by asset, with their expected return,
/// volatility and Sharpe ratio, and the solver's full solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct OptimisationResult {
    /// How the optimisation ended.
    pub status: SolveStatus,
    /// The weights by asset, absent when there is no solution.
    pub weights: Option<BTreeMap<String, f64>>,
    /// The annual expected return of the weights.
    pub expected_return: Option<f64>,
    /// The annual volatility of the weights.
    pub volatility: Option<f64>,
    /// `(expected_return - risk_free_rate) / volatility`.
    pub sharpe_ratio: Option<f64>,
    /// The annual risk-free rate used.
    pub risk_free_rate: f64,
    /// The solver's solution, with the objective, its value and the work done.
    pub solution: Solution,
}

impl Assets {
    /// Assets from their prices: the returns between consecutive dates on which every asset has
    /// a price. The number of periods in a year is detected from the dates.
    pub fn from_prices(prices: &PriceTable) -> Result<Assets> {
        let found = prices.returns()?;
        let periods = periods_per_year(prices.dates());
        Ok(Assets {
            dates: found.dates,
            returns: found.table,
            periods_per_year: periods,
            risk_free_rate: 0.0,
            risk_free_asset: None,
            covariance_method: CovarianceMethod::LedoitWolf,
            warnings: found.warnings,
        })
    }

    /// Assets from their returns: `dates` are the dates each return ends on, one per row of the
    /// table. The number of periods in a year is detected from the dates.
    pub fn from_returns(dates: Vec<Date>, returns: ReturnsTable) -> Result<Assets> {
        let mut report = ValidationReport::new();
        if dates.len() != returns.observations() {
            report.error(
                "dates",
                format!(
                    "{} dates for {} returns",
                    dates.len(),
                    returns.observations()
                ),
            );
        }
        for (i, pair) in dates.windows(2).enumerate() {
            if pair[1] <= pair[0] {
                report.error(
                    format!("dates[{}]", i + 1),
                    format!("{} does not follow {}", pair[1], pair[0]),
                );
            }
        }
        report.into_result(())?;
        let periods = periods_per_year(&dates);
        Ok(Assets {
            dates,
            returns,
            periods_per_year: periods,
            risk_free_rate: 0.0,
            risk_free_asset: None,
            covariance_method: CovarianceMethod::LedoitWolf,
            warnings: ValidationReport::new(),
        })
    }

    /// Assets from prices in CSV: a `date` column and one column of prices per asset, or the
    /// long layout of `date`, `instrument` and `price` columns. The assets keep the order of the
    /// file.
    pub fn from_csv(text: &str) -> Result<Assets> {
        let series = price_columns_from_csv(text, CsvOptions::default())?;
        let mut dates: Vec<Date> = series
            .iter()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .collect();
        dates.sort_unstable();
        dates.dedup();
        let labels: Vec<String> = series.iter().map(|(id, _)| id.to_string()).collect();
        let columns: Vec<Vec<Option<f64>>> = series
            .iter()
            .map(|(_, s)| {
                dates
                    .iter()
                    .map(|d| s.get(*d).map(|p| to_f64(*p)))
                    .collect()
            })
            .collect();
        Assets::from_prices(&PriceTable::new(dates, labels, columns)?)
    }

    /// The same assets with an annual risk-free rate, for Sharpe ratios.
    pub fn with_risk_free_rate(mut self, rate: f64) -> Assets {
        self.risk_free_rate = rate;
        self.risk_free_asset = None;
        self
    }

    /// The same assets with one of them, such as T-bills, as the risk-free asset: the
    /// risk-free rate is its mean return, annualised as expected returns are, and optimisation
    /// leaves it out, so that it finds the best portfolio of the risky assets. Its statistics,
    /// covariance and correlation are still given.
    pub fn with_risk_free_asset(mut self, asset: &str) -> Result<Assets> {
        let Some(i) = self.labels().iter().position(|l| l == asset) else {
            let mut report = ValidationReport::new();
            report.error("risk_free_asset", format!("unknown asset {asset}"));
            return Err(Error::Validation(report));
        };
        self.risk_free_rate = self.mean_returns()[i];
        self.risk_free_asset = Some(i);
        Ok(self)
    }

    /// The same assets with the number of periods in a year given rather than detected: 252 for
    /// daily returns, 52 for weekly, 12 for monthly.
    pub fn with_periods_per_year(mut self, periods: f64) -> Assets {
        self.periods_per_year = periods;
        self
    }

    /// The same assets with another covariance estimator.
    pub fn with_covariance(mut self, method: CovarianceMethod) -> Assets {
        self.covariance_method = method;
        self
    }

    /// The assets' labels.
    pub fn labels(&self) -> &[String] {
        self.returns.labels()
    }

    /// The date each return ends on.
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }

    /// The returns, one column per asset.
    pub fn returns(&self) -> &ReturnsTable {
        &self.returns
    }

    /// The number of periods in a year.
    pub fn periods_per_year(&self) -> f64 {
        self.periods_per_year
    }

    /// The annual risk-free rate.
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// The risk-free asset, if one of the assets is.
    pub fn risk_free_asset(&self) -> Option<&str> {
        self.risk_free_asset.map(|i| self.labels()[i].as_str())
    }

    /// The assets optimisation chooses among: all but the risk-free asset.
    pub fn risky_assets(&self) -> Vec<&str> {
        self.labels()
            .iter()
            .enumerate()
            .filter(|(i, _)| Some(*i) != self.risk_free_asset)
            .map(|(_, l)| l.as_str())
            .collect()
    }

    /// The returns of the risky assets.
    fn risky_returns(&self) -> Result<ReturnsTable> {
        let Some(skip) = self.risk_free_asset else {
            return Ok(self.returns.clone());
        };
        let keep = |i: &usize| *i != skip;
        ReturnsTable::new(
            (0..self.labels().len())
                .filter(keep)
                .map(|i| self.labels()[i].clone())
                .collect(),
            (0..self.labels().len())
                .filter(keep)
                .map(|i| self.returns.columns()[i].clone())
                .collect(),
        )
    }

    /// Problems met reading the prices that did not stop the assets being built, such as dates
    /// left out because an asset had no price.
    pub fn warnings(&self) -> &ValidationReport {
        &self.warnings
    }

    /// Each asset's statistics, in the order of the assets.
    pub fn statistics(&self) -> Result<Vec<AssetStatistics>> {
        let mut options = StatisticsOptions::default();
        options.periods_per_year = self.periods_per_year;
        options.risk_free_rate = self.risk_free_rate;
        self.labels()
            .iter()
            .zip(self.returns.columns())
            .map(|(label, column)| {
                Ok(AssetStatistics {
                    asset: label.clone(),
                    statistics: statistics(column, &options)?,
                    tail_risk: tail_risk(column, 0.95, TailMethod::Historical)?,
                })
            })
            .collect()
    }

    /// The covariance of the assets' returns, annualised, by the assets' estimator.
    pub fn covariance(&self) -> Result<CovarianceEstimate> {
        let mut estimate = covariance(&self.returns, self.covariance_method)?;
        estimate.matrix = estimate.matrix.scaled(self.periods_per_year);
        Ok(estimate)
    }

    /// The correlation of the assets' returns, from the covariance estimate.
    pub fn correlation(&self) -> Result<Matrix> {
        self.covariance()?.matrix.correlation()
    }

    /// Each asset's historical mean return, annualised.
    pub fn mean_returns(&self) -> Vec<f64> {
        self.returns
            .columns()
            .iter()
            .map(|c| c.iter().sum::<f64>() / c.len() as f64 * self.periods_per_year)
            .collect()
    }

    /// A builder for an optimisation over the risky assets.
    pub fn optimiser(&self) -> Optimiser<'_> {
        Optimiser {
            assets: self,
            options: OptimiseOptions::default(),
        }
    }

    /// Optimises a portfolio of the risky assets: annualised Ledoit–Wolf covariance (or the
    /// assets' estimator), historical mean returns unless others are given, and the objective
    /// and limits of `options`.
    pub fn optimise(&self, options: &OptimiseOptions) -> Result<OptimisationResult> {
        let rf = options.risk_free_rate.unwrap_or(self.risk_free_rate);
        let (problem, labels) = self.problem(options)?;
        let objective = objective(options, rf, &labels)?;
        let solution = problem.solve(&objective)?;
        let weights = solution.weights.as_ref().map(|w| {
            labels
                .iter()
                .cloned()
                .zip(w.iter().copied())
                .collect::<BTreeMap<String, f64>>()
        });
        let sharpe_ratio = match (solution.expected_return, solution.volatility) {
            (Some(r), Some(v)) if v > 0.0 => Some((r - rf) / v),
            _ => None,
        };
        Ok(OptimisationResult {
            status: solution.status,
            weights,
            expected_return: solution.expected_return,
            volatility: solution.volatility,
            sharpe_ratio,
            risk_free_rate: rf,
            solution,
        })
    }

    /// The efficient frontier: `points` portfolios from the one with the least variance to the
    /// one with the largest expected return, under the limits of `options` (its objective is not
    /// used). Each point's weights are in the order of the risky assets.
    pub fn frontier(&self, points: usize, options: &OptimiseOptions) -> Result<Vec<FrontierPoint>> {
        self.problem(options)?.0.frontier(points)
    }

    /// The problem `options` describe, over the risky assets, with their labels.
    fn problem(&self, options: &OptimiseOptions) -> Result<(Problem, Vec<String>)> {
        let returns = self.risky_returns()?;
        let labels = returns.labels().to_vec();
        let mut report = ValidationReport::new();
        let unknown = |asset: &str| match self.risk_free_asset() {
            Some(rf) if rf == asset => {
                format!("{asset} is the risk-free asset, which optimisation leaves out")
            }
            _ => format!("unknown asset {asset}"),
        };
        let mut expected: Vec<f64> = returns
            .columns()
            .iter()
            .map(|c| c.iter().sum::<f64>() / c.len() as f64 * self.periods_per_year)
            .collect();
        for (asset, r) in &options.expected_returns {
            match labels.iter().position(|l| l == asset) {
                Some(i) => expected[i] = *r,
                None => report.error(format!("options.expected_returns.{asset}"), unknown(asset)),
            }
        }
        let mut constraints = Constraints::default();
        if let Some(bounds) = options.bounds {
            constraints.bounds = bounds;
        }
        for (asset, bound) in &options.asset_bounds {
            if !labels.contains(asset) {
                report.error(format!("options.asset_bounds.{asset}"), unknown(asset));
            }
            constraints.asset_bounds.push(AssetBound {
                asset: asset.clone(),
                min: bound.min,
                max: bound.max,
            });
        }
        for (name, group) in &options.groups {
            for asset in &group.assets {
                if !labels.contains(asset) {
                    report.error(format!("options.groups.{name}"), unknown(asset));
                }
            }
            constraints.groups.push(GroupLimit {
                name: name.clone(),
                assets: group.assets.clone(),
                min: group.min,
                max: group.max,
            });
        }
        if let Some(budgets) = &options.budgets {
            for asset in budgets.keys() {
                if !labels.contains(asset) {
                    report.error(format!("options.budgets.{asset}"), unknown(asset));
                }
            }
            for asset in labels.iter().filter(|l| !budgets.contains_key(*l)) {
                report.error("options.budgets", format!("no budget for {asset}"));
            }
        }
        report.into_result(())?;
        let matrix = covariance(&returns, self.covariance_method)?
            .matrix
            .scaled(self.periods_per_year);
        let mut problem = Problem::from_covariance(matrix)
            .with_expected_returns(expected)
            .with_constraints(constraints);
        if options.objective == ObjectiveName::MinimumCvar {
            problem = problem.with_scenarios(returns);
        }
        Ok((problem, labels))
    }
}

/// The objective `options` name, with its parameters.
fn objective(
    options: &OptimiseOptions,
    risk_free_rate: f64,
    labels: &[String],
) -> Result<Objective> {
    let mut report = ValidationReport::new();
    let mut need = |value: Option<f64>, name: &str| -> f64 {
        value.unwrap_or_else(|| {
            report.error(
                format!("options.{name}"),
                format!("{:?} needs {name}", options.objective),
            );
            0.0
        })
    };
    let objective = match options.objective {
        ObjectiveName::MinimumVariance => Objective::MinimumVariance,
        ObjectiveName::MeanVariance => Objective::MeanVariance {
            risk_aversion: need(options.risk_aversion, "risk_aversion"),
        },
        ObjectiveName::RobustMeanVariance => {
            let risk_aversion = need(options.risk_aversion, "risk_aversion");
            match options.uncertainty.clone() {
                Some(uncertainty) => Objective::RobustMeanVariance {
                    risk_aversion,
                    uncertainty,
                },
                None => {
                    report.error(
                        "options.uncertainty",
                        "robust_mean_variance needs uncertainty",
                    );
                    Objective::MinimumVariance
                }
            }
        }
        ObjectiveName::TargetReturn => Objective::TargetReturn {
            target: need(options.target, "target"),
        },
        ObjectiveName::TargetVolatility => Objective::TargetVolatility {
            target: need(options.target, "target"),
        },
        ObjectiveName::MaximumSharpe => Objective::MaximumSharpe { risk_free_rate },
        ObjectiveName::MaximumDiversification => Objective::MaximumDiversification,
        ObjectiveName::MinimumCvar => Objective::MinimumCvar {
            confidence: options.confidence.unwrap_or(0.95),
        },
        ObjectiveName::RiskBudgeting => Objective::RiskBudgeting {
            budgets: options.budgets.as_ref().map(|by_asset| {
                labels
                    .iter()
                    .map(|l| by_asset.get(l).copied().unwrap_or(0.0))
                    .collect()
            }),
        },
        ObjectiveName::HierarchicalRiskParity => Objective::HierarchicalRiskParity {
            linkage: options.linkage.unwrap_or_default(),
        },
    };
    report.into_result(objective)
}

/// Builds an optimisation over [`Assets`] step by step; [`Optimiser::solve`] runs it.
#[derive(Debug, Clone)]
pub struct Optimiser<'a> {
    assets: &'a Assets,
    options: OptimiseOptions,
}

impl Optimiser<'_> {
    fn objective(mut self, name: ObjectiveName) -> Self {
        self.options.objective = name;
        self
    }

    /// The portfolio with the smallest variance.
    pub fn minimum_variance(self) -> Self {
        self.objective(ObjectiveName::MinimumVariance)
    }

    /// The largest expected return less `risk_aversion / 2` times the variance.
    pub fn mean_variance(mut self, risk_aversion: f64) -> Self {
        self.options.risk_aversion = Some(risk_aversion);
        self.objective(ObjectiveName::MeanVariance)
    }

    /// Mean–variance against the worst expected returns within `uncertainty`.
    pub fn robust_mean_variance(
        mut self,
        risk_aversion: f64,
        uncertainty: ReturnUncertainty,
    ) -> Self {
        self.options.risk_aversion = Some(risk_aversion);
        self.options.uncertainty = Some(uncertainty);
        self.objective(ObjectiveName::RobustMeanVariance)
    }

    /// The smallest variance with an annual expected return of at least `target`.
    pub fn target_return(mut self, target: f64) -> Self {
        self.options.target = Some(target);
        self.objective(ObjectiveName::TargetReturn)
    }

    /// The largest expected return with an annual volatility of at most `target`.
    pub fn target_volatility(mut self, target: f64) -> Self {
        self.options.target = Some(target);
        self.objective(ObjectiveName::TargetVolatility)
    }

    /// The largest Sharpe ratio (the default objective).
    pub fn maximum_sharpe(self) -> Self {
        self.objective(ObjectiveName::MaximumSharpe)
    }

    /// The largest diversification ratio.
    pub fn maximum_diversification(self) -> Self {
        self.objective(ObjectiveName::MaximumDiversification)
    }

    /// The smallest conditional value at risk of the periods' returns at `confidence`.
    pub fn minimum_cvar(mut self, confidence: f64) -> Self {
        self.options.confidence = Some(confidence);
        self.objective(ObjectiveName::MinimumCvar)
    }

    /// Each asset's share of the risk equal to its budget; equal shares with no budgets.
    pub fn risk_budgeting(mut self, budgets: BTreeMap<String, f64>) -> Self {
        self.options.budgets = (!budgets.is_empty()).then_some(budgets);
        self.objective(ObjectiveName::RiskBudgeting)
    }

    /// Hierarchical risk parity, joining clusters by `linkage`.
    pub fn hierarchical_risk_parity(mut self, linkage: Linkage) -> Self {
        self.options.linkage = Some(linkage);
        self.objective(ObjectiveName::HierarchicalRiskParity)
    }

    /// An annual risk-free rate for the Sharpe ratio, in place of the assets' own.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.options.risk_free_rate = Some(rate);
        self
    }

    /// Every asset's weight between `min` and `max`.
    pub fn bounds(mut self, min: f64, max: f64) -> Self {
        self.options.bounds = Some(Bound { min, max });
        self
    }

    /// One asset's weight between `min` and `max`.
    pub fn asset_bounds(mut self, asset: &str, min: f64, max: f64) -> Self {
        self.options
            .asset_bounds
            .insert(asset.to_owned(), Bound { min, max });
        self
    }

    /// The total weight of the `assets` of group `name` between `min` and `max`.
    pub fn group<S: AsRef<str>>(mut self, name: &str, assets: &[S], min: f64, max: f64) -> Self {
        self.options.groups.insert(
            name.to_owned(),
            AssetGroup {
                assets: assets.iter().map(|a| a.as_ref().to_owned()).collect(),
                min,
                max,
            },
        );
        self
    }

    /// An asset's annual expected return, in place of its historical mean.
    pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
        self.options
            .expected_returns
            .insert(asset.to_owned(), value);
        self
    }

    /// The options built so far.
    pub fn options(&self) -> &OptimiseOptions {
        &self.options
    }

    /// Runs the optimisation.
    pub fn solve(&self) -> Result<OptimisationResult> {
        self.assets.optimise(&self.options)
    }

    /// The efficient frontier under the limits built so far.
    pub fn frontier(&self, points: usize) -> Result<Vec<FrontierPoint>> {
        self.assets.frontier(points, &self.options)
    }
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}
