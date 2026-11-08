//! Backtests over [`Assets`]: optimisation methods refitted through time, or fixed weights,
//! rebalanced on a schedule, net of trading costs, and validated in sample, on a holdout or
//! walk-forward.
//!
//! [`Assets::compare`] backtests several methods side by side and gives a [`Comparison`];
//! [`Assets::backtest`] backtests one method or one set of weights and gives a [`Backtest`].
//! Both are builders, and both results have the same shape: the report in `report`, every
//! return and trade in `results`; printing one prints its report, and `show()` shows it as a
//! page. [`Assets::compare_with`] and [`Assets::backtest_with`] run the same backtests from
//! [`BacktestOptions`], as JSON gives them.
//!
//! ```
//! use finalytics::backtest::{Rebalance, Years};
//! use finalytics::{Assets, Method, datasets};
//!
//! let assets = Assets::builder()
//!     .prices(datasets::asset_class_prices())
//!     .risk_free_asset("T-Bills")
//!     .build()?;
//! let result = assets
//!     .compare([Method::EqualWeight, Method::MinimumVariance])
//!     .max_weight(0.4)
//!     .rebalance(Rebalance::Quarterly)
//!     .walk_forward(Years(5))
//!     .trading_cost_bps(10.0)
//!     .build()?;
//! let minimum_variance = &result.report.strategies[1];
//! assert_eq!(minimum_variance.name, "Minimum variance");
//! assert_eq!(minimum_variance.tested.start, assets.dates()[60]);
//! assert!(minimum_variance.tested.statistics.volatility < 0.1);
//! let returns = &result.results.strategies[1].out_of_sample.as_ref().unwrap().returns;
//! assert_eq!(returns.len(), assets.dates().len() - 60);
//! # Ok::<(), finalytics::Error>(())
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_backtest::{self as engine, Fit, Market, Plan};
use finalytics_core::{Date, Error, Result, ValidationReport};
use finalytics_reports::BacktestSetup;
pub use finalytics_reports::{CashFlow, MoneyFigures};
use finalytics_risk::PriceTable;

pub use finalytics_backtest::{
    BacktestResults, Failure, Rebalance, Run, Settings, StrategyResult, Trade, Validation,
};
pub use finalytics_reports::{
    BacktestContext, BacktestReport, CalendarReturn, ComparisonReport, History as RunHistory,
    RunReport, StrategyReport, WeightsAt,
};

use crate::assets::{AssetGroup, Assets, Method, OptimiseOptions};

/// A comparison: the comparison report on several methods backtested side by side, and the
/// raw results. Printing it prints the report, and showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Comparison {
    /// The comparison report: each method scored on the periods it was tested on.
    pub report: ComparisonReport,
    /// The raw results: every return and trade of each method, in and out of sample.
    pub results: BacktestResults,
}

/// A backtest: the backtest report on one allocation against its benchmark, and the raw
/// results. Printing it prints the report, and showing it shows the report as a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Backtest {
    /// The backtest report.
    pub report: BacktestReport,
    /// The raw results: every return and trade, in and out of sample.
    pub results: BacktestResults,
}

impl std::fmt::Display for Comparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl std::fmt::Display for Backtest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}
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
    /// The value each run starts with, to follow money as well as returns: the reports then
    /// give each run's ending value and money-weighted return. Zero, with flows, if absent.
    pub start_value: Option<f64>,
    /// Money paid in (positive) or taken out (negative) on dates, at the end of the period each
    /// falls in, after its return.
    pub flows: Vec<CashFlow>,
}

