//! Backtests of allocation strategies: weights held through time and rebalanced on a schedule,
//! net of trading costs, and validated in sample, on a holdout, or walk-forward, refitting on a
//! rolling or expanding window of history.
//!
//! The engine does not choose weights itself: a strategy is fixed weights, or a function that
//! fits weights to a range of history, which the [`finalytics`](https://docs.rs/finalytics) crate
//! provides from its optimisation methods. A refit that finds no weights keeps the weights held,
//! and is recorded.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.
//!
//! ```
//! # use finalytics_backtest as backtest;
//! use backtest::{Benchmark, Market, Plan, Rebalance, Settings, Strategy, Validation};
//! use finalytics_core::Date;
//! use finalytics_risk::ReturnsTable;
//!
//! let dates: Vec<Date> = ["2026-01-30", "2026-02-27", "2026-03-31", "2026-04-30"]
//!     .iter()
//!     .map(|d| d.parse().unwrap())
//!     .collect();
//! let returns = ReturnsTable::new(
//!     vec!["stocks".into(), "bonds".into()],
//!     vec![vec![0.04, -0.02, 0.03, 0.01], vec![0.00, 0.01, -0.01, 0.00]],
//! )?;
//! let cash = vec![0.003; 4];
//! let market = Market::new(&dates, &returns, &cash, 12.0);
//! let sixty_forty = Strategy {
//!     name: "60/40".into(),
//!     plan: Plan::Weights(vec![0.6, 0.4]),
//!     notes: vec![],
//! };
//! let settings = Settings::new(Validation::InSample).with_rebalance(Rebalance::Monthly);
//! let result = backtest::backtest(&market, vec![sixty_forty], None, &settings)?;
//! let run = &result.strategies[0].in_sample;
//! assert_eq!(run.returns.len(), 4);
//! assert!((run.returns[0] - 0.024).abs() < 1e-12);
//! assert_eq!(run.trades.len(), 3);
//! # Ok::<(), finalytics_core::Error>(())
//! ```

mod engine;
mod results;
mod settings;

pub use engine::{Benchmark, Fit, Fitter, Market, Plan, Strategy, backtest};
pub use results::{Backtest, Failure, Run, StrategyResult, Trade};
pub use settings::{Rebalance, Settings, Validation};
