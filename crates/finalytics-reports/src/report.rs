//! The six reports together.

use serde::{Deserialize, Serialize};

use finalytics_core::Result;
use finalytics_ledger::{Books, MarketData};

use crate::{
    AttributionReport, HoldingsReport, OverviewReport, PerformanceReport, ReportOptions,
    RiskReport, ScenariosReport, Source, attribution_report, holdings_report, overview_report,
    performance_report, risk_report, scenarios_report,
};

/// The six reports on a portfolio, from one set of options. The attribution report needs the
/// benchmark's segments, and the scenarios report needs scenarios, in the options; without
/// them, they are absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct PortfolioReport {
    /// Value, returns and risk at a glance, with the efficient frontier.
    pub overview: OverviewReport,
    /// Holdings, lots, income and realised gains.
    pub holdings: HoldingsReport,
    /// Returns over periods, contribution and flows.
    pub performance: PerformanceReport,
    /// Volatility, tail risk, drawdowns and where the risk comes from.
    pub risk: RiskReport,
    /// Attribution against the benchmark.
    pub attribution: Option<AttributionReport>,
    /// The effect of scenarios on value and risk.
    pub scenarios: Option<ScenariosReport>,
}

/// Every report on the books, from one set of options.
pub fn portfolio_report(
    books: &Books,
    market: &MarketData,
    source: &Source,
    options: &ReportOptions,
) -> Result<PortfolioReport> {
    Ok(PortfolioReport {
        overview: overview_report(books, market, source, options)?,
        holdings: holdings_report(books, market, source, options)?,
        performance: performance_report(books, market, source, options)?,
        risk: risk_report(books, market, source, options)?,
        attribution: match options.attribution {
            Some(_) => Some(attribution_report(books, market, source, options)?),
            None => None,
        },
        scenarios: match options.scenarios.is_empty() {
            false => Some(scenarios_report(books, market, source, options)?),
            true => None,
        },
    })
}
