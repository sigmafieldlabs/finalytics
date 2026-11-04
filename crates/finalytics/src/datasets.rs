//! Datasets for examples, tests and practice.
//!
//! - [`sp500`]: daily prices, from January 2005 to October 2026, of 55 S&P 500 companies (the
//!   five largest in each sector) and of 3-month US Treasury bills; [`sp500_index`], the
//!   S&P 500 total return index; and [`sp500_companies`], each company's name, sector and
//!   industry.
//! - [`portfolio`]: three years of a two-account household in US dollars and euros, holding five
//!   real funds at their real prices, and [`report_options`], the options of its full report.
//! - [`asset_class_prices`]: 30 years of month-end levels of six asset-class indices, synthetic.
//!
//! The market data is real, from Yahoo Finance, and `scripts/fetch-market-data.py` in the
//! repository downloads it. The S&P 500 companies are today's largest members, so their history
//! looks back with hindsight (survivorship bias). The household's transactions and the
//! asset-class indices are illustrative, written by `scripts/datasets.py`.
//!
//! Each file is also available as text, through [`file()`], as an example of the layout
//! Finalytics reads. The S&P 500 prices are part of the default `sp500` feature, which adds
//! 2.5 MB to a program that uses them.
//!
//! ```
//! use finalytics::{Assets, datasets};
//!
//! let assets = Assets::from_prices(&datasets::sp500())?.with_risk_free_asset("T-Bills")?;
//! assert_eq!(assets.risky_assets().len(), 55);
//! assert_eq!(assets.periods_per_year(), 252.0);
//!
//! let portfolio = datasets::portfolio();
//! let report = portfolio.report_with(&datasets::report_options())?;
//! assert!(report.attribution.is_some());
//! # Ok::<(), finalytics::Error>(())
//! ```

use finalytics_core::{Date, Result};
use finalytics_formats::{CsvOptions, price_columns_from_csv};
use finalytics_reports::ReportOptions;
use finalytics_risk::PriceTable;
use serde::{Deserialize, Serialize};

use crate::{Portfolio, PortfolioCsv};

const ASSET_CLASSES: &str = include_str!("../data/asset-classes.csv");
const ACCOUNTS: &str = include_str!("../data/portfolio/accounts.csv");
const INSTRUMENTS: &str = include_str!("../data/portfolio/instruments.csv");
const TRANSACTIONS: &str = include_str!("../data/portfolio/transactions.csv");
const PRICES: &str = include_str!("../data/portfolio/prices.csv");
const FX: &str = include_str!("../data/portfolio/fx.csv");
const REPORT_OPTIONS: &str = include_str!("../data/portfolio/report-options.json");
const SP500_COMPANIES: &str = include_str!("../data/sp500/companies.csv");
#[cfg(feature = "sp500")]
const SP500_PRICES: &str = include_str!("../data/sp500/prices.csv");
#[cfg(feature = "sp500")]
const SP500_INDEX: &str = include_str!("../data/sp500/index.csv");

/// The names of the dataset files, for [`file()`].
pub const FILES: [&str; 10] = [
    "sp500/prices.csv",
    "sp500/index.csv",
    "sp500/companies.csv",
    "portfolio/accounts.csv",
    "portfolio/instruments.csv",
    "portfolio/transactions.csv",
    "portfolio/prices.csv",
    "portfolio/fx.csv",
    "portfolio/report-options.json",
    "asset-classes.csv",
];

/// The text of a dataset file, by its name in [`FILES`]. Without the `sp500` feature,
/// `"sp500/prices.csv"` and `"sp500/index.csv"` are absent.
pub fn file(name: &str) -> Option<&'static str> {
    Some(match name {
        #[cfg(feature = "sp500")]
        "sp500/prices.csv" => SP500_PRICES,
        #[cfg(feature = "sp500")]
        "sp500/index.csv" => SP500_INDEX,
        "sp500/companies.csv" => SP500_COMPANIES,
        "asset-classes.csv" => ASSET_CLASSES,
        "portfolio/accounts.csv" => ACCOUNTS,
        "portfolio/instruments.csv" => INSTRUMENTS,
        "portfolio/transactions.csv" => TRANSACTIONS,
        "portfolio/prices.csv" => PRICES,
        "portfolio/fx.csv" => FX,
        "portfolio/report-options.json" => REPORT_OPTIONS,
        _ => return None,
    })
}

