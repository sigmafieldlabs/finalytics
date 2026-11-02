//! Illustrative datasets, for examples, tests and practice.
//!
//! The data is synthetic: drawn from fixed distributions with a fixed seed, so that it gives the
//! same figures everywhere. It is not the history of any market, index or fund, and it
//! redistributes no third party's data. `scripts/datasets.py` in the repository writes it.
//!
//! - [`asset_class_prices`]: 30 years of month-end levels of six asset-class indices.
//! - [`portfolio`]: three years of a two-account portfolio in US dollars and euros, and
//!   [`report_options`], the options of its full report.
//!
//! Each file is also available as text, through [`file()`], as an example of the layout
//! Finalytics reads.
//!
//! ```
//! use finalytics::{Assets, datasets};
//!
//! let assets = Assets::from_prices(&datasets::asset_class_prices())?;
//! assert_eq!(assets.labels().len(), 6);
//! assert_eq!(assets.periods_per_year(), 12.0);
//!
//! let portfolio = datasets::portfolio();
//! let report = portfolio.report(&datasets::report_options())?;
//! assert!(report.attribution.is_some());
//! # Ok::<(), finalytics::Error>(())
//! ```

use finalytics_core::{Currency, Date};
use finalytics_formats::{CsvOptions, price_columns_from_csv};
use finalytics_reports::ReportOptions;
use finalytics_risk::PriceTable;

use crate::{Portfolio, PortfolioCsv};

const ASSET_CLASSES: &str = include_str!("../data/asset-classes.csv");
const ACCOUNTS: &str = include_str!("../data/portfolio/accounts.csv");
const INSTRUMENTS: &str = include_str!("../data/portfolio/instruments.csv");
const TRANSACTIONS: &str = include_str!("../data/portfolio/transactions.csv");
const PRICES: &str = include_str!("../data/portfolio/prices.csv");
const FX: &str = include_str!("../data/portfolio/fx.csv");
const REPORT_OPTIONS: &str = include_str!("../data/portfolio/report-options.json");

/// The names of the dataset files, for [`file()`].
pub const FILES: [&str; 7] = [
    "asset-classes.csv",
    "portfolio/accounts.csv",
    "portfolio/instruments.csv",
    "portfolio/transactions.csv",
    "portfolio/prices.csv",
    "portfolio/fx.csv",
    "portfolio/report-options.json",
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

/// Month-end levels, from October 1996 to October 2026, of six asset-class indices that start
/// at 100: US Equity, Intl Equity, Bonds, T-Bills, Gold and Real Estate. The indices include
/// income. Their bear markets are modelled loosely on those of 2000–02, 2007–09, 2020 and 2022.
pub fn asset_class_prices() -> PriceTable {
    let series = price_columns_from_csv(ASSET_CLASSES, CsvOptions::default())
        .expect("the asset-class dataset is valid");
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
    PriceTable::new(dates, labels, columns).expect("the asset-class dataset is valid")
}

/// A portfolio of two accounts held by one person, from November 2023 to October 2026, in US
/// dollars:
///
/// - a taxable brokerage account of US and European equity, bond, gold and real estate funds,
///   with monthly contributions, a rebalance, an advisory fee and a withdrawal;
/// - a tax-deferred retirement account of US equity and bonds.
///
/// It has 148 transactions, with dividends (the European fund's in euros, with tax withheld),
/// daily prices, a EUR/USD rate, and the levels of a balanced benchmark, `BENCH`: 55% equity,
/// 35% bonds and 10% real assets.
pub fn portfolio() -> Portfolio {
    let files = PortfolioCsv::new(TRANSACTIONS)
        .instruments(INSTRUMENTS)
        .accounts(ACCOUNTS)
        .prices(PRICES)
        .fx(FX)
        .base_currency(Currency::USD);
    Portfolio::from_csv(&files).expect("the portfolio dataset is valid")
}

/// The options of the full report on [`portfolio`]: holdings grouped by asset class and region,
/// the benchmark `BENCH` with its segments by asset class for attribution, a risk-free rate of
/// 3%, and four scenarios (an equity sell-off with stressed correlations, rates up 1%, the
/// dollar up 10%, and a replay of spring 2025).
pub fn report_options() -> ReportOptions {
    finalytics_formats::from_json(REPORT_OPTIONS, "options").expect("the report options are valid")
}
