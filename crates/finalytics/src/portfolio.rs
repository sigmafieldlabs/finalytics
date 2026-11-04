//! Portfolio: a portfolio's records and market data, with the books kept from them.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use finalytics_core::{Currency, Date, Decimal, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, PortfolioDocument, market_from_csv, portfolio_from_csv};
use finalytics_ledger::LotMethod;
use finalytics_ledger::{
    AccountId, Books, HoldingKey, MarketData, PortfolioData, Transaction, Valuation, ValueSeries,
};
use finalytics_opt::{
    RebalanceOptions, RebalanceResults, SolveStatus, TaxRates, rebalance_accounts,
};
use finalytics_perf::{FlowTiming, Performance};
use finalytics_reports::{
    AttributionInputs, AttributionReport, HoldingsReport, NamedScenario, OverviewReport,
    PerformanceReport, PortfolioReport, RebalanceReport, ReportOptions, RiskReport,
    ScenariosReport, Source, attribution_report, holdings_report, overview_report,
    performance_report, portfolio_report, risk_report, scenarios_report,
};
use finalytics_risk::{holding_returns, holding_weights};

use crate::{Assets, OptimisationResult};

/// A portfolio: its records (base currency, accounts, instruments and transactions), the market
/// data that values it, and the books kept from them. Everything Finalytics measures about a
/// portfolio starts here.
///
/// A portfolio does not change: [`Portfolio::with_transactions`] returns a new one.
///
/// ```
/// use finalytics::{Portfolio, PortfolioCsv};
///
/// let transactions = "\
/// date,type,instrument,quantity,price,amount,currency
/// 2026-01-02,deposit,,,,10000,USD
/// 2026-01-02,buy,VOO,10,500,,
/// ";
/// let prices = "date,VOO\n2026-01-02,500\n2026-01-05,510\n";
/// let portfolio = Portfolio::from_csv(&PortfolioCsv::new(transactions).prices(prices))?;
/// assert_eq!(portfolio.valuation(None)?.total_base.to_string(), "10100");
/// # Ok::<(), finalytics::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Portfolio {
    data: PortfolioData,
    market: MarketData,
    books: Books,
}

/// The CSV files of a portfolio, for [`Portfolio::from_csv`]. Only the transactions are
/// required. The [data guide] describes every column, and
/// [`datasets::file`](crate::datasets::file) gives an example of each file.
///
/// [data guide]: https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md
#[derive(Debug, Clone, Copy)]
pub struct PortfolioCsv<'a> {
    transactions: &'a str,
    instruments: Option<&'a str>,
    accounts: Option<&'a str>,
    prices: Option<&'a str>,
    fx: Option<&'a str>,
    base_currency: Option<Currency>,
    options: CsvOptions,
}

impl<'a> PortfolioCsv<'a> {
    /// The portfolio's transactions: a `date`, a `type` and the columns the type needs (see
    /// [`formats::transactions_from_csv`](crate::formats::transactions_from_csv)).
    pub fn new(transactions: &'a str) -> PortfolioCsv<'a> {
        PortfolioCsv {
            transactions,
            instruments: None,
            accounts: None,
            prices: None,
            fx: None,
            base_currency: None,
            options: CsvOptions::default(),
        }
    }

    /// Its instruments, with their kinds, currencies and classifications. Without them, every
    /// instrument the transactions name is of kind `other`, in the currency given with it in
    /// the transactions or else in the base currency.
    pub fn instruments(mut self, text: &'a str) -> Self {
        self.instruments = Some(text);
        self
    }

    /// Its accounts. Without them, every account the transactions name is created.
    pub fn accounts(mut self, text: &'a str) -> Self {
        self.accounts = Some(text);
        self
    }

    /// The prices of its instruments, and of any benchmark.
    pub fn prices(mut self, text: &'a str) -> Self {
        self.prices = Some(text);
        self
    }

    /// Exchange rates, for instruments and cash in other currencies than the base currency.
    pub fn fx(mut self, text: &'a str) -> Self {
        self.fx = Some(text);
        self
    }

    /// The currency results are reported in. Without it, the currency of the first deposit.
    pub fn base_currency(mut self, currency: Currency) -> Self {
        self.base_currency = Some(currency);
        self
    }

    /// The field delimiter of every file: a comma by default.
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.options.delimiter = delimiter;
        self
    }
}

impl Portfolio {
    /// A portfolio from its records and market data: the books are kept by applying the
    /// transactions in order. Invalid records, and transactions the books cannot apply, are
    /// errors.
    pub fn new(data: PortfolioData, market: MarketData) -> Result<Portfolio> {
        let books = Books::build(&data, &market)?;
        Ok(Portfolio {
            data,
            market,
            books,
        })
    }

