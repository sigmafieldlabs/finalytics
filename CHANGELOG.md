# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A sample universe for screening: `datasets::universe()` in Rust, `fl.datasets.universe()` in
  Python and `datasets.universe()` in JavaScript (also through the worker, as
  `engine.static("datasets", "universe")`). It holds 200 fictional companies in eleven sectors,
  with six years of daily closes, volumes and quarterly dividends to October 2026, five fields
  each (`name`, `sector`, `market_cap`, `trailing_pe` and `price_to_book`) and a market index,
  `INDEX`, as the benchmark. The library draws it with a fixed seed, the same on every platform.

- `set_show_handler` in Python and `setShowHandler` in JavaScript say what `show()` does with a
  report's page: the handler receives the page's HTML instead of a notebook or a browser showing
  it. For programs that show pages themselves, such as Python running on Pyodide, which has no
  browser to open and raised an error before, or an engine in a worker, which has no page.

- Guides with examples in Python, JavaScript and Rust, in `docs/guides` and `docs/examples`.
  `scripts/guides.py` runs every example in each of its languages and checks that they print
  the same, that the output is the one recorded and that every number in a page's prose is one
  its examples print; CI runs it on every change.

### Fixed

- Equal weight, inverse volatility, risk parity, risk budgeting and hierarchical risk parity
  refused `benchmark_weights` that held cash in another currency, such as `portfolio.weights()`
  with its `EUR cash`, saying that they "take no other constraints"; so did a comparison or
  backtest with one of them and such a benchmark. The engine held that cash at zero in their
  problem so that the tracking error counted its risk. They now solve without it, and their
  report measures the tracking error with it, as for the other methods.

## [1.0.0-rc.6] - 2026-11-09

### Added

- `keep_cash` in `optimise`, as in `rebalance`: `keep_cash=True` (`.keep_all_cash()` in Rust)
  holds every currency's cash in `current_weights` at its current weight, and
  `keep_cash=["USD"]` (`.keep_cash(["USD"])`) that of the currencies named, and optimises the
  rest, with every limit measured on the whole portfolio. The weights hold the kept cash, so
  `portfolio.rebalance(result, keep_cash=True)` trades what the limits allow, and its turnover is
  the optimisation's. It needs `current_weights`, takes no `cash` limits on the cash it keeps,
  and applies to the frontier and to each refit of a walk-forward backtest (`keep_cash` in
  `compare` and `backtest`). `methods()` lists it with the limits of every method that takes
  limits, as a new kind of option, `currencies`; `finalytics::KeepCash` is its type in Rust.

- `include_benchmark` in `screen` (`.include_benchmark()` in Rust) screens the universe's
  benchmark as a candidate too; see Changed.

- A Python test runs every example in the package's docstrings, and JavaScript and Python tests
  hold the contract sweep's cases: every problem reported together, at the option given.

### Fixed

- Every problem with a call's inputs is reported together. A value that could not be read hid
  every other problem: an unknown method hid a limit out of range, a number that is not finite
  hid an unknown asset, fixed weights given limits hid an unknown asset, and a rebalancing's
  targets, a report's account and an asset setting hid the checks after them. Options are read
  field by field and checked on the fields that read; with an unknown method, only the checks
  that do not depend on it run, so no option is said to be unused by a method not chosen.

- Five of the Python package's docstring examples used names they did not define; they run as
  written now. The cash and `optimise` docstrings said optimisation never holds cash, which
  `cash` limits and `keep_cash` change, and JavaScript's `Result` left out `evaluate` and
  `rebalance`. The methods page said one rule for unused options applies "in every verb"; it
  says which verbs now, and what the shared options `risk_free_rate`, `expected_returns`,
  `current_weights` and `benchmark_weights` change in a backtest: the fits of the methods that
  use them, not the report's ratios.

