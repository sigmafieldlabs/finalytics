//! The simulation: strategies held through time, refitted and rebalanced on schedule.

use std::collections::BTreeMap;
use std::ops::Range;

use finalytics_core::{Date, Error, Result, ValidationReport};
use finalytics_risk::ReturnsTable;

use crate::results::{BacktestResults, Failure, Run, StrategyResult, Trade};
use crate::settings::{Rebalance, Settings, Validation};

/// The data a backtest runs on: the returns of the assets the strategies hold, and of cash.
#[derive(Debug, Clone, Copy)]
pub struct Market<'a> {
    dates: &'a [Date],
    returns: &'a ReturnsTable,
    cash: &'a [f64],
    periods_per_year: f64,
}

impl<'a> Market<'a> {
    /// The market of `returns`, one column per asset, whose rows end on `dates`; `cash` is the
    /// return of cash (the risk-free return) in each period, and `periods_per_year` the number
    /// of periods in a year.
    pub fn new(
        dates: &'a [Date],
        returns: &'a ReturnsTable,
        cash: &'a [f64],
        periods_per_year: f64,
    ) -> Market<'a> {
        Market {
            dates,
            returns,
            cash,
            periods_per_year,
        }
    }

    fn assets(&self) -> usize {
        self.returns.labels().len()
    }

    fn periods(&self) -> usize {
        self.returns.observations()
    }

    fn by_asset(&self, weights: &[f64]) -> BTreeMap<String, f64> {
        self.returns
            .labels()
            .iter()
            .cloned()
            .zip(weights.iter().copied())
            .collect()
    }
}

/// What fitting a strategy to some history found.
#[derive(Debug, Clone, PartialEq)]
pub enum Fit {
    /// Weights, one per asset in the order of the market's columns; what they leave is held in
    /// cash.
    Weights(Vec<f64>),
    /// No weights, and why, such as `infeasible`.
    NoSolution(String),
}

/// A function that fits weights to a range of the market's periods (rows of its returns).
pub type Fitter<'a> = Box<dyn Fn(Range<usize>) -> Result<Fit> + 'a>;

/// How a strategy chooses its weights.
pub enum Plan<'a> {
    /// Fixed weights, one per asset; what they leave is held in cash.
    Weights(Vec<f64>),
    /// Weights fitted to history, as the validation says.
    Fit(Fitter<'a>),
}

/// A strategy to backtest.
pub struct Strategy<'a> {
    /// Its name, unique among the strategies.
    pub name: String,
    /// How it chooses its weights.
    pub plan: Plan<'a>,
    /// Anything the reader should know about how it is run.
    pub notes: Vec<String>,
}

/// What strategies are compared with.
#[derive(Debug, Clone, PartialEq)]
pub enum Benchmark {
    /// Fixed weights, held and rebalanced as the strategies are, with the same costs.
    Weights(Vec<f64>),
    /// The return of each of the market's periods, such as an index's.
    Returns(Vec<f64>),
}

/// The periods each part of a validation covers.
#[derive(Debug, Clone, Copy)]
enum Split {
    InSample,
    Holdout(usize),
    WalkForward { window: usize, expanding: bool },
}

/// Backtests `strategies` on `market`, and the benchmark beside them if one is given, by
/// `settings`. Every problem with the inputs is reported together.
pub fn backtest(
    market: &Market<'_>,
    strategies: Vec<Strategy<'_>>,
    benchmark: Option<(String, Benchmark)>,
    settings: &Settings,
) -> Result<BacktestResults> {
    let split = check(market, &strategies, benchmark.as_ref(), settings)?;
    let simulation = Simulation {
        market,
        rebalance: settings.rebalance,
        cost: settings.trading_cost_bps / 10_000.0,
    };
    let strategies = strategies
        .iter()
        .map(|s| simulation.strategy(s, split))
        .collect::<Result<Vec<_>>>()?;
    let benchmark = benchmark
        .map(|(name, b)| simulation.benchmark(name, &b, split))
        .transpose()?;
    Ok(BacktestResults {
        assets: market.returns.labels().to_vec(),
        dates: market.dates.to_vec(),
        cash: market.cash.to_vec(),
        periods_per_year: market.periods_per_year,
        settings: settings.clone(),
        strategies,
        benchmark,
    })
}