    /// A portfolio from CSV files. Problems in every file are reported together, with paths
    /// under `transactions`, `instruments`, `accounts`, `prices` and `fx`.
    pub fn from_csv(files: &PortfolioCsv<'_>) -> Result<Portfolio> {
        let data = portfolio_from_csv(
            files.base_currency,
            files.instruments,
            files.transactions,
            files.accounts,
            files.options,
        );
        let market = market_from_csv(files.prices, files.fx, files.options);
        match (data, market) {
            (Ok(data), Ok(market)) => Portfolio::new(data, market),
            (Err(Error::Validation(mut a)), Err(Error::Validation(b))) => {
                a.merge("", b);
                Err(Error::Validation(a))
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
    }

    /// A portfolio from a portfolio document, with the market data it holds, if any.
    pub fn from_document(document: PortfolioDocument) -> Result<Portfolio> {
        let market = document.market.unwrap_or_default();
        Portfolio::new(document.portfolio, market)
    }

    /// A portfolio from a portfolio document in JSON.
    pub fn from_json(text: &str) -> Result<Portfolio> {
        Portfolio::from_document(PortfolioDocument::from_json(text)?)
    }

    /// The portfolio document of the portfolio and its market data.
    pub fn to_document(&self) -> PortfolioDocument {
        let mut document = PortfolioDocument::new(self.data.clone());
        document.market = Some(self.market.clone());
        document
    }

    /// The portfolio document in JSON.
    pub fn to_json(&self) -> String {
        self.to_document().to_json()
    }

    /// The same portfolio with more transactions, applied after its own.
    pub fn with_transactions(
        &self,
        transactions: impl IntoIterator<Item = Transaction>,
    ) -> Result<Portfolio> {
        let mut data = self.data.clone();
        data.transactions.extend(transactions);
        Portfolio::new(data, self.market.clone())
    }

    /// The same portfolio with the trades of a rebalancing made on its date: each account's
    /// sales, conversions and purchases, as transactions. It takes the rebalancing or its raw
    /// results.
    pub fn with_rebalance<'r>(
        &self,
        proposal: impl Into<&'r RebalanceResults>,
    ) -> Result<Portfolio> {
        let proposal: &RebalanceResults = proposal.into();
        let mut added = Vec::new();
        for account in &proposal.accounts {
            let stem = format!("rebalance-{}-{}", account.date, account.account);
            let taken = |prefix: &str| {
                self.data
                    .transactions
                    .iter()
                    .any(|t| t.id.as_str().starts_with(prefix))
            };
            let mut prefix = format!("{stem}-");
            let mut k = 2;
            while taken(&prefix) {
                prefix = format!("{stem}-{k}-");
                k += 1;
            }
            added.extend(account.transactions(&prefix)?);
        }
        self.with_transactions(added)
    }

    /// The same portfolio valued with other market data.
    pub fn with_market(&self, market: MarketData) -> Result<Portfolio> {
        Portfolio::new(self.data.clone(), market)
    }

    /// The records: base currency, accounts, instruments and transactions.
    pub fn data(&self) -> &PortfolioData {
        &self.data
    }

    /// The market data.
    pub fn market(&self) -> &MarketData {
        &self.market
    }

    /// The books: positions, lots, cash and events, for analysis beyond the methods here.
    pub fn books(&self) -> &Books {
        &self.books
    }

    /// The currency results are reported in.
    pub fn base_currency(&self) -> Currency {
        self.data.base_currency
    }

    /// The last date with a price, or of the last transaction when there are no prices: the date
    /// the portfolio is valued on when no other is given.
    pub fn as_of(&self) -> Option<Date> {
        let last_price = self
            .market
            .all_prices()
            .filter_map(|(_, s)| s.dates().last().copied())
            .max();
        last_price.or_else(|| self.data.transactions.iter().map(|t| t.trade_date).max())
    }

    /// The dates the portfolio is measured on: every date with a price, from the first
    /// transaction to the last price.
    pub fn dates(&self) -> Vec<Date> {
        let first = self.data.transactions.iter().map(|t| t.trade_date).min();
        let all: BTreeSet<Date> = self
            .market
            .all_prices()
            .flat_map(|(_, s)| s.dates().iter().copied())
            .filter(|d| first.is_none_or(|f| *d >= f))
            .collect();
        let mut dates: Vec<Date> = all.into_iter().collect();
        if let Some(first) = first
            && dates.first() != Some(&first)
        {
            dates.insert(0, first);
        }
        dates
    }

    fn date_or_last(&self, date: Option<Date>) -> Result<Date> {
        date.or_else(|| self.as_of()).ok_or_else(|| {
            let mut report = ValidationReport::new();
            report.error(
                "date",
                "give a date: the portfolio has no prices or transactions",
            );
            Error::Validation(report)
        })
    }

    /// The holdings valued at the end of `date`, or of the last price date.
    pub fn valuation(&self, date: Option<Date>) -> Result<Valuation> {
        self.books.value_on(self.date_or_last(date)?, &self.market)
    }

    /// Each holding's share of the portfolio's value at the end of `date`, or of the last price
    /// date.
    pub fn weights(&self, date: Option<Date>) -> Result<BTreeMap<HoldingKey, f64>> {
        holding_weights(&self.valuation(date)?)
    }

    /// The portfolio's value and external flows on each of its [dates](Portfolio::dates).
    pub fn value_series(&self) -> Result<ValueSeries> {
        self.books.value_series(&self.dates(), &self.market)
    }

    /// The portfolio's time-weighted performance on its [dates](Portfolio::dates), with
    /// external flows at the start of the day.
    pub fn performance(&self) -> Result<Performance> {
        Performance::from_values(&self.value_series()?, FlowTiming::StartOfDay)
    }

    /// The instruments held at the last price date, as [`Assets`]: their price returns in the
    /// base currency on the portfolio's dates, for statistics and optimisation.
    pub fn assets(&self) -> Result<Assets> {
        let date = self.date_or_last(None)?;
        let held: Vec<HoldingKey> = self
            .books
            .holdings_on(date)
            .positions()
            .into_iter()
            .filter(|p| !p.quantity.is_zero())
            .map(|p| HoldingKey::Instrument(p.instrument))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let dates = self.dates();
        let found = holding_returns(&self.books, &held, &dates, &self.market)?;
        Assets::from_returns(found.dates, found.table)
    }

    /// The trades that move the accounts to `targets`, weights by holding such as an
    /// optimisation's, as shares of their combined value at the last price date: one proposal
    /// per account, with the rebalance report. [`RebalanceBuilder::build`] finds them, and
    /// reports an optimisation that found no weights as an error.
    ///
    /// ```
    /// use finalytics::{Method, datasets};
    ///
    /// let portfolio = datasets::portfolio();
    /// let target = portfolio.assets()?.optimise(Method::MinimumVariance).build()?;
    /// let proposal = portfolio.rebalance(&target).whole_units().build()?;
    /// assert_eq!(proposal.results.accounts.len(), 2);
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn rebalance(&self, targets: impl Into<Targets>) -> RebalanceBuilder<'_> {
        RebalanceBuilder {
            portfolio: self,
            targets: targets.into(),
            accounts: None,
            as_of: None,
            name: None,
            options: RebalanceOptions::default(),
            plain: PlainCosts::default(),
        }
    }

