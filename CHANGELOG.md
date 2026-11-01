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
  base currency, holdings on any date, previews of the lots a sale would close, and valuation
  with caller-supplied prices and FX rates (direct, inverse or crossed through a pivot
  currency), with carried-forward values reported.
- `finalytics-formats`: CSV readers for instruments (with classification and identifier
  columns), transactions of every type, and prices and FX rates in long or wide layouts, each
  reporting every problem by line and column; and the versioned portfolio document in JSON,
  with market data, that reports where a malformed document goes wrong.
- `finalytics-perf`: daily time-weighted returns chain-linked from values and external flows
  (flows at the start or end of the day), growth of an investment, money-weighted returns
  (internal rate of return), Modified Dietz, month-, quarter- and year-to-date, trailing-year
  and since-inception periods with annualisation for periods of a year or more, and excess
  returns over a benchmark, arithmetic and geometric.
- Daily value, trade cash and income for each holding and cash balance, and contribution to
  return by holding and by classification, linked so that contributions add up to the
  time-weighted return, with each holding's return in local and base currency and the currency
  effect.
- `finalytics-risk`: return statistics (annualised return, volatility, downside deviation, Sharpe,
  Sortino and Calmar ratios, maximum drawdown, skewness and excess kurtosis), statistics against
  a benchmark (active return, tracking error, information ratio, beta, alpha and correlation),
  drawdowns with their peak, trough and recovery dates, value at risk and expected shortfall
  (historical, parametric and Cornish–Fisher), the standard normal distribution, and rolling
  versions of each; covariance estimation (sample, exponentially weighted, Ledoit–Wolf and
  oracle approximating shrinkage) with correlation; the decomposition of volatility and ex-ante
  tracking error into each holding's marginal and component risk, by holding and by group; and
  holding weights and base-currency returns from a portfolio's books. Validated against
  empyrical, SciPy, pandas and scikit-learn.
- `finalytics-opt`: portfolio optimisation on the Clarabel interior-point solver: minimum
  variance, mean–variance, target return, target volatility, maximum Sharpe ratio, maximum
  diversification and minimum CVaR, with budget, bound, group, leverage, turnover, trading-cost
  and tracking-error constraints for long-only and long-short portfolios; risk budgeting and risk
  parity by Newton's method; hierarchical risk parity with single, complete, average or Ward
  linkage; and the efficient frontier. Every solution reports its status, objective value,
  largest constraint violation, iterations and time. Validated against CVXPY and PyPortfolioOpt.
  Rebalancing proposes the trades that move an account to target weights, with the lots each
  sale closes, its realised gain, and the currency conversions that fund purchases, as
  transactions ready to apply to the books.
- WebAssembly bindings and an npm package with TypeScript types.

### Removed

- Data fetching from third-party websites, screeners and technical indicators.
- Charts and HTML reports; the engine now returns data for callers to present.
- Go bindings and the C interface.

## [0.9.0] - 2026-05-01

The last release of the 0.x series. See the `0.9.x` branch.
