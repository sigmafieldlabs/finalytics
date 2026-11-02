<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/finalytics)](https://crates.io/crates/finalytics)
[![docs.rs](https://img.shields.io/docsrs/finalytics)](https://docs.rs/finalytics)
[![PyPI](https://img.shields.io/pypi/v/finalytics)](https://pypi.org/project/finalytics/)
[![npm](https://img.shields.io/npm/v/finalytics)](https://www.npmjs.com/package/finalytics)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#licence)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange)](https://www.rust-lang.org)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine written in Rust, with bindings for Python and for
JavaScript and TypeScript through WebAssembly. It keeps the books of a multi-currency portfolio
from its transactions, measures performance and risk, and optimises portfolios under real-world
constraints.

> **Status:** Finalytics 1.0 is in active development. It is a redesign of the 0.x series; the
> 0.9 release remains available on the `0.9.x` branch, and the
> [migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md) explains what replaces each part of it.

## Design

- **A pure engine.** Finalytics performs no input or output. You supply instruments,
  transactions, prices and FX rates; it returns typed results. The same inputs give the same
  outputs on every platform.
- **Accounting first.** Tax lots, corporate actions and cash in several currencies are part of
  the data model, with exact decimal arithmetic in the ledger.
- **Runs anywhere.** The core compiles to WebAssembly and runs in the browser, in Node and in
  Python, from one Rust code base.
- **Validated.** Every model is tested against independent reference implementations, and its
  methods and tolerances are documented in [docs/methods](docs/methods).
- **Fast.** The books for 100,000 transactions build in under a second, natively and in
  WebAssembly, and a 500-asset optimisation solves in under half a second; see
  [docs/performance.md](docs/performance.md).

## Scope of 1.0

| Area | Includes |
| --- | --- |
| Data | Import and validation of accounts, instruments, transactions, prices, FX rates and benchmarks (CSV and JSON); a versioned portfolio document |
| Holdings | Positions and tax lots on any date; cost basis; realised and unrealised gains in local and base currency; weights by any classification |
| Performance | Time-weighted and money-weighted returns, standard periods, contribution, benchmark-relative returns, Brinson–Fachler and currency attribution; households: performance and contribution by account and by group of accounts |
| Risk | Return statistics, drawdowns, covariance estimators, factor models (regression, statistical and fundamental), value at risk and expected shortfall, risk decomposition by holding, group and factor, ex-ante tracking error; scenarios (shocks to instruments, groups, currencies and factors, historical replay) with their effect on value and risk, including stressed correlations |
| Optimisation | Minimum variance, mean–variance, maximum Sharpe, target return or volatility, maximum diversification, minimum CVaR, risk parity, hierarchical risk parity, robust mean–variance; Black–Litterman returns; resampled optimisation; budget, bound, group, turnover, tracking-error, factor exposure and trade constraints; linear and quadratic trading costs; factor models in factor form; efficient frontiers; rebalancing trades with fees and tax-aware lot selection, for one account or several together |
| Reports | Holdings, overview, performance, risk, attribution and scenario reports with chart-ready series, breakdowns, grids and histograms; provenance with a digest of the inputs; typed tables for export |

## Packages

| Language | Package | Install |
| --- | --- | --- |
| Rust | [`finalytics`](https://crates.io/crates/finalytics) | `cargo add finalytics` |
| Python (3.10+) | [`finalytics`](https://pypi.org/project/finalytics/) | `pip install finalytics` |
| JavaScript / TypeScript | [`finalytics`](https://www.npmjs.com/package/finalytics) | `npm install finalytics` |

Until 1.0.0 is released, these commands install the 0.x series (0.9 on crates.io and PyPI, 0.2 on
npm). Install a 1.0 pre-release with `cargo add finalytics@<version>`, `pip install --pre
finalytics` or `npm install finalytics@next`.

## Quick start

Estimate risk and return from a year of monthly returns, then find the portfolio with the highest
Sharpe ratio that holds between 10% and 60% of each asset:

```rust
use finalytics::opt::{Bound, Constraints, Objective, Problem};
use finalytics::risk::{CovarianceMethod, ReturnsTable, covariance};

fn main() -> Result<(), finalytics::Error> {
    // Twelve months of returns for three assets.
    let returns = ReturnsTable::new(
        vec!["Equities".into(), "Bonds".into(), "Gold".into()],
        vec![
            vec![-0.028, 0.021, -0.044, 0.043, -0.021, 0.020, 0.055, -0.028, -0.009, 0.017, 0.044, 0.014],
            vec![-0.009, 0.020, 0.002, 0.021, -0.013, 0.005, 0.014, 0.002, -0.001, -0.011, -0.028, 0.032],
            vec![-0.013, -0.029, 0.004, 0.069, -0.009, 0.000, 0.017, 0.005, -0.030, 0.003, 0.057, -0.001],
        ],
    )?;

    // Annualised estimates of risk and return.
    let estimate = covariance(&returns, CovarianceMethod::LedoitWolf)?;
    let expected = returns.columns().iter().map(|r| 12.0 * r.iter().sum::<f64>() / r.len() as f64).collect();

    // The portfolio with the highest Sharpe ratio, holding 10% to 60% of each asset.
    let mut constraints = Constraints::default();
    constraints.bounds = Bound { min: 0.1, max: 0.6 };
    let solution = Problem::new(estimate.matrix.scaled(12.0))
        .with_expected_returns(expected)
        .with_constraints(constraints)
        .solve(&Objective::MaximumSharpe { risk_free_rate: 0.02 })?;

    let weights: Vec<String> = solution.labels.iter().zip(solution.weights.unwrap_or_default())
        .map(|(asset, w)| format!("{asset} {w:.3}")).collect();
    assert_eq!(weights, ["Equities 0.444", "Bonds 0.180", "Gold 0.375"]);
    Ok(())
}
```

The same example is in the READMEs of the
[Python](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/python#quick-start) and
[JavaScript](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#quick-start) packages, and it runs as a test, so
the weights shown are the engine's. The [methods pages](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) define
every model, with more examples.

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/finalytics` | The public Rust API |
| `crates/finalytics-*` | The layers behind it: core, ledger, formats, performance, risk, optimisation, reports |
| `bindings/python` | The Python package |
| `bindings/wasm` | The WebAssembly module and npm package |
| `schema` | JSON Schema of the data types, including the portfolio document |
| `docs/methods` | How each model is defined and validated |
| `docs/performance.md` | Performance budgets and benchmark results |
| `tools` | Benchmarks, fuzz checks and the schema generator |
| `fuzz` | Fuzz targets and their seed corpus |

See [ARCHITECTURE.md](ARCHITECTURE.md) for how the pieces fit together.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md) first. To report a security issue, see
[SECURITY.md](SECURITY.md).

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT licence ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project, as defined in the Apache-2.0 licence, shall be dual licensed as above, without any
additional terms or conditions.

Finalytics provides analysis tools only. It is not investment advice.
