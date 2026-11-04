//! Reports on backtests: the comparison report, which compares several methods tested the same
//! way, and the backtest report, on one allocation against its benchmark.
//!
//! Both are computed from a [`BacktestResults`] and the returns of the assets it held. Every strategy
//! is scored on the periods it was tested on: out of sample when it was validated on a holdout
//! or walk-forward, and in sample otherwise. Ratios are taken against the risk-free return over
//! the same periods.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_backtest::{BacktestResults, Failure, Run, Settings, StrategyResult, Validation};
use finalytics_core::{Date, Error, Result, ValidationReport};
use finalytics_risk::{
    CovarianceMethod, MeanMethod, RelativeStatistics, ReturnsTable, Statistics, StatisticsOptions,
    TailMethod, TailRisk, relative_statistics, statistics, tail_risk,
};

use crate::shapes::{Grid, Series, Unit};

/// The confidence of the value at risk and expected shortfall in the reports.
const CONFIDENCE: f64 = 0.95;

/// The years of returns in each rolling Sharpe ratio.
const ROLLING_YEARS: f64 = 3.0;

/// The most assets the weights held are shown for; the rest are shown together.
const HELD: usize = 7;

/// How a backtest was set up, beside what [`BacktestResults`] records: what the reports need to
/// describe it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct BacktestSetup {
    /// The risk-free asset, if one of the assets is: it is held as cash, and its return is the
    /// risk-free return.
    pub risk_free_asset: Option<String>,
    /// The covariance estimator of the optimisations.
    pub covariance: CovarianceMethod,
    /// The estimator of the optimisations' mean returns.
    #[serde(default)]
    pub mean_returns: MeanMethod,
    /// The limits on the weights, in words, such as "At most 40% in any asset".
    pub limits: Vec<String>,
    /// The date the first period starts from, when it is known: the date of the prices the
    /// first returns are measured from.
    #[serde(default)]
    pub origin: Option<Date>,
}

impl BacktestSetup {
    /// A setup with `risk_free_asset`, the `covariance` estimator and the `limits` in words; the
    /// mean returns are historical unless [`BacktestSetup::with_mean_returns`] says otherwise.
    pub fn new(
        risk_free_asset: Option<String>,
        covariance: CovarianceMethod,
        limits: Vec<String>,
    ) -> BacktestSetup {
        BacktestSetup {
            risk_free_asset,
            covariance,
            mean_returns: MeanMethod::Historical,
            limits,
            origin: None,
        }
    }

    /// The same setup with the date the first period starts from.
    #[must_use]
    pub fn with_origin(mut self, origin: Option<Date>) -> BacktestSetup {
        self.origin = origin;
        self
    }

    /// The same setup with the optimisations' estimator of mean returns.
    #[must_use]
    pub fn with_mean_returns(mut self, method: MeanMethod) -> BacktestSetup {
        self.mean_returns = method;
        self
    }
}

/// What a backtest report describes: the data, how the strategies were tested, and the digest
/// of the inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct BacktestContext {
    /// The engine's version.
    pub version: String,
    /// A SHA-256 digest of the assets' returns and the settings, in hexadecimal: the same
    /// inputs give the same report.
    pub digest: String,
    /// The assets the strategies hold, other than the risk-free asset.
    pub assets: Vec<String>,
    /// The risk-free asset, held as cash, if one of the assets is.
    pub risk_free_asset: Option<String>,
    /// The number of periods in a year.
    pub periods_per_year: f64,
    /// The covariance estimator of the optimisations.
    pub covariance: CovarianceMethod,
    /// The estimator of the optimisations' mean returns.
    #[serde(default)]
    pub mean_returns: MeanMethod,
    /// The limits on the weights, in words.
    pub limits: Vec<String>,
    /// How the backtest was run.
    pub settings: Settings,
    /// The date the first period of the data ends on.
    pub start: Date,
    /// The date the last period of the data ends on.
    pub end: Date,
    /// The first period the strategies are scored on.
    pub test_start: Date,
    /// The last period the strategies are scored on.
    pub test_end: Date,
    /// For a holdout, the last period the weights were fitted to.
    pub fit_end: Option<Date>,
    /// For walk-forward validation, the periods of history before the first fit.
    pub window: Option<usize>,
    /// The annual risk-free return over the test, compounded from the periods' returns.
    pub risk_free_rate: f64,
    /// The date the first period of the data starts from, when it is known.
    #[serde(default)]
    pub origin: Option<Date>,
}

