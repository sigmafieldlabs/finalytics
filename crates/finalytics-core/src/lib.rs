//! Foundations shared by every Finalytics crate: dates and business-day calendars, currencies and
//! exact decimal amounts, time series, errors and validation reports, and numerical helpers. It
//! performs no input or output.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which
//! re-exports the public API of every layer.

mod calendar;
mod currency;
mod date;
mod error;
mod money;
pub mod numeric;
mod series;

pub use calendar::{BusinessDayConvention, Calendar};
pub use currency::Currency;
pub use date::{Date, Weekday, days_in_month, is_leap_year};
pub use error::{Error, Issue, Result, Severity, ValidationReport};
pub use money::{Money, Rounding};
pub use series::{Fill, Observation, TimeSeries, intersection_of_dates, union_of_dates};

/// Exact decimal numbers, used for quantities, prices and amounts in the ledger.
pub use rust_decimal::Decimal;
