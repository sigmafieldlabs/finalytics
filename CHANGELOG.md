# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A guide to migrating from 0.9 (`docs/migrating-from-0.9.md`): what changed, what replaces
  each part of 0.9, and a 0.9 optimisation worked through in 1.0, run as a test.
- Reporting periods are read from their labels by the engine (`Period` implements `FromStr` and
  `Display` in Rust), so Python and JavaScript accept the same labels, now including
  `since 2026-01-31`.
- Two front doors to the library. `Assets` takes the prices or returns of things one might hold
  (a pandas DataFrame, CSV or a `PriceTable`), detects their frequency, annualises them, and
  gives each asset's statistics, the covariance and correlation, and optimisation by objective
  name with bounds, per-asset bounds and group limits given by name, such as
  `assets.optimise(objective="maximum_sharpe", bounds=(0.0, 0.4))`; one asset, such as T-bills,
  may serve as the risk-free asset. `Portfolio` holds a portfolio's records, its market data and
  the books kept from them, and gives its valuation, weights, performance, rebalancing (with
  `with_rebalance` to make the trades), all six reports in one call (`report`) and the
  instruments it holds as `Assets`. Rust has builders (`assets.optimiser().maximum_sharpe()`,
  `PortfolioCsv::new(transactions).prices(prices)`), Python keyword arguments, and JavaScript
  options objects.
- Illustrative datasets (`finalytics::datasets`, `fl.datasets`, `datasets` in JavaScript):
  30 years of month-end levels of six asset-class indices, and a three-year, two-account
  portfolio in US dollars and euros with the options of its full report. The data is synthetic,
  drawn with a fixed seed by `scripts/datasets.py`; each file is also available as text, as an
  example of the layout Finalytics reads.
- A portfolio can be read from its transactions alone: `date` may stand for `trade_date`, `id`
  and `account` may be left out (`t1`, `t2` and so on, and `main`), the instruments file may be
  left out (each instrument takes the currency given with it in the transactions, or the base
  currency), and the base currency may be left out (that of the first deposit).
- `formats::market_from_csv` builds market data from price and FX files, and
  `formats::price_columns_from_csv` reads prices in the order of the file.
- In Node.js the npm package is ready on import: its `node` entry point loads the WebAssembly
  file, so `init` is needed only in browsers.
- Rust constructors for inputs that gained `#[non_exhaustive]`: `TaxRates::new`,
  `BlackLitterman::new`, `Resampling::new`, `Turnover::new`, `HistoricalWindow::new`,
  `FactorShocks::new`, `ScenarioRiskModel::from_covariance` and `from_factor_model`;
  `TradeLimits` implements `Default`.

### Changed

- Rust: the result types, the option types and the enumerations that may grow are
  `#[non_exhaustive]`, so that 1.x releases can add fields and variants without breaking code.
  Options start from `Default::default()` and have their fields set; `match` on an enumeration
  needs a wildcard arm. `Transaction::new` builds a transaction as `Instrument::new`,
  `Account::new` and `PortfolioData::new` already do for theirs; `SchemeRisk::new` builds a
  scheme's risk. Python and JavaScript are unchanged.
- The descriptions of the shared data types no longer name Rust methods or `None`; they read the
  same in the JSON Schema, the Python types and the TypeScript declarations.
- The holdings report counts a lot or a gain as long-term when it has been held for more than
  `long_term_after_days` days, as tax-aware rebalancing does. A lot held for exactly that many
  days was long-term in the report and short-term for rebalancing.
- Rust documentation examples use the paths of the `finalytics` crate.
- A holding is written as text in data: an instrument's identifier, such as `"SPY"`, or a
  currency followed by ` cash`, such as `"USD cash"`. It was an object such as
  `{"holding": "instrument", "id": "SPY"}`. In Python a holding is a `str` (`fl.cash("USD")` gives
  `"USD cash"`; `fl.holding` and `finalytics.pandas.label` are gone); in TypeScript a `string`.
  An instrument's identifier may not take the cash form.
- Values keyed by an instrument, a currency, a factor, a year or a holding are maps, such as
  `{"SPY": 0.6, "USD cash": 0.4}`, rather than lists of pairs: weights before and after a
  rebalance, shocks, proxies, factor returns, currency exposures, expected returns and benchmark
  weights for reports, and realised gains and income by year, instrument or holding. Rebalancing
  targets are maps in JavaScript too, and `holding_weights` returns a map in Rust. Validation
  paths into these maps end in the key, such as `shocks.instruments.SPY`.