/// Checks the inputs, and finds the periods the validation splits them into.
/// The problems with `settings` on `market`: its trading cost, rebalancing and validation. A
/// caller that checks its own inputs first adds these, so that every problem is reported
/// together.
pub fn check_settings(market: &Market<'_>, settings: &Settings) -> ValidationReport {
    let mut report = ValidationReport::new();
    settings_split(market, settings, &mut report);
    report
}

fn check(
    market: &Market<'_>,
    strategies: &[Strategy<'_>],
    benchmark: Option<&(String, Benchmark)>,
    settings: &Settings,
) -> Result<Split> {
    let mut report = ValidationReport::new();
    let (n, k) = (market.periods(), market.assets());
    if k == 0 {
        report.error("returns", "there are no assets");
    }
    if market.dates.len() != n {
        report.error(
            "dates",
            format!("{} dates for {n} periods of returns", market.dates.len()),
        );
    }
    if market.cash.len() != n {
        report.error(
            "cash",
            format!("{} cash returns for {n} periods", market.cash.len()),
        );
    }
    if market.periods_per_year.is_nan() || market.periods_per_year <= 0.0 {
        report.error("periods_per_year", "must be greater than zero");
    }
    let split = settings_split(market, settings, &mut report);
    if strategies.is_empty() {
        report.error("strategies", "there are none to test");
    }
    for (i, strategy) in strategies.iter().enumerate() {
        if strategies[..i].iter().any(|s| s.name == strategy.name) {
            report.error(
                format!("strategies[{i}].name"),
                format!("{} is the name of an earlier strategy", strategy.name),
            );
        }
        if let Plan::Weights(weights) = &strategy.plan {
            check_weights(weights, k, &format!("strategies[{i}].weights"), &mut report);
        }
    }
    match benchmark {
        Some((_, Benchmark::Weights(weights))) => {
            check_weights(weights, k, "benchmark.weights", &mut report);
        }
        Some((_, Benchmark::Returns(returns))) => {
            if returns.len() != n {
                report.error(
                    "benchmark.returns",
                    format!("{} returns for {n} periods", returns.len()),
                );
            } else if let Some(t) = returns.iter().position(|r| !r.is_finite()) {
                report.error(format!("benchmark.returns[{t}]"), "must be a finite number");
            }
        }
        None => {}
    }
    report.into_result(split)
}

/// The split of the periods the validation of `settings` asks for, with its problems and those
/// of the trading cost and rebalancing in `report`.
fn settings_split(
    market: &Market<'_>,
    settings: &Settings,
    report: &mut ValidationReport,
) -> Split {
    let n = market.periods();
    if !(settings.trading_cost_bps >= 0.0 && settings.trading_cost_bps.is_finite()) {
        report.error(
            "trading_cost_bps",
            "must be zero or more, in basis points of the value traded",
        );
    }
    if let Rebalance::Drift { band } = settings.rebalance
        && !(band > 0.0 && band <= 1.0)
    {
        report.error(
            "rebalance.band",
            "must be between 0 and 1, such as 0.05 for five percentage points",
        );
    }
    if let Rebalance::Every { periods } = settings.rebalance {
        if periods == 0 {
            report.error("rebalance.periods", "must be at least 1, such as 21");
        } else if periods >= n {
            report.error(
                "rebalance.periods",
                format!("is {periods}, but the data has {n} periods: it would never rebalance"),
            );
        }
    }
    match &settings.validation {
        Validation::InSample => Split::InSample,
        Validation::Holdout { start } => {
            let h = market.dates.partition_point(|d| d < start);
            if h < 2 {
                report.error(
                    "validation.start",
                    format!("leaves fewer than two periods before {start} to fit to"),
                );
            } else if h >= n {
                report.error(
                    "validation.start",
                    format!("leaves no periods from {start} on to test"),
                );
            }
            Split::Holdout(h)
        }
        Validation::WalkForward {
            years,
            periods,
            expanding,
        } => {
            let (w, field) = match (years, periods) {
                (Some(years), None) if years.is_finite() && *years > 0.0 => {
                    ((years * market.periods_per_year).round() as usize, "years")
                }
                (None, Some(periods)) => (*periods, "periods"),
                _ => {
                    report.error(
                        "validation",
                        "give the walk-forward window as a number of years or of periods",
                    );
                    (0, "")
                }
            };
            let periods = |k: usize| {
                if k == 1 {
                    "1 period".to_owned()
                } else {
                    format!("{k} periods")
                }
            };
            if !field.is_empty() && w < 2 {
                report.error(
                    format!("validation.{field}"),
                    format!("the window is {}: a fit needs at least 2", periods(w)),
                );
            } else if !field.is_empty() && w >= n {
                report.error(
                    format!("validation.{field}"),
                    format!(
                        "the window is {}, but the data has {}: it must leave periods to test",
                        periods(w),
                        periods(n)
                    ),
                );
            }
            if !settings.rebalance.is_scheduled() {
                report.error(
                    "rebalance",
                    "walk-forward validation refits at each scheduled rebalance: choose \
                     monthly, quarterly, yearly or every so many periods",
                );
            }
            Split::WalkForward {
                window: w,
                expanding: *expanding,
            }
        }
    }
}

fn check_weights(weights: &[f64], assets: usize, path: &str, report: &mut ValidationReport) {
    if weights.len() != assets {
        report.error(
            path,
            format!("{} weights for {assets} assets", weights.len()),
        );
    } else if weights.iter().any(|w| !w.is_finite()) {
        report.error(path, "must all be finite numbers");
    } else if weights.iter().sum::<f64>() > 1.0 + 1e-6 {
        report.error(path, "must not add up to more than one");
    }
}

struct Simulation<'m, 'a> {
    market: &'m Market<'a>,
    rebalance: Rebalance,
    /// The cost of trading as a share of the value traded.
    cost: f64,
}

impl Simulation<'_, '_> {
    fn strategy(&self, strategy: &Strategy<'_>, split: Split) -> Result<StrategyResult> {
        let n = self.market.periods();
        let mut failures = Vec::new();
        let (in_sample, out_of_sample) = match (&strategy.plan, split) {
            (Plan::Weights(w), Split::InSample) => {
                (self.run(0..n, Some(w), None, &mut failures)?, None)
            }
            (Plan::Weights(w), Split::Holdout(h)) => (
                self.run(0..h, Some(w), None, &mut failures)?,
                Some(self.run(h..n, Some(w), None, &mut failures)?),
            ),
            (Plan::Weights(w), Split::WalkForward { window, .. }) => {
                let run = self.run(window..n, Some(w), None, &mut failures)?;
                (run.clone(), Some(run))
            }
            (Plan::Fit(fit), Split::InSample) => {
                let weights = self.fit(fit, 0..n, &mut failures)?;
                (
                    self.run(0..n, weights.as_deref(), None, &mut failures)?,
                    None,
                )
            }
            (Plan::Fit(fit), Split::Holdout(h)) => {
                let weights = self.fit(fit, 0..h, &mut failures)?;
                (
                    self.run(0..h, weights.as_deref(), None, &mut failures)?,
                    Some(self.run(h..n, weights.as_deref(), None, &mut failures)?),
                )
            }
            (Plan::Fit(fit), Split::WalkForward { window, expanding }) => {
                let first = self.fit(fit, 0..window, &mut failures)?;
                let mut refit = |t: usize| {
                    let from = if expanding { 0 } else { t + 1 - window };
                    fit(from..t + 1)
                };
                let out = self.run(window..n, first.as_deref(), Some(&mut refit), &mut failures)?;
                let mut whole = Vec::new();
                let full = self.fit(fit, 0..n, &mut whole)?;
                failures.extend(whole.into_iter().map(|f| Failure {
                    reason: format!("in-sample fit: {}", f.reason),
                    ..f
                }));
                let ins = self.run(window..n, full.as_deref(), None, &mut failures)?;
                (ins, Some(out))
            }
        };
        Ok(StrategyResult {
            name: strategy.name.clone(),
            fitted: matches!(strategy.plan, Plan::Fit(_)),
            in_sample,
            out_of_sample,
            failures,
            notes: strategy.notes.clone(),
        })
    }

    fn benchmark(
        &self,
        name: String,
        benchmark: &Benchmark,
        split: Split,
    ) -> Result<StrategyResult> {
        match benchmark {
            Benchmark::Weights(weights) => self.strategy(
                &Strategy {
                    name,
                    plan: Plan::Weights(weights.clone()),
                    notes: Vec::new(),
                },
                split,
            ),
            Benchmark::Returns(returns) => {
                let n = self.market.periods();
                let run = |rows: Range<usize>| Run {
                    dates: self.market.dates[rows.clone()].to_vec(),
                    returns: returns[rows].to_vec(),
                    start: BTreeMap::new(),
                    trades: Vec::new(),
                };
                let (in_sample, out_of_sample) = match split {
                    Split::InSample => (run(0..n), None),
                    Split::Holdout(h) => (run(0..h), Some(run(h..n))),
                    Split::WalkForward { window, .. } => (run(window..n), Some(run(window..n))),
                };
                Ok(StrategyResult {
                    name,
                    fitted: false,
                    in_sample,
                    out_of_sample,
                    failures: Vec::new(),
                    notes: Vec::new(),
                })
            }
        }
    }

    /// Fits weights to `rows`, recording a failure dated at the last of them.
    fn fit(
        &self,
        fit: &Fitter<'_>,
        rows: Range<usize>,
        failures: &mut Vec<Failure>,
    ) -> Result<Option<Vec<f64>>> {
        let date = self.market.dates[rows.end - 1];
        match fit(rows)? {
            Fit::Weights(weights) => {
                self.check_fitted(&weights, date)?;
                Ok(Some(weights))
            }
            Fit::NoSolution(reason) => {
                failures.push(Failure { date, reason });
                Ok(None)
            }
        }
    }

    fn check_fitted(&self, weights: &[f64], date: Date) -> Result<()> {
        let mut report = ValidationReport::new();
        check_weights(
            weights,
            self.market.assets(),
            &format!("weights fitted to {date}"),
            &mut report,
        );
        report.into_result(())
    }

    /// Holds `start` (cash without it) over `rows`, trading on the schedule, and refitting at
    /// each scheduled rebalance with `refit` if given.
    fn run(
        &self,
        rows: Range<usize>,
        start: Option<&[f64]>,
        mut refit: Option<&mut dyn FnMut(usize) -> Result<Fit>>,
        failures: &mut Vec<Failure>,
    ) -> Result<Run> {
        let market = self.market;
        let columns = market.returns.columns();
        let k = market.assets();
        let mut held: Vec<f64> = start.map_or_else(|| vec![0.0; k], <[f64]>::to_vec);
        let mut cash = 1.0 - held.iter().sum::<f64>();
        let mut target = held.clone();
        let mut pending = 0.0;
        let mut returns = Vec::with_capacity(rows.len());
        let mut trades = Vec::new();
        for t in rows.clone() {
            let gross: f64 =
                (0..k).map(|i| held[i] * columns[i][t]).sum::<f64>() + cash * market.cash[t];
            if gross.is_nan() || gross <= -1.0 {
                let mut report = ValidationReport::new();
                report.error(
                    format!("returns[{t}]"),
                    format!("the portfolio lost all its value on {}", market.dates[t]),
                );
                return Err(Error::Validation(report));
            }
            returns.push((1.0 + gross) * (1.0 - pending) - 1.0);
            pending = 0.0;
            for (i, w) in held.iter_mut().enumerate() {
                *w *= (1.0 + columns[i][t]) / (1.0 + gross);
            }
            cash *= (1.0 + market.cash[t]) / (1.0 + gross);
            if t + 1 == rows.end {
                break;
            }
            let date = market.dates[t];
            let calendar = self
                .rebalance
                .due(date, market.dates[t + 1], t + 1 - rows.start);
            let mut refitted = false;
            if calendar && let Some(refit) = refit.as_mut() {
                match refit(t)? {
                    Fit::Weights(weights) => {
                        self.check_fitted(&weights, date)?;
                        target = weights;
                        refitted = true;
                    }
                    Fit::NoSolution(reason) => {
                        // Keep the weights held, without trading.
                        failures.push(Failure { date, reason });
                        continue;
                    }
                }
            }
            let target_cash = 1.0 - target.iter().sum::<f64>();
            let drifted = match self.rebalance {
                Rebalance::Drift { band } => {
                    target.iter().zip(&held).any(|(a, b)| (a - b).abs() > band)
                        || (target_cash - cash).abs() > band
                }
                _ => false,
            };
            if !(calendar || drifted) {
                continue;
            }
            let traded: f64 = target.iter().zip(&held).map(|(a, b)| (a - b).abs()).sum();
            let turnover = 0.5 * (traded + (target_cash - cash).abs());
            if turnover > 1e-12 || refitted {
                let cost = self.cost * traded;
                pending = cost;
                trades.push(Trade {
                    date,
                    weights: market.by_asset(&target),
                    turnover,
                    cost,
                    refit: refitted,
                });
            }
            held.clone_from(&target);
            cash = target_cash;
        }
        Ok(Run {
            dates: market.dates[rows].to_vec(),
            returns,
            start: start.map(|w| market.by_asset(w)).unwrap_or_default(),
            trades,
        })
    }
}
