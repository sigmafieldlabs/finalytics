//! The S&P 500 prices in `tests/sp500` at the repository root, for the tests on a long daily
//! history. They are test data, not part of the package.

use finalytics::PriceTable;
use finalytics::formats::{CsvOptions, price_columns_from_csv};

/// Daily prices of 55 S&P 500 companies, with `T-Bills` as the last column.
pub(crate) fn sp500() -> PriceTable {
    price_table(include_str!("../../../../tests/sp500/prices.csv"))
}

/// The S&P 500 total return index on the same dates, in one column, `S&P 500`.
pub(crate) fn sp500_index() -> PriceTable {
    price_table(include_str!("../../../../tests/sp500/index.csv"))
}

fn price_table(text: &str) -> PriceTable {
    let series = price_columns_from_csv(text, CsvOptions::default()).unwrap();
    let dates = series[0].1.dates().to_vec();
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
    PriceTable::new(dates, labels, columns).unwrap()
}
