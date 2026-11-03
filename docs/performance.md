# Performance

Finalytics is single-threaded and runs the same code natively and in WebAssembly. This page sets
out the performance budgets that CI checks, and the measured times of the main operations, so
that you can judge whether the engine fits your workload.

## Budgets

| Operation | Size | Budget | Measured |
| --- | --- | --- | --- |
| Build the books (native) | 100,000 transactions, 2,000 instruments, 10 accounts, 10 years | 2 s | 0.35 s |
| Build the books (WebAssembly, Node.js) | the same portfolio, passed as JavaScript objects | 2 s | 0.86 s |
| Minimum-variance portfolio | 500 assets, bounds and 10 group limits | 0.5 s | 0.36 s |
| Walk-forward comparison | 7 methods on the S&P 500 dataset: 55 stocks, 5,491 daily returns, 5-year window, 67 quarterly refits each | 5 s | 2.26 s |

Each measurement is the median of five runs (three for the comparison). The WebAssembly time includes reading the
100,000 transactions out of JavaScript, as an application would call `new Portfolio(data,
market)`.

The CI job "Performance budgets" fails when a budget is exceeded:

```sh
cargo run --release -p finalytics-bench --bin budgets -- --write-document large.json
bindings/wasm/build.sh
node bindings/wasm/test/budget.mjs large.json
```

## Benchmarks

Median times from `cargo bench -p finalytics-bench`:

| Group | Operation | Time |
| --- | --- | --- |
| Ledger | Build the books, 100,000 transactions | 313 ms |
| Ledger | Value series, 120 month ends | 168 ms |
| Ledger | Holding series, 120 month ends | 295 ms |
| Ledger | Performance from the value series, 120 month ends | 6.9 µs |
| Risk | Return statistics, 1,000 returns | 16 µs |
| Risk | Cornish–Fisher value at risk and expected shortfall, 1,000 returns | 7.4 µs |
| Risk | Ledoit–Wolf covariance, 500 assets by 1,000 returns | 154 ms |
| Risk | Regression factor model, 3 factors, 500 assets by 1,000 returns | 8.2 ms |
| Risk | Statistical factor model, 10 components, 500 assets by 1,000 returns | 459 ms |
| Optimisation | Minimum variance, 500 assets with bounds and groups | 394 ms |
| Optimisation | Maximum Sharpe ratio, 500 assets with bounds and groups | 386 ms |
| Optimisation | Risk parity, 500 assets | 229 ms |
| Optimisation | Hierarchical risk parity, 500 assets | 74 ms |
| Optimisation | Minimum variance, 500 assets with a 10-factor model, bounds and groups | 9.6 ms |
| Optimisation | Maximum Sharpe ratio, 500 assets with a 10-factor model, bounds and groups | 12.7 ms |
| Optimisation | Black–Litterman posterior, 500 assets, 10 views | 51 ms |
| Optimisation | Resampled mean–variance, 50 assets, 100 simulated histories of 120 periods | 201 ms |

The portfolio holds 2,000 instruments in USD, EUR and GBP across ten accounts that use four lot
methods, with deposits, currency conversions, purchases, sales and dividends, daily FX rates and
month-end prices. The returns for the risk and optimisation benchmarks come from a factor model.
Every input is generated from a fixed seed (`tools/bench/src/lib.rs`), so each run measures the
same work.

## Machine

Measured on an Intel Xeon processor at 2.8 GHz (one core of a two-core cloud virtual machine),
Linux, Rust 1.97.0 with the release profile, and Node.js 22. Expect other machines to differ;
compare runs on the same machine when judging a change.

## What the times depend on

- **Books** grow linearly with the number of transactions. Lots are relieved from the account's
  lots in the instrument, so an account with many open lots in one instrument costs more per
  sale.
- **Value and holding series** make one pass over the transactions and value each instrument
  once per date, so they grow with the number of transactions plus the number of instruments
  times the number of dates.
- **Covariance** grows with the square of the number of assets times the number of
  observations.
- **Optimisation** with a full covariance matrix grows roughly with the cube of the number of
  assets, as the solver factorises a dense system at each iteration. With a factor model the
  system stays sparse, and the time grows roughly with the number of assets times the square of
  the number of factors, which is why large problems are much faster in factor form.
  Hierarchical risk parity needs no solver.
- **Resampled optimisation** solves one problem per simulated history, each with a full sample
  covariance matrix, so it takes about as long as that many dense solves.
- **A statistical factor model** needs the eigendecomposition of the covariance matrix, which
  grows with the cube of the number of assets; a regression model needs one least-squares fit per
  asset.
