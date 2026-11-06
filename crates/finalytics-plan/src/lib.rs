//! Goal planning: Monte Carlo simulations of money saved and spent towards one goal, month by
//! month, in today's money.
//!
//! A plan has one goal: a [`Goal::Target`], an amount by a date, or [`Goal::Payments`], an amount
//! paid regularly between two dates. Money is paid in and taken out by [`Flow`]s, invested in a
//! [`Mix`] of the assets, fixed or along a glide path, and each month's returns are drawn from
//! the assets' monthly history by a [`Model`]: a stationary block bootstrap, a normal
//! distribution, or every stretch of the history in turn. Every amount is in today's money: each
//! month's return becomes a return after inflation and fees.
//!
//! [`simulate`] gives the chance of meeting the goal, the value over time, what would change the
//! outcome (one change at a time, on the same simulations), the plan when markets do worse, and
//! the same for other mixes. Each simulation draws its numbers from a stream of its own, so the
//! results are the same on any number of threads.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod engine;
mod month;
mod options;
mod results;

pub use engine::{History, MAX_MONTHS, MAX_SIMULATED_MONTHS, MIN_HISTORY, simulate};
pub use month::Month;
pub use options::{Flow, Frequency, Goal, Inflation, Mix, Model, PlanOptions};
pub use results::{
    AmountChange, Changes, CrashWindow, FlowSummary, MixOutcome, MixRisk, MonthChange, Outcome,
    PaymentsOutcome, Percentiles, PlanResults, SamplePath, Status, WhatIf,
};
