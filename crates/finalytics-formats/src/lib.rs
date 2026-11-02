//! Import and export: CSV readers for accounts, instruments, transactions, prices and FX rates,
//! the versioned portfolio document in JSON, and a JSON reader for any of the engine's types.
//! Every reader reports all the problems it finds, located by line and column or by path.
//! Parsing works on strings; there is no file system access.
//!
//! This crate is one layer of Finalytics. Depend on the [`finalytics`](https://docs.rs/finalytics)
//! crate instead: it re-exports the public API of every layer, and it is the crate whose API
//! follows semantic versioning. This crate's own API may change in any release.

mod csv_import;
mod document;
mod json;

pub use csv_import::{
    CsvOptions, FxSeries, accounts_from_csv, fx_from_csv, instruments_from_csv, portfolio_from_csv,
    prices_from_csv, transactions_from_csv,
};
pub use document::{FORMAT, FORMAT_VERSION, MarketDocument, PortfolioDocument};
pub use json::from_json;
