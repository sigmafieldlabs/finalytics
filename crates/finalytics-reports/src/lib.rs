//! Report data: one report per workstation screen (holdings, overview, performance, risk,
//! attribution and scenarios), each computed in one call from a portfolio's books and market
//! data.
//!
//! A report carries everything its screen draws and exports, not only summary figures: series
//! over time, breakdowns, grids, distributions and tables, each labelled with its units, so a
//! front end draws charts and tables without computing anything. Every report has a
//! [`Provenance`]: the engine version, the dates, the options and a SHA-256 digest of the
//! portfolio document and market data it was computed from. [`report_tables`] turns any report
//! into labelled tables for export to CSV, spreadsheets or PDF. The engine renders and writes
//! nothing.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod attribution;
mod context;
mod holdings;
mod overview;
mod performance;
mod risk;
mod scenarios;
mod shapes;
mod tables;

pub use attribution::{
    AttributionInputs, AttributionReport, BenchmarkPeriod, BenchmarkSegment, attribution_report,
};
pub use context::{Provenance, ReportKind, ReportOptions, Source};
pub use holdings::{
    AccountHoldings, Concentration, HoldingRow, HoldingsReport, IncomeSummary, LotRow,
    RealisedSummary, holdings_report,
};
pub use overview::{Headline, HoldingRiskReturn, OverviewReport, PortfolioPoint, overview_report};
pub use performance::{
    MonthFlows, PerformanceReport, PeriodRow, SchemeContributions, YearReturn, performance_report,
};
pub use risk::{HoldingsRisk, RiskReport, TailRow, risk_report};
pub use scenarios::{
    NamedScenario, ScenarioDefinition, ScenarioOutcome, ScenarioSummary, ScenariosReport,
    scenarios_report,
};
pub use shapes::{Amount, Bin, Breakdown, Grid, Histogram, Series, Slice, Unit};
pub use tables::{Cell, Column, ColumnKind, ReportTable, report_tables};