    /// The trades that move every account together to `targets`, as `options` say, with the
    /// rebalance report. [`Portfolio::rebalance`] builds the same options step by step.
    pub fn rebalance_with(
        &self,
        targets: &BTreeMap<HoldingKey, f64>,
        options: &RebalanceOptions,
    ) -> Result<Rebalancing> {
        let accounts: Vec<AccountId> = self.data.accounts.iter().map(|a| a.id.clone()).collect();
        self.rebalance_accounts_with(&accounts, targets, options)
    }

    /// The trades that move `accounts` together to `targets`, as shares of their combined value
    /// at the last price date, with the rebalance report.
    pub fn rebalance_accounts_with(
        &self,
        accounts: &[AccountId],
        targets: &BTreeMap<HoldingKey, f64>,
        options: &RebalanceOptions,
    ) -> Result<Rebalancing> {
        self.rebalancing(
            accounts,
            self.date_or_last(None)?,
            targets,
            (None, Vec::new()),
            options,
        )
    }

    /// The trades that move `accounts` to `targets` on `date`, and the rebalance report on them:
    /// the volatility and expected return before and after come from the holdings' returns up
    /// to `date`, as the optimisation of [`Portfolio::assets`] estimates them.
    fn rebalancing(
        &self,
        accounts: &[AccountId],
        date: Date,
        targets: &BTreeMap<HoldingKey, f64>,
        (method, limits): (Option<String>, Vec<String>),
        options: &RebalanceOptions,
    ) -> Result<Rebalancing> {
        let list: Vec<(HoldingKey, f64)> = targets.iter().map(|(k, w)| (k.clone(), *w)).collect();
        let results =
            rebalance_accounts(&self.books, &self.market, date, accounts, &list, options)?;
        let instruments: Vec<HoldingKey> = results
            .weights_before
            .keys()
            .chain(results.weights_after.keys())
            .chain(targets.keys())
            .filter(|k| matches!(k, HoldingKey::Instrument(_)))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let dates: Vec<Date> = self.dates().into_iter().filter(|d| *d <= date).collect();
        let assets = holding_returns(&self.books, &instruments, &dates, &self.market)
            .ok()
            .and_then(|found| Assets::from_returns(found.dates, found.table).ok());
        let covariance = assets.as_ref().and_then(|a| a.covariance().ok());
        let means = assets.as_ref().map(Assets::mean_returns);
        let estimates = match (&assets, &covariance, &means) {
            (Some(a), Some(c), Some(m)) => match (a.dates().first(), a.dates().last()) {
                (Some(&start), Some(&end)) => Some(finalytics_reports::RebalanceEstimateInputs {
                    covariance: &c.matrix,
                    expected_returns: m,
                    covariance_method: a.covariance_method(),
                    periods_per_year: a.periods_per_year(),
                    start,
                    end,
                }),
                _ => None,
            },
            _ => None,
        };
        let valuation = self.books.value_on(date, &self.market)?;
        let unrealised_gain_base: f64 = valuation
            .positions
            .iter()
            .filter(|p| accounts.contains(&p.account))
            .map(|p| f64::try_from(p.unrealised_base).unwrap_or(0.0))
            .sum();
        let report = finalytics_reports::rebalance_report(finalytics_reports::RebalanceInputs {
            results: &results,
            targets,
            method,
            limits,
            accounts: self.books.accounts(),
            options,
            unrealised_gain_base,
            estimates,
        })?;
        Ok(Rebalancing { report, results })
    }