- Minimum CVaR with limits on cash, such as `cash=(0, 0.2)`, failed in its report ("weights: 6
  weights for 5 assets") on every kind of assets, though `methods()` lists the limit for it. The
  report now splits the expected shortfall over every weight, the cash's included, on the
  returns the method minimised.

- An optimisation's weights could fall a little below zero, of the order of 1e-13, and the engine
  then refused its own output: `portfolio.rebalance(optimisation)`, the weights given back as
  `current_weights`, and walk-forward backtests with trade limits, whose refits checked the
  weights the run held as if they had been given. The weights of `optimise` and the frontier are
  now within their bounds, at zero or the current weight where the solver was within 1e-7 of
  it, and add up to one; their expected return and volatility are those of the weights given.
  A refit starts from the weights the run holds without checking them as user input.

- A walk-forward refit with trade limits after fits that found no weights, holding only cash,
  sent NaN weights without a risk-free asset (and could not invest with one). With nothing
  invested, a refit now enters from cash, as the first fit does.

- Rebalancing to an optimisation traded every instrument: the assets the optimisation did not
  trade were within about 1e-10 of their current weights, and each difference became a separate
  trade of a fraction of a unit, paying the minimum fee, so the optimisation's `max_trades` and
  `min_trade` did not hold in the trades. An optimisation's weights for the assets it does not
  trade are now their current weights exactly, and `rebalance` treats a change worth less than
  one minor unit of the instrument's currency, or a billionth of the account's value, as no
  trade.

- Every input that takes weights by asset now follows one rule, and `portfolio.weights()` is
  valid in each. `current_weights`, `benchmark_weights` (in `optimise` and the frontier), fixed
  weights and benchmark weights in backtests and comparisons refused cash in another currency,
  such as `EUR cash`, which `Portfolio.assets()` lists and `evaluate` takes; `current_weights`,
  `benchmark_weights` and plans refused `Cash` at the risk-free rate unless `cash` limits were
  given. Each now takes every label the assets list, cash included, and `Cash` where it stands
  for cash at the risk-free rate. An optimisation invests the cash in another currency it is
  given, counting it in the turnover and listing it among the trades; a benchmark's cash in
  another currency counts in the tracking error, with its risk; and a backtest holds such cash
  as the asset it is, moving with its exchange rate.

- A report's `benchmark_weights` (for the tracking error) that named a holding not held, or did
  not add up to one, and `expected_returns` for a holding not held, were accepted: the first was
  caught inside the risk estimate and turned into a warning that dropped the holdings' risk from
  the overview and the risk report, and the others changed nothing. They are now refused when the
  report is built, with the other options, at `options.benchmark_weights` and
  `options.expected_returns`.

- The optimisation report's chart labelled equal weight's point beside the weights' dot when
  the two nearly coincided, as for equal weight itself or given weights close to it, so the label
  read as the weights'. The label is left out then; the legend names the point.

- The backtest and comparison reports echoed fixed weights, and a benchmark's weights, without
  their cash and with every asset they left out at zero: `{"US Equity": 0.55, "Bonds": 0.35,
  "T-Bills": 0.1}` came back as five assets adding up to 0.9. The reports' weights are now those
  given, the risk-free asset, `Cash` and cash in other currencies included.

- A backtest given a name (`name="Mine"`) used it for the strategy and the report's title but
  left `report.name` empty, where `optimise`, `compare` and `evaluate` put it. It is now there.

- The Python and JavaScript documentation of `Portfolio.assets()` said it holds the instruments
  alone; it holds the portfolio's cash too, as the Rust documentation says: base-currency cash as
  the risk-free asset and cash in other currencies, so that the portfolio's own weights can be
  valued and optimised from.

### Changed

- One message for a label that is not one of the assets, `unknown asset Nope`, in every input
  that takes weights or values by asset; rebalancing targets said "is not an instrument of the
  portfolio". Current and benchmark weights and `evaluate`, like fixed weights, report weights
  that do not add up to one beside an unknown label, and fixed weights say where the cash goes
  as the others do ("give T-Bills, the risk-free asset, any cash"). Limits and risk budgets that
  name cash in another currency say that optimisation does not choose it, and expected returns
  for the risk-free asset or `Cash` point to `risk_free_rate`.

- The digests of the optimisation, backtest and comparison reports (`context.digest`, printed as
  the inputs at the foot of each page) now cover every input: the strategies, their weights and
  options, the benchmark, the validation and the money for backtests and comparisons, and the
  method's parameters, the current and benchmark weights and the given weights for `optimise`
  and `evaluate`. Different inputs gave the same digest, such as minimum variance and risk
  parity, or a target return of 6% and of 7%. Every such digest changes.

- `Assets.frontier` refuses what it does not use, in the words of the rule every verb follows
  (`options.target: the frontier does not use target`): a method (in JavaScript), a method's
  parameters, resampling, and the risk-free rate unless it is the return of `Cash` on a frontier
  that holds cash. It accepted them and did nothing. Python's `frontier` takes `risk_free_rate`
  for that case. In TypeScript, `frontier` takes `FrontierOptions`, the limits and the options
  they use, rather than `Partial<OptimiseOptions>`, and the options of `compare` and `backtest`
  (`OptimiseLimits`) no longer include `current_weights`, which backtests refuse.

- A comparison refuses an option none of its methods takes in the words one optimisation uses,
  grouping the methods by their reason, and its notes name the options that do not apply to a
  method (`The limits on cash do not apply.`). It said that maximum Sharpe or minimum CVaR "is
  fully invested and long only, and takes no limits on weights", and noted that the limits on
  weights did not apply to a method when others had. A method that takes no limits, given
  `cash`, is said to take none rather than to keep its ratio.

- Fields that options and documents do not have are refused at every depth, as the options'
  top level already was in Python and JavaScript: a group's misspelt `maxx`, which left the group
  without a limit, a goal's `nmae`, a walk-forward's `yeers`, and in Rust a misspelt top-level
  field of `ReportOptions`, `OptimiseOptions` or `BacktestOptions` read from JSON. JSON with
  extra members, which was read as if they were not there, is now refused, naming the member.

- `rebalance` with `keep_cash` takes targets that give the kept cash its weight now, as an
  optimisation that kept the same cash gives them, as shares of the whole value; targets that
  leave the kept cash out are shares of the rest, as before. A target for kept cash at another
  weight is still refused, now saying the weight it has.

- A rebalancing's `turnover`, for each account and for the accounts together, is half the sum
  of the changes in weight with the cash included, as an optimisation's is and its
  `max_turnover` limits; it was half the value traded in instruments. For the same trades the
  two figures now agree; a rebalancing that invests or raises cash shows a larger turnover.

- The overview report's efficient frontier is the instruments', the one `optimise` and
  `evaluate` draw: it chose cash in other currencies as an asset, which optimisation never does,
  so its least risky point leaned on the cash. The overview's frontier figures change.

- Attribution periods that reach past the report's start or date are attributed over the part
  within the report, with the benchmark's figures as given and a warning that says so, rather
  than refused; a period wholly outside the report is left out with a warning, and a report with
  no period inside it is refused.

- A screen's benchmark serves beta and the other measures against it and is not a candidate:
  it is no longer listed, matched or counted among the securities, unless `include_benchmark`
  asks for it.

- In Rust, `Portfolio::assets()` returns a builder ended by `.build()`, as every other verb with
  options does, so that options can be added to it later: `portfolio.assets().build()?`.
  Python and JavaScript keep `assets()`.

- Problems are reported at the option given, under `options` in every verb and language:
  `options.trading_cost_bps` and `options.validation.start` in backtests, `options.chance` in
  plans, `options.limit` and `options.rules[0].measure` in screens, `options.covariance` for
  Python's `Assets` as for JavaScript's, and `options.fee_bps`, `options.tax[0]` and
  `options.accounts[0]` in a rebalancing, rather than the fee rate and tax fields they become
  inside the engine. A comparison's strategy is at the method or weights it was given as
  (`options.methods[1]`, `options.weights.Mine`) rather than `strategies[k]`, a benchmark's
  weights at `options.benchmark`, a plan's fixed weights at `options.weights` rather than
  `weights.weights`, and a report's date before the books at `options.as_of` rather than a
  `start` not given. Problems are worded alike in JavaScript and Python: an unknown name or
  method as `minimum_varience is not one of minimum_variance, …`, a misspelt option with a
  suggestion (`is not an option; did you mean max_weight?`), a negative count as `must be a
  whole number of zero or more`, and a number that is not finite as `NaN` or `Infinity`.

## [1.0.0-rc.5] - 2026-11-09

### Added

- Formula fields in screening: `formulas={"risk_adjusted": "return_1y / volatility"}` (Python
  and JavaScript, `.formula(name, formula)` in Rust) adds numeric fields computed for each
  security from its measures, numeric fields and earlier formulas, which the rules, the sort,
  the results, the tables and both forms of the report take by name. The formulas are a small
  language that is evaluated, never run: numbers, names, `+ - * /`, parentheses, and `abs`,
  `sqrt`, `log`, `min` and `max`. A missing input or a result that is not finite gives no value,
  counted in the report's data. Problems name the token and its place.

- Goals kept with a portfolio: `portfolio.goals` and `with_goals(goals)`, each goal a plan with
  its name, saved in the portfolio's document (`goals`, absent when there are none, so existing
  documents and their digests are unchanged) and read from a goals file in the style of the
  transactions file (`goals=` in Python and JavaScript, `PortfolioCsv::goals` in Rust).
  `assets.plan(goal)` plans a stored goal as it is (`assets.plan_with(goal)` in Rust), so a
  saved portfolio and the same history reproduce the plan.

- Reports on some accounts: `accounts=["brokerage"]` (`.accounts(["brokerage"])` in Rust)
  reports on those accounts alone, from the books kept from their transactions. Transfers with
  the accounts left out count as money in and out, and every report says which accounts it
  covers and how many such transfers there are. `Books::for_accounts` keeps such books in Rust.

- The holdings report counts realised gains and income within its period as well as to date
  (`period`, from the report's start to its date, as the performance report's flows count
  them), so that a report starting this year, such as `start="2026-01-01"`, gives this year's
  realised gains and income for tax. The text and page show the period beside the totals when
  they differ, and the tables of realised gains and income by year gain a column for the
  period.

