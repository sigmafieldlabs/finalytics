//! Assets: the prices or returns of things one might hold, and what can be learnt from them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::{Date, Decimal, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, price_columns_from_csv};
use finalytics_ledger::HoldingKey;
use finalytics_opt::{
    AssetBound, Bound, Constraints, FrontierPoint, GroupLimit, Linkage, Objective, Problem,
    ReturnUncertainty, Solution, SolveStatus,
};
use finalytics_reports::OptimisationReport;
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
/// use finalytics::{Assets, Method, datasets};
///
/// let assets = Assets::builder()
///     .prices(datasets::asset_class_prices())
///     .risk_free_rate(0.02)
///     .build()?;
/// let result = assets
///     .optimise(Method::MaximumSharpe)
///     .max_weight(0.4)
///     .group("Equity", &["US Equity", "Intl Equity"], 0.3, 0.6)
///     .build()?;
/// let weights = result.results.weights.unwrap_or_default();
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

/// An optimisation method, by name. The parameters some methods need are given beside it, in
/// [`OptimiseOptions`] or with the builder's methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Method {
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
    /// Every asset with the same weight: the naive portfolio that estimated ones must beat.
    EqualWeight,
    /// Weights in proportion to the inverse of each asset's volatility.
    InverseVolatility,
    /// Every asset with the same share of the portfolio's risk.
    RiskParity,
    /// Each asset's share of the portfolio's risk equal to its budget, given with `budgets`
    /// (equal shares without them, as risk parity).
    RiskBudgeting,
    /// Hierarchical risk parity.
    HierarchicalRiskParity,
}

/// The points on the efficient frontier that an optimisation report draws.
const FRONTIER_POINTS: usize = 30;

/// An optimisation: the optimisation report on the weights chosen, and the raw results.
/// Printing it prints the report, and showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Optimisation {
    /// The optimisation report: the weights, what they are expected to deliver, where their
    /// risk comes from, where they sit against each asset (and on the efficient frontier, if
    /// asked for) and the history they were fitted to.
    pub report: OptimisationReport,
    /// The raw results: the weights by asset, their expected return, volatility and Sharpe
    /// ratio, and the solver's solution.
    pub results: OptimisationResult,
}

impl std::fmt::Display for Optimisation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl Method {
    /// What the method does, in a sentence, for reports.
    pub fn description(self) -> &'static str {
        match self {
            Method::MinimumVariance => "the weights with the smallest variance.",
            Method::MeanVariance => {
                "the largest expected return less half the risk aversion times the variance."
            }
            Method::RobustMeanVariance => {
                "mean–variance against the worst expected returns within the uncertainty given."
            }
            Method::TargetReturn => {
                "the smallest variance with at least the target expected return."
            }
            Method::TargetVolatility => {
                "the largest expected return with at most the target volatility."
            }
            Method::MaximumSharpe => {
                "the largest expected return in excess of the risk-free rate per unit of \
                 volatility, solved as a quadratic programme after the Charnes–Cooper \
                 transformation."
            }
            Method::MaximumDiversification => {
                "the largest ratio of the weighted average of the assets' volatilities to the \
                 portfolio's."
            }
            Method::MinimumCvar => {
                "the smallest average loss in the worst periods, at the confidence given, \
                 from the historical returns as scenarios."
            }
            Method::EqualWeight => "every asset with the same weight.",
            Method::InverseVolatility => {
                "weights in proportion to the inverse of each asset's volatility."
            }
            Method::RiskParity => "every asset with the same share of the portfolio's risk.",
            Method::RiskBudgeting => {
                "each asset's share of the portfolio's risk equal to its budget."
            }
            Method::HierarchicalRiskParity => {
                "clusters of assets by correlation, with weights split between clusters by \
                 their risk."
            }
        }
    }

    /// The method's name in words, such as `"Maximum Sharpe"`, for reports.
    pub fn label(self) -> &'static str {
        match self {
            Method::MinimumVariance => "Minimum variance",
            Method::MeanVariance => "Mean–variance",
            Method::RobustMeanVariance => "Robust mean–variance",
            Method::TargetReturn => "Target return",
            Method::TargetVolatility => "Target volatility",
            Method::MaximumSharpe => "Maximum Sharpe",
            Method::MaximumDiversification => "Maximum diversification",
            Method::MinimumCvar => "Minimum CVaR",
            Method::EqualWeight => "Equal weight",
            Method::InverseVolatility => "Inverse volatility",
            Method::RiskParity => "Risk parity",
            Method::RiskBudgeting => "Risk budgeting",
            Method::HierarchicalRiskParity => "Hierarchical risk parity",
        }
    }

    /// The method's name within a sentence: "maximum Sharpe", "minimum CVaR".
    pub(crate) fn in_sentence(self) -> String {
        let label = self.label();
        let mut chars = label.chars();
        match chars.next() {
            Some(first) => first.to_lowercase().chain(chars).collect(),
            None => String::new(),
        }
    }

    /// Whether the method takes limits on the weights. Equal weight, inverse volatility, risk
    /// parity, risk budgeting and hierarchical risk parity give a fully invested, long-only
    /// portfolio and take none.
    pub fn takes_limits(self) -> bool {
        !matches!(
            self,
            Method::EqualWeight
                | Method::InverseVolatility
                | Method::RiskParity
                | Method::RiskBudgeting
                | Method::HierarchicalRiskParity
        )
    }
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

