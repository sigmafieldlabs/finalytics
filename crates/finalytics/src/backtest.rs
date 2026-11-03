//! Backtests over [`Assets`]: optimisation methods refitted through time, or fixed weights,
//! rebalanced on a schedule, net of trading costs, and validated in sample, on a holdout or
//! walk-forward.
//!
//! [`Assets::compare`] backtests several methods side by side and builds an
//! [`OptimisationReport`]; [`Assets::backtest`] backtests one method or one set of weights and
//! builds a [`BacktestReport`]. Both are builders. Each report carries the backtest behind it,
//! every return and trade, as `results`. [`Assets::optimisation_report`] and
//! [`Assets::backtest_report`] run the same backtests from [`BacktestOptions`], as JSON gives
//! them.
//!
//! ```
//! use finalytics::backtest::{Rebalance, Years};
//! use finalytics::{Assets, Method, datasets};
//!
//! let assets = Assets::builder()
//!     .prices(datasets::asset_class_prices())
//!     .risk_free_asset("T-Bills")
//!     .build()?;
//! let report = assets
//!     .compare([Method::EqualWeight, Method::MinimumVariance])
//!     .max_weight(0.4)
//!     .rebalance(Rebalance::Quarterly)
//!     .walk_forward(Years(5))
//!     .trading_cost_bps(10.0)
//!     .build()?;
//! let minimum_variance = &report.strategies[1];
//! assert_eq!(minimum_variance.name, "Minimum variance");
//! assert_eq!(minimum_variance.tested.start, assets.dates()[60]);
//! assert!(minimum_variance.tested.statistics.volatility < 0.1);
//! let returns = &report.results.strategies[1].out_of_sample.as_ref().unwrap().returns;
//! assert_eq!(returns.len(), assets.dates().len() - 60);
//! # Ok::<(), finalytics::Error>(())
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_backtest::{self as engine, Fit, Market, Plan};
use finalytics_core::{Date, Error, Result, ValidationReport};
use finalytics_reports::BacktestSetup;
use finalytics_risk::PriceTable;

pub use finalytics_backtest::{
    Backtest, Failure, Rebalance, Run, Settings, StrategyResult, Trade, Validation,
};
pub use finalytics_reports::{
    BacktestContext, BacktestReport, CalendarReturn, History as RunHistory, OptimisationReport,
    RunReport, StrategyReport, WeightsAt,
};

use crate::assets::{AssetGroup, Assets, Method, OptimiseOptions};
use finalytics_opt::Bound;

/// A strategy to backtest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Strategy {
    /// An optimisation, refitted as the validation says. The limits of `options` apply to the
    /// methods that take them; equal weight, inverse volatility, risk parity, risk budgeting
    /// and hierarchical risk parity are fully invested and long only.
    Optimise {
        /// The name in the results; the method's own by default, such as `Maximum Sharpe`.
        #[serde(default)]
        name: Option<String>,
        /// The method, its parameters and the limits on the weights.
        #[serde(default)]
        options: Box<OptimiseOptions>,
    },
    /// Fixed weights by asset, adding up to one. The risk-free asset, if named, is held as cash.
    Weights {
        /// The name in the results: `Weights` by default.
        #[serde(default)]
        name: Option<String>,
        /// The weights, by asset.
        weights: BTreeMap<String, f64>,
    },
}

impl Strategy {
    /// The optimisation `options`, under the method's own name.
    pub fn optimise(options: OptimiseOptions) -> Strategy {
        Strategy::Optimise {
            name: None,
            options: Box::new(options),
        }
    }

    /// Fixed `weights` by asset, under `name`.
    pub fn weights<S: Into<String>>(
        name: &str,
        weights: impl IntoIterator<Item = (S, f64)>,
    ) -> Strategy {
        Strategy::Weights {
            name: Some(name.to_owned()),
            weights: weights.into_iter().map(|(a, w)| (a.into(), w)).collect(),
        }
    }

