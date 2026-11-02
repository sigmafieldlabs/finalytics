//! What every report shares: its options, its provenance, and the dates and series it is
//! computed on.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Currency, Date, Decimal, Error, Result, TimeSeries, ValidationReport};
use finalytics_formats::PortfolioDocument;
use finalytics_ledger::{Books, HoldingKey, InstrumentId, MarketData, PortfolioData};
use finalytics_perf::FlowTiming;
use finalytics_risk::{CovarianceMethod, StatisticsOptions};

use crate::attribution::AttributionInputs;
use crate::scenarios::NamedScenario;

/// Which report a report is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReportKind {
    /// The holdings report.
    Holdings,
    /// The overview report.
    Overview,
    /// The performance report.
    Performance,
    /// The risk report.
    Risk,
    /// The attribution report.
    Attribution,
    /// The scenarios report.
    Scenarios,
}

/// The options every report takes. Each report uses the ones that concern it; the others are
/// ignored, so one set of options can serve every screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
#[non_exhaustive]
pub struct ReportOptions {
    /// The date of the report: the last price date by default.
    pub as_of: Option<Date>,
    /// The first valuation date: the first transaction's date by default.
    pub start: Option<Date>,
    /// Classification schemes of the instruments to break holdings, contribution and risk down
    /// by, such as `asset_class` or `region`.
    pub group_by: Vec<String>,
    /// An instrument whose prices in the market data are the benchmark's levels, such as an
    /// index. It need not be held.
    pub benchmark: Option<InstrumentId>,
    /// The annual risk-free rate, for Sharpe ratios. The default is 0.
    pub risk_free_rate: f64,
    /// Valuation periods in a year, for annualising: 252 (trading days) by default.
    pub periods_per_year: f64,
    /// When external flows happen within the day: at the start (the default) or the end.
    pub timing: FlowTiming,
    /// How the covariance of the holdings' returns is estimated: Ledoit–Wolf by default.
    pub covariance: CovarianceMethod,
    /// The confidences of value at risk and expected shortfall: 0.95 and 0.99 by default.
    pub confidences: Vec<f64>,
    /// The window of rolling statistics, in valuation periods: 63 by default.
    pub rolling_window: usize,
    /// The number of bins of return histograms: 30 by default.
    pub histogram_bins: usize,
    /// How many holdings lists of the largest, the top contributors and the like show: 10 by
    /// default.
    pub top: usize,
    /// The holding period, in days, after which a lot or a gain is long-term: a lot held for
    /// more than this many days is long-term, as in rebalancing's tax rates. 365 by default.
    pub long_term_after_days: i32,
    /// Annual expected returns by instrument for the efficient frontier; instruments not listed,
    /// and every instrument when none is, take their historical mean return.
    pub expected_returns: BTreeMap<InstrumentId, f64>,
    /// The number of portfolios on the efficient frontier: 25 by default.
    pub frontier_points: usize,
    /// Benchmark weights by holding, for the ex-ante tracking error.
    pub benchmark_weights: BTreeMap<HoldingKey, f64>,
    /// The benchmark's segments, for the attribution report.
    pub attribution: Option<AttributionInputs>,
    /// The scenarios of the scenarios report.
    pub scenarios: Vec<NamedScenario>,
}

impl ReportOptions {
    /// The same options with `benchmark`: an instrument, such as an index, whose prices in the
    /// market data are the benchmark's levels.
    pub fn with_benchmark(mut self, benchmark: InstrumentId) -> Self {
        self.benchmark = Some(benchmark);
        self
    }

    /// The same options breaking holdings, contribution and risk down by each classification
    /// scheme in `schemes`, such as `asset_class` or `region`.
    pub fn with_group_by<S: Into<String>>(mut self, schemes: impl IntoIterator<Item = S>) -> Self {
        self.group_by = schemes.into_iter().map(Into::into).collect();
        self
    }

    /// The same options with an annual risk-free rate, for Sharpe ratios.
    pub fn with_risk_free_rate(mut self, rate: f64) -> Self {
        self.risk_free_rate = rate;
        self
    }

    /// The same options reporting on `date` rather than the last price date.
    pub fn with_as_of(mut self, date: Date) -> Self {
        self.as_of = Some(date);
        self
    }