/// The method, its parameters and the limits on the weights. Everything is optional: by default
/// the portfolio with the largest Sharpe ratio, fully invested and long only.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct OptimiseOptions {
    /// The method: the largest Sharpe ratio by default.
    pub method: Method,
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
    /// The largest weight of any asset, as `bounds` of 0 and `max_weight`; give one or the
    /// other.
    pub max_weight: Option<f64>,
    /// Bounds for particular assets, which take the place of `bounds` for them.
    pub asset_bounds: BTreeMap<String, Bound>,
    /// Limits on the total weight of groups of assets, by group name.
    pub groups: BTreeMap<String, AssetGroup>,
    /// Annual expected returns by asset, in place of the historical means.
    pub expected_returns: BTreeMap<String, f64>,
    /// Whether the optimisation report draws the efficient frontier under the same limits.
    /// Off by default: the frontier takes 30 more optimisations, so it adds time that grows with
    /// the number of assets, to seconds for hundreds of assets. Only an optimisation reads it;
    /// backtests leave it out.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[cfg_attr(feature = "schema", schemars(extend("default" = false)))]
    pub frontier: bool,
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

/// The weights as text: each asset's weight, the largest first, with the expected return,
/// volatility and Sharpe ratio.
impl std::fmt::Display for OptimisationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pct = |v: Option<f64>| {
            v.filter(|v| v.is_finite()).map_or_else(
                || "–".to_owned(),
                |v| format!("{:.1}%", v * 100.0).replace('-', "−"),
            )
        };
        let method = serde_json::to_value(&self.solution.objective)
            .ok()
            .and_then(|v| serde_json::from_value::<Method>(v["type"].clone()).ok())
            .map_or("Optimisation", Method::label);
        let title = format!("OptimisationResult · {method} · {}", self.status);
        let mut figures = vec![
            ("Expected return".to_owned(), pct(self.expected_return)),
            ("volatility".to_owned(), pct(self.volatility)),
            (
                "Sharpe ratio".to_owned(),
                self.sharpe_ratio
                    .map_or_else(|| "–".to_owned(), |v| format!("{v:.2}").replace('-', "−")),
            ),
        ];
        figures.push(("risk-free".to_owned(), pct(Some(self.risk_free_rate))));
        let empty = BTreeMap::new();
        f.write_str(&finalytics_reports::weights_text(
            &title,
            self.weights.as_ref().unwrap_or(&empty),
            &figures,
        ))
    }
}

impl OptimisationResult {
    /// The weights as holdings, for [`Portfolio::rebalance`](crate::Portfolio::rebalance): each
    /// asset as the instrument of the same name. Empty when there is no solution.
    pub fn holdings(&self) -> BTreeMap<HoldingKey, f64> {
        self.weights
            .iter()
            .flatten()
            .filter_map(|(asset, w)| asset.parse().ok().map(|key| (key, *w)))
            .collect()
    }
}

impl Assets {
    /// A builder for assets from their prices or returns, with a risk-free asset or rate.
    pub fn builder() -> AssetsBuilder {
        AssetsBuilder::default()
    }

    /// Assets from their prices: the returns between consecutive dates on which every asset has
    /// a price. Prices should include income, as adjusted closes do. The number of periods in a
    /// year is detected from the dates.
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