- `Assets.evaluate(weights)` values given weights on the estimates optimisation uses: the
  expected return, volatility, Sharpe ratio and shares of risk in `results`, and the optimisation
  report's page and text for them, titled "Given weights", in `report`, with the efficient
  frontier under limits when asked (`frontier=True`). The weights may hold cash, so a portfolio's
  own weights on `Portfolio.assets()` give the overview report's current point. Python
  `assets.evaluate(weights, ...)`, JavaScript `assets.evaluate({ weights, ... })`, Rust
  `assets.evaluate(weights).build()`.

- Cash in optimisation: `cash` limits (`.cash(0.0, 0.2)` in Rust, `cash=(0, 0.2)` in Python,
  `cash: { min: 0, max: 0.2 }` in JavaScript) let `optimise`, `compare` and `backtest` hold cash
  beside the risky assets, riskless, in the risk-free asset or, for assets without one, in `Cash`
  at the risk-free rate; the frontier then includes it. Without them, the weights are fully
  invested in the risky assets, as before. `risky_assets` and `cash_assets` list which assets
  optimisation chooses among and which are cash. The overview report adds
  `current_without_base_cash`, the current mix with base-currency cash set aside, on the axes of
  its frontier; cash in other currencies counts as a holding there, as its value moves with the
  exchange rate.

- Group limits by classification: a group may name a classification rather than list its assets,
  such as at most 60% in the assets classed `Equity` under `asset_class` (Rust
  `.class_limit("asset_class", "Equity", 0.0, 0.6)`, Python `groups={"Equity": ("asset_class", 0,
  0.6)}`, JavaScript `groups: { Equity: { classification: "asset_class", max: 0.6 } }`). `Assets`
  carry classes: a portfolio's assets take its instruments', and others take `classes`.

- Rebalancing can keep cash as it is, for every currency or those named (`keep_cash=True` or
  `keep_cash=["USD"]` in Python and JavaScript, `.keep_all_cash()` or `.keep_cash(["USD"])` in
  Rust): the targets are then shares of the rest of the value, so the trades neither invest that
  cash nor add to it. The results and the report say what was kept. By default the targets are
  still shares of the whole value, cash included.

- Blended benchmarks: the reports' `benchmark` may be weights by holding, such as
  `{"VTI": 0.55, "BND": 0.35, "GLD": 0.1}` (`.benchmark_blend(...)` in Rust), held at those
  weights in the base currency and rebalanced monthly. From a blend, the attribution report
  derives the benchmark's segments in each month by the portfolio's classification, given only
  the scheme (`attribution="asset_class"`, `.attribution_by("asset_class")`), so Brinson–Fachler
  attribution works for any portfolio rather than only with segments given by hand.

- Trade and tracking-error limits in `Assets.optimise`, with plain inputs in every language:
  `current_weights` (the weights held now, with any cash on the risk-free asset),
  `max_turnover`, `min_trade`, `max_trades`, `benchmark_weights` and `max_tracking_error`. They
  were available only through the low-level `solve(problem, objective)`. Every problem with them
  is reported together; the frontier is drawn under the same limits; and the optimisation report
  lists the trades from the current weights with the turnover (a `trades` table too) and gives the
  tracking error against the benchmark weights. In `compare` and `backtest`, the tracking-error
  limit applies at every fit and the turnover and trade limits at each walk-forward refit, from
  the weights held then.

- `methods()` (`finalytics::methods()` in Rust, `fl.methods()` in Python, `methods()` in
  JavaScript) and `Method::info()` describe every optimisation method: its parameters, with
  whether it needs each, its default and the values allowed, the limits on weights it takes, and
  whether it resamples or is measured on the periods' returns. The checks on the options follow
  the same descriptions, so that a form built from them offers only what the method uses.

- `PortfolioReport::tables()` in Rust, and `reportTables` of the six reports of
  `Portfolio.report` in JavaScript, give the tables of each report in turn, as `tables()` does in
  Python.

### Changed

- `Portfolio.assets()` includes the portfolio's cash: base-currency cash, such as `USD cash`, as
  the risk-free asset, riskless and earning nothing, and cash in other currencies, such as
  `EUR cash`, estimated with the instruments as it moves with its exchange rate. The estimates are
  the overview report's, so the portfolio's own weights valued on them give the overview's
  current point, and the rebalance report's figures for an optimisation's target are the
  optimisation's. Optimisation still chooses among the instruments alone, but its estimates now
  include the foreign cash, so its weights can differ slightly from rc.4's; `labels` lists the
  cash after the instruments. `Assets.covariance()` gives a risk-free asset whose return is the
  same in every period, such as idle cash, no variance, rather than the share a shrinkage
  estimator lent it.

- Rust: `AssetGroup` has a `classification` field, and its `min` and `max` default to 0 and 1
  in JSON.

- Rust: `ReportOptions::benchmark` is a `ReportBenchmark`, an instrument or a blend; an
  instrument converts with `.into()`, as in `options.benchmark = Some(id.into())`. The JSON form
  of an instrument, its identifier, is unchanged.

- The sample portfolio's instruments list BENCH, its benchmark, in USD.

- The npm package's WebAssembly engine is smaller, about 1.7 MB gzipped with this release's
  additions rather than 2.0 MB, and no slower: the build leaves out the functions' names, which
  only debugging uses, and optimises the module with Binaryen's `wasm-opt`. Errors are
  unchanged; only the stack trace of an internal failure shows function numbers rather than
  names. Building the package from source needs `wasm-opt` from Binaryen version 123, which
  `scripts/install-binaryen.sh` installs and checks against its pinned checksum.

