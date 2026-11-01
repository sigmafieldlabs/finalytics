//! Import and export: CSV readers for instruments, transactions, prices and FX rates, and the
//! versioned portfolio document in JSON. Every reader reports all the problems it finds, located
//! by line and column or by path. Parsing works on strings; there is no file system access.
//!
//! Most users should depend on the [`finalytics`](https://docs.rs/finalytics) crate, which
//! re-exports the public API of every layer.

mod csv_import;
mod document;

pub use csv_import::{
    CsvOptions, FxSeries, fx_from_csv, instruments_from_csv, portfolio_from_csv, prices_from_csv,
    transactions_from_csv,
};
pub use document::{FORMAT, FORMAT_VERSION, MarketDocument, PortfolioDocument};
