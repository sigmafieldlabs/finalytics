//! Backtests over [`Assets`]: optimisation methods refitted through time, or fixed weights,
//! rebalanced on a schedule, net of trading costs, and validated in sample, on a holdout or
//! walk-forward.
//!
//! [`Assets::simulate`] runs a backtest from [`BacktestOptions`] and returns every strategy's
//! returns and trades, in and out of sample.
//!
//! ```
//! use finalytics::backtest::{BacktestOptions, Strategy, Validation};
//! use finalytics::{Assets, ObjectiveName, OptimiseOptions, datasets};
//!
//! let assets = Assets::from_prices(&datasets::asset_class_prices())?
//!     .with_risk_free_asset("T-Bills")?;
//! let mut options = BacktestOptions::default();
//! for objective in [ObjectiveName::EqualWeight, ObjectiveName::MinimumVariance] {
//!     let mut method = OptimiseOptions::default();
//!     method.objective = objective;
//!     options.strategies.push(Strategy::optimise(method));
//! }
//! options.validation = Validation::walk_forward(5.0);
//! options.trading_cost_bps = 10.0;
//! let backtest = assets.simulate(&options)?;
//! let minimum_variance = &backtest.strategies[1];
//! assert_eq!(minimum_variance.name, "Minimum variance");
//! let out = minimum_variance.out_of_sample.as_ref().unwrap();
//! assert_eq!(out.dates.len(), assets.dates().len() - 60);
//! # Ok::<(), finalytics::Error>(())
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use finalytics_backtest::{self as engine, Fit, Market, Plan};
use finalytics_core::{Date, Result, ValidationReport};
use finalytics_risk::PriceTable;

pub use finalytics_backtest::{
    Backtest, Failure, Rebalance, Run, Settings, StrategyResult, Trade, Validation,
};

use crate::assets::{Assets, ObjectiveName, OptimiseOptions};

/// A strategy to backtest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Strategy {
    /// An optimisation, refitted as the validation says. The limits of `options` apply to the
    /// methods that take them; equal weight, inverse volatility, risk parity and hierarchical
    /// risk parity are fully invested and long only.
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
            } => {
                if options.objective == ObjectiveName::RiskBudgeting && options.budgets.is_some() {
                    "Risk budgeting".to_owned()
                } else {
                    options.objective.label().to_owned()
                }
            }
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
    pub fn simulate(&self, options: &BacktestOptions) -> Result<Backtest> {
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
                    let (method, notes) = applicable(method, &name);
                    strategies.push(engine::Strategy {
                        name,
                        plan: Plan::Fit(Box::new(move |rows| {
                            let result = self.window(rows)?.optimise(&method)?;
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

/// The options a method takes, and notes on any limits it does not.
fn applicable(options: &OptimiseOptions, name: &str) -> (OptimiseOptions, Vec<String>) {
    let mut method = options.clone();
    let mut notes = Vec::new();
    if !options.objective.takes_limits() {
        let limited = options.bounds.is_some()
            || !options.asset_bounds.is_empty()
            || !options.groups.is_empty();
        method.bounds = None;
        method.asset_bounds.clear();
        method.groups.clear();
        if limited {
            notes.push(format!(
                "The limits on weights do not apply to {}, which is fully invested and long only.",
                name.to_lowercase()
            ));
        }
    }
    (method, notes)
}