    fn name(&self) -> String {
        match self {
            Strategy::Optimise {
                name: Some(name), ..
            }
            | Strategy::Weights {
                name: Some(name), ..
            } => name.clone(),
            Strategy::Optimise {
                name: None,
                options,
            } => options.method.label().to_owned(),
            Strategy::Weights { name: None, .. } => "Weights".to_owned(),
        }
    }
}

/// What the strategies are compared with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Benchmark {
    /// Fixed weights by asset, held and rebalanced as the strategies are, with the same costs.
    Weights {
        /// The name in the results: `Benchmark` by default.
        #[serde(default)]
        name: Option<String>,
        /// The weights, by asset, adding up to one.
        weights: BTreeMap<String, f64>,
    },
    /// An index or fund, by its prices: one column, with a price on or before each date of the
    /// assets and one before the first.
    Prices {
        /// The name in the results: the column's by default.
        #[serde(default)]
        name: Option<String>,
        /// The prices.
        prices: PriceTable,
    },
}

/// What a backtest tests and how.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct BacktestOptions {
    /// The strategies, in the order of the results.
    pub strategies: Vec<Strategy>,
    /// How the strategies are validated: in sample by default.
    pub validation: Validation,
    /// When the portfolio trades back to its targets: quarterly by default.
    pub rebalance: Rebalance,
    /// The cost of trading, in basis points of the value traded: 10 is 0.1%.
    pub trading_cost_bps: f64,
    /// What the strategies are compared with, if anything.
    pub benchmark: Option<Benchmark>,
}

impl Assets {
    /// Backtests the strategies of `options` on these assets: each held from its start,
    /// rebalanced on the schedule, refitted as the validation says, and net of trading costs.
    /// Optimisations use the assets' settings on each window of history: the periods per year,
    /// the covariance estimator, and the risk-free asset's mean return over the window (or the
    /// risk-free rate). Cash, held before a first fit succeeds, earns the risk-free return.
    fn simulate(&self, options: &BacktestOptions) -> Result<Backtest> {
        let returns = self.risky_returns()?;
        let labels = returns.labels().to_vec();
        let cash = self.cash_returns();
        let mut report = ValidationReport::new();
        let mut strategies = Vec::with_capacity(options.strategies.len());
        for (i, strategy) in options.strategies.iter().enumerate() {
            let name = strategy.name();
            let path = format!("strategies[{i}]");
            match strategy {
                Strategy::Optimise {
                    options: method, ..
                } => {
                    let (method, notes) = applicable(method);
                    strategies.push(engine::Strategy {
                        name,
                        plan: Plan::Fit(Box::new(move |rows| {
                            let result = self.window(rows)?.optimise_with(&method)?;
                            Ok(match result.weights {
                                Some(weights) => Fit::Weights(
                                    self.risky_assets()
                                        .iter()
                                        .map(|a| weights.get(*a).copied().unwrap_or(0.0))
                                        .collect(),
                                ),
                                None => Fit::NoSolution(result.status.to_string()),
                            })
                        })),
                        notes,
                    });
                }
                Strategy::Weights { weights, .. } => {
                    let fixed = self.fixed(weights, &format!("{path}.weights"), &mut report);
                    strategies.push(engine::Strategy {
                        name,
                        plan: Plan::Weights(fixed),
                        notes: Vec::new(),
                    });
                }
            }
        }
        let benchmark = match &options.benchmark {
            None => None,
            Some(Benchmark::Weights { name, weights }) => Some((
                name.clone().unwrap_or_else(|| "Benchmark".to_owned()),
                engine::Benchmark::Weights(self.fixed(weights, "benchmark.weights", &mut report)),
            )),
            Some(Benchmark::Prices { name, prices }) => {
                let label = prices.labels().first().cloned().unwrap_or_default();
                Some((
                    name.clone().unwrap_or(label),
                    engine::Benchmark::Returns(self.aligned(prices, &mut report)),
                ))
            }
        };
        report.into_result(())?;
        let settings = Settings::new(options.validation.clone())
            .with_rebalance(options.rebalance)
            .with_trading_cost_bps(options.trading_cost_bps);
        let market = Market::new(self.dates(), &returns, &cash, self.periods_per_year());
        let mut backtest = engine::backtest(&market, strategies, benchmark, &settings)?;
        backtest.assets = labels;
        Ok(backtest)
    }

