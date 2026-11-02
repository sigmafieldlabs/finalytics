//! Portfolio: a portfolio's records and market data, with the books kept from them.

use std::collections::{BTreeMap, BTreeSet};

use finalytics_core::{Currency, Date, Error, Result, ValidationReport};
use finalytics_formats::{CsvOptions, PortfolioDocument, market_from_csv, portfolio_from_csv};
use finalytics_ledger::{
    AccountId, Books, HoldingKey, MarketData, PortfolioData, Transaction, Valuation, ValueSeries,
};
use finalytics_opt::{CombinedRebalance, RebalanceOptions, rebalance_accounts};
use finalytics_perf::{FlowTiming, Performance};
use finalytics_reports::{
    AttributionReport, HoldingsReport, OverviewReport, PerformanceReport, PortfolioReport,
    ReportOptions, RiskReport, ScenariosReport, Source, attribution_report, holdings_report,
    overview_report, performance_report, portfolio_report, risk_report, scenarios_report,
};
use finalytics_risk::{holding_returns, holding_weights};

use crate::Assets;

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
/// required.
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

    /// The same portfolio with the trades of a rebalancing proposal made on its date: each
    /// account's sales, conversions and purchases, as transactions.
    pub fn with_rebalance(&self, proposal: &CombinedRebalance) -> Result<Portfolio> {
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

    /// The trades that move every account together to `targets`, weights by holding as shares
    /// of their combined value at the last price date: one proposal per account.
    pub fn rebalance(
        &self,
        targets: &BTreeMap<HoldingKey, f64>,
        options: &RebalanceOptions,
    ) -> Result<CombinedRebalance> {
        let accounts: Vec<AccountId> = self.data.accounts.iter().map(|a| a.id.clone()).collect();
        self.rebalance_accounts(&accounts, targets, options)
    }

    /// The trades that move `accounts` together to `targets`, as shares of their combined value
    /// at the last price date.
    pub fn rebalance_accounts(
        &self,
        accounts: &[AccountId],
        targets: &BTreeMap<HoldingKey, f64>,
        options: &RebalanceOptions,
    ) -> Result<CombinedRebalance> {
        let targets: Vec<(HoldingKey, f64)> =
            targets.iter().map(|(k, w)| (k.clone(), *w)).collect();
        rebalance_accounts(
            &self.books,
            &self.market,
            self.date_or_last(None)?,
            accounts,
            &targets,
            options,
        )
    }

    fn source(&self) -> Source {
        Source::of(&self.data, &self.market)
    }

    /// Every report on the portfolio, from one set of options. The report prints as text
    /// (`to_string()` or `to_text(name)`).
    pub fn report(&self, options: &ReportOptions) -> Result<PortfolioReport> {
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