/// One run of a strategy, summarised: its statistics, trading and the series its charts draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct RunReport {
    /// The date the first period ends on.
    pub start: Date,
    /// The date the last period ends on.
    pub end: Date,
    /// Return, risk and ratios, against the risk-free return over the run.
    pub statistics: Statistics,
    /// Historical value at risk and expected shortfall over one period, at 95%.
    pub tail_risk: TailRisk,
    /// The annual risk-free return over the run.
    pub risk_free_rate: f64,
    /// Against the benchmark over the same periods, if there is one.
    pub relative: Option<RelativeStatistics>,
    /// The number of trades after the start.
    pub rebalances: usize,
    /// The share of value bought (or sold) in a year.
    pub turnover_per_year: f64,
    /// The cost of trading as a share of value in a year.
    pub costs_per_year: f64,
    /// The largest trade, as a share of value bought.
    pub largest_trade: f64,
    /// The growth of 100, from 100 on the date the run enters (when it is known), at the end of
    /// each month for daily or weekly returns, and of each period otherwise.
    pub growth: Series,
    /// The fall from the highest value so far, from 0 on the date the run enters: the deepest
    /// in each month for daily or weekly returns, and at each period otherwise.
    pub drawdown: Series,
    /// The Sharpe ratio over the past three years, at the growth's dates once three years have
    /// passed, when the run is longer than three years.
    pub rolling_sharpe: Option<Series>,
    /// The return in each calendar year.
    pub years: Vec<CalendarReturn>,
    /// The return in each month, a row per year and a column per month.
    pub months: Grid,
}

/// The return over one calendar year, or the part of it in the run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct CalendarReturn {
    /// The year.
    pub year: i32,
    /// The date the year's first period ends on.
    pub start: Date,
    /// The date the year's last period ends on.
    pub end: Date,
    /// Whether the run covers the whole year.
    pub complete: bool,
    /// The compounded return.
    pub value: f64,
}

/// Weights at a date, by asset; what they leave is held in cash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct WeightsAt {
    /// The date at whose end the weights were bought.
    pub date: Date,
    /// The weights, by asset.
    pub weights: BTreeMap<String, f64>,
}

/// One strategy, scored on the periods it was tested on, with its weights over time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct StrategyReport {
    /// The strategy's name.
    pub name: String,
    /// Whether a method fitted the weights to returns: false for fixed weights and for the
    /// benchmark.
    pub fitted: bool,
    /// The run the strategy is scored on: out of sample for a holdout or walk-forward
    /// validation, in sample otherwise.
    pub tested: RunReport,
    /// For a holdout or walk-forward validation, the run in sample: on the history before the
    /// test for a holdout, and with weights fitted to the whole history over the same periods
    /// as the test for walk-forward. Fixed weights have none, as they were not fitted; the
    /// benchmark has the same periods as the strategies' run in sample, for reference.
    pub in_sample: Option<RunReport>,
    /// The weights bought at the start of the test, then at each refit.
    pub weights: Vec<WeightsAt>,
    /// Each refit that found no weights, when the weights held were kept.
    pub failures: Vec<Failure>,
    /// Anything the reader should know about how the strategy was run.
    pub notes: Vec<String>,
}

impl StrategyReport {
    /// The weights bought last: at the last refit, or at the start.
    pub fn latest(&self) -> Option<&WeightsAt> {
        self.weights.last()
    }
}

/// Methods tested the same way, side by side: the comparison report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct ComparisonReport {
    /// What was tested and how.
    pub context: BacktestContext,
    /// Each method, in the order given.
    pub strategies: Vec<StrategyReport>,
    /// The benchmark, if one was given, tested alike.
    pub benchmark: Option<StrategyReport>,
    /// The report's title, such as "S&P 500 stocks": shown when it is printed or laid out as a
    /// page.
    #[serde(default)]
    pub name: Option<String>,
}