    /// The covariance estimator.
    pub fn covariance_method(&self) -> CovarianceMethod {
        self.covariance_method
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
    pub(crate) fn risky_returns(&self) -> Result<ReturnsTable> {
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

    /// The return of cash in each period: the risk-free asset's, or the annual risk-free rate
    /// converted to a rate per period by compounding, `(1 + rate)^(1 / periods) - 1`, as the
    /// statistics convert it.
    pub(crate) fn cash_returns(&self) -> Vec<f64> {
        match self.risk_free_asset {
            Some(i) => self.returns.columns()[i].clone(),
            None => {
                let per_period =
                    (1.0 + self.risk_free_rate).powf(1.0 / self.periods_per_year) - 1.0;
                vec![per_period; self.dates.len()]
            }
        }
    }

    /// The same assets over the periods `rows` only, with the same settings: the periods per
    /// year, the estimator, and the risk-free asset (whose mean over `rows` becomes the rate) or
    /// rate.
    pub(crate) fn window(&self, rows: std::ops::Range<usize>) -> Result<Assets> {
        let returns = ReturnsTable::new(
            self.labels().to_vec(),
            self.returns
                .columns()
                .iter()
                .map(|c| c[rows.clone()].to_vec())
                .collect(),
        )?;
        let mut window = Assets {
            dates: self.dates[rows].to_vec(),
            returns,
            periods_per_year: self.periods_per_year,
            risk_free_rate: self.risk_free_rate,
            risk_free_asset: self.risk_free_asset,
            covariance_method: self.covariance_method,
            warnings: ValidationReport::new(),
        };
        if let Some(i) = self.risk_free_asset {
            window.risk_free_rate = window.mean_returns()[i];
        }
        Ok(window)
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
        options.target_return = self.risk_free_rate;
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

    /// An optimisation of the risky assets by `method`, with the limits and parameters the
    /// builder adds; [`Optimise::build`] runs it.
    ///
    /// ```
    /// use finalytics::{Assets, Method, datasets};
    ///
    /// let assets = Assets::builder()
    ///     .prices(datasets::asset_class_prices())
    ///     .risk_free_asset("T-Bills")
    ///     .build()?;
    /// let result = assets.optimise(Method::MaximumSharpe).max_weight(0.4).build()?;
    /// assert!(result.results.weights.unwrap().values().all(|w| *w <= 0.4 + 1e-6));
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn optimise(&self, method: Method) -> Optimise<'_> {
        Optimise {
            assets: self,
            options: OptimiseOptions {
                method,
                ..OptimiseOptions::default()
            },
        }
    }

    /// Optimises a portfolio of the risky assets as `options` describe: annualised Ledoit–Wolf
    /// covariance (or the assets' estimator), historical mean returns unless others are given,
    /// and the method and limits of `options`. The result holds the optimisation report and
    /// the raw results. [`Assets::optimise`] builds the same options step by step.
    pub fn optimise_with(&self, options: &OptimiseOptions) -> Result<Optimisation> {
        self.check_options(options, true).into_result(())?;
        let results = self.solve(options)?;
        let report = self.optimisation_report(options, &results)?;
        Ok(Optimisation { report, results })
    }

    /// The optimisation report on `results`, the solution of `options`.
    fn optimisation_report(
        &self,
        options: &OptimiseOptions,
        results: &OptimisationResult,
    ) -> Result<OptimisationReport> {
        let (problem, labels) = self.problem(options)?;
        let n = labels.len();
        let covariance = match problem.covariance.clone() {
            Some(c) => c,
            None => Matrix::new(labels.clone(), vec![0.0; n * n])?,
        };
        let (Some(&start), Some(&end)) = (self.dates.first(), self.dates.last()) else {
            let mut report = ValidationReport::new();
            report.error("dates", "there are none");
            return Err(Error::Validation(report));
        };
        let expected = problem
            .expected_returns
            .clone()
            .unwrap_or_else(|| vec![0.0; n]);
        let frontier = if options.frontier {
            problem
                .frontier(FRONTIER_POINTS)
                .map(|points| {
                    points
                        .iter()
                        .map(|p| (p.volatility, p.expected_return))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let history = results.weights.as_ref().and_then(|weights| {
            let equal: BTreeMap<String, f64> =
                labels.iter().map(|l| (l.clone(), 1.0 / n as f64)).collect();
            let mut held = weights.clone();
            let total: f64 = held.values().sum();
            if let Some(cash) = self.risk_free_asset().filter(|_| total < 1.0 - 1e-9) {
                held.insert(cash.to_owned(), 1.0 - total);
            }
            let history = crate::backtest::BacktestOptions {
                strategies: vec![crate::backtest::Strategy::Weights {
                    name: Some("The weights".to_owned()),
                    weights: held,
                }],
                benchmark: Some(crate::backtest::Benchmark::Weights {
                    name: Some("Equal weight".to_owned()),
                    weights: equal,
                }),
                ..crate::backtest::BacktestOptions::default()
            };
            let report = self.backtest_with(&history).ok()?.report;
            Some((report.strategy.tested, report.benchmark?.tested))
        });
        let solver = serde_json::to_value(results.solution.method)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        finalytics_reports::optimisation_report(finalytics_reports::OptimisationInputs {
            method: options.method.label().to_owned(),
            description: options.method.description().to_owned(),
            status: results.status.to_string(),
            solver,
            iterations: results.solution.iterations,
            labels: &labels,
            weights: results.weights.as_ref(),
            expected_returns: &expected,
            historical_means: options.expected_returns.is_empty(),
            covariance: &covariance,
            risk_free_asset: self.risk_free_asset().map(str::to_owned),
            risk_free_rate: results.risk_free_rate,
            periods_per_year: self.periods_per_year,
            covariance_method: self.covariance_method,
            limits: crate::backtest::limits(options),
            start,
            end,
            frontier,
            history,
        })
    }

    /// Solves the optimisation `options` describe, without a report: the weights a backtest
    /// refits.
    pub(crate) fn solve(&self, options: &OptimiseOptions) -> Result<OptimisationResult> {
        let rf = options.risk_free_rate.unwrap_or(self.risk_free_rate);
        let (problem, labels) = self.problem(options)?;
        let objective = objective(options, rf, &labels)?;
        let solution = problem
            .solve(&objective)
            .map_err(|e| user_paths(e, options))?;
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
    /// one with the largest expected return, under the limits of `options` (its method is not
    /// used). Each point's weights are in the order of the risky assets.
    pub fn frontier(&self, points: usize, options: &OptimiseOptions) -> Result<Vec<FrontierPoint>> {
        self.problem(options)?.0.frontier(points)
    }

    /// Every problem with `options` on these assets, at the paths of the options
    /// (`options.<name>`), gathered together: unknown assets, numbers out of range, limits no
    /// fully invested portfolio can meet, and the parameters the method needs. With `single`,
    /// when one method is asked for, a parameter the method does not use is a problem too; a
    /// comparison applies each to the methods that take it.
    pub(crate) fn check_options(
        &self,
        options: &OptimiseOptions,
        single: bool,
    ) -> ValidationReport {
        let mut report = ValidationReport::new();
        let risky = self.risky_assets();
        let n = risky.len();
        let method = options.method;
        let name = method.in_sentence();
        let unknown = |asset: &str| match self.risk_free_asset() {
            Some(rf) if rf == asset => {
                format!("{asset} is the risk-free asset, which optimisation leaves out")
            }
            _ => format!("unknown asset {asset}"),
        };
        let known = |asset: &str| risky.contains(&asset);
        let finite = |report: &mut ValidationReport, value: Option<f64>, path: &str| match value {
            Some(v) if !v.is_finite() => {
                report.error(path, format!("must be a finite number, not {v}"));
                false
            }
            _ => true,
        };
        // A weight's limits: from 0 to 1, the smaller first.
        let share = |report: &mut ValidationReport, min: f64, max: f64, path: &str| {
            if !(min.is_finite() && max.is_finite()) {
                report.error(path, "the limits must be finite numbers");
            } else if min > max {
                report.error(
                    path,
                    format!("the smallest weight {min} is above the largest {max}"),
                );
            } else if min < 0.0 || max > 1.0 {
                report.error(
                    path,
                    "weights are shares of a long-only portfolio, from 0 to 1; for short \
                     positions or leverage, use finalytics.opt.Problem",
                );
            }
        };

        // Limits on weights.
        if options.bounds.is_some() && options.max_weight.is_some() {
            report.error("options.max_weight", "give bounds or max_weight, not both");
        }
        let (default, limit_path) = match (options.max_weight, options.bounds) {
            (Some(max), _) => {
                if !(max.is_finite() && max > 0.0 && max <= 1.0) {
                    report.error(
                        "options.max_weight",
                        format!("must be a share above 0 and at most 1, such as 0.4, not {max}"),
                    );
                }
                (Bound { min: 0.0, max }, "options.max_weight")
            }
            (None, Some(b)) => {
                share(&mut report, b.min, b.max, "options.bounds");
                (b, "options.bounds")
            }
            (None, None) => (Bound { min: 0.0, max: 1.0 }, "options.bounds"),
        };
        let mut lowest = 0.0;
        let mut highest = 0.0;
        for asset in &risky {
            let b = options.asset_bounds.get(*asset).copied().unwrap_or(default);
            lowest += b.min;
            highest += b.max;
        }
        for (asset, b) in &options.asset_bounds {
            let path = format!("options.asset_bounds.{asset}");
            if known(asset) {
                share(&mut report, b.min, b.max, &path);
            } else {
                report.error(path, unknown(asset));
            }
        }
        if report.is_empty() && method.takes_limits() && n > 0 {
            let path = if options.asset_bounds.is_empty() {
                limit_path
            } else {
                "options.asset_bounds"
            };
            if highest < 1.0 - 1e-9 {
                report.error(
                    path,
                    format!(
                        "the {n} assets can hold at most {:.4} of the portfolio together, so it \
                         cannot be fully invested; with {n} assets the largest weight must be at \
                         least {:.4}",
                        highest,
                        1.0 / n as f64
                    ),
                );
            } else if lowest > 1.0 + 1e-9 {
                report.error(
                    path,
                    format!(
                        "the {n} assets must hold at least {lowest:.4} of the portfolio together, \
                         more than all of it"
                    ),
                );
            }
        }
        for (group, g) in &options.groups {
            let path = format!("options.groups.{group}");
            share(&mut report, g.min, g.max, &path);
            for asset in &g.assets {
                if !known(asset) {
                    report.error(&path, unknown(asset));
                }
            }
        }
        if single && !method.takes_limits() {
            let limited = [
                (options.max_weight.is_some(), "options.max_weight"),
                (options.bounds.is_some(), "options.bounds"),
                (!options.asset_bounds.is_empty(), "options.asset_bounds"),
                (!options.groups.is_empty(), "options.groups"),
            ];
            for (_, path) in limited.iter().filter(|(given, _)| *given) {
                report.error(
                    *path,
                    format!(
                        "{name} is fully invested and long only, and takes no limits on weights"
                    ),
                );
            }
        }

        // Numbers.
        finite(
            &mut report,
            options.risk_free_rate,
            "options.risk_free_rate",
        );
        for (asset, r) in &options.expected_returns {
            let path = format!("options.expected_returns.{asset}");
            if !known(asset) {
                report.error(path, unknown(asset));
            } else {
                finite(&mut report, Some(*r), &path);
            }
        }

        // The method's own parameters: those it needs, and, for one method, those it does not
        // use.
        let uses = |parameter: &str| match parameter {
            "target" => matches!(method, Method::TargetReturn | Method::TargetVolatility),
            "risk_aversion" => matches!(method, Method::MeanVariance | Method::RobustMeanVariance),
            "uncertainty" => method == Method::RobustMeanVariance,
            "confidence" => method == Method::MinimumCvar,
            "budgets" => method == Method::RiskBudgeting,
            "linkage" => method == Method::HierarchicalRiskParity,
            _ => true,
        };
        let given = [
            ("target", options.target.is_some()),
            ("risk_aversion", options.risk_aversion.is_some()),
            ("uncertainty", options.uncertainty.is_some()),
            ("confidence", options.confidence.is_some()),
            ("budgets", options.budgets.is_some()),
            ("linkage", options.linkage.is_some()),
        ];
        for (parameter, is_given) in given {
            let path = format!("options.{parameter}");
            if is_given && !uses(parameter) {
                if method == Method::RiskParity && parameter == "budgets" {
                    report.error(
                        path,
                        "risk parity gives every asset the same budget; use risk budgeting for others",
                    );
                } else if single {
                    report.error(path, format!("{name} does not use {parameter}"));
                }
            } else if !is_given
                && uses(parameter)
                && matches!(parameter, "target" | "risk_aversion" | "uncertainty")
            {
                report.error(path, format!("{name} needs {parameter}"));
            }
        }
        if let Some(target) = options.target {
            finite(&mut report, Some(target), "options.target");
            if method == Method::TargetVolatility && target <= 0.0 {
                report.error(
                    "options.target",
                    "a volatility to aim for must be above zero",
                );
            }
        }
        if let Some(a) = options.risk_aversion
            && finite(&mut report, Some(a), "options.risk_aversion")
            && a < 0.0
        {
            report.error(
                "options.risk_aversion",
                format!("must not be negative, not {a}"),
            );
        }
        if let Some(c) = options.confidence
            && !(c > 0.0 && c < 1.0)
        {
            report.error(
                "options.confidence",
                format!("must be between 0 and 1, such as 0.95, not {c}"),
            );
        }
        if let Some(budgets) = options.budgets.as_ref().filter(|_| uses("budgets")) {
            for (asset, b) in budgets {
                let path = format!("options.budgets.{asset}");
                if !known(asset) {
                    report.error(path, unknown(asset));
                } else if !(b.is_finite() && *b >= 0.0) {
                    report.error(path, format!("must be a budget of zero or more, not {b}"));
                }
            }
            for asset in risky.iter().filter(|a| !budgets.contains_key(**a)) {
                report.error("options.budgets", format!("no budget for {asset}"));
            }
        }
        report
    }

    /// The problem `options` describe, over the risky assets, with their labels. The options
    /// are checked first, by [`Assets::check_options`].
    fn problem(&self, options: &OptimiseOptions) -> Result<(Problem, Vec<String>)> {
        self.check_options(options, false).into_result(())?;
        let returns = self.risky_returns()?;
        let labels = returns.labels().to_vec();
        let mut expected: Vec<f64> = returns
            .columns()
            .iter()
            .map(|c| c.iter().sum::<f64>() / c.len() as f64 * self.periods_per_year)
            .collect();
        for (asset, r) in &options.expected_returns {
            if let Some(i) = labels.iter().position(|l| l == asset) {
                expected[i] = *r;
            }
        }
        let mut constraints = Constraints::default();
        if let Some(bounds) = options.bounds {
            constraints.bounds = bounds;
        }
        if let Some(max) = options.max_weight {
            constraints.bounds = Bound { min: 0.0, max };
        }
        for (asset, bound) in &options.asset_bounds {
            constraints.asset_bounds.push(AssetBound {
                asset: asset.clone(),
                min: bound.min,
                max: bound.max,
            });
        }
        for (name, group) in &options.groups {
            constraints.groups.push(GroupLimit {
                name: name.clone(),
                assets: group.assets.clone(),
                min: group.min,
                max: group.max,
            });
        }
        let matrix = covariance(&returns, self.covariance_method)?
            .matrix
            .scaled(self.periods_per_year);
        let mut problem = Problem::from_covariance(matrix)
            .with_expected_returns(expected)
            .with_constraints(constraints);
        if options.method == Method::MinimumCvar {
            problem = problem.with_scenarios(returns);
        }
        Ok((problem, labels))
    }
}

/// `error` with the optimiser's paths (`constraints.bounds`, `objective.confidence`) turned into
/// those of the options that set them (`options.max_weight`, `options.confidence`).
fn user_paths(error: Error, options: &OptimiseOptions) -> Error {
    let Error::Validation(report) = error else {
        return error;
    };
    let nth = |keys: Vec<&String>, rest: &str| -> Option<String> {
        let (k, _) = rest.strip_prefix('[')?.split_once(']')?;
        keys.get(k.parse::<usize>().ok()?).map(|key| (*key).clone())
    };
    let mut mapped = ValidationReport::new();
    for issue in report.issues() {
        let path = &issue.path;
        let user = if path == "constraints.bounds" {
            if options.max_weight.is_some() {
                "options.max_weight".to_owned()
            } else {
                "options.bounds".to_owned()
            }
        } else if let Some(rest) = path.strip_prefix("constraints.asset_bounds") {
            nth(options.asset_bounds.keys().collect(), rest).map_or_else(
                || "options.asset_bounds".to_owned(),
                |a| format!("options.asset_bounds.{a}"),
            )
        } else if let Some(rest) = path.strip_prefix("constraints.groups") {
            nth(options.groups.keys().collect(), rest).map_or_else(
                || "options.groups".to_owned(),
                |g| format!("options.groups.{g}"),
            )
        } else if let Some(rest) = path.strip_prefix("objective.") {
            format!("options.{rest}")
        } else if path == "constraints" || path == "objective" {
            "options".to_owned()
        } else {
            path.clone()
        };
        let mut one = ValidationReport::new();
        match issue.severity {
            finalytics_core::Severity::Error => one.error(user, issue.message.clone()),
            _ => one.warning(user, issue.message.clone()),
        }
        mapped.merge("", one);
    }
    Error::Validation(mapped)
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
                format!("{} needs {name}", options.method.in_sentence()),
            );
            0.0
        })
    };
    let objective = match options.method {
        Method::MinimumVariance => Objective::MinimumVariance,
        Method::MeanVariance => Objective::MeanVariance {
            risk_aversion: need(options.risk_aversion, "risk_aversion"),
        },
        Method::RobustMeanVariance => {
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
        Method::TargetReturn => Objective::TargetReturn {
            target: need(options.target, "target"),
        },
        Method::TargetVolatility => Objective::TargetVolatility {
            target: need(options.target, "target"),
        },
        Method::MaximumSharpe => Objective::MaximumSharpe { risk_free_rate },
        Method::MaximumDiversification => Objective::MaximumDiversification,
        Method::MinimumCvar => Objective::MinimumCvar {
            confidence: options.confidence.unwrap_or(0.95),
        },
        Method::EqualWeight => Objective::EqualWeight,
        Method::InverseVolatility => Objective::InverseVolatility,
        Method::RiskParity => Objective::RiskBudgeting { budgets: None },
        Method::RiskBudgeting => Objective::RiskBudgeting {
            budgets: options.budgets.as_ref().map(|by_asset| {
                labels
                    .iter()
                    .map(|l| by_asset.get(l).copied().unwrap_or(0.0))
                    .collect()
            }),
        },
        Method::HierarchicalRiskParity => Objective::HierarchicalRiskParity {
            linkage: options.linkage.unwrap_or_default(),
        },
    };
    report.into_result(objective)
}

/// An optimisation over [`Assets`], built step by step; [`Optimise::build`] runs it. The
/// limits apply to the methods that take them: equal weight, inverse volatility, risk parity,
/// risk budgeting and hierarchical risk parity are fully invested and long only, and take none.
#[derive(Debug, Clone)]
#[must_use = "an optimisation does nothing until `build` runs it"]
pub struct Optimise<'a> {
    assets: &'a Assets,
    options: OptimiseOptions,
}

impl Optimise<'_> {
    /// The largest weight of any asset, such as 0.4 for 40%.
    pub fn max_weight(mut self, max: f64) -> Self {
        self.options.max_weight = Some(max);
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

    /// The total weight of the `assets` of group `name` between `min` and `max`, such as a
    /// sector's.
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

    /// The annual expected return (target return) or volatility (target volatility) to aim
    /// for.
    pub fn target(mut self, target: f64) -> Self {
        self.options.target = Some(target);
        self
    }

    /// The weight on variance, for mean–variance methods.
    pub fn risk_aversion(mut self, risk_aversion: f64) -> Self {
        self.options.risk_aversion = Some(risk_aversion);
        self
    }

    /// The uncertainty of the expected returns, for robust mean–variance.
    pub fn uncertainty(mut self, uncertainty: ReturnUncertainty) -> Self {
        self.options.uncertainty = Some(uncertainty);
        self
    }

    /// The confidence of the conditional value at risk, for minimum CVaR: 0.95 by default.
    pub fn confidence(mut self, confidence: f64) -> Self {
        self.options.confidence = Some(confidence);
        self
    }

    /// Each asset's risk budget, for risk budgeting; equal shares without them.
    pub fn budget(mut self, asset: &str, budget: f64) -> Self {
        self.options
            .budgets
            .get_or_insert_with(BTreeMap::new)
            .insert(asset.to_owned(), budget);
        self
    }

    /// How hierarchical risk parity joins clusters: single linkage by default.
    pub fn linkage(mut self, linkage: Linkage) -> Self {
        self.options.linkage = Some(linkage);
        self
    }

    /// An annual risk-free rate for the Sharpe ratio, in place of the assets' own.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.options.risk_free_rate = Some(rate);
        self
    }

    /// An asset's annual expected return, in place of its historical mean.
    pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
        self.options
            .expected_returns
            .insert(asset.to_owned(), value);
        self
    }