    /// The optimisation report on the backtest of `options`: every strategy, scored on the
    /// periods it was tested on, side by side. Each strategy is held from its start, rebalanced
    /// on the schedule, refitted as the validation says, and net of trading costs.
    /// Optimisations use the assets' settings on each window of history: the periods per year,
    /// the covariance estimator, and the risk-free asset's mean return over the window (or the
    /// risk-free rate). Cash, held before a first fit succeeds, earns the risk-free return.
    pub fn optimisation_report(&self, options: &BacktestOptions) -> Result<OptimisationReport> {
        let backtest = self.simulate(options)?;
        finalytics_reports::optimisation_report(
            &backtest,
            &self.risky_returns()?,
            &self.setup(options),
        )
    }

    /// The backtest report on the first strategy of `options`, against the benchmark.
    pub fn backtest_report(&self, options: &BacktestOptions) -> Result<BacktestReport> {
        let backtest = self.simulate(options)?;
        finalytics_reports::backtest_report(&backtest, &self.risky_returns()?, &self.setup(options))
    }

    /// What the reports on the backtest of `options` say about how it was set up.
    fn setup(&self, options: &BacktestOptions) -> BacktestSetup {
        let limits = options
            .strategies
            .iter()
            .find_map(|s| match s {
                Strategy::Optimise { options, .. } => Some(limits(options)),
                Strategy::Weights { .. } => None,
            })
            .unwrap_or_default();
        BacktestSetup::new(
            self.risk_free_asset().map(str::to_owned),
            self.covariance_method(),
            limits,
        )
    }

    /// Fixed weights by asset as one weight per risky asset, the risk-free asset as cash.
    fn fixed(
        &self,
        weights: &BTreeMap<String, f64>,
        path: &str,
        report: &mut ValidationReport,
    ) -> Vec<f64> {
        let risky = self.risky_assets();
        for asset in weights.keys() {
            if !risky.contains(&asset.as_str()) && self.risk_free_asset() != Some(asset.as_str()) {
                report.error(format!("{path}.{asset}"), format!("unknown asset {asset}"));
            }
        }
        let total: f64 = weights.values().sum();
        if (total - 1.0).abs() > 1e-6 {
            report.error(
                path,
                format!("add up to {total}, not one; give the risk-free asset any cash"),
            );
        }
        risky
            .iter()
            .map(|a| weights.get(*a).copied().unwrap_or(0.0))
            .collect()
    }

    /// The return of `prices` over each of the assets' periods.
    fn aligned(&self, prices: &PriceTable, report: &mut ValidationReport) -> Vec<f64> {
        if prices.labels().len() != 1 {
            report.error(
                "benchmark.prices",
                format!(
                    "has {} columns: give the benchmark's prices alone",
                    prices.labels().len()
                ),
            );
            return Vec::new();
        }
        let known: Vec<(Date, f64)> = prices
            .dates()
            .iter()
            .zip(&prices.columns()[0])
            .filter_map(|(d, p)| p.map(|p| (*d, p)))
            .collect();
        // The last price on or before `date`, or strictly before it.
        let price = |date: Date, strictly: bool| {
            let k = known.partition_point(|(d, _)| if strictly { *d < date } else { *d <= date });
            k.checked_sub(1).map(|k| known[k].1)
        };
        let dates = self.dates();
        let mut out = Vec::with_capacity(dates.len());
        for (t, date) in dates.iter().enumerate() {
            let before = if t == 0 {
                price(*date, true)
            } else {
                price(dates[t - 1], false)
            };
            match (before, price(*date, false)) {
                (Some(p0), Some(p1)) if p0 > 0.0 => out.push(p1 / p0 - 1.0),
                _ => {
                    report.error(
                        "benchmark.prices",
                        format!("has no price for the period ending {date}"),
                    );
                    return Vec::new();
                }
            }
        }
        out
    }
}