impl Assets {
    /// Backtests the strategies of `options` on these assets: each held from its start,
    /// rebalanced on the schedule, refitted as the validation says, and net of trading costs.
    /// Optimisations use the assets' settings on each window of history: the periods per year,
    /// the covariance estimator, and the risk-free asset's mean return over the window (or the
    /// risk-free rate). Cash, held before a first fit succeeds, earns the risk-free return.
    ///
    /// With `single`, one strategy was asked for, so that an option its method does not use is
    /// a problem. A comparison applies each option to the strategies that use it and notes,
    /// for each other strategy given it, that it does not apply; an option that no strategy
    /// given it uses is a problem, naming them.
    fn simulate(&self, options: &BacktestOptions, single: bool) -> Result<BacktestResults> {
        let returns = self.risky_returns()?;
        let labels = returns.labels().to_vec();
        let cash = self.cash_returns();
        let mut report = ValidationReport::new();
        if !single {
            unused_options(&options.strategies, &mut report);
        }
        let walk_forward = matches!(options.validation, Validation::WalkForward { .. });
        trade_limits(&options.strategies, walk_forward, &mut report);
        let mut strategies = Vec::with_capacity(options.strategies.len());
        for (i, strategy) in options.strategies.iter().enumerate() {
            let name = strategy.name();
            let path = format!("strategies[{i}]");
            match strategy {
                Strategy::Optimise {
                    options: method, ..
                } => {
                    // The options the strategies share are checked once: their problems are
                    // the same for each. Trades are limited from the weights each refit holds,
                    // which the run supplies, so they are checked from any.
                    let mut to_check = if single {
                        (**method).clone()
                    } else {
                        applicable(method).0
                    };
                    let trading = trades_limited(&to_check);
                    if trading && to_check.current_weights.is_empty() {
                        let risky = self.risky_assets();
                        let share = 1.0 / risky.len().max(1) as f64;
                        to_check.current_weights =
                            risky.iter().map(|a| ((*a).to_owned(), share)).collect();
                    }
                    let checked = self.check_options(&to_check, single);
                    for issue in checked.issues() {
                        if !report.issues().contains(issue) {
                            report.merge("", single_issue(issue));
                        }
                    }
                    let (method, notes) = applicable(method);
                    let weights_of = move |result: crate::OptimisationResult| match result.weights {
                        Some(weights) => Fit::Weights(
                            self.risky_assets()
                                .iter()
                                .map(|a| weights.get(*a).copied().unwrap_or(0.0))
                                .collect(),
                        ),
                        None => Fit::NoSolution(result.status.to_string().replace('_', " ")),
                    };
                    let plan = if trading {
                        Plan::Refit(Box::new(move |rows, held: Option<&[f64]>| {
                            let mut options = method.clone();
                            match held {
                                // From the weights held, the risk-free asset holding the rest.
                                Some(held) => {
                                    options.current_weights = self
                                        .risky_assets()
                                        .iter()
                                        .map(|a| (*a).to_owned())
                                        .zip(held.iter().copied())
                                        .collect();
                                    let cash = 1.0 - held.iter().sum::<f64>();
                                    if let Some(rf) = self.risk_free_asset()
                                        && cash > 1e-12
                                    {
                                        options.current_weights.insert(rf.to_owned(), cash);
                                    } else if cash > 1e-9 {
                                        // Without a risk-free asset, the cash a refit holds is
                                        // left out of the turnover: the weights it starts from
                                        // are the assets' shares of what is invested.
                                        let invested = 1.0 - cash;
                                        for w in options.current_weights.values_mut() {
                                            *w /= invested;
                                        }
                                    }
                                }
                                // A first fit enters from cash, without limits on its trades.
                                None => {
                                    for name in ["max_turnover", "min_trade", "max_trades"] {
                                        crate::methods::without(&mut options, name);
                                    }
                                }
                            }
                            Ok(weights_of(self.window(rows)?.solve(&options)?))
                        }))
                    } else {
                        Plan::Fit(Box::new(move |rows| {
                            Ok(weights_of(self.window(rows)?.solve(&method)?))
                        }))
                    };
                    strategies.push(engine::Strategy { name, plan, notes });
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
        if let Some(v) = options.start_value
            && !(v.is_finite() && v >= 0.0)
        {
            report.error("start_value", "must be zero or more");
        }
        for (i, f) in options.flows.iter().enumerate() {
            if !f.amount.is_finite() {
                report.error(format!("flows[{i}].amount"), "must be a number");
            }
        }
        let settings = Settings::new(options.validation.clone())
            .with_rebalance(options.rebalance)
            .with_trading_cost_bps(options.trading_cost_bps);
        let market = Market::new(self.dates(), &returns, &cash, self.periods_per_year());
        // The settings' problems with the strategies', all together.
        if report.has_errors() {
            report.merge("", engine::check_settings(&market, &settings));
        }
        report.into_result(())?;
        let mut backtest = engine::backtest(&market, strategies, benchmark, &settings)?;
        backtest.assets = labels;
        Ok(backtest)
    }

    /// The comparison of the strategies of `options`: each backtested, scored on the periods it
    /// was tested on and set side by side in the report, with every return and trade in the
    /// results. Each strategy is held from its start, rebalanced on the schedule, refitted as
    /// the validation says, and net of trading costs. Optimisations use the assets' settings on
    /// each window of history: the periods per year, the covariance estimator, and the
    /// risk-free asset's mean return over the window (or the risk-free rate). Cash, held before
    /// a first fit succeeds, earns the risk-free return. [`Assets::compare`] builds the same
    /// options step by step.
    pub fn compare_with(&self, options: &BacktestOptions) -> Result<Comparison> {
        let results = self.simulate(options, false)?;
        let report = finalytics_reports::comparison_report(
            &results,
            &self.risky_returns()?,
            &self.setup(options),
        )?;
        Ok(Comparison { report, results })
    }

    /// The backtest of the first strategy of `options`, against the benchmark: the backtest
    /// report, with every return and trade in the results. [`Assets::backtest`] builds the same
    /// options step by step.
    pub fn backtest_with(&self, options: &BacktestOptions) -> Result<Backtest> {
        let results = self.simulate(options, true)?;
        let report = finalytics_reports::backtest_report(
            &results,
            &self.risky_returns()?,
            &self.setup(options),
        )?;
        Ok(Backtest { report, results })
    }

    /// What the reports on the backtest of `options` say about how it was set up.
    fn setup(&self, options: &BacktestOptions) -> BacktestSetup {
        let limits = options
            .strategies
            .iter()
            .find_map(|s| match s {
                Strategy::Optimise { options, .. } => Some(limits(options, true)),
                Strategy::Weights { .. } => None,
            })
            .unwrap_or_default();
        BacktestSetup::new(
            self.risk_free_asset().map(str::to_owned),
            self.covariance_method(),
            limits,
        )
        .with_mean_returns(self.mean_method())
        .with_origin(self.origin())
        .with_money(options.start_value, options.flows.clone())
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

/// A share as a percentage in words, such as "12.5%", with at most two decimals.
pub(crate) fn percent_text(x: f64) -> String {
    let text = format!("{:.2}", x * 100.0);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{}%", text.replace('-', "−"))
}

/// The limits of `options` in words: of one optimisation from the current weights, or, with
/// `refits`, of a backtest's refits.
pub(crate) fn limits(options: &OptimiseOptions, refits: bool) -> Vec<String> {
    let pct = percent_text;
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
            g.classification
                .clone()
                .unwrap_or_else(|| g.assets.join(", ")),
            pct(g.min),
            pct(g.max)
        ));
    }
    if let Some(target) = options.target.filter(|_| options.method.uses_scenarios()) {
        out.push(format!("Expected return at least {} a year", pct(target)));
    }
    if let Some(k) = options.min_effective_assets {
        let text = format!("{k:.2}");
        out.push(format!(
            "At least {} effective assets",
            text.trim_end_matches('0').trim_end_matches('.')
        ));
    }
    if let Some(t) = options.max_turnover {
        out.push(if refits {
            format!("At most {} turnover at each refit", pct(t))
        } else {
            format!("At most {} turnover from the current weights", pct(t))
        });
    }
    if let Some(m) = options.min_trade {
        out.push(format!(
            "Trades of at least {} of the portfolio, or none",
            pct(m)
        ));
    }
    if let Some(k) = options.max_trades {
        out.push(format!(
            "At most {k} {}{}",
            if k == 1 { "trade" } else { "trades" },
            if refits { " at each refit" } else { "" }
        ));
    }
    if let Some(te) = options.max_tracking_error {
        out.push(format!(
            "Tracking error at most {} a year against the benchmark weights",
            pct(te)
        ));
    }
    out
}

/// A report of one issue.
fn single_issue(issue: &finalytics_core::Issue) -> ValidationReport {
    let mut one = ValidationReport::new();
    match issue.severity {
        finalytics_core::Severity::Error => one.error(issue.path.clone(), issue.message.clone()),
        _ => one.warning(issue.path.clone(), issue.message.clone()),
    }
    one
}

/// The options a method takes, and notes on those given that it does not: the limits on
/// weights, for a method fully invested and long only, and the parameters of other methods.
fn applicable(options: &OptimiseOptions) -> (OptimiseOptions, Vec<String>) {
    let mut method = options.clone();
    let mut notes = Vec::new();
    let info = options.method.info();
    let given = crate::methods::given(options);
    let mut limited = false;
    let mut unused = Vec::new();
    for name in given.iter().filter(|name| !info.takes(name)) {
        crate::methods::without(&mut method, name);
        if crate::methods::is_limit(name) {
            limited = true;
        } else {
            unused.push(crate::methods::in_words(name));
        }
    }
    if limited {
        notes.push("Fully invested and long only; the limits on weights do not apply.".to_owned());
    }
    if !unused.is_empty() {
        let verb = if unused.len() == 1 { "does" } else { "do" };
        let text = crate::methods::listed(&unused);
        let mut chars = text.chars();
        let text: String = chars
            .next()
            .map(|c| c.to_uppercase().chain(chars).collect())
            .unwrap_or_default();
        notes.push(format!("{text} {verb} not apply."));
    }
    (method, notes)
}

/// Whether `options` limit the trades from the weights held: by turnover, size or number.
fn trades_limited(options: &OptimiseOptions) -> bool {
    options.max_turnover.is_some() || options.min_trade.is_some() || options.max_trades.is_some()
}

/// Adds to `report` the problems with the trade limits of `strategies`: a backtest trades from
/// the weights it holds, so current weights are not given; and the limits apply at the refits
/// of a walk-forward validation, the only one that refits.
fn trade_limits(strategies: &[Strategy], walk_forward: bool, report: &mut ValidationReport) {
    let mut found = ValidationReport::new();
    for s in strategies {
        let Strategy::Optimise { options, .. } = s else {
            continue;
        };
        if !options.current_weights.is_empty() {
            found.error(
                "options.current_weights",
                "a backtest trades from the weights it holds at each refit, so it takes no \
                 current weights; they are for one optimisation",
            );
        }
        if !walk_forward {
            for name in ["max_turnover", "min_trade", "max_trades"] {
                if crate::methods::given(options).contains(&name) {
                    found.error(
                        format!("options.{name}"),
                        "limits the trades at each refit of a walk-forward validation; in sample \
                         and on a holdout, the weights are fitted once, from cash",
                    );
                }
            }
        }
    }
    for issue in found.issues() {
        if !report.issues().contains(issue) {
            report.merge("", single_issue(issue));
        }
    }
}

/// Adds to `report` each option given to the comparison's optimisations that none of them
/// uses, naming them all; an option one of them uses applies to it and is noted for the others.
fn unused_options(strategies: &[Strategy], report: &mut ValidationReport) {
    let fitted: Vec<(String, &OptimiseOptions)> = strategies
        .iter()
        .filter_map(|s| match s {
            Strategy::Optimise { options, .. } => Some((s.name(), options.as_ref())),
            Strategy::Weights { .. } => None,
        })
        .collect();
    let mut names: Vec<&'static str> = Vec::new();
    for (_, options) in &fitted {
        for name in crate::methods::given(options) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    for name in names {
        let given: Vec<&(String, &OptimiseOptions)> = fitted
            .iter()
            .filter(|(_, o)| crate::methods::given(o).contains(&name))
            .collect();
        if given.iter().any(|(_, o)| o.method.info().takes(name)) {
            continue;
        }
        let who: Vec<String> = given.iter().map(|(n, _)| lower_first(n)).collect();
        let one = who.len() == 1;
        let message = if crate::methods::is_limit(name) {
            format!(
                "{} {} fully invested and long only, and {} no limits on weights",
                crate::methods::listed(&who),
                if one { "is" } else { "are" },
                if one { "takes" } else { "take" },
            )
        } else if one && given[0].1.method == Method::RiskParity && name == "budgets" {
            "risk parity gives every asset the same budget; use risk budgeting for others"
                .to_owned()
        } else {
            format!(
                "{} {} not use {name}",
                crate::methods::listed(&who),
                if one { "does" } else { "do" },
            )
        };
        report.error(format!("options.{name}"), message);
    }
}

/// A strategy's name within a sentence: a method's own name in lower case ("minimum
/// variance"), a name the user gave as it is.
fn lower_first(name: &str) -> String {
    match Method::ALL.iter().find(|m| m.label() == name) {
        Some(m) => m.in_sentence(),
        None => name.to_owned(),
    }
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
    benchmark_name: Option<String>,
    start_value: Option<f64>,
    flows: Vec<(String, f64)>,
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
        if !strategies
            .iter()
            .any(|s| matches!(s, Strategy::Optimise { .. }))
        {
            let mut report = ValidationReport::new();
            for name in crate::methods::given(&self.options) {
                report.error(
                    format!("options.{name}"),
                    "fixed weights take no limits or parameters",
                );
            }
            report.into_result(())?;
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
        let mut benchmark = self.benchmark.clone();
        if let (
            Some(Benchmark::Weights { name, .. } | Benchmark::Prices { name, .. }),
            Some(given),
        ) = (&mut benchmark, &self.benchmark_name)
        {
            *name = Some(given.clone());
        } else if self.benchmark_name.is_some() {
            let mut report = ValidationReport::new();
            report.error(
                "benchmark_name",
                "names a benchmark: give the benchmark too",
            );
            return Err(Error::Validation(report));
        }
        let mut report = ValidationReport::new();
        let mut flows = Vec::with_capacity(self.flows.len());
        for (i, (text, amount)) in self.flows.iter().enumerate() {
            match text.parse::<Date>() {
                Ok(date) => flows.push(CashFlow {
                    date,
                    amount: *amount,
                }),
                Err(_) => report.error(
                    format!("flows[{i}].date"),
                    format!("{text:?} is not a date: write it as 2020-01-31"),
                ),
            }
        }
        report.into_result(())?;
        Ok(BacktestOptions {
            strategies,
            validation,
            rebalance: self.rebalance,
            trading_cost_bps: self.trading_cost_bps,
            benchmark,
            start_value: self.start_value,
            flows,
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
                        classification: None,
                        min,
                        max,
                    },
                );
                self
            }

            /// The total weight of the assets classed `class` under `scheme`, such as `Equity`
            /// under `asset_class`, between `min` and `max`.
            pub fn class_limit(mut self, scheme: &str, class: &str, min: f64, max: f64) -> Self {
                self.common.options.groups.insert(
                    class.to_owned(),
                    AssetGroup {
                        assets: Vec::new(),
                        classification: Some(scheme.to_owned()),
                        min,
                        max,
                    },
                );
                self
            }

            /// The annual expected return or volatility to aim for, for target return and
            /// target volatility, or the smallest annual expected return for minimum CVaR,
            /// semivariance and CDaR.
            pub fn target(mut self, target: f64) -> Self {
                self.common.options.target = Some(target);
                self
            }

            /// The smallest effective number of assets, `1 / sum w^2`, such as 5, for the
            /// methods that take limits on weights.
            pub fn min_effective_assets(mut self, k: f64) -> Self {
                self.common.options.min_effective_assets = Some(k);
                self
            }

            /// Resampled optimisation, for the methods that use estimates: the average of the
            /// optimal weights for `samples` histories, such as 200, simulated from the
            /// estimates at each fit.
            pub fn resample(mut self, samples: usize) -> Self {
                self.common.options.resample = Some(samples);
                self
            }

            /// The weight on variance, for mean–variance methods.
            pub fn risk_aversion(mut self, risk_aversion: f64) -> Self {
                self.common.options.risk_aversion = Some(risk_aversion);
                self
            }

            /// The confidence of the conditional value at risk or drawdown at risk, for minimum
            /// CVaR and CDaR.
            pub fn confidence(mut self, confidence: f64) -> Self {
                self.common.options.confidence = Some(confidence);
                self
            }

            /// The uncertainty of the expected returns, for robust mean–variance.
            pub fn uncertainty(mut self, uncertainty: finalytics_opt::ReturnUncertainty) -> Self {
                self.common.options.uncertainty = Some(uncertainty);
                self
            }

            /// Each asset's risk budget, for risk budgeting; equal shares without them.
            pub fn budget(mut self, asset: &str, budget: f64) -> Self {
                self.common
                    .options
                    .budgets
                    .get_or_insert_with(BTreeMap::new)
                    .insert(asset.to_owned(), budget);
                self
            }

            /// How hierarchical risk parity joins clusters: single linkage by default.
            pub fn linkage(mut self, linkage: finalytics_opt::Linkage) -> Self {
                self.common.options.linkage = Some(linkage);
                self
            }

            /// An asset's annual expected return, in place of its historical mean.
            pub fn expected_return(mut self, asset: &str, value: f64) -> Self {
                self.common
                    .options
                    .expected_returns
                    .insert(asset.to_owned(), value);
                self
            }

            /// The largest one-way turnover at each refit of a walk-forward validation, from
            /// the weights held then, such as 0.2; the first fit enters from cash without it.
            pub fn max_turnover(mut self, turnover: f64) -> Self {
                self.common.options.max_turnover = Some(turnover);
                self
            }

            /// The smallest trade at each refit of a walk-forward validation, as a weight, such
            /// as 0.01: each asset is traded by at least this much, or not at all.
            pub fn min_trade(mut self, weight: f64) -> Self {
                self.common.options.min_trade = Some(weight);
                self
            }

            /// The largest number of assets traded at each refit of a walk-forward validation.
            pub fn max_trades(mut self, count: usize) -> Self {
                self.common.options.max_trades = Some(count);
                self
            }

            /// A benchmark's weights by asset, for `max_tracking_error`, adding up to one with
            /// any cash on the risk-free asset.
            pub fn benchmark_weights<S: Into<String>>(
                mut self,
                weights: impl IntoIterator<Item = (S, f64)>,
            ) -> Self {
                self.common.options.benchmark_weights =
                    weights.into_iter().map(|(a, w)| (a.into(), w)).collect();
                self
            }

            /// The largest ex-ante tracking error a year against the benchmark weights, at
            /// every fit, such as 0.03.
            pub fn max_tracking_error(mut self, limit: f64) -> Self {
                self.common.options.max_tracking_error = Some(limit);
                self
            }

            /// An annual risk-free rate for the Sharpe ratio of the maximum Sharpe method, in place
            /// of the assets' own.
            pub fn risk_free_rate(mut self, rate: f64) -> Self {
                self.common.options.risk_free_rate = Some(rate);
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

            /// Weights refitted at each scheduled rebalance on the last `window` of history,
            /// such as `Years(5)`, and tested on what follows.
            pub fn walk_forward(mut self, window: impl History) -> Self {
                self.common.validation = window.walk_forward(false);
                self.common.holdout = None;
                self
            }

            /// Weights refitted at each scheduled rebalance on all the history so far, from a
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

            /// The benchmark's name in the results and reports: `Benchmark` by default, or the
            /// column's name for an index's prices.
            pub fn benchmark_name(mut self, name: &str) -> Self {
                self.common.benchmark_name = Some(name.to_owned());
                self
            }

            /// The value each run starts with, to follow money as well as returns: the reports
            /// then give each run's ending value and money-weighted return.
            pub fn start_value(mut self, value: f64) -> Self {
                self.common.start_value = Some(value);
                self
            }

            /// Money paid in (a positive `amount`) or taken out (a negative one) on `date`, such
            /// as `"2020-01-31"`, at the end of the period it falls in. Repeatable.
            pub fn flow(mut self, date: &str, amount: f64) -> Self {
                self.common.flows.push((date.to_owned(), amount));
                self
            }
        }
    };
}

/// Several methods backtested side by side on the same assets; [`Compare::build`] runs them
/// and builds the comparison report.
#[derive(Debug, Clone)]
#[must_use = "a comparison does nothing until `build` runs it"]
pub struct Compare<'a> {
    assets: &'a Assets,
    strategies: Vec<Strategy>,
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

    /// Fixed `weights` by asset, under `name`, compared with the methods and scored on the same
    /// periods, such as `.weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])`. Repeatable;
    /// the strategies keep the order they are given in, so these come after those already given.
    pub fn weights<S: Into<String>>(
        mut self,
        name: &str,
        weights: impl IntoIterator<Item = (S, f64)>,
    ) -> Self {
        self.strategies.push(Strategy::weights(name, weights));
        self
    }

    /// Another method, after the strategies already given.
    pub fn method(mut self, method: Method) -> Self {
        self.strategies.push(Strategy::from(method));
        self
    }

    /// The options of the comparison, as [`Assets::compare_with`] takes them.
    pub fn options(&self) -> Result<BacktestOptions> {
        self.common.options(self.strategies.clone())
    }

    /// Runs the comparison: the comparison report in `report`, and every return and trade in
    /// `results`.
    pub fn build(&self) -> Result<Comparison> {
        let mut comparison = self.assets.compare_with(&self.options()?)?;
        comparison.report.name.clone_from(&self.title);
        Ok(comparison)
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

    /// The options of the backtest, as [`Assets::backtest_with`] takes them.
    pub fn options(&self) -> Result<BacktestOptions> {
        self.common.options(vec![self.strategy.clone()])
    }

    /// Runs the backtest: the backtest report in `report`, titled with the strategy's name, and
    /// every return and trade in `results`.
    pub fn build(&self) -> Result<Backtest> {
        self.assets.backtest_with(&self.options()?)
    }
}

impl Assets {
    /// A comparison of `strategies`, methods such as `Method::MinimumVariance` or fixed weights
    /// such as `Strategy::weights("60/40", [("US Equity", 0.6), ("Bonds", 0.4)])`, each
    /// backtested with the same limits, rebalancing, validation and costs. The results and the
    /// report keep the order of the strategies.
    pub fn compare<S: Into<Strategy>>(
        &self,
        strategies: impl IntoIterator<Item = S>,
    ) -> Compare<'_> {
        Compare {
            assets: self,
            strategies: strategies.into_iter().map(Into::into).collect(),
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