    /// Draws the efficient frontier under the same limits in the optimisation report. It takes
    /// 30 more optimisations, so it adds time that grows with the number of assets, to seconds
    /// for hundreds of assets; without it, the report shows the weights against each asset and
    /// equal weight.
    pub fn with_frontier(mut self) -> Self {
        self.options.frontier = true;
        self
    }

    /// The options built so far.
    pub fn options(&self) -> &OptimiseOptions {
        &self.options
    }

    /// Runs the optimisation: the weights in `results`, with the optimisation report in
    /// `report`.
    pub fn build(&self) -> Result<Optimisation> {
        self.assets.optimise_with(&self.options)
    }

    /// The efficient frontier under the limits built so far.
    pub fn frontier(&self, points: usize) -> Result<Vec<FrontierPoint>> {
        self.assets.frontier(points, &self.options)
    }
}

/// [`Assets`] built step by step from prices or returns; [`AssetsBuilder::build`] makes them.
///
/// ```
/// use finalytics::{Assets, datasets};
///
/// let assets = Assets::builder()
///     .prices(datasets::sp500())
///     .risk_free_asset("T-Bills")
///     .build()?;
/// assert_eq!(assets.periods_per_year(), 252.0);
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
#[must_use = "the assets are made by `build`"]
pub struct AssetsBuilder {
    prices: Option<PriceTable>,
    csv: Option<String>,
    #[cfg(feature = "io")]
    file: Option<std::path::PathBuf>,
    returns: Option<(Vec<Date>, ReturnsTable)>,
    risk_free_asset: Option<String>,
    risk_free_rate: Option<f64>,
    periods_per_year: Option<f64>,
    covariance: Option<CovarianceMethod>,
}