/// One allocation, tested against its benchmark: the backtest report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct BacktestReport {
    /// What was tested and how.
    pub context: BacktestContext,
    /// The allocation.
    pub strategy: StrategyReport,
    /// The benchmark, if one was given, held alike.
    pub benchmark: Option<StrategyReport>,
    /// For a holdout, both periods together, in sample then out of sample.
    pub history: Option<History>,
    /// The weights held in each period of the test, by asset: the largest few, then the rest
    /// together, then cash; at the same dates as the growth.
    pub held: Vec<Series>,
    /// The report's title, such as "Maximum Sharpe, fitted to 2016": shown when it is printed
    /// or laid out as a page.
    #[serde(default)]
    pub name: Option<String>,
}

/// A holdout's two periods together, for the charts of the whole history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct History {
    /// The first period out of sample.
    pub split: Date,
    /// The allocation over the whole history.
    pub strategy: RunReport,
    /// The benchmark over the whole history.
    pub benchmark: Option<RunReport>,
}

/// The comparison report on `backtest`, whose strategies held the assets of `returns` (the
/// risky assets, on the backtest's dates).
pub fn comparison_report(
    backtest: &BacktestResults,
    returns: &ReturnsTable,
    setup: &BacktestSetup,
) -> Result<ComparisonReport> {
    let inputs = Inputs::new(backtest, returns, setup)?;
    let benchmark = backtest
        .benchmark
        .as_ref()
        .map(|b| inputs.benchmark(b))
        .transpose()?;
    let strategies = backtest
        .strategies
        .iter()
        .map(|s| inputs.strategy(s, backtest.benchmark.as_ref()))
        .collect::<Result<_>>()?;
    Ok(ComparisonReport {
        context: inputs.context,
        strategies,
        benchmark,
        name: None,
    })
}

/// The backtest report on the first strategy of `backtest`, whose strategies held the assets of
/// `returns` (the risky assets, on the backtest's dates).
pub fn backtest_report(
    backtest: &BacktestResults,
    returns: &ReturnsTable,
    setup: &BacktestSetup,
) -> Result<BacktestReport> {
    let Some(first) = backtest.strategies.first() else {
        return Err(invalid("strategies", "there are none to report on"));
    };
    let inputs = Inputs::new(backtest, returns, setup)?;
    let strategy = inputs.strategy(first, backtest.benchmark.as_ref())?;
    let benchmark = backtest
        .benchmark
        .as_ref()
        .map(|b| inputs.benchmark(b))
        .transpose()?;
    let history = match (&backtest.settings.validation, &first.out_of_sample) {
        (Validation::Holdout { .. }, Some(out)) => {
            let whole = chain(&first.in_sample, out);
            let bench = backtest
                .benchmark
                .as_ref()
                .and_then(|b| b.out_of_sample.as_ref().map(|o| chain(&b.in_sample, o)));
            Some(History {
                split: out.dates[0],
                strategy: inputs.run(&whole, bench.as_ref())?,
                benchmark: bench.as_ref().map(|b| inputs.run(b, None)).transpose()?,
            })
        }
        _ => None,
    };
    let tested = first.out_of_sample.as_ref().unwrap_or(&first.in_sample);
    let held = inputs.held(tested);
    Ok(BacktestReport {
        context: inputs.context,
        strategy,
        benchmark,
        history,
        held,
        name: None,
    })
}

fn invalid(path: &str, message: &str) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}

/// A run, then another that follows it, as one.
fn chain(first: &Run, second: &Run) -> Run {
    let mut run = first.clone();
    run.dates.extend_from_slice(&second.dates);
    run.returns.extend_from_slice(&second.returns);
    run.trades.extend(second.trades.iter().cloned());
    run
}

/// The backtest and the assets' returns, checked against each other, with the context.
struct Inputs<'a> {
    backtest: &'a BacktestResults,
    returns: &'a ReturnsTable,
    context: BacktestContext,
}