    fn source(&self) -> Source {
        Source::of(&self.data, &self.market)
    }

    /// Every report on the portfolio, built step by step: [`ReportBuilder::build`] makes them.
    /// The report prints as text, and lays out as one HTML page.
    ///
    /// ```
    /// use finalytics::datasets;
    ///
    /// let portfolio = datasets::portfolio();
    /// let report = portfolio
    ///     .report()
    ///     .benchmark("BENCH")
    ///     .group_by(["asset_class", "region"])
    ///     .build()?;
    /// assert!(report.to_html(None).starts_with("<!doctype html>"));
    /// # Ok::<(), finalytics::Error>(())
    /// ```
    pub fn report(&self) -> ReportBuilder<'_> {
        ReportBuilder {
            portfolio: self,
            options: ReportOptions::default(),
            benchmark: None,
            as_of: None,
            start: None,
            name: None,
        }
    }

    /// Every report on the portfolio, from one set of options. [`Portfolio::report`] builds
    /// the same options step by step.
    pub fn report_with(&self, options: &ReportOptions) -> Result<PortfolioReport> {
        portfolio_report(&self.books, &self.market, &self.source(), options)
    }

    /// The overview report.
    pub fn overview_report(&self, options: &ReportOptions) -> Result<OverviewReport> {
        overview_report(&self.books, &self.market, &self.source(), options)
    }

    /// The holdings report.
    pub fn holdings_report(&self, options: &ReportOptions) -> Result<HoldingsReport> {
        holdings_report(&self.books, &self.market, &self.source(), options)
    }

    /// The performance report.
    pub fn performance_report(&self, options: &ReportOptions) -> Result<PerformanceReport> {
        performance_report(&self.books, &self.market, &self.source(), options)
    }

    /// The risk report.
    pub fn risk_report(&self, options: &ReportOptions) -> Result<RiskReport> {
        risk_report(&self.books, &self.market, &self.source(), options)
    }

    /// The attribution report; the options need the benchmark's segments.
    pub fn attribution_report(&self, options: &ReportOptions) -> Result<AttributionReport> {
        attribution_report(&self.books, &self.market, &self.source(), options)
    }

    /// The scenarios report.
    pub fn scenarios_report(&self, options: &ReportOptions) -> Result<ScenariosReport> {
        scenarios_report(&self.books, &self.market, &self.source(), options)
    }
}

/// Target weights by holding, for [`Portfolio::rebalance`]: an optimisation's weights by asset,
/// or weights by instrument identifier (`"USD cash"` for cash). An optimisation that found no
/// weights is not a target: [`RebalanceBuilder::build`] reports it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Targets {
    weights: Vec<(String, f64)>,
    /// How an optimisation that found no weights ended.
    unsolved: Option<SolveStatus>,
    /// The method of the optimisation the weights came from, for the report.
    method: Option<String>,
    /// That optimisation's limits in words.
    limits: Vec<String>,
}

impl Targets {
    fn new(weights: Vec<(String, f64)>) -> Targets {
        Targets {
            weights,
            ..Targets::default()
        }
    }

    /// The weights by holding, with a problem in `report` for each name that is not a holding
    /// and for an optimisation that found no weights.
    fn holdings(&self, report: &mut ValidationReport) -> BTreeMap<HoldingKey, f64> {
        if let Some(status) = self.unsolved {
            report.error(
                "targets",
                format!("the optimisation found no weights: {}", status.reason()),
            );
        }
        let mut targets = BTreeMap::new();
        for (key, weight) in &self.weights {
            match key.parse::<HoldingKey>() {
                Ok(holding) => {
                    targets.insert(holding, *weight);
                }
                Err(_) => report.error(format!("targets.{key}"), "is not a holding"),
            }
        }
        targets
    }
}

