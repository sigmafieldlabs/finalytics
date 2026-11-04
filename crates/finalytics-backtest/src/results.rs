//! What a backtest finds: each strategy's returns, trades and failed refits, in and out of
//! sample.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use finalytics_core::Date;

use crate::settings::{Settings, Validation};

/// The result of a backtest: every strategy, and the benchmark if one was given, on the same
/// assets and dates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct BacktestResults {
    /// The assets the strategies hold.
    pub assets: Vec<String>,
    /// The date each period of the data ends on.
    pub dates: Vec<Date>,
    /// The return of cash in each period: the risk-free return.
    pub cash: Vec<f64>,
    /// The number of periods in a year.
    pub periods_per_year: f64,
    /// How the backtest was run.
    pub settings: Settings,
    /// Each strategy, in the order given.
    pub strategies: Vec<StrategyResult>,
    /// The benchmark, if one was given.
    pub benchmark: Option<StrategyResult>,
}

/// One strategy's backtest.
///
/// `in_sample` holds the returns of weights fitted to the history they are tested on: the
/// whole history for in-sample validation; the history before the test for a holdout; and, for
/// walk-forward validation, the whole history, tested on the same periods as `out_of_sample`, so
/// that the two can be compared directly. `out_of_sample` holds the returns of weights tested on
/// history they had not seen; it is absent for in-sample validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct StrategyResult {
    /// The strategy's name.
    pub name: String,
    /// Whether a method fitted the weights to returns: false for fixed weights and for the
    /// benchmark, which were not fitted to any history.
    pub fitted: bool,
    /// The returns of weights tested on the history they were fitted to; for fixed weights,
    /// the same periods as a fitted strategy's.
    pub in_sample: Run,
    /// The returns of weights tested on history they had not seen.
    pub out_of_sample: Option<Run>,
    /// Each refit that found no weights, when the weights held were kept.
    pub failures: Vec<Failure>,
    /// Anything the reader should know about how the strategy was run, such as limits that did
    /// not apply to it.
    pub notes: Vec<String>,
}

/// A strategy held over a run of periods, rebalanced on the schedule, net of trading costs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Run {
    /// The date each period ends on.
    pub dates: Vec<Date>,
    /// The return of each period, net of the cost of the trades made at the end of the one
    /// before.
    pub returns: Vec<f64>,
    /// The weights bought at the start, by asset; what they leave is held in cash. The start
    /// costs nothing.
    pub start: BTreeMap<String, f64>,
    /// Every trade after the start.
    pub trades: Vec<Trade>,
}

/// A trade back to target weights, or to new ones after a refit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Trade {
    /// The date at whose end the trade is made.
    pub date: Date,
    /// The weights after the trade, by asset; what they leave is held in cash.
    pub weights: BTreeMap<String, f64>,
    /// The share of the portfolio's value bought (or, equally, sold): half the sum of the
    /// changes in weight, cash included.
    pub turnover: f64,
    /// The cost of the trade as a share of the portfolio's value.
    pub cost: f64,
    /// Whether the weights were refitted, rather than restored.
    pub refit: bool,
}

/// A refit that found no weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Failure {
    /// The date of the refit.
    pub date: Date,
    /// Why there were no weights, such as `infeasible`.
    pub reason: String,
}

impl Run {
    /// The trading cost over the run, as a share of value per year, with `periods_per_year`
    /// periods in a year.
    pub fn costs_per_year(&self, periods_per_year: f64) -> f64 {
        per_year(
            self.trades.iter().map(|t| t.cost).sum(),
            self.returns.len(),
            periods_per_year,
        )
    }

    /// The turnover over the run, as a share of value per year.
    pub fn turnover_per_year(&self, periods_per_year: f64) -> f64 {
        per_year(
            self.trades.iter().map(|t| t.turnover).sum(),
            self.returns.len(),
            periods_per_year,
        )
    }

    /// The growth of one unit over the run.
    pub fn growth(&self) -> f64 {
        self.returns.iter().map(|r| 1.0 + r).product()
    }
}