impl AssetsBuilder {
    /// The assets' prices: a date column and one column per asset. Prices should include
    /// income, as adjusted closes do.
    pub fn prices(mut self, prices: PriceTable) -> Self {
        self.prices = Some(prices);
        self
    }

    /// The assets' prices as CSV text: a `date` column and one column per asset, or the long
    /// layout of `date`, `instrument` and `price` columns.
    pub fn prices_csv(mut self, text: &str) -> Self {
        self.csv = Some(text.to_owned());
        self
    }

    /// The assets' prices from a CSV file, as [`AssetsBuilder::prices_csv`] reads them.
    #[cfg(feature = "io")]
    pub fn prices_file(mut self, path: impl AsRef<std::path::Path>) -> Self {
        self.file = Some(path.as_ref().to_owned());
        self
    }

    /// The assets' returns, in place of prices: `dates` are the dates each return ends on.
    pub fn returns(mut self, dates: Vec<Date>, returns: ReturnsTable) -> Self {
        self.returns = Some((dates, returns));
        self
    }

    /// One of the assets, such as T-bills, as the risk-free asset: its mean return is the
    /// risk-free rate, and optimisation leaves it out.
    pub fn risk_free_asset(mut self, asset: &str) -> Self {
        self.risk_free_asset = Some(asset.to_owned());
        self
    }