impl From<&OptimisationResult> for Targets {
    fn from(result: &OptimisationResult) -> Targets {
        let method = serde_json::to_value(&result.solution.objective)
            .ok()
            .and_then(|v| serde_json::from_value::<crate::Method>(v["type"].clone()).ok())
            .map(|m| m.label().to_owned());
        match &result.weights {
            Some(weights) => Targets {
                method,
                ..Targets::new(weights.iter().map(|(a, w)| (a.clone(), *w)).collect())
            },
            None => Targets {
                unsolved: Some(result.status),
                method,
                ..Targets::default()
            },
        }
    }
}

impl From<&crate::Optimisation> for Targets {
    fn from(optimisation: &crate::Optimisation) -> Targets {
        let context = &optimisation.report.context;
        Targets {
            method: Some(context.method.clone()),
            limits: context.limits.clone(),
            ..Targets::from(&optimisation.results)
        }
    }
}

impl From<&BTreeMap<HoldingKey, f64>> for Targets {
    fn from(weights: &BTreeMap<HoldingKey, f64>) -> Targets {
        Targets::new(weights.iter().map(|(k, w)| (k.to_string(), *w)).collect())
    }
}

impl From<BTreeMap<String, f64>> for Targets {
    fn from(weights: BTreeMap<String, f64>) -> Targets {
        Targets::new(weights.into_iter().collect())
    }
}

impl<S: Into<String>, const N: usize> From<[(S, f64); N]> for Targets {
    fn from(weights: [(S, f64); N]) -> Targets {
        Targets::new(weights.into_iter().map(|(k, w)| (k.into(), w)).collect())
    }
}

/// Rebalancing targets given as JSON: weights by holding, or an optimisation, whole or its raw
/// results, whose weights are the targets. [`RebalanceBuilder::build`] reports an optimisation
/// that found no weights, and a name that is not a holding, at `targets`. The Python and
/// JavaScript packages use it; from Rust, pass the optimisation to [`Portfolio::rebalance`].
#[doc(hidden)]
pub fn targets_from_json(text: &str) -> Result<Targets> {
    let mut value: serde_json::Value = finalytics_formats::from_json(text, "targets")?;
    let context = value
        .get("report")
        .and_then(|r| r.get("context"))
        .filter(|c| c.is_object())
        .cloned();
    if let Some(results) = value.get_mut("results").filter(|v| v.is_object()) {
        value = results.take();
    }
    if value
        .get("solution")
        .is_some_and(serde_json::Value::is_object)
    {
        let result: OptimisationResult =
            finalytics_formats::from_json(&value.to_string(), "targets")?;
        let mut targets = Targets::from(&result);
        if let Some(context) = context {
            if let Some(method) = context.get("method").and_then(serde_json::Value::as_str) {
                targets.method = Some(method.to_owned());
            }
            if let Some(limits) = context.get("limits").and_then(serde_json::Value::as_array) {
                targets.limits = limits
                    .iter()
                    .filter_map(|l| l.as_str().map(str::to_owned))
                    .collect();
            }
        }
        Ok(targets)
    } else {
        let weights: BTreeMap<String, f64> =
            finalytics_formats::from_json(&value.to_string(), "targets")?;
        Ok(Targets::from(weights))
    }
}

/// A rebalancing: the rebalance report on the trades, and the raw results, each account's
/// proposal. Printing it prints the report, showing it shows the report as a page, and the
/// portfolio's `with_rebalance` makes the trades.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Rebalancing {
    /// The rebalance report: how the weights move, what the trades change and cost, each
    /// account's trades, what to check, and how the trades were found.
    pub report: RebalanceReport,
    /// The raw results: each account's trades, conversions, weights and costs.
    pub results: RebalanceResults,
}

impl std::fmt::Display for Rebalancing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.report.fmt(f)
    }
}

impl<'a> From<&'a Rebalancing> for &'a RebalanceResults {
    fn from(rebalancing: &'a Rebalancing) -> &'a RebalanceResults {
        &rebalancing.results
    }
}

/// A rebalancing of a portfolio's accounts to target weights, built step by step;
/// [`RebalanceBuilder::build`] finds the trades.
#[derive(Debug, Clone)]
#[must_use = "a rebalancing does nothing until `build` runs it"]
pub struct RebalanceBuilder<'a> {
    portfolio: &'a Portfolio,
    targets: Targets,
    accounts: Option<Vec<String>>,
    as_of: Option<String>,
    name: Option<String>,
    options: RebalanceOptions,
    plain: PlainCosts,
}

/// A rebalancing's choices as JSON gives them, with the names of the builder's methods: the
/// Python and JavaScript packages use it; from Rust, use the builder.
#[doc(hidden)]
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RebalanceChoices {
    pub accounts: Option<Vec<String>>,
    pub date: Option<String>,
    pub name: Option<String>,
    pub lot_method: Option<LotMethod>,
    pub whole_units: bool,
    pub minimum_trade: Option<f64>,
    pub fee_bps: Option<f64>,
    pub fee_per_trade: Option<f64>,
    pub minimum_fee: Option<f64>,
    pub tax: Option<(f64, f64)>,
    pub long_term_after_days: Option<u32>,
}