/// The limits of `options` in words.
fn limits(options: &OptimiseOptions) -> Vec<String> {
    let pct = |x: f64| {
        let text = format!("{:.2}", x * 100.0);
        let text = text.trim_end_matches('0').trim_end_matches('.');
        format!("{}%", text.replace('-', "−"))
    };
    let mut out = Vec::new();
    if let Some(max) = options.max_weight {
        out.push(format!("At most {} in any asset", pct(max)));
    }
    if let Some(b) = options.bounds {
        out.push(format!("{} to {} in each asset", pct(b.min), pct(b.max)));
    }
    for (asset, b) in &options.asset_bounds {
        out.push(format!("{asset}: {} to {}", pct(b.min), pct(b.max)));
    }
    for (name, g) in &options.groups {
        out.push(format!(
            "{name} ({}): {} to {} together",
            g.assets.join(", "),
            pct(g.min),
            pct(g.max)
        ));
    }
    out
}

/// The options a method takes, and notes on any limits it does not.
fn applicable(options: &OptimiseOptions) -> (OptimiseOptions, Vec<String>) {
    let mut method = options.clone();
    let mut notes = Vec::new();
    if !options.method.takes_limits() {
        let limited = options.bounds.is_some()
            || options.max_weight.is_some()
            || !options.asset_bounds.is_empty()
            || !options.groups.is_empty();
        method.bounds = None;
        method.max_weight = None;
        method.asset_bounds.clear();
        method.groups.clear();
        if limited {
            notes.push(
                "Fully invested and long only; the limits on weights do not apply.".to_owned(),
            );
        }
    }
    (method, notes)
}

impl From<Method> for Strategy {
    fn from(method: Method) -> Strategy {
        Strategy::optimise(OptimiseOptions {
            method,
            ..OptimiseOptions::default()
        })
    }
}

impl<S: Into<String>, const N: usize> From<[(S, f64); N]> for Strategy {
    fn from(weights: [(S, f64); N]) -> Strategy {
        Strategy::Weights {
            name: None,
            weights: weights.into_iter().map(|(a, w)| (a.into(), w)).collect(),
        }
    }
}

impl From<BTreeMap<String, f64>> for Strategy {
    fn from(weights: BTreeMap<String, f64>) -> Strategy {
        Strategy::Weights {
            name: None,
            weights,
        }
    }
}

impl<S: Into<String>, const N: usize> From<[(S, f64); N]> for Benchmark {
    fn from(weights: [(S, f64); N]) -> Benchmark {
        Benchmark::Weights {
            name: None,
            weights: weights.into_iter().map(|(a, w)| (a.into(), w)).collect(),
        }
    }
}

impl From<PriceTable> for Benchmark {
    fn from(prices: PriceTable) -> Benchmark {
        Benchmark::Prices { name: None, prices }
    }
}

/// A walk-forward window in years: `Years(5)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Years(pub u32);

/// A walk-forward window in months: `Months(18)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Months(pub u32);

/// A walk-forward window in periods of the data: `Periods(60)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periods(pub usize);

/// A length of history: [`Years`], [`Months`] or [`Periods`].
pub trait History {
    /// Walk-forward validation on a window of this length.
    fn walk_forward(self, expanding: bool) -> Validation;
}

impl History for Years {
    fn walk_forward(self, expanding: bool) -> Validation {
        Validation::WalkForward {
            years: Some(f64::from(self.0)),
            periods: None,
            expanding,
        }
    }
}

impl History for Months {
    fn walk_forward(self, expanding: bool) -> Validation {
        Validation::WalkForward {
            years: Some(f64::from(self.0) / 12.0),
            periods: None,
            expanding,
        }
    }
}

impl History for Periods {
    fn walk_forward(self, expanding: bool) -> Validation {
        Validation::WalkForward {
            years: None,
            periods: Some(self.0),
            expanding,
        }
    }
}