/// A company in [`sp500`]: its ticker, name, GICS sector and GICS sub-industry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Company {
    /// The ticker, the company's column in [`sp500`], such as `"AAPL"`.
    pub ticker: String,
    /// The company's name, such as `"Apple Inc."`.
    pub name: String,
    /// The GICS sector, such as `"Information Technology"`.
    pub sector: String,
    /// The GICS sub-industry, such as `"Technology Hardware, Storage & Peripherals"`.
    pub industry: String,
}

/// Daily prices, from 3 January 2005 to 30 October 2026, of 55 S&P 500 companies: the five
/// largest in each of the 11 sectors, by market value in November 2026, among members that
/// traded throughout. The prices are adjusted closes, which include dividends and splits. A
/// last column, `T-Bills`, is the return index of 3-month US Treasury bills (the 13-week rate,
/// accrued daily), for use as the risk-free asset. [`sp500_companies`] gives each company's
/// sector.
///
/// The companies are chosen with hindsight: they are today's largest members, so their past
/// returns are higher than those of the index's members at the time (survivorship bias).
#[cfg(feature = "sp500")]
pub fn sp500() -> PriceTable {
    price_table(SP500_PRICES).expect("the S&P 500 dataset is valid")
}

/// The S&P 500 total return index, daily from 3 January 2005 to 30 October 2026, in one column,
/// `S&P 500`: a benchmark for [`sp500`].
#[cfg(feature = "sp500")]
pub fn sp500_index() -> PriceTable {
    price_table(SP500_INDEX).expect("the S&P 500 index is valid")
}

/// The companies of [`sp500`], in the order of its columns: by sector, and by market value
/// within each sector.
pub fn sp500_companies() -> Vec<Company> {
    let mut reader = csv::Reader::from_reader(SP500_COMPANIES.as_bytes());
    reader
        .deserialize()
        .collect::<std::result::Result<_, _>>()
        .expect("the S&P 500 companies are valid")
}

/// Prices in CSV text, a `date` column and one column per asset, as a [`PriceTable`]: the
/// format of the price datasets. The JavaScript package uses it to load [`sp500`] on demand.
#[doc(hidden)]
pub fn price_table(text: &str) -> Result<PriceTable> {
    let series = price_columns_from_csv(text, CsvOptions::default())?;
    let dates: Vec<Date> = series
        .first()
        .map(|(_, s)| s.dates().to_vec())
        .unwrap_or_default();
    let labels = series.iter().map(|(id, _)| id.to_string()).collect();
    let columns = series
        .iter()
        .map(|(_, s)| {
            dates
                .iter()
                .map(|d| s.get(*d).and_then(|p| f64::try_from(*p).ok()))
                .collect()
        })
        .collect();
    PriceTable::new(dates, labels, columns)
}

/// Month-end levels, from October 1996 to October 2026, of six asset-class indices that start
/// at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate. The data is synthetic:
/// drawn with a fixed seed, with bear markets modelled loosely on those of 2000–02, 2007–09,
/// 2020 and 2022. The indices include income.
pub fn asset_class_prices() -> PriceTable {
    price_table(ASSET_CLASSES).expect("the asset-class dataset is valid")
}

/// A portfolio of two accounts held by one person, from November 2023 to October 2026, in US
/// dollars:
///
/// - a taxable brokerage account of five funds: Vanguard Total Stock Market (`VTI`), iShares
///   STOXX Europe 600 (`EXSA`, in euros on Xetra), Vanguard Total Bond Market (`BND`), SPDR Gold
///   Shares (`GLD`) and Vanguard Real Estate (`VNQ`), with monthly contributions, a rebalance, an
///   advisory fee and a withdrawal;
/// - a tax-deferred retirement account of US equity and bonds.
///
/// The prices, the funds' distributions and the EUR/USD rate are real; the transactions are
/// illustrative. The prices include `BENCH`, a balanced benchmark built from the funds' total
/// returns: 55% equity, 35% bonds and 10% real assets, rebalanced monthly.
pub fn portfolio() -> Portfolio {
    let files = PortfolioCsv::new(TRANSACTIONS)
        .instruments(INSTRUMENTS)
        .accounts(ACCOUNTS)
        .prices(PRICES)
        .fx(FX)
        .base_currency("USD");
    Portfolio::from_csv(&files).expect("the portfolio dataset is valid")
}

/// The options of the full report on [`portfolio`]: holdings grouped by asset class and region,
/// the benchmark `BENCH` with its segments by asset class for attribution, the T-bills' return
/// over the period as the risk-free rate, and four scenarios (an equity sell-off with stressed
/// correlations, rates up 1%, the dollar up 10%, and a replay of the spring 2025 sell-off).
pub fn report_options() -> ReportOptions {
    finalytics_formats::from_json(REPORT_OPTIONS, "options").expect("the report options are valid")
}