impl RebalanceChoices {
    /// `builder` with these choices.
    pub fn apply(self, mut builder: RebalanceBuilder<'_>) -> RebalanceBuilder<'_> {
        if let Some(accounts) = self.accounts {
            builder = builder.accounts(accounts);
        }
        if let Some(date) = &self.date {
            builder = builder.as_of(date);
        }
        if let Some(name) = &self.name {
            builder = builder.name(name);
        }
        if let Some(method) = self.lot_method {
            builder = builder.lot_method(method);
        }
        if self.whole_units {
            builder = builder.whole_units();
        }
        builder.plain = PlainCosts {
            minimum_trade: self.minimum_trade,
            fee_bps: self.fee_bps,
            fee_per_trade: self.fee_per_trade,
            minimum_fee: self.minimum_fee,
            tax: self.tax,
            long_term_after_days: self.long_term_after_days,
        };
        builder
    }
}

/// The costs a rebalancing builder takes as plain numbers, checked and converted at `build`.
#[derive(Debug, Clone, Default)]
struct PlainCosts {
    minimum_trade: Option<f64>,
    fee_bps: Option<f64>,
    fee_per_trade: Option<f64>,
    minimum_fee: Option<f64>,
    tax: Option<(f64, f64)>,
    long_term_after_days: Option<u32>,
}

/// `value` as a decimal, as it is written: 0.37 is exactly 0.37.
fn decimal(value: f64, path: &str, report: &mut ValidationReport) -> Decimal {
    match format!("{value}").parse::<Decimal>() {
        Ok(d) if value.is_finite() => d,
        _ => {
            report.error(path, format!("{value} is not a number Finalytics can use"));
            Decimal::ZERO
        }
    }
}

impl PlainCosts {
    /// `options` with these costs, each checked into `report`.
    fn apply(
        &self,
        mut options: RebalanceOptions,
        report: &mut ValidationReport,
    ) -> RebalanceOptions {
        if let Some(v) = self.minimum_trade {
            options.minimum_trade = decimal(v, "minimum_trade", report);
        }
        if let Some(v) = self.fee_bps {
            options.fees.rate = decimal(v, "fee_bps", report) / Decimal::from(10_000);
        }
        if let Some(v) = self.fee_per_trade {
            options.fees.per_trade = decimal(v, "fee_per_trade", report);
        }
        if let Some(v) = self.minimum_fee {
            options.fees.minimum = decimal(v, "minimum_fee", report);
        }
        if let Some((short, long)) = self.tax {
            let mut rates = TaxRates::new(
                decimal(short, "tax.short_term", report),
                decimal(long, "tax.long_term", report),
            );
            if let Some(days) = self.long_term_after_days {
                rates.long_term_after_days = i32::try_from(days).unwrap_or(i32::MAX);
            }
            options.tax = Some(rates);
        } else if let (Some(days), Some(rates)) = (self.long_term_after_days, options.tax.as_mut())
        {
            rates.long_term_after_days = i32::try_from(days).unwrap_or(i32::MAX);
        } else if self.long_term_after_days.is_some() {
            report.error("long_term_after_days", "applies only with tax rates");
        }
        options
    }
}

impl RebalanceBuilder<'_> {
    /// The accounts to rebalance together, by identifier; every account by default.
    pub fn accounts<S: AsRef<str>>(mut self, accounts: impl IntoIterator<Item = S>) -> Self {
        self.accounts = Some(
            accounts
                .into_iter()
                .map(|a| a.as_ref().to_owned())
                .collect(),
        );
        self
    }

    /// Trades in whole units of each instrument.
    pub fn whole_units(mut self) -> Self {
        self.options.whole_units = true;
        self
    }

    /// How sales choose the lots they close: each account's own method by default.
    pub fn lot_method(mut self, method: LotMethod) -> Self {
        self.options.lot_method = Some(method);
        self
    }

    /// The fee on each trade, in basis points of its value, such as 5 for 0.05%.
    pub fn fee_bps(mut self, bps: f64) -> Self {
        self.plain.fee_bps = Some(bps);
        self
    }

    /// A fixed fee on each trade, in the instrument's currency.
    pub fn fee_per_trade(mut self, fee: f64) -> Self {
        self.plain.fee_per_trade = Some(fee);
        self
    }

    /// The smallest fee a trade pays, in the instrument's currency.
    pub fn minimum_fee(mut self, fee: f64) -> Self {
        self.plain.minimum_fee = Some(fee);
        self
    }

