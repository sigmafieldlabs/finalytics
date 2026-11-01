# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Finalytics 1.0 is a redesign of the library. See the migration guide (to follow) for moving from
0.9.

### Added

- A workspace of crates behind the `finalytics` facade: core, ledger, formats, performance, risk
  and optimisation.
- Python bindings (PyO3, abi3 wheels for Python 3.10 and later).
- `finalytics-core`: ISO 8601 dates with day and month arithmetic, business-day calendars with
  caller-supplied holidays and the standard adjustment conventions, ISO 4217 currencies, exact
  decimal money with currency-checked arithmetic and rounding, date-indexed time series with
  as-of lookups and reindexing that marks carried-forward values, validation reports that list
  every problem with its location, and compensated summation.
- `finalytics-ledger`: instruments with classifications and multipliers, accounts, sixteen
  transaction types including splits, spin-offs, mergers, symbol changes and returns of capital,
  tax lots relieved first-in first-out, last-in first-out, by highest or lowest cost, at average
  cost or by specific identification, cash in several currencies, realised gains in local and
  base currency, holdings on any date, and valuation with caller-supplied prices and FX rates
  (direct, inverse or crossed through a pivot currency), with carried-forward values reported.
- WebAssembly bindings and an npm package with TypeScript types.

### Removed

- Data fetching from third-party websites, screeners and technical indicators.
- Charts and HTML reports; the engine now returns data for callers to present.
- Go bindings and the C interface.

## [0.9.0] - 2026-05-01

The last release of the 0.x series. See the `0.9.x` branch.