/// What [`Compare`] and [`BacktestBuilder`] share: the limits and parameters of the methods,
/// and how the backtest runs.
#[derive(Debug, Clone, Default)]
struct Common {
    options: OptimiseOptions,
    validation: Validation,
    holdout: Option<String>,
    rebalance: Rebalance,
    trading_cost_bps: f64,
    benchmark: Option<Benchmark>,
}

impl Common {
    /// The options for `strategies`, with the shared limits given to each optimisation.
    fn options(&self, strategies: Vec<Strategy>) -> Result<BacktestOptions> {
        let mut validation = self.validation.clone();
        if let Some(text) = &self.holdout {
            match text.parse::<Date>() {
                Ok(start) => validation = Validation::Holdout { start },
                Err(_) => {
                    let mut report = ValidationReport::new();
                    report.error(
                        "validation.start",
                        format!("{text:?} is not a date: write it as 2016-11-01"),
                    );
                    return Err(Error::Validation(report));
                }
            }
        }
        let strategies = strategies
            .into_iter()
            .map(|strategy| match strategy {
                Strategy::Optimise { name, options } => {
                    let mut shared = self.options.clone();
                    shared.method = options.method;
                    Strategy::Optimise {
                        name,
                        options: Box::new(shared),
                    }
                }
                weights => weights,
            })
            .collect();
        Ok(BacktestOptions {
            strategies,
            validation,
            rebalance: self.rebalance,
            trading_cost_bps: self.trading_cost_bps,
            benchmark: self.benchmark.clone(),
        })
    }
}

/// Setters shared by the comparison and backtest builders.
macro_rules! common_setters {
    ($builder:ident) => {
        impl $builder<'_> {
            /// The largest weight of any asset, such as 0.4 for 40%.
            pub fn max_weight(mut self, max: f64) -> Self {
                self.common.options.max_weight = Some(max);
                self
            }

            /// Every asset's weight between `min` and `max`.
            pub fn bounds(mut self, min: f64, max: f64) -> Self {
                self.common.options.bounds = Some(Bound { min, max });
                self
            }

            /// One asset's weight between `min` and `max`.
            pub fn asset_bounds(mut self, asset: &str, min: f64, max: f64) -> Self {
                self.common
                    .options
                    .asset_bounds
                    .insert(asset.to_owned(), Bound { min, max });
                self
            }

            /// The total weight of the `assets` of group `name` between `min` and `max`, such
            /// as a sector's.
            pub fn group<S: AsRef<str>>(
                mut self,
                name: &str,
                assets: &[S],
                min: f64,
                max: f64,
            ) -> Self {
                self.common.options.groups.insert(
                    name.to_owned(),
                    AssetGroup {
                        assets: assets.iter().map(|a| a.as_ref().to_owned()).collect(),
                        min,
                        max,
                    },
                );
                self
            }

            /// The annual expected return or volatility to aim for, for target return and
            /// target volatility.
            pub fn target(mut self, target: f64) -> Self {
                self.common.options.target = Some(target);
                self
            }

            /// The weight on variance, for mean–variance methods.
            pub fn risk_aversion(mut self, risk_aversion: f64) -> Self {
                self.common.options.risk_aversion = Some(risk_aversion);
                self
            }

            /// The confidence of the conditional value at risk, for minimum CVaR.
            pub fn confidence(mut self, confidence: f64) -> Self {
                self.common.options.confidence = Some(confidence);
                self
            }

            /// Weights fitted once to the whole history and tested on it (the default).
            pub fn in_sample(mut self) -> Self {
                self.common.validation = Validation::InSample;
                self.common.holdout = None;
                self
            }

            /// Weights fitted to the history before `start`, a date such as `"2016-11-01"`,
            /// and tested on the history from it on.
            pub fn holdout_from(mut self, start: &str) -> Self {
                self.common.holdout = Some(start.to_owned());
                self
            }

            /// Weights refitted at each calendar rebalance on the last `window` of history,
            /// such as `Years(5)`, and tested on what follows.
            pub fn walk_forward(mut self, window: impl History) -> Self {
                self.common.validation = window.walk_forward(false);
                self.common.holdout = None;
                self
            }

            /// Weights refitted at each calendar rebalance on all the history so far, from a
            /// first `window`.
            pub fn walk_forward_expanding(mut self, window: impl History) -> Self {
                self.common.validation = window.walk_forward(true);
                self.common.holdout = None;
                self
            }

            /// When the portfolio trades back to its targets: quarterly by default.
            pub fn rebalance(mut self, rebalance: Rebalance) -> Self {
                self.common.rebalance = rebalance;
                self
            }

            /// The cost of trading, in basis points of the value traded: 10 is 0.1%.
            pub fn trading_cost_bps(mut self, bps: f64) -> Self {
                self.common.trading_cost_bps = bps;
                self
            }

            /// What to compare with: weights by asset, such as
            /// `[("US Equity", 0.6), ("Bonds", 0.4)]`, or an index's prices.
            pub fn benchmark(mut self, benchmark: impl Into<Benchmark>) -> Self {
                self.common.benchmark = Some(benchmark.into());
                self
            }
        }
    };
}