    /// Tax-aware rebalancing at these rates on short-term and long-term gains, such as 0.37 and
    /// 0.20: sales in taxable accounts take the lots that cost the least tax, and the tax is
    /// estimated. A gain is long-term after 365 days unless `long_term_after_days` says
    /// otherwise.
    pub fn tax(mut self, short_term: f64, long_term: f64) -> Self {
        self.plain.tax = Some((short_term, long_term));
        self
    }

    /// The holding period, in days, after which a gain is long-term: 365 by default.
    pub fn long_term_after_days(mut self, days: u32) -> Self {
        self.plain.long_term_after_days = Some(days);
        self
    }

    /// Leaves out trades worth less than `value` in the base currency.
    pub fn minimum_trade(mut self, value: f64) -> Self {
        self.plain.minimum_trade = Some(value);
        self
    }

    /// The date of the prices, such as `"2026-06-30"`: the last price date by default.
    pub fn as_of(mut self, date: &str) -> Self {
        self.as_of = Some(date.to_owned());
        self
    }

    /// The report's title, such as `"Morgan family portfolio"`.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// Every option at once, as JSON gives them, in place of those set so far.
    pub fn options(mut self, options: RebalanceOptions) -> Self {
        self.options = options;
        self.plain = PlainCosts::default();
        self
    }

    /// Finds the trades, one proposal per account, with the rebalance report.
    pub fn build(&self) -> Result<Rebalancing> {
        let mut report = ValidationReport::new();
        let targets = self.targets.holdings(&mut report);
        let date = match &self.as_of {
            Some(text) => match text.parse::<Date>() {
                Ok(d) => Some(d),
                Err(_) => {
                    report.error("as_of", format!("{text:?} is not a date"));
                    None
                }
            },
            None => None,
        };
        let known = &self.portfolio.data.accounts;
        let accounts: Vec<AccountId> = match &self.accounts {
            Some(ids) => ids
                .iter()
                .filter_map(|id| match id.parse::<AccountId>() {
                    Ok(account) if known.iter().any(|a| a.id == account) => Some(account),
                    _ => {
                        let names: Vec<&str> = known.iter().map(|a| a.id.as_str()).collect();
                        report.error(
                            format!("accounts.{id}"),
                            format!(
                                "is not an account of the portfolio; its accounts are {}",
                                names.join(", ")
                            ),
                        );
                        None
                    }
                })
                .collect(),
            None => self
                .portfolio
                .data
                .accounts
                .iter()
                .map(|a| a.id.clone())
                .collect(),
        };
        let options = self.plain.apply(self.options.clone(), &mut report);
        report.into_result(())?;
        let mut rebalancing = self.portfolio.rebalancing(
            &accounts,
            self.portfolio.date_or_last(date)?,
            &targets,
            (self.targets.method.clone(), self.targets.limits.clone()),
            &options,
        )?;
        rebalancing.report.name.clone_from(&self.name);
        Ok(rebalancing)
    }
}

/// The reports on a portfolio, built step by step; [`ReportBuilder::build`] makes them.
#[derive(Debug, Clone)]
#[must_use = "the reports are made by `build`"]
pub struct ReportBuilder<'a> {
    portfolio: &'a Portfolio,
    options: ReportOptions,
    benchmark: Option<String>,
    as_of: Option<String>,
    start: Option<String>,
    name: Option<String>,
}

impl ReportBuilder<'_> {
    /// The benchmark: an instrument whose prices are given with the portfolio's, such as an
    /// index, by its identifier.
    pub fn benchmark(mut self, instrument: &str) -> Self {
        self.benchmark = Some(instrument.to_owned());
        self
    }

    /// The classification schemes to group holdings by, such as `asset_class` and `region`:
    /// the `class.<scheme>` columns of the instruments.
    pub fn group_by<S: Into<String>>(mut self, schemes: impl IntoIterator<Item = S>) -> Self {
        self.options.group_by = schemes.into_iter().map(Into::into).collect();
        self
    }

    /// The annual risk-free rate, for Sharpe ratios.
    pub fn risk_free_rate(mut self, rate: f64) -> Self {
        self.options.risk_free_rate = rate;
        self
    }

    /// The date to report as of, such as `"2026-06-30"`: the last price date by default.
    pub fn as_of(mut self, date: &str) -> Self {
        self.as_of = Some(date.to_owned());
        self
    }

    /// The first date of the history to report on: the first transaction by default.
    pub fn start(mut self, date: &str) -> Self {
        self.start = Some(date.to_owned());
        self
    }

    /// The benchmark's segments, for attribution against it.
    pub fn attribution(mut self, inputs: AttributionInputs) -> Self {
        self.options.attribution = Some(inputs);
        self
    }

    /// A scenario to run on the holdings.
    pub fn scenario(mut self, scenario: NamedScenario) -> Self {
        self.options.scenarios.push(scenario);
        self
    }

    /// Every option at once, such as those of `datasets::report_options()`; the builder's other
    /// methods change them.
    pub fn options(mut self, options: ReportOptions) -> Self {
        self.options = options;
        self
    }

    /// The portfolio's name, shown under each report's title.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// Makes the reports.
    pub fn build(&self) -> Result<PortfolioReport> {
        let mut report = ValidationReport::new();
        let mut options = self.options.clone();
        if let Some(id) = &self.benchmark {
            match id.parse() {
                Ok(id) => options.benchmark = Some(id),
                Err(_) => report.error(
                    "benchmark",
                    format!("{id:?} is not an instrument identifier"),
                ),
            }
        }
        for (field, text, slot) in [
            ("as_of", &self.as_of, &mut options.as_of),
            ("start", &self.start, &mut options.start),
        ] {
            if let Some(text) = text {
                match text.parse::<Date>() {
                    Ok(date) => *slot = Some(date),
                    Err(_) => report.error(
                        field,
                        format!("{text:?} is not a date: write it as 2026-06-30"),
                    ),
                }
            }
        }
        report.into_result(())?;
        let mut built = self.portfolio.report_with(&options)?;
        built.name.clone_from(&self.name);
        Ok(built)
    }
}

