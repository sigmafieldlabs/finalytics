//! Risk analytics: return statistics, drawdowns, value at risk and expected shortfall, and
//! rolling versions of each.
//!
//! The functions take returns as plain slices, one per period, so they serve both a portfolio's
//! returns from [`finalytics_perf`](https://docs.rs/finalytics-perf) and returns supplied
//! directly. Inputs are validated: a non-finite value or a return below -100% is reported by
//! position.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which re-exports
//! the public API of every layer.

mod distribution;
mod drawdown;
mod input;
mod rolling;
mod statistics;
mod tail;

pub use distribution::{normal_cdf, normal_pdf, normal_quantile};
pub use drawdown::{Drawdown, drawdown_series, drawdowns};
pub use rolling::{rolling, rolling_relative_statistics, rolling_statistics, rolling_tail_risk};
pub use statistics::{
    RelativeStatistics, Statistics, StatisticsOptions, max_drawdown, relative_statistics,
    statistics, total_return,
};
pub use tail::{TailMethod, TailRisk, tail_risk};
