//! Datasets for examples, tests and practice.
//!
//! - [`portfolio`]: three years of a two-account household in US dollars and euros, holding five
//!   real funds at their real prices, and [`report_options`], the options of its full report.
//! - [`asset_class_prices`]: 30 years of month-end levels of six asset-class indices, synthetic.
//! - [`universe`]: six years of daily closes, volumes and dividends of 200 fictional companies in
//!   eleven sectors, with their fields and a market index, synthetic, for screening.
//!
//! The funds' prices and the EUR/USD rate are real, from Yahoo Finance;
//! `scripts/fetch-market-data.py` in the repository downloads them. The household's
//! transactions and the asset-class indices are illustrative, written by `scripts/datasets.py`.
//! The universe is drawn by the library itself, with a fixed seed, the same on every platform.
//!
//! Each file is also available as text, through [`file()`], as an example of the layout
//! Finalytics reads.
//!
//! ```
//! use finalytics::{Assets, datasets};
//!
//! let assets =
//!     Assets::from_prices(&datasets::asset_class_prices())?.with_risk_free_asset("T-Bills")?;
//! assert_eq!(assets.risky_assets().len(), 5);
//! assert_eq!(assets.periods_per_year(), 12.0);
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

use crate::{Portfolio, PortfolioCsv, Universe};

mod universe;

const ASSET_CLASSES: &str = include_str!("../data/asset-classes.csv");
const ACCOUNTS: &str = include_str!("../data/portfolio/accounts.csv");
const INSTRUMENTS: &str = include_str!("../data/portfolio/instruments.csv");
const TRANSACTIONS: &str = include_str!("../data/portfolio/transactions.csv");
const PRICES: &str = include_str!("../data/portfolio/prices.csv");
const FX: &str = include_str!("../data/portfolio/fx.csv");
const REPORT_OPTIONS: &str = include_str!("../data/portfolio/report-options.json");

/// The names of the dataset files, for [`file()`].
pub const FILES: [&str; 7] = [
    "portfolio/accounts.csv",
    "portfolio/instruments.csv",
    "portfolio/transactions.csv",
    "portfolio/prices.csv",
    "portfolio/fx.csv",
    "portfolio/report-options.json",
    "asset-classes.csv",
];

/// The text of a dataset file, by its name in [`FILES`].
pub fn file(name: &str) -> Option<&'static str> {
    Some(match name {
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

/// Prices in CSV text, a `date` column and one column per asset, as a [`PriceTable`]: the
/// format of the price datasets.
fn price_table(text: &str) -> Result<PriceTable> {
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

/// A universe of 200 fictional companies in eleven sectors, for screening: their daily closes on
/// every weekday from November 2020 to October 2026, with volumes and quarterly dividends, and
/// the market index `INDEX` as the benchmark, at a risk-free rate of 4% a year.
///
/// Each company has five fields: `name`, `sector`, `market_cap` (in US dollars), `trailing_pe`
/// (none for the companies that make a loss) and `price_to_book`. Its identifier is the first
/// four letters of its name, such as `CALW` for Calwick Systems. The closes include the
/// dividends, as adjusted closes do, and the dividends paid are listed for the dividend yield.
///
/// The data is synthetic: drawn with a fixed seed from a market factor with changing volatility,
/// a factor for each sector and each company's own risk, with the market falling through 2022
/// and selling off in the spring of 2025, loosely as markets did then. Fourteen companies list
/// after November 2020, one stops trading in August 2026, two miss some of their closes and
/// three have no volumes, as real data does. The companies and their figures are invented; any
/// likeness to a listed company is chance. Every platform draws the same universe.
///
/// ```
/// use finalytics::prelude::*;
///
/// let universe = datasets::universe();
/// assert_eq!(universe.len(), 201);
/// assert_eq!(universe.benchmark(), Some("INDEX"));
/// let screen = universe
///     .screen()
///     .one_of("sector", ["Utilities"])
///     .above("dividend_yield", 0.03)
///     .sort("sharpe_ratio", Order::Descending)
///     .build()?;
/// assert!(!screen.results.matches.is_empty());
/// # Ok::<(), finalytics::Error>(())
/// ```
pub fn universe() -> Universe {
    let series = universe::series().expect("the universe's dates are valid");
    Universe::from_series(series)
        .and_then(|u| u.with_benchmark(universe::BENCHMARK))
        .expect("the universe is valid")
        .with_risk_free_rate(universe::RISK_FREE_RATE)
        .with_field_order(&universe::FIELDS)
}