/// Several methods backtested side by side on the same assets; [`Compare::build`] runs them
/// and builds the optimisation report.
#[derive(Debug, Clone)]
#[must_use = "a comparison does nothing until `build` runs it"]
pub struct Compare<'a> {
    assets: &'a Assets,
    methods: Vec<Method>,
    common: Common,
    title: Option<String>,
}

/// One method or one set of weights backtested; [`BacktestBuilder::build`] runs it and builds
/// the backtest report.
#[derive(Debug, Clone)]
#[must_use = "a backtest does nothing until `build` runs it"]
pub struct BacktestBuilder<'a> {
    assets: &'a Assets,
    strategy: Strategy,
    common: Common,
}

common_setters!(Compare);
common_setters!(BacktestBuilder);

impl Compare<'_> {
    /// The report's title, such as `"S&P 500 stocks"`.
    pub fn name(mut self, name: &str) -> Self {
        self.title = Some(name.to_owned());
        self
    }

    /// The options of the comparison, as [`Assets::optimisation_report`] takes them.
    pub fn options(&self) -> Result<BacktestOptions> {
        let strategies = self.methods.iter().map(|m| Strategy::from(*m)).collect();
        self.common.options(strategies)
    }

    /// Runs the comparison and builds the optimisation report, with every return and trade
    /// behind it in `results`.
    pub fn build(&self) -> Result<OptimisationReport> {
        let mut report = self.assets.optimisation_report(&self.options()?)?;
        report.name.clone_from(&self.title);
        Ok(report)
    }
}

impl BacktestBuilder<'_> {
    /// The name of the strategy in the results, in place of the method's or `Weights`.
    pub fn name(mut self, name: &str) -> Self {
        match &mut self.strategy {
            Strategy::Optimise { name: n, .. } | Strategy::Weights { name: n, .. } => {
                *n = Some(name.to_owned());
            }
        }
        self
    }

    /// The options of the backtest, as [`Assets::backtest_report`] takes them.
    pub fn options(&self) -> Result<BacktestOptions> {
        self.common.options(vec![self.strategy.clone()])
    }

    /// Runs the backtest and builds the backtest report, titled with the strategy's name, with
    /// every return and trade behind it in `results`.
    pub fn build(&self) -> Result<BacktestReport> {
        self.assets.backtest_report(&self.options()?)
    }
}

impl Assets {
    /// A comparison of `methods`, each backtested with the same limits, rebalancing,
    /// validation and costs.
    pub fn compare(&self, methods: impl IntoIterator<Item = Method>) -> Compare<'_> {
        Compare {
            assets: self,
            methods: methods.into_iter().collect(),
            common: Common::default(),
            title: None,
        }
    }

    /// A backtest of one strategy: a method, such as `Method::MaximumSharpe`, or weights by
    /// asset, such as `[("US Equity", 0.6), ("Bonds", 0.4)]`.
    pub fn backtest(&self, strategy: impl Into<Strategy>) -> BacktestBuilder<'_> {
        BacktestBuilder {
            assets: self,
            strategy: strategy.into(),
            common: Common::default(),
        }
    }
}