impl<'a> Inputs<'a> {
    fn new(
        backtest: &'a BacktestResults,
        returns: &'a ReturnsTable,
        setup: &BacktestSetup,
    ) -> Result<Inputs<'a>> {
        let n = backtest.dates.len();
        if returns.observations() != n || backtest.cash.len() != n {
            return Err(invalid(
                "returns",
                "must have one return per date of the backtest",
            ));
        }
        let (Some(&start), Some(&end)) = (backtest.dates.first(), backtest.dates.last()) else {
            return Err(invalid("dates", "there are none"));
        };
        let tested = backtest
            .strategies
            .first()
            .or(backtest.benchmark.as_ref())
            .map(|s| s.out_of_sample.as_ref().unwrap_or(&s.in_sample));
        let (test_start, test_end) = tested
            .and_then(|r| Some((*r.dates.first()?, *r.dates.last()?)))
            .unwrap_or((start, end));
        let first_test = backtest.dates.partition_point(|d| *d < test_start);
        let (fit_end, window) = match backtest.settings.validation {
            Validation::Holdout { .. } => {
                (first_test.checked_sub(1).map(|i| backtest.dates[i]), None)
            }
            Validation::WalkForward { .. } => (None, Some(first_test)),
            _ => (None, None),
        };
        let last_test = backtest.dates.partition_point(|d| *d <= test_end);
        let ppy = backtest.periods_per_year;
        let context = BacktestContext {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            digest: digest(backtest, returns),
            assets: returns.labels().to_vec(),
            risk_free_asset: setup.risk_free_asset.clone(),
            periods_per_year: ppy,
            covariance: setup.covariance,
            mean_returns: setup.mean_returns,
            limits: setup.limits.clone(),
            settings: backtest.settings.clone(),
            start,
            end,
            test_start,
            test_end,
            fit_end,
            window,
            risk_free_rate: annual(&backtest.cash[first_test..last_test], ppy),
            origin: setup.origin,
        };
        Ok(Inputs {
            backtest,
            returns,
            context,
        })
    }

    /// The index of `date` among the backtest's dates.
    fn index(&self, date: Date) -> usize {
        self.backtest.dates.partition_point(|d| *d < date)
    }

    /// The cash returns over `run`'s periods.
    fn cash(&self, run: &Run) -> &[f64] {
        let Some(&first) = run.dates.first() else {
            return &[];
        };
        let i = self.index(first);
        &self.backtest.cash[i..(i + run.dates.len()).min(self.backtest.cash.len())]
    }

    /// A strategy's report, against `benchmark`.
    fn strategy(
        &self,
        result: &StrategyResult,
        benchmark: Option<&StrategyResult>,
    ) -> Result<StrategyReport> {
        self.summarise(result, benchmark, result.fitted)
    }

    /// The benchmark's report, with its run over the strategies' in-sample periods.
    fn benchmark(&self, result: &StrategyResult) -> Result<StrategyReport> {
        self.summarise(result, None, true)
    }

    fn summarise(
        &self,
        result: &StrategyResult,
        benchmark: Option<&StrategyResult>,
        in_sample: bool,
    ) -> Result<StrategyReport> {
        let (tested, in_sample) = match &result.out_of_sample {
            Some(out) => (out, in_sample.then_some(&result.in_sample)),
            None => (&result.in_sample, None),
        };
        let bench_tested = benchmark.map(|b| b.out_of_sample.as_ref().unwrap_or(&b.in_sample));
        let bench_in = benchmark.map(|b| &b.in_sample);
        let mut weights = vec![WeightsAt {
            date: tested
                .dates
                .first()
                .map(|d| self.index(*d))
                .and_then(|i| i.checked_sub(1))
                .map_or(self.context.start, |i| self.backtest.dates[i]),
            weights: tested.start.clone(),
        }];
        weights.extend(tested.trades.iter().filter(|t| t.refit).map(|t| WeightsAt {
            date: t.date,
            weights: t.weights.clone(),
        }));
        Ok(StrategyReport {
            name: result.name.clone(),
            fitted: result.fitted,
            tested: self.run(tested, bench_tested)?,
            in_sample: in_sample
                .map(|r| self.run(r, bench_in.filter(|b| b.dates == r.dates)))
                .transpose()?,
            weights,
            failures: result.failures.clone(),
            notes: result.notes.clone(),
        })
    }

    fn run(&self, run: &Run, benchmark: Option<&Run>) -> Result<RunReport> {
        let ppy = self.backtest.periods_per_year;
        let (Some(&start), Some(&end)) = (run.dates.first(), run.dates.last()) else {
            return Err(invalid("strategies", "a run has no periods"));
        };
        let cash = self.cash(run);
        let risk_free_rate = annual(cash, ppy);
        // Sharpe and Sortino ratios are both measured against the risk-free return.
        let mut options = StatisticsOptions::default();
        options.periods_per_year = ppy;
        options.risk_free_rate = risk_free_rate;
        options.target_return = risk_free_rate;
        let stats = statistics(&run.returns, &options)?;
        let tail = tail_risk(&run.returns, CONFIDENCE, TailMethod::Historical)?;
        let relative = match benchmark {
            Some(b) if b.dates == run.dates => {
                Some(relative_statistics(&run.returns, &b.returns, &options)?)
            }
            _ => None,
        };
        // The growth of 100, from a value before the first period.
        let mut level = 100.0;
        let mut peak = 100.0f64;
        let mut levels = Vec::with_capacity(run.returns.len());
        let mut falls = Vec::with_capacity(run.returns.len());
        for r in &run.returns {
            level *= 1.0 + r;
            peak = peak.max(level);
            levels.push(level);
            falls.push(level / peak - 1.0);
        }
        let points = sample(&run.dates, ppy);
        // The growth starts at 100, and the drawdown at 0, on the date the run enters: the end
        // of the period before its first, or the start of the data.
        let entry = match self.index(start) {
            0 => self.context.origin,
            i => Some(self.backtest.dates[i - 1]),
        };
        let at_entry = |values: Vec<f64>, first: f64| -> (Vec<Date>, Vec<f64>) {
            let mut dates: Vec<Date> = points.iter().map(|(i, _)| run.dates[*i]).collect();
            let mut values = values;
            if let Some(entry) = entry {
                dates.insert(0, entry);
                values.insert(0, first);
            }
            (dates, values)
        };
        let (growth_dates, growth_values) =
            at_entry(points.iter().map(|(i, _)| levels[*i]).collect(), 100.0);
        let (drawdown_dates, drawdown_values) = at_entry(
            points
                .iter()
                .map(|(i, from)| falls[*from..=*i].iter().copied().fold(0.0, f64::min))
                .collect(),
            0.0,
        );
        let window = (ROLLING_YEARS * ppy).round() as usize;
        let rolling_sharpe = if run.returns.len() > window && window >= 2 {
            // Only the windows that end at the points charted.
            let mut dates = Vec::new();
            let mut values = Vec::new();
            for (i, _) in points.iter().filter(|(i, _)| *i + 1 >= window) {
                // Each window against the risk-free return over that window.
                let span = i + 1 - window..=*i;
                let mut rolling = options;
                rolling.risk_free_rate = cash
                    .get(span.clone())
                    .map_or(risk_free_rate, |c| annual(c, ppy));
                rolling.target_return = rolling.risk_free_rate;
                let rolled = statistics(&run.returns[span], &rolling)?;
                if let Some(sharpe) = rolled.sharpe_ratio {
                    dates.push(run.dates[*i]);
                    values.push(sharpe);
                }
            }
            Some(Series {
                name: "Sharpe ratio over three years".to_owned(),
                unit: Unit::Ratio,
                dates,
                values,
            })
        } else {
            None
        };
        Ok(RunReport {
            start,
            end,
            statistics: stats,
            tail_risk: tail,
            risk_free_rate,
            relative,
            rebalances: run.trades.len(),
            turnover_per_year: run.turnover_per_year(ppy),
            costs_per_year: run.costs_per_year(ppy),
            largest_trade: run.trades.iter().map(|t| t.turnover).fold(0.0, f64::max),
            growth: Series {
                name: "Growth of 100".to_owned(),
                unit: Unit::Level,
                dates: growth_dates,
                values: growth_values,
            },
            drawdown: Series {
                name: "Drawdown".to_owned(),
                unit: Unit::Fraction,
                dates: drawdown_dates,
                values: drawdown_values,
            },
            rolling_sharpe,
            years: years(&run.dates, &run.returns, ppy),
            months: months(&run.dates, &run.returns),
        })
    }

    /// The weights held in each period of `run`, drifting with the assets' returns between
    /// trades: the largest few assets, the rest together, and cash.
    fn held(&self, run: &Run) -> Vec<Series> {
        let labels = self.returns.labels();
        let columns = self.returns.columns();
        let Some(&first) = run.dates.first() else {
            return Vec::new();
        };
        let offset = self.index(first);
        let weights_of = |w: &BTreeMap<String, f64>| -> Vec<f64> {
            labels
                .iter()
                .map(|l| w.get(l).copied().unwrap_or(0.0))
                .collect()
        };
        let mut held = weights_of(&run.start);
        let mut cash = 1.0 - held.iter().sum::<f64>();
        let mut trades = run.trades.iter().peekable();
        // The weights held during each period, as they stood at its start.
        let mut during: Vec<Held> = Vec::with_capacity(run.dates.len());
        for (k, date) in run.dates.iter().enumerate() {
            during.push(Held {
                assets: held.clone(),
                cash,
            });
            let t = offset + k;
            let c = self.backtest.cash.get(t).copied().unwrap_or(0.0);
            let gross: f64 = held
                .iter()
                .zip(columns)
                .map(|(w, col)| w * col.get(t).copied().unwrap_or(0.0))
                .sum::<f64>()
                + cash * c;
            if (1.0 + gross).abs() > f64::EPSILON {
                for (w, col) in held.iter_mut().zip(columns) {
                    *w *= (1.0 + col.get(t).copied().unwrap_or(0.0)) / (1.0 + gross);
                }
                cash *= (1.0 + c) / (1.0 + gross);
            }
            if let Some(trade) = trades.next_if(|tr| tr.date == *date) {
                held = weights_of(&trade.weights);
                cash = 1.0 - held.iter().sum::<f64>();
            }
        }
        let points = sample(&run.dates, self.backtest.periods_per_year);
        let dates: Vec<Date> = points.iter().map(|(i, _)| run.dates[*i]).collect();
        // The assets by their average weight, largest first.
        let mut order: Vec<(usize, f64)> = (0..labels.len())
            .map(|a| {
                let mean = during.iter().map(|h| h.assets[a]).sum::<f64>() / during.len() as f64;
                (a, mean)
            })
            .filter(|(_, mean)| *mean > 1e-9)
            .collect();
        order.sort_by(|x, y| y.1.total_cmp(&x.1));
        let shown: Vec<usize> = order.iter().take(HELD).map(|(a, _)| *a).collect();
        let rest: Vec<usize> = order.iter().skip(HELD).map(|(a, _)| *a).collect();
        let series = |name: String, value: &dyn Fn(&Held) -> f64| Series {
            name,
            unit: Unit::Fraction,
            dates: dates.clone(),
            values: points.iter().map(|(i, _)| value(&during[*i])).collect(),
        };
        let mut out: Vec<Series> = shown
            .iter()
            .map(|&a| series(labels[a].clone(), &|h| h.assets[a]))
            .collect();
        if !rest.is_empty() {
            let name = if rest.len() == 1 {
                labels[rest[0]].clone()
            } else {
                format!("{} others", rest.len())
            };
            out.push(series(name, &|h| rest.iter().map(|&a| h.assets[a]).sum()));
        }
        if during.iter().any(|h| h.cash.abs() > 1e-9) {
            let name = self
                .context
                .risk_free_asset
                .clone()
                .unwrap_or_else(|| "Cash".to_owned());
            out.push(series(name, &|h| h.cash));
        }
        out
    }
}