fn per_year(total: f64, periods: usize, periods_per_year: f64) -> f64 {
    if periods == 0 {
        return 0.0;
    }
    total / (periods as f64 / periods_per_year)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn month(d: Date) -> String {
    format!(
        "{} {}",
        MONTHS[(d.month().clamp(1, 12) - 1) as usize],
        d.year()
    )
}

/// `text` cut to `max` characters, ending in "…" when cut, and padded to `max`.
fn cell(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count > max {
        let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    } else {
        format!("{text}{}", " ".repeat(max - count))
    }
}

impl BacktestResults {
    /// The validation and data in a few words, such as "walk-forward on 5 years".
    fn describe(&self) -> String {
        let frequency = match self.periods_per_year.round() as i64 {
            250..=260 | 365 => "daily",
            52 => "weekly",
            12 => "monthly",
            4 => "quarterly",
            1 => "yearly",
            _ => "periodic",
        };
        let validation = match &self.settings.validation {
            Validation::InSample => "in sample".to_owned(),
            Validation::Holdout { start } => format!("holdout from {}", month(*start)),
            Validation::WalkForward { years, periods, .. } => match (years, periods) {
                (Some(y), _) if y.fract() == 0.0 => format!("walk-forward on {y} years"),
                (Some(y), _) => format!("walk-forward on {} months", (y * 12.0).round()),
                (None, Some(p)) => format!("walk-forward on {p} periods"),
                (None, None) => "walk-forward".to_owned(),
            },
        };
        let strategies = self.strategies.len();
        format!(
            "{validation} · {strategies} {} · {} assets · {frequency}",
            if strategies == 1 {
                "strategy"
            } else {
                "strategies"
            },
            self.assets.len()
        )
    }
}

/// The backtest in brief: each strategy's periods and growth in and out of sample, its trades,
/// refits and failed refits. Every return and trade is in the fields.
impl fmt::Display for BacktestResults {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "BacktestResults · {}", self.describe())?;
        writeln!(f)?;
        let out_of_sample = self.strategies.iter().any(|s| s.out_of_sample.is_some());
        let rows = self.strategies.iter().chain(self.benchmark.iter());
        let tested = |s: &StrategyResult| s.out_of_sample.clone().unwrap_or(s.in_sample.clone());
        if out_of_sample {
            writeln!(
                f,
                "  {}{:>15}  {:>35}",
                cell("", 24),
                "In sample",
                "Out of sample"
            )?;
            writeln!(
                f,
                "  {}{:>8}{:>7}  {:>8}{:>7}{:>7}{:>7}{:>7}",
                cell("Strategy", 24),
                "Periods",
                "Growth",
                "Periods",
                "Growth",
                "Trades",
                "Refits",
                "Failed"
            )?;
        } else {
            writeln!(
                f,
                "  {}{:>8}{:>7}{:>7}{:>7}",
                cell("Strategy", 24),
                "Periods",
                "Growth",
                "Trades",
                "Failed"
            )?;
        }
        for s in rows {
            let t = tested(s);
            let refits = t.trades.iter().filter(|x| x.refit).count();
            if out_of_sample {
                writeln!(
                    f,
                    "  {}{:>8}{:>7}  {:>8}{:>7}{:>7}{:>7}{:>7}",
                    cell(&s.name, 24),
                    s.in_sample.returns.len(),
                    format!("{:.2}x", s.in_sample.growth()),
                    t.returns.len(),
                    format!("{:.2}x", t.growth()),
                    t.trades.len(),
                    refits,
                    s.failures.len()
                )?;
            } else {
                writeln!(
                    f,
                    "  {}{:>8}{:>7}{:>7}{:>7}",
                    cell(&s.name, 24),
                    t.returns.len(),
                    format!("{:.2}x", t.growth()),
                    t.trades.len(),
                    s.failures.len()
                )?;
            }
        }
        writeln!(f)?;
        let first = self
            .strategies
            .first()
            .map(|s| s.out_of_sample.clone().unwrap_or(s.in_sample.clone()));
        if let Some(run) = first
            && let (Some(a), Some(b)) = (run.dates.first(), run.dates.last())
        {
            writeln!(
                f,
                "  {} {} to {}. Each strategy's returns and trades are in",
                if out_of_sample {
                    "Out of sample"
                } else {
                    "In sample"
                },
                month(*a),
                month(*b)
            )?;
            writeln!(f, "  results.strategies[k].out_of_sample and .in_sample.")?;
        }
        Ok(())
    }
}
