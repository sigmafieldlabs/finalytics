# Finalytics

Finalytics is a portfolio analytics engine written in Rust, with bindings for Python and for
JavaScript and TypeScript through WebAssembly. It keeps the books of a multi-currency portfolio
from its transactions, measures performance and risk, and optimises portfolios under real-world
constraints.

> **Status:** Finalytics 1.0 is in active development. It is a redesign of the 0.x series; the
> 0.9 release remains available on the `0.9.x` branch.

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

A portfolio is its instruments and transactions; with prices, the engine keeps its books and
measures it. Here, a deposit buys two funds at the end of January, and their prices at the end of
each month value the portfolio:

```rust
use finalytics::formats::{CsvOptions, portfolio_from_csv, prices_from_csv};
use finalytics::{Books, Currency, Date, FlowTiming, HoldingKey, MarketData, Performance};

const INSTRUMENTS: &str = "\
id,kind,currency,class.asset_class
SPY,fund,USD,Equity
AGG,fund,USD,Bonds
";

const TRANSACTIONS: &str = "\
id,account,trade_date,type,instrument,quantity,price,amount,currency
t1,main,2026-01-30,deposit,,,,10000,USD
t2,main,2026-01-30,buy,SPY,12,500,,
t3,main,2026-01-30,buy,AGG,40,100,,
";

const PRICES: &str = "\
date,SPY,AGG
2026-01-30,500,100
2026-02-27,520,100.8
2026-03-31,490,101.5
2026-04-30,530,101
";

fn main() -> Result<(), finalytics::Error> {
    let options = CsvOptions::default();
    let portfolio = portfolio_from_csv(Currency::USD, INSTRUMENTS, TRANSACTIONS, None, options)?;
    let mut market = MarketData::new();
    for (instrument, prices) in prices_from_csv(PRICES, options)? {
        market = market.with_prices(instrument, prices);
    }
    let books = Books::build(&portfolio, &market)?;

    let dates: Vec<Date> = ["2026-01-30", "2026-02-27", "2026-03-31", "2026-04-30"]
        .into_iter()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    let (first, last) = (dates[0], dates[3]);

    // The value at the end of April, in the base currency.
    let valuation = books.value_on(last, &market)?;
    assert_eq!(valuation.total_base.to_string(), "10400");

    // The time-weighted return since the purchase, and each holding's contribution to it.
    let values = books.value_series(&dates, &market)?;
    let performance = Performance::from_values(&values, FlowTiming::StartOfDay)?;
    let twr = performance.time_weighted(first, last).unwrap_or(f64::NAN);
    assert_eq!(format!("{twr:.4}"), "0.0400");

    let holdings = books.holding_series(&dates, &market)?;
    let contributions: Vec<String> = performance
        .contributions(&holdings, first, last)?
        .iter()
        .filter_map(|c| match &c.holding {
            HoldingKey::Instrument(id) => Some(format!("{id} {:.4}", c.contribution)),
            _ => None,
        })
        .collect();
    assert_eq!(contributions, ["AGG 0.0040", "SPY 0.0360"]);
    Ok(())
}
```

The same example is in the READMEs of the
[Python](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/python#quick-start) and
[JavaScript](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#quick-start)
packages. The examples run as tests, so their results are
the engine's own.

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