/// The weights held during a period: of each asset, and in cash.
struct Held {
    assets: Vec<f64>,
    cash: f64,
}

/// The annual return compounded from `returns`, with `ppy` periods in a year; zero without
/// returns.
fn annual(returns: &[f64], ppy: f64) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let growth: f64 = returns.iter().map(|r| 1.0 + r).product();
    growth.powf(ppy / returns.len() as f64) - 1.0
}

/// The periods a chart shows: each period for monthly or less frequent returns, and the last of
/// each month for daily or weekly ones; with each, the first period it stands for.
fn sample(dates: &[Date], ppy: f64) -> Vec<(usize, usize)> {
    if ppy <= 12.5 {
        return (0..dates.len()).map(|i| (i, i)).collect();
    }
    let mut out = Vec::new();
    let mut from = 0;
    for i in 0..dates.len() {
        let last = dates
            .get(i + 1)
            .is_none_or(|next| (next.year(), next.month()) != (dates[i].year(), dates[i].month()));
        if last {
            out.push((i, from));
            from = i + 1;
        }
    }
    out
}

/// The compounded return of each calendar year, with `ppy` periods in a year.
fn years(dates: &[Date], returns: &[f64], ppy: f64) -> Vec<CalendarReturn> {
    let mut out: Vec<CalendarReturn> = Vec::new();
    for (date, r) in dates.iter().zip(returns) {
        match out.last_mut() {
            Some(y) if y.year == date.year() => {
                y.value = (1.0 + y.value) * (1.0 + r) - 1.0;
                y.end = *date;
            }
            _ => out.push(CalendarReturn {
                year: date.year(),
                start: *date,
                end: *date,
                complete: false,
                value: *r,
            }),
        }
    }
    // A year is whole when the run covers it from its start to its end. A year after the
    // first starts with the run's period that crosses into it; the run's first year starts
    // with it when its first period is no longer than a period after 1 January. Likewise, the
    // last year ends with the run when its last period is within a period of 31 December.
    let length = (366.0 / ppy).ceil() as i32 + 3;
    let last = out.len().saturating_sub(1);
    for (k, y) in out.iter_mut().enumerate() {
        let starts =
            k > 0 || Date::from_ymd(y.year, 1, 1).is_ok_and(|jan| jan.days_until(y.start) < length);
        let ends = k < last
            || Date::from_ymd(y.year, 12, 31).is_ok_and(|dec| y.end.days_until(dec) < length);
        y.complete = starts && ends;
    }
    out
}