- Every tagged union is tagged by `type`, such as `{"type": "maximum_sharpe"}`, in place of
  tags named after each union (`objective`, `method`, `prior` and others). Transactions already
  used `type`, so portfolio documents keep their shape.
- Renamed for consistency: in scenario results, `local_return` and `total_return` of a holding
  are `return_local` and `return_base`, and the money values at risk are `value_at_risk_base`
  and `value_at_risk_after_base`; a proposed trade's `fees` are `fees_local`; Rust
  `opt::Method` is `SolveMethod`, and `Problem::new` is `Problem::from_covariance`.
- Rust: `Problem`, `TaxRates`, `BlackLitterman`, `Resampling`, `Turnover`, `TradeLimits`,
  `ScenarioRiskModel`, `HistoricalWindow` and `FactorShocks` are `#[non_exhaustive]`. A new
  parameter of an enumeration's variant will arrive as a new variant.
- The input records of a portfolio (base currency, accounts, instruments and transactions) are
  named `PortfolioData`, in Rust, Python, TypeScript and the JSON Schema, so that `Portfolio` can
  name the portfolio itself. The portfolio document's `portfolio` member is unchanged.
- Python: `Portfolio` replaces `Books` (`fl.Portfolio(data, market)` or
  `fl.Portfolio.from_csv(transactions=..., prices=...)`), and the reports are its methods
  (`portfolio.performance_report(...)`) rather than functions of `fl.reports`; `rebalance` takes
  the targets and, optionally, the `accounts` (every account by default), and returns one
  proposal for all of them. `valuation` and `weights` default to the last price date.
- JavaScript: the `Portfolio` class replaces `Books`, with `Portfolio.fromCsv(files)` in place of
  `portfolioFromCsv`; `rebalance(targets, { accounts })` replaces `rebalance` and
  `rebalanceAccounts`, and `dates()` replaces `defaultDates()`. The worker client's `books` is
  `portfolio`.
- Cash in the base currency is riskless in the reports: it is left out of the holdings'
  covariance estimate, so that Ledoit–Wolf shrinkage no longer lends it part of the average
  variance, and its variance and covariances are zero. The overview's efficient frontier is that
  of the invested holdings, so its least risky point is no longer nearly all cash; each point's
  weights still follow the holdings, with none in base-currency cash.
- Fees and taxes charged to an account rather than to a holding are expenses: a new allocation,
  `expense`, records them, holding lines carry `expenses_local` and `expenses_base`, and they are
  no longer a loss on the cash that paid them. Their share of the return is their own line:
  `Performance::expenses` (`performance.expenses()` in Python and JavaScript), the performance
  report's `expenses`, and a "Fees and taxes" row in its contribution table. With the holdings'
  contributions they add up to the time-weighted return. Attribution segments' returns are
  before them.
- `formats::portfolio_from_csv` takes the base currency and the instruments file as options.
- Rebalancing one account through `rebalance_accounts` gives exactly `rebalance`'s proposal.
- Rust: the performance types and functions are in `finalytics::performance` rather than at the
  crate's root, and `finalytics::numeric` is no longer public. Semantic versioning covers the
  `finalytics` crate; the crates it is built from say that their own API may change in any
  release.
- JavaScript: `writeDocument` accepts the `MarketData` that `readDocument` returns, and
  `impliedRiskAversion` takes its options as an object, as `statistics` does.
- A solution, and a resampled one, compare equal to another when everything but the time taken
  is equal.

### Fixed

- JavaScript: the valuation dates of a `Performance` are its `dates` property; they were
  published as `parse_dates`.
- Python: `finalytics.performance` is listed in the package's `__all__`, and lists its own
  public names.

## [1.0.0-alpha.0] - 2026-11-02

Finalytics 1.0 is a redesign of the library. See the
[migration guide](docs/migrating-from-0.9.md) for moving from 0.9.

### Added

- A workspace of crates behind the `finalytics` facade: core, ledger, formats, performance, risk,
  optimisation and reports.
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

[Unreleased]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-alpha.0...HEAD
[1.0.0-alpha.0]: https://github.com/sigmafieldlabs/finalytics/releases/tag/v1.0.0-alpha.0
[0.9.0]: https://github.com/sigmafieldlabs/finalytics/tree/0.9.x
