//! Report data: one report per workstation screen (holdings, overview, performance, risk,
//! attribution and scenarios), each computed in one call from a portfolio's books and market
//! data; three reports on assets: the optimisation report, on the weights an optimisation
//! chose, the comparison report, on several methods backtested side by side, and the backtest
//! report, on one allocation; the rebalance report, on the trades that move a portfolio's
//! accounts to target weights; and the planning report, on a plan's chance of meeting its goal.
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
mod backtest_html;
mod backtest_text;
mod backtests;
mod context;
mod holdings;
mod html;
mod optimisation;
mod overview;
mod performance;
mod planning;
mod planning_html;
mod planning_text;
mod rebalance;
mod report;
mod risk;
mod scenarios;
mod shapes;
mod tables;
mod text;

pub use attribution::{
    AttributionInputs, AttributionReport, BenchmarkPeriod, BenchmarkSegment, attribution_report,
};
pub use backtests::{
    BacktestContext, BacktestReport, BacktestSetup, CalendarReturn, CashFlow, ComparisonReport,
    History, MoneyFigures, RunReport, StrategyReport, WeightsAt, backtest_report,
    comparison_report,
};
pub use context::{Provenance, ReportKind, ReportOptions, Source};
pub use holdings::{
    AccountHoldings, Concentration, HoldingRow, HoldingsReport, IncomeSummary, LotRow,
    RealisedSummary, holdings_report,
};
pub use html::Theme;
pub use optimisation::{
    AllocationRow, DrawdownFigure, GroupRow, OptimisationContext, OptimisationHistory,
    OptimisationReport, OptimisationSummary, ResamplingSummary, RiskReturn, ShortfallFigure,
};
#[doc(hidden)]
pub use optimisation::{OptimisationInputs, optimisation_report, weights_text};
pub use overview::{Headline, HoldingRiskReturn, OverviewReport, PortfolioPoint, overview_report};
pub use performance::{
    MonthFlows, PerformanceReport, PeriodRow, SchemeContributions, YearReturn, performance_report,
};
pub use planning::{InflationBasis, PlanContext, PlanReport};
#[doc(hidden)]
pub use planning::{PlanInputs, plan_report};
pub use rebalance::{
    AccountTrades, ConversionRow, HoldingMove, RebalanceContext, RebalanceEstimates,
    RebalanceReport, RebalanceSummary, TradeRow,
};
#[doc(hidden)]
pub use rebalance::{RebalanceEstimateInputs, RebalanceInputs, rebalance_report};
pub use report::{PortfolioReport, portfolio_report};
pub use risk::{HoldingsRisk, RiskReport, TailRow, risk_report};
pub use scenarios::{
    NamedScenario, ScenarioDefinition, ScenarioOutcome, ScenarioSummary, ScenariosReport,
    scenarios_report,
};
pub use shapes::{Amount, Bin, Breakdown, Grid, Histogram, Series, Slice, Unit};
pub use tables::{Cell, Column, ColumnKind, ReportTable, report_html, report_tables, report_text};