/// The compounded return of each month, a row per year.
fn months(dates: &[Date], returns: &[f64]) -> Grid {
    let mut rows: Vec<String> = Vec::new();
    let mut values: Vec<Vec<Option<f64>>> = Vec::new();
    for (date, r) in dates.iter().zip(returns) {
        let year = date.year().to_string();
        if rows.last() != Some(&year) {
            rows.push(year);
            values.push(vec![None; 12]);
        }
        if let Some(row) = values.last_mut() {
            let cell = &mut row[date.month() as usize - 1];
            *cell = Some((1.0 + cell.unwrap_or(0.0)) * (1.0 + r) - 1.0);
        }
    }
    Grid {
        name: "Monthly returns".to_owned(),
        unit: Unit::Fraction,
        rows,
        columns: [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ]
        .map(str::to_owned)
        .to_vec(),
        values,
    }
}

/// A SHA-256 digest of the assets' returns, the cash returns, the dates and the settings.
fn digest(backtest: &BacktestResults, returns: &ReturnsTable) -> String {
    let mut hash = Sha256::new();
    for date in &backtest.dates {
        hash.update(date.to_string().as_bytes());
    }
    for (label, column) in returns.labels().iter().zip(returns.columns()) {
        hash.update(label.as_bytes());
        for r in column {
            hash.update(r.to_le_bytes());
        }
    }
    for r in &backtest.cash {
        hash.update(r.to_le_bytes());
    }
    hash.update(
        serde_json::to_string(&backtest.settings)
            .unwrap_or_default()
            .as_bytes(),
    );
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Date {
        text.parse().unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn daily_returns_are_shown_at_month_ends() {
        let dates = [
            d("2026-01-29"),
            d("2026-01-30"),
            d("2026-02-02"),
            d("2026-02-27"),
        ];
        assert_eq!(sample(&dates, 252.0), [(1, 0), (3, 2)]);
        assert_eq!(sample(&dates, 12.0).len(), 4);
    }

    #[test]
    fn calendar_years_compound_and_know_when_they_are_whole() {
        let dates = [
            d("2024-11-30"),
            d("2024-12-31"),
            d("2025-01-31"),
            d("2025-12-31"),
        ];
        let y = years(&dates, &[0.1, 0.1, -0.5, 0.2], 12.0);
        assert_eq!(y.len(), 2);
        assert!((y[0].value - 0.21).abs() < 1e-12 && !y[0].complete);
        assert!((y[1].value - -0.4).abs() < 1e-12 && y[1].complete);
        let grid = months(&dates, &[0.1, 0.1, -0.5, 0.2]);
        assert_eq!(grid.rows, ["2024", "2025"]);
        assert_eq!(grid.values[1][0], Some(-0.5));
        assert_eq!(grid.values[0][0], None);
    }

    #[test]
    fn an_annual_rate_compounds_the_periods() {
        assert!((annual(&[0.01; 12], 12.0) - (1.01f64.powi(12) - 1.0)).abs() < 1e-12);
        assert_eq!(annual(&[], 12.0), 0.0);
    }
}
