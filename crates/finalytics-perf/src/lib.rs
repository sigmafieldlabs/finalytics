//! Performance measurement: time-weighted returns chain-linked from daily values, money-weighted
//! returns, Modified Dietz, standard reporting periods with annualisation, and comparison with a
//! benchmark.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which
//! re-exports the public API of every layer.

mod performance;
mod periods;

pub use performance::{
    DAYS_PER_YEAR, ExcessReturn, FlowTiming, Performance, PeriodReturn, annualise,
    internal_rate_of_return,
};
pub use periods::Period;
