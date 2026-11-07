# Performance

Finalytics runs the same code natively and in WebAssembly. Natively, independent pieces of work
run on several threads (see [Threads](#threads)); in WebAssembly the engine runs on one. This page
sets out the performance budgets that CI checks, and the measured times of the main operations,
so that you can judge whether the engine fits your workload.

## Budgets

| Operation | Size | Budget | Measured |
| --- | --- | --- | --- |
| Build the books (native) | 100,000 transactions, 2,000 instruments, 10 accounts, 10 years | 2 s | 0.35 s |
| Build the books (WebAssembly, Node.js) | the same portfolio, passed as JavaScript objects | 2 s | 0.86 s |
| Minimum-variance portfolio | 500 assets, bounds and 10 group limits | 0.5 s | 0.36 s |
| Walk-forward comparison | 7 methods on the S&P 500 prices in `tests/sp500`: 55 stocks, 5,491 daily returns, 5-year window, 67 quarterly refits each, with the optimisation report | 5 s | 2.43 s |
| Screen (native) | every measure on 5,000 securities with up to 10 years of daily closes, volumes and dividends (about 12 million closes), with the screening report | 1 s | 0.26 s |
| Screen (WebAssembly, Node.js) | the same screen, on a universe built once with `Universe.fromColumns` from typed arrays | 1 s | 0.36 s |

Each measurement is the median of five runs (three for the comparison). The WebAssembly time includes reading the
100,000 transactions out of JavaScript, as an application would call `new Portfolio(data,
market)`. The WebAssembly screen includes laying out its results as one `Float64Array` per
measure; building its universe of 12 million closes from typed arrays takes a further 0.43 s.
The two screens were measured on a two-core cloud virtual machine (Intel Xeon at 2.1 GHz), the
native one on one thread.

The CI job "Performance budgets" fails when a budget is exceeded. The measured times are on one
thread; the job itself uses every core of the runner:

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
| Screening | Every measure, 5,000 securities by 10 years of daily closes, with the report | 257 ms |
| Screening | Two rules, a sort and a limit on the same universe | 258 ms |

The screening times were measured on one thread of the two-core machine of the screening
budgets. The portfolio holds 2,000 instruments in USD, EUR and GBP across ten accounts that use
four lot methods, with deposits, currency conversions, purchases, sales and dividends, daily FX
rates and month-end prices. The universe holds 5,000 random walks with volumes and quarterly
dividends, each starting up to a year into the ten. The returns for the risk and optimisation benchmarks come from a factor model.
Every input is generated from a fixed seed (`tools/bench/src/lib.rs`), so each run measures the
same work.

## Threads

With the `parallel` feature, on by default in the `finalytics` crate and in the native Python
package, five kinds of independent work run at once on `rayon`'s thread pool: the strategies of a
comparison or backtest, the walk-forward fits, the histories of resampled optimisation, the
points of an efficient frontier and the securities of a screen. A single optimisation, the books and the reports run on one
thread. Each piece of work is computed on its own and the results are collected in order, so they
are the same to the last bit on any number of threads.

| Operation | One thread | Two threads |
| --- | --- | --- |
| Walk-forward comparison, seven methods on 55 stocks (the budget above) | 1.60 s | 0.90 s |
| Resampled mean–variance, 50 assets, 100 simulated histories of 120 periods | 157 ms | 78 ms |
| Screen of every measure on 5,000 securities by 10 years of daily closes (the budget above) | 261 ms | 147 ms |

Measured on a two-core cloud virtual machine (Intel Xeon at 2.1 GHz), with `RAYON_NUM_THREADS`
set to 1 and 2. Compare the two columns with each other rather than with the tables above, which
were measured on one thread of another machine. On more cores the same work spreads further.

The thread pool is the caller's. By default it is `rayon`'s global pool, which uses every core
unless `RAYON_NUM_THREADS` says otherwise; in Rust, run the engine inside a pool of your own with
`ThreadPool::install`. Without the feature, as in the WebAssembly package and Pyodide, the same
work runs in order on the calling thread. The Python package releases the GIL during the engine's
longer calls, so other Python threads run meanwhile.

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
- **A screen** measures each security in one pass over its own closes, so it grows with the
  number of closes up to the date measured to; the report adds a sort of each measure's values
  for its quartiles. Measures against a benchmark find the benchmark's close on each date in one
  step.
