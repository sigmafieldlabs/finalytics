# Architecture

This document is a short map of the code base for contributors.

## Principles

1. **No input or output in the engine.** No network, files, environment variables or global
   state, and no async runtime. The only clock is the timer behind the solve time an optimisation
   reports. Callers supply data; the engine returns results.
2. **WebAssembly first.** Every library crate builds for `wasm32-unknown-unknown`. In
   WebAssembly the engine runs on one thread; natively, the `parallel` feature runs independent
   fits and solves on the caller's `rayon` thread pool, with the same results on any number of
   threads.
3. **Exact where money is counted.** Ledger quantities and amounts use decimal arithmetic;
   analytics use `f64` with documented tolerances.
4. **Deterministic.** Results do not depend on hash order or platform; the solve time is the one
   field that varies from run to run.
5. **Thin bindings.** The Python and WebAssembly layers convert data and call the engine. They add
   only what their language needs, such as period labels, default dates and grouping by a
   function the caller passes in; every model is computed in Rust.

## Crates

```
finalytics-core      dates, calendars, currencies, time series, errors, numerical helpers
   ↑
finalytics-ledger    instruments, transactions, corporate actions, tax lots, positions, valuation
   ↑         ↑
   |    finalytics-formats   CSV and JSON import and export, validation reports, portfolio document
   |
finalytics-perf      returns, periods, contribution, benchmark comparison
finalytics-risk      statistics, drawdowns, covariance, VaR and expected shortfall, risk decomposition
   ↑
finalytics-opt       optimisation: convex formulations, risk parity, HRP, frontiers, rebalancing
finalytics-backtest  backtests: strategies held through time, validation, rebalancing, costs
   ↑
finalytics-reports   reports on portfolios and backtests: chart-ready data, text, HTML, tables
   ↑
finalytics           the public API, re-exporting what users need
   ↑
bindings/python      PyO3 extension and the Python package
bindings/wasm        wasm-bindgen module and the npm package
```

Each arrow points from a crate to a crate it depends on. A crate depends only on crates above
it in this list.

The engine crates read and write no files. Input and output live in the `finalytics` facade,
behind its default `io` feature: reading portfolios and prices by path, saving reports and
showing them in a browser. The WebAssembly module leaves the feature out, and the npm package
shows reports in the page instead.

## Data types across languages

The Rust types are the single description of the data the engine reads and returns. With the
`schema` feature, each public type also describes itself as JSON Schema:

```
Rust types ──cargo run -p finalytics-codegen──▶ schema/finalytics.schema.json
                                                schema/portfolio-document.schema.json
schema ──python scripts/generate-types.py──▶ bindings/python/python/finalytics/_types.py
                                           bindings/wasm/types.d.ts
```

The generated Python dataclasses and TypeScript declarations carry the Rust documentation, and
CI fails if any generated file is out of date. Values cross into Python as JSON text and into
JavaScript as plain objects, both in the schema's formats; numeric series cross as `float64`
buffers (NumPy arrays in Python, `Float64Array` in JavaScript). Decimals are written as strings,
so that amounts keep their exact value in every language.

## Conventions

- Public items are documented; `missing_docs` is enabled.
- `unsafe` code is forbidden in the library crates.
- Fallible functions return the crate's error type. Input validation collects every problem it
  finds into a report rather than stopping at the first.
- Tests that compare against reference implementations keep their fixtures in the crate's
  `tests/` directory, with the script that produced them in `scripts/reference/`. The Python
  and JavaScript tests check the same fixtures, and share the scenario in `tests/scenario/`.
