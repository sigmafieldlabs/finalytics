//! What a backtest finds: each strategy's returns, trades and failed refits, in and out of
//! sample.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_core::Date;

use crate::settings::Settings;

/// The result of a backtest: every strategy, and the benchmark if one was given, on the same
/// assets and dates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Backtest {
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
    /// The returns of weights tested on the history they were fitted to.
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
