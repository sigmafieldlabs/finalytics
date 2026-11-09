//! What every report shares: its options, its provenance, and the dates and series it is
//! computed on.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use finalytics_core::{Currency, Date, Decimal, Error, Result, TimeSeries, ValidationReport};
use finalytics_formats::PortfolioDocument;
use finalytics_ledger::{AccountId, Books, HoldingKey, InstrumentId, MarketData, PortfolioData};
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
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct ReportOptions {
    /// The date of the report: the last price date by default.
    pub as_of: Option<Date>,
    /// The first valuation date: the first transaction's date by default.
    pub start: Option<Date>,
    /// The accounts to report on, by identifier: every account by default. The figures are
    /// those of the books kept from their transactions alone, so transfers with the accounts
    /// left out count as money in and out, and each report says how many there are.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AccountId>,
    /// Classification schemes of the instruments to break holdings, contribution and risk down
    /// by, such as `asset_class` or `region`.
    pub group_by: Vec<String>,
    /// The benchmark: an instrument whose prices in the market data are its levels, such as an
    /// index, which need not be held; or a blend of instruments by weight, such as
    /// `{"VTI": 0.6, "BND": 0.4}`. Either is in the base currency, converted at the market
    /// data's exchange rates.
    pub benchmark: Option<ReportBenchmark>,
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

impl Default for ReportOptions {
    fn default() -> Self {
        ReportOptions {
            as_of: None,
            start: None,
            accounts: Vec::new(),
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

/// The benchmark of the portfolio reports.
///
/// An instrument's levels are in its currency, as the portfolio's instruments give it (the base
/// currency for one they do not list), and are converted to the base currency at the market
/// data's rates. A blend holds its instruments, and any cash, at the weights given, rebalanced to
/// them at the start of each month, each in the base currency in the same way; the attribution
/// report derives the benchmark's segments from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ReportBenchmark {
    /// An instrument, such as an index, by its identifier.
    Instrument(InstrumentId),
    /// Weights by holding, adding up to one: instruments, such as funds or indices with prices,
    /// and cash, such as `USD cash`.
    Blend(BTreeMap<HoldingKey, f64>),
}

impl From<InstrumentId> for ReportBenchmark {
    fn from(id: InstrumentId) -> ReportBenchmark {
        ReportBenchmark::Instrument(id)
    }
}

impl ReportBenchmark {
    /// The instrument, for a benchmark that is one.
    pub fn instrument(&self) -> Option<&InstrumentId> {
        match self {
            ReportBenchmark::Instrument(id) => Some(id),
            ReportBenchmark::Blend(_) => None,
        }
    }
}

/// The benchmark resolved on the reports' dates: its name, a line saying what it is, and its
/// levels in the base currency.
#[derive(Debug, Clone)]
pub(crate) struct BenchmarkLevels {
    /// The name in charts and tables: the instrument's, or "Benchmark" for a blend.
    pub(crate) name: String,
    /// What it is, for the reports' headers.
    pub(crate) description: String,
    /// Its levels on the valuation dates, when every date has them.
    pub(crate) levels: Option<Vec<f64>>,
    /// Why it has no levels, when it has none.
    pub(crate) missing: Option<String>,
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
    /// The benchmark, as the reports describe it, such as "BENCH, in USD, converted to EUR at
    /// the market data's exchange rates".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub benchmark: Option<String>,
    /// The accounts the report covers, when not every one, as the reports describe them, such
    /// as "Brokerage only (1 of 2 accounts). 3 transfers with Retirement count as money in and
    /// out."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accounts: Option<String>,
}

/// The books of the accounts `options` choose: `books` themselves when they choose none, or
/// every account the books keep, and otherwise the books kept from the chosen accounts'
/// transactions. An unknown account is an error at `options.accounts`.
pub(crate) fn chosen_books<'a>(
    books: &'a Books,
    options: &ReportOptions,
) -> Result<Cow<'a, Books>> {
    let chosen = &options.accounts;
    let held: Vec<&AccountId> = books.accounts().iter().map(|a| &a.id).collect();
    if chosen.is_empty() || (held.len() == chosen.len() && chosen.iter().all(|a| held.contains(&a)))
    {
        return Ok(Cow::Borrowed(books));
    }
    books
        .for_accounts(chosen)
        .map(Cow::Owned)
        .map_err(|e| match e {
            Error::Validation(report) => {
                let mut moved = ValidationReport::new();
                for issue in report.issues() {
                    moved.error(format!("options.{}", issue.path), issue.message.clone());
                }
                Error::Validation(moved)
            }
            other => other,
        })
}

/// The accounts books keep, in words: "Brokerage only (1 of 2 accounts). 3 transfers with
/// Retirement count as money in and out."
fn describe_scope(
    scope: &finalytics_ledger::AccountScope,
    kept: &[finalytics_ledger::Account],
) -> String {
    let and_list = |names: &[String]| match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    let name = |id: &AccountId, accounts: &[finalytics_ledger::Account]| {
        accounts
            .iter()
            .find(|a| a.id == *id)
            .and_then(|a| a.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let kept: Vec<String> = scope.accounts.iter().map(|id| name(id, kept)).collect();
    let mut text = format!(
        "{} only ({} of {} accounts).",
        and_list(&kept),
        scope.accounts.len(),
        scope.of
    );
    let count: usize = scope.transfers.values().sum();
    if count > 0 {
        let with: Vec<String> = scope
            .transfers
            .keys()
            .map(|id| name(id, &scope.left_out))
            .collect();
        text.push_str(&format!(
            " {count} {} with {} {} as money in and out.",
            if count == 1 { "transfer" } else { "transfers" },
            and_list(&with),
            if count == 1 { "counts" } else { "count" }
        ));
    }
    text
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
    /// The benchmark, if the options name one.
    pub(crate) benchmark: Option<BenchmarkLevels>,
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
        match &options.benchmark {
            Some(ReportBenchmark::Instrument(b)) if market.prices(b).is_none() => {
                report.error(
                    "options.benchmark",
                    format!("{b} has no prices in the market data"),
                );
            }
            Some(ReportBenchmark::Blend(weights)) => {
                if weights.is_empty() {
                    report.error("options.benchmark", "a blend needs at least one holding");
                }
                for (key, w) in weights {
                    let path = format!("options.benchmark.{key}");
                    if !(w.is_finite() && *w >= 0.0) {
                        report.error(&path, format!("must be a weight of zero or more, not {w}"));
                    }
                    if let HoldingKey::Instrument(id) = key
                        && market.prices(id).is_none()
                    {
                        report.error(path, format!("{id} has no prices in the market data"));
                    }
                }
                let total: f64 = weights.values().sum();
                if !weights.is_empty() && (total - 1.0).abs() > 1e-6 {
                    report.error(
                        "options.benchmark",
                        format!("the blend's weights add up to {total}, not one"),
                    );
                }
            }
            _ => {}
        }
        let known: BTreeSet<&String> = books
            .instruments()
            .flat_map(|i| i.classifications.keys())
            .collect();
        let mut schemes = BTreeSet::new();
        for (k, scheme) in options.group_by.iter().enumerate() {
            if !schemes.insert(scheme) {
                report.error(
                    format!("options.group_by[{k}]"),
                    format!("{scheme} is listed twice"),
                );
            } else if !known.contains(scheme) {
                let names: Vec<&str> = known.iter().map(|s| s.as_str()).collect();
                report.error(
                    format!("options.group_by[{k}]"),
                    if names.is_empty() {
                        format!(
                            "no instrument is classified by {scheme}: give class.{scheme} columns"
                        )
                    } else {
                        format!(
                            "no instrument is classified by {scheme}; the schemes are {}",
                            names.join(", ")
                        )
                    },
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
        let base = books.base_currency();
        held_options(books, options, as_of, base)?;
        let benchmark = options
            .benchmark
            .as_ref()
            .map(|b| resolve_benchmark(books, market, &dates, base, b));
        Ok(Context {
            books,
            market,
            source,
            options,
            as_of,
            start,
            dates,
            base,
            benchmark,
        })
    }

    pub(crate) fn provenance(&self, report: ReportKind, warnings: ValidationReport) -> Provenance {
        let mut warnings = warnings;
        if let Some(why) = self.benchmark.as_ref().and_then(|b| b.missing.clone()) {
            warnings.warning("options.benchmark", why);
        }
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
            benchmark: self.benchmark.as_ref().map(|b| b.description.clone()),
            accounts: self
                .books
                .scope()
                .map(|scope| describe_scope(scope, self.books.accounts())),
        }
    }

    /// The benchmark's name in charts and tables.
    pub(crate) fn benchmark_name(&self) -> String {
        self.benchmark
            .as_ref()
            .map_or_else(String::new, |b| b.name.clone())
    }

    pub(crate) fn statistics_options(&self) -> StatisticsOptions {
        let mut options = StatisticsOptions::default();
        options.periods_per_year = self.options.periods_per_year;
        options.risk_free_rate = self.options.risk_free_rate;
        // Sharpe and Sortino ratios are both measured against the risk-free rate.
        options.target_return = self.options.risk_free_rate;
        options
    }

    /// The benchmark's levels on the valuation dates in the base currency, carried forward as
    /// prices are.
    pub(crate) fn benchmark_levels(&self) -> Option<Vec<f64>> {
        self.benchmark.as_ref()?.levels.clone()
    }
}

/// A holding's value per unit in the base currency on `date`: an instrument's price at the
/// rate of its currency (the base currency for an instrument the books do not list), and a unit
/// of cash at its rate.
pub(crate) fn unit_value(
    books: &Books,
    market: &MarketData,
    key: &HoldingKey,
    date: Date,
    base: Currency,
) -> UnitValue {
    let (price, currency) = match key {
        HoldingKey::Cash(c) => (Some(1.0), *c),
        HoldingKey::Instrument(id) => (
            market.price(id, date).value().copied().map(to_f64),
            books.instrument(id).map_or(base, |i| i.currency),
        ),
    };
    let Some(price) = price.filter(|p| *p > 0.0) else {
        return UnitValue::NoPrice;
    };
    match market.fx_rate(currency, base, date).value().copied() {
        Some(rate) => UnitValue::Value(price * to_f64(rate)),
        None => UnitValue::NoRate(price),
    }
}

/// A holding's value per unit in the base currency, or why there is none.
pub(crate) enum UnitValue {
    /// The value.
    Value(f64),
    /// No price on or shortly before the date.
    NoPrice,
    /// A price, in its own currency, but no rate to the base currency.
    NoRate(f64),
}

/// Percentages in words, such as "55%".
/// Checks the options that name holdings by what is held at the report's date, `as_of`, so that
/// they are refused when the report is built rather than dropping a section: the tracking
/// error's benchmark weights, adding up to one with the cash, and the expected returns.
fn held_options(books: &Books, options: &ReportOptions, as_of: Date, base: Currency) -> Result<()> {
    if options.benchmark_weights.is_empty() && options.expected_returns.is_empty() {
        return Ok(());
    }
    let holdings = books.holdings_on(as_of);
    let held = |key: &HoldingKey| match key {
        HoldingKey::Instrument(id) => holdings
            .positions()
            .iter()
            .any(|p| &p.instrument == id && !p.quantity.is_zero()),
        HoldingKey::Cash(c) => {
            *c == base
                || holdings
                    .cash()
                    .any(|(_, cash, amount)| cash == *c && !amount.is_zero())
        }
    };
    let known = |key: &HoldingKey| match key {
        HoldingKey::Instrument(id) => books.instrument(id).is_some(),
        HoldingKey::Cash(_) => true,
    };
    let mut report = ValidationReport::new();
    let mut check = |key: &HoldingKey, value: f64, path: String| {
        if !known(key) {
            report.error(path, format!("unknown asset {key}"));
            false
        } else if !held(key) {
            report.error(
                path,
                format!(
                    "{} is not held on {as_of}, so its risk is not estimated",
                    label(key)
                ),
            );
            false
        } else if !value.is_finite() {
            report.error(path, format!("must be a finite number, not {value}"));
            false
        } else {
            true
        }
    };
    let mut fine = true;
    for (key, w) in &options.benchmark_weights {
        fine &= check(key, *w, format!("options.benchmark_weights.{key}"));
    }
    for (id, r) in &options.expected_returns {
        check(
            &HoldingKey::Instrument(id.clone()),
            *r,
            format!("options.expected_returns.{id}"),
        );
    }
    let total: f64 = options.benchmark_weights.values().sum();
    if fine && !options.benchmark_weights.is_empty() && (total - 1.0).abs() > 1e-6 {
        report.error(
            "options.benchmark_weights",
            format!(
                "add up to {total}, not one; give {}, the risk-free asset, any cash",
                HoldingKey::Cash(base)
            ),
        );
    }
    report.into_result(())
}

fn share_words(w: f64) -> String {
    let text = format!("{:.2}", w * 100.0);
    format!("{}%", text.trim_end_matches('0').trim_end_matches('.'))
}

/// The benchmark `b` on `dates`, in `base`.
fn resolve_benchmark(
    books: &Books,
    market: &MarketData,
    dates: &[Date],
    base: Currency,
    b: &ReportBenchmark,
) -> BenchmarkLevels {
    match b {
        ReportBenchmark::Instrument(id) => {
            let key = HoldingKey::Instrument(id.clone());
            let listed = books.instrument(id).map(|i| i.currency);
            let currency = listed.unwrap_or(base);
            let values: Vec<UnitValue> = dates
                .iter()
                .map(|&d| unit_value(books, market, &key, d, base))
                .collect();
            let no_price = values.iter().any(|v| matches!(v, UnitValue::NoPrice));
            let no_rate = values
                .iter()
                .filter(|v| matches!(v, UnitValue::NoRate(_)))
                .count();
            let mut description = match listed {
                None => format!(
                    "{id}, taken to be in {base}, the base currency, as the instruments do not \
                     list it"
                ),
                Some(c) if c == base => format!("{id}, in {base}"),
                Some(c) => format!("{id}, in {c}, converted to {base} at the market data's rates"),
            };
            let (levels, missing) = if no_price {
                (
                    None,
                    Some(format!(
                        "{id} has no price on some valuation dates, so the benchmark is left out"
                    )),
                )
            } else if no_rate == values.len() {
                // No rate at all: the levels are compared as they are, and the reports say so.
                description = format!(
                    "{id}, in {currency}: the market data has no {currency} to {base} rate, so \
                     its levels are compared unconverted"
                );
                (
                    Some(
                        values
                            .iter()
                            .map(|v| match v {
                                UnitValue::NoRate(p) => *p,
                                _ => f64::NAN,
                            })
                            .collect(),
                    ),
                    None,
                )
            } else if no_rate > 0 {
                (
                    None,
                    Some(format!(
                        "the market data has no {currency} to {base} rate on some valuation \
                         dates, so the benchmark is left out"
                    )),
                )
            } else {
                (
                    Some(
                        values
                            .iter()
                            .map(|v| match v {
                                UnitValue::Value(x) => *x,
                                _ => f64::NAN,
                            })
                            .collect(),
                    ),
                    None,
                )
            };
            BenchmarkLevels {
                name: id.to_string(),
                description,
                levels,
                missing,
            }
        }
        ReportBenchmark::Blend(weights) => {
            let mut parts: Vec<(&HoldingKey, f64)> = weights.iter().map(|(k, w)| (k, *w)).collect();
            parts.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            let named: Vec<String> = parts
                .iter()
                .map(|(k, w)| format!("{} {k}", share_words(*w)))
                .collect();
            let foreign = parts.iter().any(|(k, _)| {
                let c = match k {
                    HoldingKey::Cash(c) => *c,
                    HoldingKey::Instrument(id) => books.instrument(id).map_or(base, |i| i.currency),
                };
                c != base
            });
            let description = format!(
                "a blend of {}, rebalanced monthly{}",
                and_list(&named),
                if foreign {
                    format!(", in {base} at the market data's rates")
                } else {
                    String::new()
                }
            );
            let (levels, missing) = match blend_levels(books, market, dates, base, &parts) {
                Ok(levels) => (Some(levels), None),
                Err(why) => (None, Some(why)),
            };
            BenchmarkLevels {
                name: "Benchmark".to_owned(),
                description,
                levels,
                missing,
            }
        }
    }
}

/// The blend's levels on `dates`, from 100: each holding at its weight, rebalanced after the last
/// date of each month.
fn blend_levels(
    books: &Books,
    market: &MarketData,
    dates: &[Date],
    base: Currency,
    parts: &[(&HoldingKey, f64)],
) -> std::result::Result<Vec<f64>, String> {
    let mut prices: Vec<Vec<f64>> = Vec::with_capacity(parts.len());
    for (key, _) in parts {
        let mut column = Vec::with_capacity(dates.len());
        for &d in dates {
            match unit_value(books, market, key, d, base) {
                UnitValue::Value(v) => column.push(v),
                UnitValue::NoPrice => {
                    return Err(format!(
                        "{key}, in the blended benchmark, has no price on {d}, so the benchmark \
                         is left out"
                    ));
                }
                UnitValue::NoRate(_) => {
                    return Err(format!(
                        "the market data has no rate to {base} on {d} for {key}, in the blended \
                         benchmark, so the benchmark is left out"
                    ));
                }
            }
        }
        prices.push(column);
    }
    let mut level = 100.0;
    let mut levels = Vec::with_capacity(dates.len());
    let mut units: Vec<f64> = parts
        .iter()
        .zip(&prices)
        .map(|((_, w), p)| level * w / p[0])
        .collect();
    for t in 0..dates.len() {
        level = units.iter().zip(&prices).map(|(u, p)| u * p[t]).sum();
        levels.push(level);
        let month_ends = t + 1 < dates.len()
            && (dates[t + 1].year(), dates[t + 1].month()) != (dates[t].year(), dates[t].month());
        if month_ends {
            units = parts
                .iter()
                .zip(&prices)
                .map(|((_, w), p)| level * w / p[t])
                .collect();
        }
    }
    Ok(levels)
}

/// Names joined as in a sentence: "a, b and c".
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
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
