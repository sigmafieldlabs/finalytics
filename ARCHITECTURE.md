# Architecture

This document is a short map of the code base for contributors.

## Principles

1. **No input or output in the engine.** No network, files, clocks, environment variables or
   global state, and no async runtime. Callers supply data; the engine returns results.
2. **WebAssembly first.** Every library crate builds for `wasm32-unknown-unknown`. Parallelism
   is optional, behind the `parallel` feature.
3. **Exact where money is counted.** Ledger quantities and amounts use decimal arithmetic;
   analytics use `f64` with documented tolerances.
4. **Deterministic.** Results do not depend on hash order, thread scheduling or platform.
5. **Thin bindings.** The Python and WebAssembly layers convert data and add no logic.

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
   ↑
finalytics           the public API, re-exporting what users need
   ↑
bindings/python      PyO3 extension and the Python package
bindings/wasm        wasm-bindgen module and the npm package
```

Arrows point from a crate to the crates that depend on it. A crate never depends on a crate
above it in this list.

## Conventions

- Public items are documented; `missing_docs` is enabled.
- `unsafe` code is forbidden in the library crates.
- Fallible functions return the crate's error type. Input validation collects every problem it
  finds into a report rather than stopping at the first.
- Tests that compare against reference implementations keep their fixtures in the crate's
  `tests/` directory, with the script that produced them.
