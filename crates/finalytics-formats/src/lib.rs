//! Import and export: CSV readers for accounts, instruments, transactions, prices and FX rates,
//! goals for planning, and the closes and fields of securities to screen; the versioned portfolio document in
//! JSON; and a JSON reader for any of the engine's types.
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
    Closes, CsvOptions, FieldCells, FieldTable, FxSeries, accounts_from_csv, closes_from_csv,
    fields_from_csv, fx_from_csv, goals_from_csv, instruments_from_csv, market_from_csv,
    portfolio_from_csv, price_columns_from_csv, prices_from_csv, transactions_from_csv,
};
pub use document::{FORMAT, FORMAT_VERSION, MarketDocument, PortfolioDocument, goal_names};
pub use json::{from_json, from_json_by_field};