    /// An annual risk-free rate, for Sharpe ratios; zero by default.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = Some(rate);
        self
    }

    /// The number of periods in a year, in place of the one detected from the dates: 252 for
    /// daily returns, 52 for weekly, 12 for monthly.
    pub fn periods_per_year(mut self, periods: f64) -> Self {
        self.periods_per_year = Some(periods);
        self
    }

    /// The covariance estimator: Ledoit–Wolf by default.
    pub fn covariance(mut self, method: CovarianceMethod) -> Self {
        self.covariance = Some(method);
        self
    }

    /// Makes the assets, reporting every problem with what was given.
    pub fn build(self) -> Result<Assets> {
        let mut report = ValidationReport::new();
        #[cfg(feature = "io")]
        let file = match &self.file {
            Some(path) => match std::fs::read_to_string(path) {
                Ok(text) => Some(text),
                Err(e) => {
                    report.error("prices", format!("cannot read {}: {e}", path.display()));
                    None
                }
            },
            None => None,
        };
        #[cfg(not(feature = "io"))]
        let file: Option<String> = None;
        let given = usize::from(self.prices.is_some())
            + usize::from(self.csv.is_some())
            + usize::from(file.is_some())
            + usize::from(self.returns.is_some());
        if given != 1 && report.is_empty() {
            report.error("prices", "give the assets' prices, or their returns, once");
        }
        let settings = AssetSettings {
            risk_free_rate: self.risk_free_rate,
            risk_free_asset: self.risk_free_asset,
            periods_per_year: self.periods_per_year,
            covariance: self.covariance,
        };
        settings.check(&mut report);
        report.into_result(())?;
        let assets = if let Some(prices) = &self.prices {
            Assets::from_prices(prices)?
        } else if let Some(text) = self.csv.as_deref().or(file.as_deref()) {
            Assets::from_csv(text)?
        } else if let Some((dates, returns)) = self.returns {
            Assets::from_returns(dates, returns)?
        } else {
            unreachable!("one source of prices or returns was checked above")
        };
        settings.apply(assets)
    }
}

