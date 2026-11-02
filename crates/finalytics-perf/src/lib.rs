//! Performance measurement: time-weighted returns chain-linked from daily values, money-weighted
//! returns, Modified Dietz, standard reporting periods with annualisation, comparison with a
//! benchmark, attribution of the excess return by segment and by currency, and households:
//! performance by account and by group of accounts, with each account's contribution.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod attribution;
mod contribution;
mod household;
mod performance;
mod periods;

pub use attribution::{
    Attribution, AttributionPeriod, CurrencyAttribution, CurrencyAttributionPeriod, CurrencyEffect,
    CurrencyPeriodAttribution, CurrencyReturn, CurrencySegment, Linking, PeriodAttribution,
    Segment, SegmentEffects, brinson_fachler, karnosky_singer,
};
pub use contribution::{Contribution, GroupContribution, group_contributions};
pub use household::{
    AccountsPerformance, Household, HouseholdOptions, SchemePerformance, household,
};
pub use performance::{
    DAYS_PER_YEAR, ExcessReturn, FlowTiming, Performance, PeriodReturn, annualise,
    internal_rate_of_return,
};
pub use periods::Period;
