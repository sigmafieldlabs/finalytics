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
- A Python package (abi3 wheels for Python 3.10 and later) with the whole engine: portfolios
  from dataclasses, JSON, documents or CSV; market data from mappings, pandas Series or CSV;
  books, valuations, performance, contribution, risk and optimisation, with NumPy arrays in and
  out, frozen dataclasses for results, `ValidationError` listing every problem, and optional
  pandas conversions (`finalytics[pandas]`).
- `finalytics-core`: ISO 8601 dates with day and month arithmetic, business-day calendars with
  caller-supplied holidays and the standard adjustment conventions, ISO 4217 currencies, exact
  decimal money with currency-checked arithmetic and rounding, date-indexed time series with
  as-of lookups and reindexing that marks carried-forward values, validation reports that list
  every problem with its location, and compensated summation.
- `finalytics-ledger`: instruments with classifications and multipliers, accounts, sixteen
  transaction types including splits, spin-offs, mergers, symbol changes and returns of capital,
  tax lots relieved first-in first-out, last-in first-out, by highest or lowest cost, at average
  cost or by specific identification, cash in several currencies, realised gains in local and
  base currency, holdings on any date (with cash balances as records), previews of the lots a
  sale would close, and valuation with caller-supplied prices and FX rates (direct, inverse or
  crossed through a pivot currency), with carried-forward values reported.
- `finalytics-formats`: CSV readers for instruments (with classification and identifier
  columns), transactions of every type, and prices and FX rates in long or wide layouts, each
  reporting every problem by line and column; and the versioned portfolio document in JSON,
  with market data, that reports where a malformed document goes wrong; market data on its own
  in the document's format.
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
- Trading costs and limits: quadratic costs (such as market impact) in mean–variance, robust
  mean–variance and target volatility objectives; a minimum trade size and a maximum number of
  trades, met by a heuristic that is reported as such; fees in rebalancing proposals, which size
  purchases to fit and report gains after fees exactly as the books record them; tax-aware sales
  that take the lots costing the least tax and estimate it, from short- and long-term rates; and
  rebalancing several accounts together to targets for their combined value.
- Factor models: estimation by time-series regression on factor returns, from the principal
  components of the sample covariance, or by cross-sectional regression on given exposures, with
  each asset's fit; models built from exposures and risk estimates supplied by the caller; the
  covariance a model implies; and the decomposition of volatility and ex-ante tracking error by
  factor, by the specific part and by asset. Validated against statsmodels, scikit-learn and
  NumPy.
- Optimisation with a factor model in place of a covariance matrix, solved in factor form (a
  500-asset problem with ten factors solves in about 10 ms), and limits on factor exposures,
  absolute or relative to a benchmark.
- Black–Litterman expected returns: an equilibrium prior implied by market weights, or given prior
  returns; absolute, relative and portfolio views whose uncertainty is proportional to their
  variance, set from a confidence (Idzorek) or given; the posterior returns and covariance; and
  the market's implied risk aversion. Validated against PyPortfolioOpt.
- Robust mean–variance optimisation, maximising the worst case over an ellipsoid or a box of
  possible expected returns, and resampled optimisation (Michaud), whose seeded random numbers
  give the same result on every platform. Validated against CVXPY and an independent
  reimplementation of the simulation.
- Performance attribution: Brinson–Fachler allocation, selection and interaction by segment,
  linked over periods by Cariño's or Menchero's method so that the effects add up to the excess
  of the compounded returns; and currency attribution by the method of Karnosky and Singer, with
  market effects on returns in excess of cash, currency effects that include interest-rate
  differences, and given currency exposures for hedged portfolios. Validated against an
  independent NumPy implementation.
- Scenarios: shocks to instruments, groups of instruments, currencies and factors (with the
  factors not shocked moved by their conditional expectation, optionally), and historical
  periods replayed on current holdings, with proxies for instruments that had no prices then;
  the effect on value by holding and by group, the path of value over a historical period, and
  the volatility, risk decomposition and value at risk before and after, with volatilities and
  correlations optionally stressed. Validated against an independent implementation in pandas,
  NumPy and SciPy.
- Households: classifications on accounts and an accounts CSV reader; each account's values and
  external flows on every date; and performance by account, by group of accounts and
  consolidated, with time- and money-weighted returns and each account's contribution to the
  consolidated return, adding up exactly. Validated against an independent implementation in
  pandas and SciPy.
- Reports: the data for each screen of a portfolio workstation, computed in one call from the
  books: holdings (positions by holding and account, tax lots, allocation by every dimension,
  concentration, realised gains and income), an overview (headline figures, allocation, risk
  and return by holding, the efficient frontier with the current portfolio), performance
  (standard periods against a benchmark, monthly and calendar-year returns, contribution by
  holding and group, flows by month, the household by account), risk (statistics, drawdowns,
  tail risk, the return distribution, correlation, risk decomposition, rolling statistics),
  attribution measured from the books, and scenarios side by side. Every report carries its
  provenance, with a SHA-256 digest of its inputs that is the same in every language; its series,
  breakdowns, grids and histograms are ready for charts; and it turns into typed tables for
  export. Validated against an independent implementation in pandas and NumPy.
- A JSON reader for any of the engine's types (`formats::from_json`) that reports every problem
  with its path, including those found by a nested value's own checks; both bindings use it.
- An npm package that runs the engine as WebAssembly, with the same API (`Books`,
  `Performance`, `MarketData` and functions for risk and optimisation), `Float64Array`s for
  series, TypeScript declarations for every data type, and a worker with a promise-based client
  for running the engine off the main thread. The engine is about 1.0 MB gzipped.
- Methods and validation pages (`docs/methods/`) for every model: definitions, conventions,
  references, how each is checked against independent implementations, and examples that run as
  tests.
- JSON Schema for every data type the bindings exchange, and for the portfolio document on its
  own, generated from the Rust types; the Python dataclasses and TypeScript declarations are
  generated from it, with the Rust documentation.

- Benchmarks of the main operations and performance budgets checked in CI: books for 100,000
  transactions in under 2 seconds, natively and in WebAssembly, and a 500-asset optimisation in
  under half a second. Results are in `docs/performance.md`.
- Fuzz targets for every reader and for the numerical entry points, with a seed corpus that the
  tests replay.

### Removed

- Data fetching from third-party websites, screeners and technical indicators.
- Charts and HTML reports; the engine now returns data for callers to present.
- Go bindings and the C interface.

## [0.9.0] - 2026-05-01

The last release of the 0.x series. See the `0.9.x` branch.