- One rule for an option a method does not use, in every verb. `optimise` and a single
  `backtest` refuse it, as before. `compare` applies each option to the methods that take it and
  notes in the report each method it does not apply to (such as "Maximum Sharpe: the confidence
  does not apply"). Two cases that used to pass now raise a validation error, so code that relied
  on them needs the option removed:
  - `compare` given an option that none of its methods takes, such as a `target` for minimum
    variance and maximum Sharpe, which it used to ignore, now refuses it, naming them all
    ("minimum variance and maximum Sharpe do not use target");
  - limits or parameters given to `compare` with fixed weights alone, or to a Rust `backtest` of
    fixed weights, which were dropped, are now refused ("fixed weights take no limits or
    parameters"), as Python and JavaScript already refused them for `backtest`.

- A risk aversion must be above zero, as the solver requires: zero was let through the checks
  and refused by the solver.

### Fixed

- A benchmark is converted to the base currency: its levels are in its own currency, as the
  instruments list it, converted at the market data's rates, so a report in EUR no longer
  compares with a dollar benchmark's levels. When the data has no rate for it, or the
  instruments do not list it, each report's header says so, as it says how the benchmark is
  measured in every case.

- `compare` keeps the strategies in the order they are given, in the results, the tables, the
  legends and both forms of the report: it listed the methods first and, in JavaScript, the fixed
  weights by name. Fixed weights may now come among the methods: in Rust `compare` takes
  strategies, such as `[Strategy::weights("60/40", [...]), Method::MinimumVariance.into()]`, and
  `.method()` adds one in turn; in Python `methods` takes `("60/40", {...})` pairs, and in
  JavaScript `{ name, weights }` objects.

- JavaScript: typed arrays, such as `Float64Array`, are read wherever an input holds arrays of
  numbers: `Assets.fromReturns` took them by its types but refused them ("invalid type: map,
  expected a sequence"), and so did returns tables, matrices and the other structured inputs. A
  number that is not finite in one is reported at its index.

- Every report table's title is a valid Excel sheet name (at most 31 characters, none of
  `[ ] : * ? / \`, and unique within the report), so that the tables of a report export to one
  workbook: "Holdings: return and risk" is now "Return and risk by holding", "Value at risk and
  expected shortfall" is "VaR and expected shortfall", and the planning report's changes are
  "Changes that reach the chance" or "Changes that keep the chance". The tables' names are
  unchanged.

## [1.0.0-rc.4] - 2026-11-08

### Added

- Screening: `Universe` holds many securities, each on its own dates (no alignment), with a
  close on each and, where known, volumes, dividends and fields: numbers or text the library does
  not compute, such as a sector or a price-to-earnings ratio. It is built from series, from CSV in
  the long layout (`id`, `date`, `close`, with `volume` and `dividends`) or the wide one, or with
  `Universe::builder()`, with fields from CSV, a benchmark and a risk-free rate.
  `universe.screen()` (`universe.screen(...)` in Python and JavaScript) measures every security up
  to the date measured to: returns over calendar windows from a week to five years, volatility,
  downside deviation, drawdowns, Sharpe, Sortino and Calmar ratios and value at risk, beta,
  correlation and relative returns against the benchmark, moving averages, Wilder's RSI, the MACD
  histogram, 52-week ranges, dividend yield, volumes and the state of the data. A measure whose
  window is longer than a security's history has no value, and a security whose last close is
  more than `max_carry_days` (5) old is stale: listed but not measured. Rules compare a measure or
  a numeric field with a number or a range, or test a text field against a list of values; a
  security without the value a rule tests fails it, and every problem with a screen is reported
  together. The matches are sorted and limited. Like every verb, it returns the screening report,
  which shows how the rules narrow the universe, the matches with a mark beside a value close to
  a threshold, the securities that fail just one rule, where each threshold falls among every
  security's values, the quartiles of every measure some security has and what is out of the
  ordinary in the data, with each measure named as people say it, as a page, in 80 columns of
  text and as tables, and the raw results, one row per security. Python takes a long DataFrame, a
  mapping of Series or CSV, and `finalytics.pandas.securities` turns the results into a
  DataFrame; JavaScript adds `Universe.fromColumns` for flat typed arrays and returns the results
  as one `Float64Array` per measure, and the worker client holds universes (`engine.universe`).
  Securities are measured in one pass each, on several threads with the `parallel` feature:
  5,000 securities with ten years of daily closes take about 0.4 seconds in WebAssembly.
  The engine is the `screening` module of `finalytics-risk`, and `finalytics::screening`
  re-exports it. The methods page is `docs/methods/screening.md`, and every measure is checked
  against an independent implementation in pandas (`scripts/reference/screening.py`).
- JavaScript: the worker client calls static methods of `Portfolio`, `Assets`, `MarketData` and
  `datasets` (`engine.static("Assets", "fromCsv", csv)`), so the sample datasets and every reader
  of CSV text work off the main thread. Market data returned by the worker stays there as a
  handle, like portfolios, assets and performance records.

## [1.0.0-rc.3] - 2026-11-06

### Added

- Goal planning: `Assets::plan` (`assets.plan(...)` in Python and JavaScript) tests a plan of
  saving, spending and investing against one goal, an amount by a date (`Target`) or payments
  over a period (`Payments`), or projects it to a month without one. The plan runs month by month
  through simulated markets, 10,000 by default: a stationary block bootstrap of the assets'
  monthly returns (the default), a normal distribution, or every stretch of the history. Every
  amount is in today's money: returns are after inflation, a fixed rate or a price index, and
  fees. Flows are paid in (`Saving`) or taken out (`Spending`) once or regularly, and a mix is
  fixed weights or a glide path. The results give the chance of the goal and its status against
  the chance wanted; the value over time in strong, average and weak markets; for payments, the
  share paid and when the money runs short; the smallest change to the saving, the start value,
  the amount or the date that reaches the chance wanted (or the largest that keeps it); the plan
  with returns a point a year lower and with the worst twelve months of the history placed in
  it; and other mixes compared on the same simulations. The planning report shows them as a
  page, in 80 columns of text and as tables. A new crate, `finalytics-plan`, holds the engine;
  the `finalytics::planning` module re-exports it. The methods page is
  `docs/methods/planning.md`, and the engine is checked against an independent implementation
  in NumPy (`scripts/reference/planning.py`).
- Backtests and comparisons can follow money as well as returns: `start_value` and `flows`
  (`.start_value(...)` and `.flow(date, amount)` in Rust) start each run with a value and pay
  money in or take it out on dates. The comparison and backtest reports then show each run's
  money paid in and taken out, its ending value and its money-weighted return, as text, on the
  page and in a `money` table.
- A `parallel` feature, on by default in the `finalytics` crate and in the native Python package,
  runs independent work on several threads with `rayon`: the strategies of a comparison or
  backtest, the walk-forward fits (the first window is fitted first; then the refit dates are
  found, the other fits run at once, and the simulation runs in order), the histories of
  resampled optimisation and the points of an efficient frontier. Results are the same to the
  last bit on any number of threads, and the first error in order is the one reported. The
  thread pool is the caller's: `rayon`'s global pool by default, sized by `RAYON_NUM_THREADS`.
  The WebAssembly package and Pyodide leave the feature out and run on one thread.
  `finalytics_core::parallel::map` is the helper.
- The Python package releases the GIL during the engine's longer calls (building assets and
  portfolios, optimising, comparing, backtesting, reports, value and holding series, performance,
  rebalancing, scenarios, rolling statistics, statistical factor models, and solving, resampling
  and tracing the frontier of `fl.opt.Problem`), so other Python threads run meanwhile.
- The minimum-semivariance optimisation report shows the downside deviation the weights reach
  (`OptimisationSummary.downside_deviation`), and the minimum-CDaR report the conditional
  drawdown at risk (`OptimisationSummary.drawdown_at_risk`, a `DrawdownFigure`), as the
  minimum-CVaR report shows its expected shortfall.

### Changed

- **Breaking:** resampled optimisation seeds each history with the `k`-th number SplitMix64 gives
  from the seed, so that histories can be simulated in any order. Resampled weights differ from
  rc.2's for the same seed. The reference script reimplements the new seeding. The random numbers
  are now `finalytics_core::random`, which plans draw from too; resampled weights are unchanged
  by the move.
- **Breaking (Rust):** the backtest engine's `Fitter` is `Send + Sync`, so that fits can run on
  several threads, and it reports problems by returning an error, never by panicking: walk-forward
  validation fits every window before it uses the refits.
- Reports of fixed weights scored on the returns given, with no method fitted, no longer call
  the periods "in sample": the header says "Fixed weights", headings name the dates alone, and
  the strategies are counted and named as portfolios ("2 portfolios", "How the portfolios were
  tested"), or as strategies when methods and fixed weights are mixed. A holdout of fixed weights
  is split "Before" and "From" the start of the test in its charts.
- The comparison report says that its highlights and bold figures rank the strategies, beside a
  benchmark that is not ranked. With one strategy, its tiles name the figures plainly ("Sharpe
  ratio", not "Highest Sharpe ratio").
- Names keep their case in the comparison report's tiles and notes ("Min-CVaR", not
  "min-CVaR").
- The optimisation report's header counts three or more group limits ("11 group limits"); the
  Method section still lists each one.
- The text backtest report gives the strategy's and benchmark's names more room before
  shortening them.
- The methods pages on tail risk and risk decomposition explain how the historical expected
  shortfall and the Rockafellar–Uryasev tail mean differ.

### Removed

- **Breaking:** the S&P 500 dataset: `datasets::sp500`, `sp500_index`, `sp500_companies` and
  `Company` in Rust, with the `sp500` feature; `fl.datasets.sp500`, `sp500_table`, `sp500_index`,
  `sp500_index_table` and `sp500_companies` in Python; `datasets.sp500()`, `sp500Index()` and
  `sp500Companies()` in JavaScript; and its three files from the dataset files. The files, 2.6 MB
  of CSV, are now test data in `tests/sp500` in the repository, outside the packages. The
  household portfolio and the asset-class indices remain.

## [1.0.0-rc.2] - 2026-11-04

### Added

- `Statistics` adds the conditional drawdown at risk at 95% (the mean of the worst 5% of the
  drawdowns, as a negative return like the maximum drawdown) and the Ulcer index (the root mean
  square of the drawdowns). `RelativeStatistics` adds downside and upside betas and capture
  ratios. The reports show them, and `scripts/reference/downside.py` checks them against a
  linear programme, pandas, least squares and empyrical's definitions.
- `shortfall_decomposition` (`fl.risk.shortfall_decomposition`, `shortfallDecomposition`)
  splits a portfolio's historical expected shortfall into each asset's contribution, adding up
  to it exactly, with `group_contributions` for groups. The minimum-CVaR optimisation report
  shows the expected shortfall the weights reach, at its confidence, and gives each asset's
  share of it rather than of the variance.
- With limits on groups of assets, the optimisation report shows each group's weight against
  its limits and its share of the risk (`OptimisationReport.groups`).
- `StrategyResult.fitted` and `StrategyReport.fitted` say whether a method fitted the weights to
  returns. Fixed weights have no run in sample in the reports, and are described as fixed
  weights, not fitted to the returns they are scored on.
- `TailRisk.shortfall_at_var` flags a Cornish–Fisher expected shortfall raised to the value at
  risk, as the expansion can need when the excess kurtosis is large; the risk report marks it.
- `ScenarioSummary.stress`: the scenarios report names the stress of the covariance behind a
  scenario's risk after.
- A cash balance below zero is a warning in the valuation and the reports that use it
  (`cash.<account>.<currency>`), as when a purchase is recorded without the deposit that paid for
  it. It is not an error, so that margin and late deposits can still be recorded.
- `CovarianceMethod` reads from its name alone (`"ledoit_wolf"`) as well as from an object, and an
  exponentially weighted covariance without a decay has 0.94, in every option that takes one.
- `SolveStatus::reason`: why an optimisation found no weights, in words.
- The Rust comparison and backtest builders take every option `optimise` takes:
  `uncertainty`, `budget`, `linkage`, `expected_return` and `risk_free_rate`; Python's
  `compare` and `backtest` take `risk_free_rate`.
- `name` on `optimise` in Rust (`.name(..)`) and JavaScript (`{ name }`), as in Python.
- `saveHtml(path, name?)` on reports and results in Node.js, as `save_html` in Python;
  JavaScript results' `toText` and `toHtml` take the report's name, and TypeScript declares
  them.
- The Rust report builder has a setter for every report option: `periods_per_year`,
  `covariance`, `confidences`, `rolling_window`, `histogram_bins`, `top`,
  `long_term_after_days`, `expected_return`, `frontier_points` and `benchmark_weight`.
- `compare` takes fixed weights beside methods: `.weights("60/40", [..])` in Rust (repeatable),
  `weights={"60/40": {..}}` in Python and `{ weights: { "60/40": {..} } }` in JavaScript. They
  are held and rebalanced as the methods are and scored on the same periods, and described as
  fixed weights; `methods` may be left out when weights are given.
- `benchmark_name` names the benchmark of a comparison or backtest in every language:
  `Benchmark` by default, or the column's name for an index's prices.
- `Assets::frontier` returns `FrontierPortfolio`s, with their weights by asset (in the assets'
  order in JavaScript), rather than unlabelled lists.
- A methods page for backtests and comparisons (`docs/methods/backtests.md`): how weights are
  held, rebalanced and charged for, the three kinds of validation, and what the reports show.
- Four covariance estimators: shrinkage towards constant correlation (`constant_correlation`),
  a statistical factor model of the first principal components (`{"type": "statistical",
  "factors": 3}`), the sample correlation denoised by the Marchenko–Pastur edge (`denoised`),
  and the analytical nonlinear shrinkage of Ledoit and Wolf (2020) (`nonlinear_shrinkage`), in
  `Assets(covariance=...)` and `fl.risk.covariance`. `scripts/reference/estimators.py` checks
  them against translations of the authors' code and NumPy.
- Estimators of mean returns: `Assets(mean_returns="james_stein")` shrinks the means towards
  their average with the Efron–Morris intensity, and `"ewma"` weights recent returns more
  (`MeanMethod`, `finalytics::risk::mean_returns`). Optimisation, the efficient frontier,
  backtests and `mean_returns()` use them, and the reports name them. The historical mean is the
  default; the risk-free asset's rate stays its historical mean.
- `minimum_semivariance` and `minimum_cdar` optimise the downside of the periods' returns: the
  mean square of the returns below zero, and the conditional drawdown at risk of the summed
  returns (Chekhlov, Uryasev and Zabarankin, 2005). With `target`, they and `minimum_cvar` keep
  an annual expected return of at least `target`. In the optimiser they are
  `Objective::MinimumSemivariance` and `Objective::MinimumCdar`, with
  `Constraints.min_return` for the floor.
- `min_effective_assets` holds the effective number of assets, `1 / sum w^2`, at or above a
  floor, the norm constraint of DeMiguel, Garlappi, Nogales and Uppal (2009)
  (`Constraints.min_effective_assets` in the optimiser). Bounds that leave room for fewer are
  reported before solving. `scripts/reference/downside_objectives.py` checks the new objectives
  and limits against CVXPY.
- `optimise(resample=200)` averages the optimal weights for 200 histories simulated from the
  assets' estimates (Michaud, 1998), in `optimise`, `compare` and `backtest`; the result's
  `resampled` holds each weight's standard deviation across them, and the report says how many
  were solved. `Resampling.periods_per_year` simulates annualised estimates period by period, so
  that they are as uncertain as estimates from that many periods of returns.
- Rebalancing every so many periods, `Rebalance::Every { periods }` (`fl.Every(periods=21)`,
  `{ every: { periods: 21 } }`), counted from the first period of each run; walk-forward
  validation refits on it as on a calendar schedule.
- A theme for report pages: `theme="light"` or `"dark"` on `to_html`, `save_html` and `show` in
  Python, the last argument of `toHtml`, `saveHtml` and `show` in JavaScript, and
  `report.with_theme(Theme::Dark)` in Rust. The default, `"auto"`, follows the reader's setting
  as before.
- `Assets::origin()`: the date of the prices the first returns are measured from. Backtest and
  optimisation reports start the growth of 100 at 100, and the drawdown at 0, on the date a run
  enters: that date, or the end of the period before a test.

### Changed

- Turnover is one-way everywhere: half the sum of the absolute weight changes, the share of the
  portfolio bought (or, equally, sold). Backtests already measured it so; the turnover limit of an
  optimisation (`Turnover.limit`) and the turnover of a rebalance proposal now do too, so a limit of
  0.2 allows 20% of the portfolio to be bought, where it allowed 10% before, and a proposal's
  turnover is half what rc.1 reported. The reports say which convention they use.
- The Sortino ratio is measured against the risk-free rate, as the Sharpe ratio is, in the
  portfolio, comparison and backtest reports, which state the rate. It was measured against zero.
- Cash at a risk-free rate earns the rate compounded to each period, `(1 + r)^(1 / P) - 1`, as the
  statistics convert it, rather than `r / P`.
- Money-weighted returns over periods shorter than a year (MTD, QTD, YTD) are shown over the
  period, as the time-weighted returns beside them are, rather than annualised.
- The date of a rebalance is `as_of` in Python and JavaScript, as in Rust and every report
  (`portfolio.rebalance(targets, as_of="2026-06-30")`); it was `date`.
- Python's `datasets.price_table()` is `datasets.asset_class_table()`, beside `sp500_table()`.
- Rust takes plain inputs throughout: `PortfolioCsv::base_currency("USD")` takes the code, and
  `Portfolio::valuation()` and `weights()` value the portfolio on the last price date, with
  `valuation_on("2026-06-30")` and `weights_on(..)` for another, in place of `Option<Date>`.
- Python's `show()` returns the path of the page it opens outside a notebook, as Rust and
  JavaScript do, and shows the page in a notebook as raw HTML, without IPython's warning about
  frames. `Holdout(start="2016-11-01")` passes type checking, and holds a date.
- The JavaScript types refer to TypeScript's `esnext.disposable` library themselves, so that
  they check without it in a project's settings. JavaScript `meanReturns` lists the assets in
  their order.
- Rebalancing sells lots by each account's own lot method by default, as documented, rather
  than first in, first out; `RebalanceOptions.lot_method` is optional, and Python's
  `lot_method` defaults to `None`.
- Optimisation options are checked before solving, with every problem reported together at the
  option's name: numbers that are not finite, weights outside 0 to 1, limits no fully invested
  portfolio can meet (six assets at most 10% each), the parameters a method needs, unknown
  assets, and, with the settings of a backtest, its validation and rebalancing. When one method
  is asked for (`optimise`, `backtest`), a parameter it does not use, such as `confidence` with
  maximum Sharpe, or limits with equal weight, is an error; `compare` applies each to the methods
  that take it, as before. Paths name the options (`options.max_weight`), not the optimiser's
  internals (`constraints.bounds`).
- `Assets` settings are checked together: `periods_per_year` above zero, a finite
  `risk_free_rate` above -1 (in Python, a rate of 0 with a risk-free asset is now an error too),
  and an exponentially weighted decay between 0 and 1. The covariance and mean estimators chosen
  are tried on the risky assets' returns when the assets are built, so that a choice the returns
  cannot support is reported at `covariance` or `mean_returns` then.
- The optimisation report's context names the estimator of the expected returns,
  `mean_returns`, in place of `historical_means`; it is absent when expected returns were given.
- An interior-point solution that stops short of the tolerance of 1e-10 is `optimal` when
  its duality gap is within the solver's default tolerance of 1e-8 and its weights meet
  every constraint to 1e-9; it was `nearly_optimal`. Constraints that bind on the boundary of
  a second-order cone often end so.
- Limits in a sentence start in lower case after the first ("at most 40% in any asset; at least 4
  effective assets").
- Report pages are easier to read: muted text has a contrast of at least 4.5:1 in light mode, and
  labels on coloured bars are black or white, whichever reads better on that colour; charts keep
  a legible width on narrow screens and scroll, and wide tables and charts show a shadow at an
  edge with more to see; long tile values keep to one line; growth charts and the portfolio's
  efficient frontier have axis titles. On risk-and-return charts, labels avoid points and each
  other and stay inside the chart, the optimised weights are drawn in the ink rather than an
  asset's colour, and the legend shows the colours of the assets held.
- Text reports wrap long headings rather than cutting them short, number the columns of
  correlation matrices as the rows are, name accounts in the lots table as the accounts table
  does, and say what the HTML says about the in-sample history (rebalanced quarterly, without
  trading costs) and what in sample covers in a comparison.
- `Resampling` no longer implements `Eq`, as it holds a number of periods a year.
- The migration guide describes 1.0 as it is: reports print and show as pages, files are read and
  written where asked, and backtests simulate schedules and holdouts. CONTRIBUTING lists every
  check CI runs and the Python test dependencies, and the Python package is classified as beta.
- Portfolio documents reject unknown fields, such as a misspelt `quantty`, at their path.
- A report grouped by a scheme no instrument is classified by is an error at
  `options.group_by[i]`, naming the schemes there are, rather than a single "unclassified" group.
- Errors raised in Python before the engine is called are `ValidationError`s with paths (still
  `ValueError`s), and messages are clearer: a backtest without a method or weights, `methods`
  given as one name, an account the portfolio does not have, a walk-forward window too long for
  the data, and an optimisation that found no weights now say what is wrong.

### Fixed

- The Sharpe ratio over three years in the comparison and backtest reports measures each window
  against the risk-free return over that window, not over the whole test.
- The rebalance report's value after each trade, and the combined weights after a rebalance of
  several accounts, allow for the fees paid.
- Rebalancing several accounts checks the target weights as given, at `targets.<holding>`; it
  checked them only after spreading them across the accounts, which hid weights above one or
  below zero.
- JavaScript reports NaN and infinite numbers in options at their paths, where JSON had turned
  them into absent options (a `max_weight` of `NaN` meant no limit); Python reports them at their
  paths rather than as a JSON error. `frontier` checks its option names, as `optimise` does.
- Differences shown beside rounded figures (excess returns, changes in the Sharpe ratio, the total
  of fees and tax) are the differences of the figures as shown, so that they add up.
- The portfolio report's correlation columns are headed with the holdings' full names, as the
  rows are ("EUR cash", not "EUR").
- A method that never found weights is marked as holding cash in the comparison tables, is left
  out of the summary's best figures, and its note says that it held only cash, rather than that
  it kept the weights it held. Failures of the fit to the whole history, for the run in sample,
  are reported apart from the refits.
- Concentration in the holdings report is measured on one basis: every share, like the
  Herfindahl index, is of the value invested in instruments, cash aside.
- A held weight too small to show is shown as "<0.1%" (or "<1%"), not as "–" or "not held".
  The text optimisation report lists every held asset, the smaller ones together when there are
  many, and no longer pads its table with assets not held.
- The holdout year that runs across the split is marked in the backtest report's calendar
  years; a portfolio bought and held is described without rebalances; the minimum-variance risk
  note no longer contrasts a weight with an equal share of risk; the covariance estimator is
  named once ("the sample covariance"), and the shading of weights is described without
  reference to the colour scheme.

## [1.0.0-rc.1] - 2026-11-03

### Added

- Backtests: `assets.compare(methods)` tests several optimisation methods side by side under the
  same limits and settings, and `assets.backtest(method)` or `assets.backtest(weights)` tests one
  allocation, optionally against a benchmark (`finalytics::backtest`). Each returns its report and
  every return and trade behind it (`BacktestResults`); `Assets::compare_with` and
  `Assets::backtest_with` run the same backtests from `BacktestOptions`, as JSON gives them.
  Optimisation methods are refitted through time, or fixed weights held, rebalanced monthly,
  quarterly, yearly, when a weight drifts beyond a band, or never, net of trading costs in basis
  points of the value traded. Strategies are validated in sample, on a holdout from a date, or
  walk-forward, refitting at each calendar rebalance on a rolling or expanding window of years or
  periods; for walk-forward validation the in-sample results use the same periods, so the two
  compare directly. In Python the choices are `fl.InSample()`, `fl.Holdout(start=...)`,
  `fl.WalkForward(years=...)` and `fl.Drift(band=...)`, or names such as `"quarterly"`. A refit that finds no weights keeps the weights held and is recorded, and cash,
  held before a first fit, earns the risk-free return. A benchmark is fixed weights held alike or an
  index's prices. The limits on weights apply to the methods that take them, and the results note
  where they did not. Every return, trade, turnover and cost is checked against an independent
  simulation (`scripts/reference/backtest.py`). A walk-forward comparison of seven methods on the
  S&P 500 dataset is a new performance budget: under 5 seconds natively.
- Three reports on assets: the optimisation report of `optimise`, the comparison report of `compare`
  and the backtest report of `backtest`. The optimisation report explains the weights: what they are
  expected to deliver, each asset's weight against its share of the risk, the diversification ratio
  and the number of assets in effect, the correlations, where the weights sit against each asset and
  equal weight, how they and equal weight would have done on the returns they were fitted to, and
  how they were found. It draws the efficient frontier under the same limits when asked
  (`.with_frontier()` in Rust, `frontier=True` in Python, `frontier: true` in JavaScript); this is
  off by default, since the frontier takes 30 more optimisations, and the page says how to turn it
  on. The comparison report scores each method on the periods it was tested on: return, volatility,
  Sharpe and Sortino ratios, drawdown, CVaR and turnover side by side; growth, calendar-year
  returns, drawdowns and the Sharpe ratio over three years; in sample against out of sample; the
  weights at the last refit and at each refit; and how the methods were tested, including refits
  that found no weights. The backtest report shows the allocation, its statistics in and out of
  sample against the benchmark (with tracking error, information ratio and beta), growth and
  drawdown over the whole history with a holdout shaded apart, the weights held as they drift
  between rebalances, turnover and trading costs, and returns by year and month. All three are data
  (`OptimisationReport`, `ComparisonReport`, `BacktestReport`), print as text 80 columns wide, lay
  out as one HTML page with `show()`, and export as tables. Daily returns are charted month by
  month, and the S&P 500's 55 stocks are summarised as the largest holdings and the rest.
- One shape for every result: `optimise`, `compare` and `backtest` on assets, and `rebalance`
  on a portfolio, each return the report and the raw results (`Optimisation`, `Comparison`,
  `Backtest` and `Rebalancing`), as `result.report` and `result.results`. Printing a result
  prints its report as text, and `result.show()` shows it as a page; the raw results print as a
  table too (`OptimisationResult`, `BacktestResults`, `RebalanceResults`). An optimisation's
  weights are `result.results.weights`, and `portfolio.rebalance` takes the whole result in
  Rust, Python and JavaScript; an optimisation that found no weights is a validation error at
  `targets` rather than a target of all cash. `portfolio.with_rebalance` takes the rebalancing
  or its raw results. `Assets::optimise_with` runs an optimisation from `OptimiseOptions`, as
  JSON gives them.
- The rebalance report, built by `portfolio.rebalance`: each holding's weight before, at the
  target and after; the volatility and expected return before and after, with the estimates the
  optimisation of the portfolio's assets uses; each account's trades and currency conversions;
  the fees, the realised gains split into long-term and short-term, and the estimated tax by
  account; and what to check before trading. It is data (`RebalanceReport`), prints 80 columns
  wide, shows as a page and exports as tables, and names the method and limits of the
  optimisation the targets came from. Fees and tax rates are plain numbers in every language:
  `fee_bps`, `fee_per_trade` and `minimum_fee`, the last two in the instrument's currency, and
  `tax` as the rates on short-term and long-term gains, with `long_term_after_days` (365 unless
  given); each is checked when the rebalancing is built, with the other inputs. The Rust builder
  is `RebalanceBuilder`, with `as_of`, `minimum_trade` and `name` too; Python and JavaScript take
  `name`.
- Accounts that are not taxable: `taxable: false` in an account's record (a `taxable` column in
  an accounts file), for a retirement or other tax-deferred or tax-exempt account. Tax-aware
  rebalancing estimates no tax on its sales, which close lots by the lot method. The portfolio
  dataset's retirement account is not taxable. With tax rates, each proposed sale also reports
  the part of its gain from lots held long-term (`long_term_gain_base`).
- Baseline methods: equal weight and inverse volatility (`Objective::EqualWeight`,
  `Objective::InverseVolatility`, `method="equal_weight"` and `"inverse_volatility"`), solved
  by formula, and risk parity by name (`Method::RiskParity`, `method="risk_parity"`), risk
  budgeting with equal budgets. `Method::label` names each method in words, and `SolveStatus`
  displays as it is written in JSON.
- The READMEs show each front door in each language as a program that builds its result and
  shows it: a comparison, a backtest and an optimisation of the asset-class dataset, then the
  portfolio dataset's report and its rebalancing with fees and tax rates. Each program is followed
  by a screenshot of its page, in light and dark, which links to the full sample report. The Rust
  programs are the crate's examples (`cargo run --example compare`, and likewise `backtest`,
  `optimise`, `report` and `rebalance`), and the tests run every README program in each language.
- Sample reports: `scripts/sample-reports.py` builds the five reports the READMEs show from the
  bundled datasets, with an index page and the screenshots, and the "Sample reports" workflow
  publishes them to GitHub Pages on each version tag.
- Builders for every front door in Rust, each ending in `build()`: `Assets::builder()`,
  `assets.optimise(Method::MaximumSharpe).max_weight(0.4)`, `assets.compare([...])`,
  `assets.backtest(...)`, `Portfolio::builder()`, `portfolio.report().benchmark("BENCH")` and
  `portfolio.rebalance(&result).whole_units()`. They take plain text and numbers, with units in
  their names (`max_weight`, `trading_cost_bps`, `Years(5)`), and check everything once, at
  `build()`, reporting every problem together. The option types stay for JSON and for
  `optimise_with`, `report_with` and `rebalance_with`, and `finalytics::prelude` imports what
  the builders need. Identifiers implement `FromStr`, `OptimisationResult::holdings` gives an
  optimisation's weights by holding, and a trade's `Side` displays as `buy` or `sell`.
- Reports show themselves: `report.show()` opens the report's HTML page in the default browser, or
  with the command in the `BROWSER` environment variable; in Python it shows inline in a Jupyter
  notebook, and in JavaScript in a frame inside a page's element (`report.show(element)`), with
  `report.toHtml()` and `report.toText()`. This holds for the portfolio report and for the results
  of `optimise`, `compare` and `backtest`. A report carries a name (`.name(...)` in Rust, `name=` in
  Python, `name` in JavaScript), shown as or under its title. In Rust, `show`, `save_html` and
  reading files by path (`Portfolio::builder()`, `AssetsBuilder::prices_file`) are behind the
  default `io` feature, which the WebAssembly build leaves out, and report `Error::Io` when they
  fail.
- A data guide (`docs/data.md`): every input, column by column, with the dataset files as
  examples and advice on bringing data from brokers and data providers; its examples run as
  tests.
- A guide to migrating from 0.9 (`docs/migrating-from-0.9.md`): what changed, what replaces
  each part of 0.9, and a 0.9 optimisation worked through in 1.0, run as a test.
- Reporting periods are read from their labels by the engine (`Period` implements `FromStr` and
  `Display` in Rust), so Python and JavaScript accept the same labels, now including
  `since 2026-01-31`.
- Two front doors to the library. `Assets` takes the prices or returns of things one might hold
  (a pandas DataFrame, CSV or a `PriceTable`), detects their frequency, annualises them, and
  gives each asset's statistics, the covariance and correlation, and optimisation by method
  name with a largest weight, bounds, per-asset bounds and group limits given by name, such as
  `assets.optimise(method="maximum_sharpe", max_weight=0.4)`; one asset, such as T-bills,
  may serve as the risk-free asset. `Portfolio` holds a portfolio's records, its market data and
  the books kept from them, and gives its valuation, weights, performance, rebalancing (with
  `with_rebalance` to make the trades), all six reports in one call (`report`) and the
  instruments it holds as `Assets`. Rust has builders, Python keyword arguments, and JavaScript
  options objects, in which an unknown option is an error.
- Datasets (`finalytics::datasets`, `fl.datasets`, `datasets` in JavaScript): daily prices
  from 2005 to October 2026 of 55 S&P 500 companies (the five largest in each sector), with
  3-month T-bills, the S&P 500 total return index and each company's sector (`sp500`,
  `sp500_index`, `sp500_companies`); a three-year, two-account household holding five real funds
  at their real prices, in US dollars and euros, with illustrative transactions and the options
  of its full report; and 30 years of synthetic month-end levels of six asset-class indices.
  The market data comes from Yahoo Finance (`scripts/fetch-market-data.py`). Each file is also
  available as text, as an example of the layout Finalytics reads. The S&P 500 prices are behind
  the default `sp500` feature in Rust and are read on first use in JavaScript, outside the
  WebAssembly module.
- A portfolio can be read from its transactions alone: `date` may stand for `trade_date`, `id`
  and `account` may be left out (`t1`, `t2` and so on, and `main`), the instruments file may be
  left out (each instrument takes the currency given with it in the transactions, or the base
  currency), and the base currency may be left out (that of the first deposit).
- `formats::market_from_csv` builds market data from price and FX files, and
  `formats::price_columns_from_csv` reads prices in the order of the file.
- The reports print: every report implements `Display` in Rust as text 80 columns wide, with
  tables, bars and sparklines (`print(report)` in Python, `reportText(report)` in JavaScript), and
  the full report lays out as one self-contained HTML page with the Finalytics logo and an
  optional portfolio name: inline SVG charts with hover details, no scripts or external
  resources, light and dark themes, and print layout (`PortfolioReport::to_html`, Python
  `to_html` and `save_html`, JavaScript `reportHtml`). `Portfolio::report` returns a
  `PortfolioReport`, now in the reports module with `portfolio_report`.
- In Node.js the npm package is ready on import: its `node` entry point loads the WebAssembly
  file, so `init` is needed only in browsers.
- Rust constructors for inputs that gained `#[non_exhaustive]`: `TaxRates::new`,
  `BlackLitterman::new`, `Resampling::new`, `Turnover::new`, `HistoricalWindow::new`,
  `FactorShocks::new`, `ScenarioRiskModel::from_covariance` and `from_factor_model`;
  `TradeLimits` implements `Default`.

### Changed

- The optimiser solves to a tolerance of 1e-10 rather than the solver's default of 1e-8, so that
  a weight that belongs at a bound ends within a few millionths of it rather than up to 1e-4
  away; this matters when weights are refitted and traded repeatedly. Optimised weights change
  by less than 1e-7 in the shared test scenario.
- JavaScript: `Assets.optimise` gives the weights in the order of the assets, as Python does,
  rather than in the order of their names.
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
- Rebalancing leaves room for fees. When the trades and their fees would need more
  base-currency cash than an account has, as when the targets leave no cash, the purchases are
  sized again so that they fit, and a shortfall that rounding leaves comes from the largest
  purchase; before, the proposal took the cash below zero with a warning. Rebalancing the
  portfolio dataset to its least risky mix, 5% to 40% each, in whole units and without fees, now
  buys one unit fewer of EXSA in the brokerage account and of BND and EXSA in the retirement
  account, where whole units had overspent.
- `CombinedRebalance` is `RebalanceResults`, and records the base currency;
  `Portfolio::rebalance_with` and `rebalance_accounts_with` return the rebalancing with its
  report. In Python and JavaScript, `portfolio.rebalance` returns the rebalancing, with each
  account's proposal in `result.results.accounts`.
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

[Unreleased]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.6...HEAD
[1.0.0-rc.6]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.5...v1.0.0-rc.6
[1.0.0-rc.5]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.4...v1.0.0-rc.5
[1.0.0-rc.4]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.3...v1.0.0-rc.4
[1.0.0-rc.3]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.2...v1.0.0-rc.3
[1.0.0-rc.2]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-rc.1...v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/sigmafieldlabs/finalytics/compare/v1.0.0-alpha.0...v1.0.0-rc.1
[1.0.0-alpha.0]: https://github.com/sigmafieldlabs/finalytics/releases/tag/v1.0.0-alpha.0
[0.9.0]: https://github.com/sigmafieldlabs/finalytics/tree/0.9.x