/// The settings of [`Assets`] as JSON gives them, with the names of the builder's methods: the
/// Python and JavaScript packages use it; from Rust, use the builder.
#[doc(hidden)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssetSettings {
    pub risk_free_rate: Option<f64>,
    pub risk_free_asset: Option<String>,
    pub periods_per_year: Option<f64>,
    pub covariance: Option<CovarianceMethod>,
}

impl AssetSettings {
    /// Adds each problem with the settings to `report`.
    pub fn check(&self, report: &mut ValidationReport) {
        if self.risk_free_asset.is_some() && self.risk_free_rate.is_some() {
            report.error(
                "risk_free_rate",
                "give a risk-free asset or a risk-free rate, not both",
            );
        }
        if let Some(rate) = self.risk_free_rate
            && !(rate.is_finite() && rate > -1.0)
        {
            report.error(
                "risk_free_rate",
                format!("must be an annual rate above -1, such as 0.04, not {rate}"),
            );
        }
        if let Some(periods) = self.periods_per_year
            && !(periods.is_finite() && periods > 0.0)
        {
            report.error(
                "periods_per_year",
                format!("must be greater than zero, such as 12 for monthly returns, not {periods}"),
            );
        }
        if let Some(CovarianceMethod::Ewma { decay }) = self.covariance
            && !(decay > 0.0 && decay < 1.0)
        {
            report.error(
                "covariance.decay",
                format!("must be between 0 and 1, such as 0.94, not {decay}"),
            );
        }
    }

    /// `assets` with these settings, every problem with them reported together.
    pub fn apply(&self, mut assets: Assets) -> Result<Assets> {
        let mut report = ValidationReport::new();
        self.check(&mut report);
        report.into_result(())?;
        if let Some(periods) = self.periods_per_year {
            assets = assets.with_periods_per_year(periods);
        }
        if let Some(method) = self.covariance {
            assets = assets.with_covariance(method);
        }
        if let Some(rate) = self.risk_free_rate {
            assets = assets.with_risk_free_rate(rate);
        }
        if let Some(asset) = &self.risk_free_asset {
            assets = assets.with_risk_free_asset(asset)?;
        }
        Ok(assets)
    }
}

fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}