    /// The same options with the benchmark's segments, for the attribution report.
    pub fn with_attribution(mut self, inputs: AttributionInputs) -> Self {
        self.attribution = Some(inputs);
        self
    }

    /// The same options with `scenarios`, for the scenarios report.
    pub fn with_scenarios(mut self, scenarios: impl IntoIterator<Item = NamedScenario>) -> Self {
        self.scenarios = scenarios.into_iter().collect();
        self
    }
}

impl Default for ReportOptions {
    fn default() -> Self {
        ReportOptions {
            as_of: None,
            start: None,
            group_by: Vec::new(),
            benchmark: None,
            risk_free_rate: 0.0,
            periods_per_year: 252.0,
            timing: FlowTiming::StartOfDay,
            covariance: CovarianceMethod::LedoitWolf,
            confidences: vec![0.95, 0.99],
            rolling_window: 63,
            histogram_bins: 30,
            top: 10,
            long_term_after_days: 365,
            expected_returns: BTreeMap::new(),
            frontier_points: 25,
            benchmark_weights: BTreeMap::new(),
            attribution: None,
            scenarios: Vec::new(),
        }
    }
}

/// What a report was computed from: a digest of the portfolio document and market data, and
/// their size. Computing it once and passing it to each report avoids hashing the inputs again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Source {
    /// The SHA-256 digest, in hexadecimal, of the portfolio document with its market data, as
    /// the engine writes it. The same inputs give the same digest in every language.
    pub digest: String,
    /// The number of transactions.
    pub transactions: usize,
    /// The number of accounts.
    pub accounts: usize,
    /// The number of instruments.
    pub instruments: usize,
}

impl Source {
    /// The source of reports on `portfolio` with `market`.
    pub fn of(portfolio: &PortfolioData, market: &MarketData) -> Source {
        let mut document = PortfolioDocument::new(portfolio.clone());
        document.market = Some(market.clone());
        let digest = Sha256::digest(document.to_json().as_bytes());
        Source {
            digest: digest.iter().map(|b| format!("{b:02x}")).collect(),
            transactions: portfolio.transactions.len(),
            accounts: portfolio.accounts.len(),
            instruments: portfolio.instruments.len(),
        }
    }
}

/// Where a report's numbers come from, so that a saved report can be checked to reproduce.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Provenance {
    /// Which report this is.
    pub report: ReportKind,
    /// The engine: "finalytics".
    pub engine: String,
    /// The engine's version.
    pub version: String,
    /// The date of the report.
    pub as_of: Date,
    /// The first valuation date.
    pub start: Date,
    /// The currency of every amount.
    pub base_currency: Currency,
    /// The inputs.
    pub source: Source,
    /// The options the report was computed with.
    pub options: ReportOptions,
    /// Prices and rates carried forward from earlier dates, and figures that could not be
    /// computed.
    pub warnings: ValidationReport,
}