/// A portfolio built from its CSV files by path; [`PortfolioBuilder::build`] reads them. Only
/// the transactions are required. The [data guide] describes every column.
///
/// [data guide]: https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md
#[cfg(feature = "io")]
#[derive(Debug, Clone, Default)]
#[must_use = "the portfolio is made by `build`"]
pub struct PortfolioBuilder {
    files: [Option<std::path::PathBuf>; 5],
    base_currency: Option<String>,
}

#[cfg(feature = "io")]
const FILES: [&str; 5] = ["transactions", "instruments", "accounts", "prices", "fx"];

#[cfg(feature = "io")]
impl PortfolioBuilder {
    fn file(mut self, k: usize, path: impl AsRef<std::path::Path>) -> Self {
        self.files[k] = Some(path.as_ref().to_owned());
        self
    }

    /// The transactions file: one row per deposit, purchase, sale, dividend and so on.
    pub fn transactions(self, path: impl AsRef<std::path::Path>) -> Self {
        self.file(0, path)
    }

    /// The instruments file: each instrument's kind, currency and classifications.
    pub fn instruments(self, path: impl AsRef<std::path::Path>) -> Self {
        self.file(1, path)
    }

    /// The accounts file: each account's name, lot method and classifications.
    pub fn accounts(self, path: impl AsRef<std::path::Path>) -> Self {
        self.file(2, path)
    }

    /// The prices file: a `date` column and one column per instrument, such as a benchmark.
    pub fn prices(self, path: impl AsRef<std::path::Path>) -> Self {
        self.file(3, path)
    }

    /// The exchange rates file, such as a `date` column and a `EUR/USD` column.
    pub fn fx(self, path: impl AsRef<std::path::Path>) -> Self {
        self.file(4, path)
    }

    /// The currency results are reported in, such as `"USD"`: that of the first deposit by
    /// default.
    pub fn base_currency(mut self, currency: &str) -> Self {
        self.base_currency = Some(currency.to_owned());
        self
    }

    /// Reads the files and keeps the books, reporting every problem in every file together.
    pub fn build(&self) -> Result<Portfolio> {
        let mut report = ValidationReport::new();
        let texts: Vec<Option<String>> = FILES
            .iter()
            .zip(&self.files)
            .map(|(name, path)| {
                path.as_ref()
                    .and_then(|path| match std::fs::read_to_string(path) {
                        Ok(text) => Some(text),
                        Err(e) => {
                            report.error(*name, format!("cannot read {}: {e}", path.display()));
                            None
                        }
                    })
            })
            .collect();
        if self.files[0].is_none() {
            report.error("transactions", "give the transactions file");
        }
        let currency = self.base_currency.as_ref().and_then(|c| match c.parse() {
            Ok(c) => Some(c),
            Err(_) => {
                report.error("base_currency", format!("{c:?} is not a currency code"));
                None
            }
        });
        report.into_result(())?;
        let mut csv = PortfolioCsv::new(texts[0].as_deref().unwrap_or_default());
        if let Some(t) = &texts[1] {
            csv = csv.instruments(t);
        }
        if let Some(t) = &texts[2] {
            csv = csv.accounts(t);
        }
        if let Some(t) = &texts[3] {
            csv = csv.prices(t);
        }
        if let Some(t) = &texts[4] {
            csv = csv.fx(t);
        }
        if let Some(c) = currency {
            csv = csv.base_currency(c);
        }
        Portfolio::from_csv(&csv)
    }
}

#[cfg(feature = "io")]
impl Portfolio {
    /// A portfolio from its CSV files by path: the transactions, and the instruments, accounts,
    /// prices and exchange rates as needed.
    pub fn builder() -> PortfolioBuilder {
        PortfolioBuilder::default()
    }
}