/// The dates, options and inputs a report is computed on.
pub(crate) struct Context<'a> {
    pub(crate) books: &'a Books,
    pub(crate) market: &'a MarketData,
    pub(crate) source: &'a Source,
    pub(crate) options: &'a ReportOptions,
    pub(crate) as_of: Date,
    pub(crate) start: Date,
    /// The valuation dates: every price date from `start` to `as_of`.
    pub(crate) dates: Vec<Date>,
    pub(crate) base: Currency,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        books: &'a Books,
        market: &'a MarketData,
        source: &'a Source,
        options: &'a ReportOptions,
    ) -> Result<Context<'a>> {
        let mut report = ValidationReport::new();
        if !(options.periods_per_year.is_finite() && options.periods_per_year > 0.0) {
            report.error("options.periods_per_year", "must be greater than zero");
        }
        if !options.risk_free_rate.is_finite() {
            report.error("options.risk_free_rate", "must be a finite number");
        }
        for (k, c) in options.confidences.iter().enumerate() {
            if !(*c > 0.0 && *c < 1.0) {
                report.error(
                    format!("options.confidences[{k}]"),
                    "must be between 0 and 1, exclusive",
                );
            }
        }
        if options.rolling_window < 2 {
            report.error("options.rolling_window", "must be at least 2");
        }
        if options.histogram_bins == 0 {
            report.error("options.histogram_bins", "must be at least 1");
        }
        if options.frontier_points < 2 {
            report.error("options.frontier_points", "must be at least 2");
        }
        if let Some(b) = &options.benchmark
            && market.prices(b).is_none()
        {
            report.error(
                "options.benchmark",
                format!("{b} has no prices in the market data"),
            );
        }
        let mut schemes = BTreeSet::new();
        for (k, scheme) in options.group_by.iter().enumerate() {
            if !schemes.insert(scheme) {
                report.error(
                    format!("options.group_by[{k}]"),
                    format!("{scheme} is listed twice"),
                );
            }
        }
        report.into_result(())?;

        let all: BTreeSet<Date> = market
            .all_prices()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .collect();
        let as_of = match options.as_of {
            Some(d) => d,
            None => *all.iter().next_back().ok_or_else(|| {
                invalid(
                    "options.as_of",
                    "give a date: the market data has no prices",
                )
            })?,
        };
        let first_transaction = books.events().iter().map(|e| e.date).min();
        let start = options
            .start
            .or(first_transaction)
            .or_else(|| all.iter().next().copied())
            .unwrap_or(as_of);
        if start > as_of {
            return Err(invalid(
                "options.start",
                format!("{start} is after the report's date, {as_of}"),
            ));
        }
        let mut dates: Vec<Date> = all
            .iter()
            .copied()
            .filter(|d| *d >= start && *d <= as_of)
            .collect();
        if dates.first() != Some(&start) {
            dates.insert(0, start);
        }
        if dates.last() != Some(&as_of) {
            dates.push(as_of);
        }
        Ok(Context {
            books,
            market,
            source,
            options,
            as_of,
            start,
            dates,
            base: books.base_currency(),
        })
    }

    pub(crate) fn provenance(&self, report: ReportKind, warnings: ValidationReport) -> Provenance {
        Provenance {
            report,
            engine: "finalytics".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            as_of: self.as_of,
            start: self.start,
            base_currency: self.base,
            source: self.source.clone(),
            options: self.options.clone(),
            warnings,
        }
    }

    pub(crate) fn statistics_options(&self) -> StatisticsOptions {
        let mut options = StatisticsOptions::default();
        options.periods_per_year = self.options.periods_per_year;
        options.risk_free_rate = self.options.risk_free_rate;
        options.target_return = 0.0;
        options
    }

    /// The benchmark's levels on the valuation dates, carried forward as prices are.
    pub(crate) fn benchmark_levels(&self) -> Option<Vec<f64>> {
        let id = self.options.benchmark.as_ref()?;
        self.dates
            .iter()
            .map(|&d| {
                self.market
                    .price(id, d)
                    .value()
                    .copied()
                    .map(to_f64)
                    .filter(|v| *v > 0.0)
            })
            .collect()
    }
}

pub(crate) fn invalid(path: &str, message: impl Into<String>) -> Error {
    let mut report = ValidationReport::new();
    report.error(path, message);
    Error::Validation(report)
}

pub(crate) fn to_f64(value: Decimal) -> f64 {
    f64::try_from(value).unwrap_or(f64::NAN)
}

/// A series of `values` on `dates` as a time series, which must be in order.
pub(crate) fn time_series(dates: &[Date], values: &[f64]) -> Option<TimeSeries<f64>> {
    TimeSeries::new(dates.to_vec(), values.to_vec()).ok()
}

/// The display name of a holding: the instrument's identifier, or "USD cash".
pub(crate) fn label(key: &HoldingKey) -> String {
    key.to_string()
}

/// The group of a holding under an instrument classification scheme: the instrument's
/// classification, "unclassified" without one, and "cash" for cash.
pub(crate) fn group_of(books: &Books, key: &HoldingKey, scheme: &str) -> String {
    match key {
        HoldingKey::Cash(_) => "cash".to_owned(),
        HoldingKey::Instrument(id) => books
            .instrument(id)
            .and_then(|i| i.classifications.get(scheme))
            .cloned()
            .unwrap_or_else(|| "unclassified".to_owned()),
    }
}
