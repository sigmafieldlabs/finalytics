<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/finalytics)](https://crates.io/crates/finalytics)
[![docs.rs](https://img.shields.io/docsrs/finalytics)](https://docs.rs/finalytics)
[![PyPI](https://img.shields.io/pypi/v/finalytics)](https://pypi.org/project/finalytics/)
[![npm](https://img.shields.io/npm/v/finalytics)](https://www.npmjs.com/package/finalytics)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#contributing-and-licence)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange)](https://www.rust-lang.org)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine written in Rust, with bindings for
Python and JavaScript. It compares portfolio optimisation methods on returns
they have not seen, keeps the books of a real portfolio to report its
performance, risk and attribution, tests a plan's chance of meeting a goal, and
screens thousands of securities by rules on their measures.

| Language | Package | Install |
| --- | --- | --- |
| Rust | [`finalytics`](https://crates.io/crates/finalytics) | `cargo add finalytics` |
| Python 3.10+ | [`finalytics`](https://pypi.org/project/finalytics/) | `pip install finalytics` |
| JavaScript and TypeScript | [`finalytics`](https://www.npmjs.com/package/finalytics) | `npm install finalytics` |

Until 1.0.0 is released, these commands install the 0.x series. Install a 1.0
pre-release with `cargo add finalytics@<version>`, `pip install --pre finalytics`
or `npm install finalytics@next`. The
[migration guide](docs/migrating-from-0.9.md) explains what replaces each part
of 0.9.

**Sample reports**, built by the programs below from the bundled datasets and,
for screening, the S&P 500 prices in `tests/sp500`:
[comparison](https://sigmafieldlabs.github.io/finalytics/reports/comparison.html) ·
[backtest](https://sigmafieldlabs.github.io/finalytics/reports/backtest.html) ·
[optimisation](https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html) ·
[portfolio](https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html) ·
[rebalance](https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html) ·
[planning](https://sigmafieldlabs.github.io/finalytics/reports/planning.html) ·
[screening](https://sigmafieldlabs.github.io/finalytics/reports/screening.html)

## Portfolio optimisation

Compare optimisation methods on the same assets. Each method is refitted every
quarter on the five years before, and judged on the quarter that follows.

```rust
//! Six optimisation methods compared walk-forward on 30 years of six asset
//! classes.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .risk_free_asset("T-Bills")
        .build()?;

    let result = assets
        .compare([
            Method::EqualWeight,
            Method::MinimumVariance,
            Method::MaximumSharpe,
            Method::MaximumDiversification,
            Method::RiskParity,
            Method::HierarchicalRiskParity,
        ])
        .max_weight(0.40)
        .rebalance(Rebalance::Quarterly)
        .walk_forward(Years(5))
        .trading_cost_bps(10.0)
        .build()?;

    result.show()?;
    Ok(())
}
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/comparison.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/comparison-light.png" alt="The comparison report: each method's return, volatility, Sharpe ratio, drawdown and turnover out of sample" width="760"></a>

Every result has the same shape: `result.report` is the report, which
`println!("{result}")` prints as text and `result.show()` opens as a page, and
`result.results` holds every return and trade behind it.

To test one allocation, give a method or your own weights. Here, the maximum
Sharpe ratio fitted to the years before November 2016, held from then on against
a 60/40 benchmark:

```rust
let result = assets
    .backtest(Method::MaximumSharpe)
    .max_weight(0.40)
    .benchmark([("US Equity", 0.60), ("Bonds", 0.40)])
    .rebalance(Rebalance::Quarterly)
    .holdout_from("2016-11-01")
    .build()?;

result.show()?;
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/backtest.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/backtest-light.png" alt="The backtest report: the allocation, its statistics in and out of sample against the benchmark, and its growth" width="760"></a>

To see the weights a method chooses today, where their risk comes from and
where they sit on the efficient frontier:

```rust
let result = assets
    .optimise(Method::MaximumSharpe)
    .max_weight(0.40)
    .with_frontier()
    .build()?;

result.show()?;
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/optimisation-light.png" alt="The optimisation report: the weights, what they are expected to deliver, and each asset's share of the risk" width="760"></a>

## Portfolio analytics

Load a portfolio from its transactions and prices, and report on it: holdings,
performance, risk, attribution and scenarios.

```rust
//! A portfolio from its CSV files, reported on against its benchmark. Run it
//! in a folder that holds the files.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let portfolio = Portfolio::builder()
        .transactions("transactions.csv")
        .prices("prices.csv")
        .fx("fx.csv")
        .instruments("instruments.csv")
        .accounts("accounts.csv")
        .build()?;

    let report = portfolio
        .report()
        .benchmark("BENCH")
        .group_by(["asset_class", "region"])
        .build()?;

    report.show()?;
    Ok(())
}
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/portfolio-light.png" alt="The portfolio report: value, returns, volatility, drawdown and value at risk, and growth against the benchmark" width="760"></a>

The files are the portfolio dataset: a household's brokerage and retirement
accounts, holding five funds in dollars and euros for three years.
`finalytics::datasets::file("portfolio/prices.csv")` and the like return each
of them, and `datasets::portfolio()` loads the same portfolio without them.
`BENCH` is a column of `prices.csv`, the benchmark's levels; `asset_class` and
`region` are columns of `instruments.csv` that classify each holding. The sample
report adds the attribution and scenarios of `datasets::report_options()`. A
portfolio's report is its own result: every figure in it is the portfolio's, so
`report()` returns the report, with no `results` beside it.

<details>
<summary>The start of the dataset's files</summary>

```csv
id,kind,currency,name,class.asset_class,class.region
VTI,fund,USD,Vanguard Total Stock Market ETF,Equity,North America
EXSA,fund,EUR,iShares STOXX Europe 600 UCITS ETF (DE),Equity,Europe
```

```csv
date,VTI,EXSA,BND,GLD,VNQ,BENCH
2023-11-01,208.86,43.21,68.99,183.51,73.37,100.0000
2023-11-02,213.01,43.92,69.39,184.12,75.78,101.4345
```

```csv
id,name,lot_method,taxable,class.owner,class.wrapper
brokerage,Brokerage account,fifo,true,Alex,Taxable
ira,Retirement account,fifo,false,Alex,Tax-deferred
```

The [data guide](docs/data.md) describes every file, column by column.

</details>

To move the same funds to their least risky mix, 5% to 40% each, optimise the
holdings and rebalance the accounts together, with fees and tax rates:

```rust
let target = portfolio
    .assets()
    .build()?
    .optimise(Method::MinimumVariance)
    .bounds(0.05, 0.40)
    .build()?;

let result = portfolio
    .rebalance(&target)
    .whole_units()
    .fee_bps(5.0)
    .minimum_fee(1.0)
    .tax(0.37, 0.20)
    .build()?;

result.show()?;
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/rebalance-light.png" alt="The rebalance report: the trades, how each holding's weight moves, and what the trades change and cost" width="760"></a>

Sales in the brokerage account take the lots that cost the least tax; the
retirement account is not taxable (`taxable` is false in `accounts.csv`), so its
sales pay none. `portfolio.with_rebalance(&result)` makes the trades.

## Financial Planning

Test a plan of saving, spending and investing against one goal: an amount by a
date, such as a house deposit, or payments over a period, such as an income in
retirement. The plan runs month by month through 10,000 markets drawn from the
assets' monthly returns, in today's money: returns are after inflation and
fees.

```rust
//! A retirement plan: saving until November 2046, then an income of $4,000 a
//! month for 30 years, in today's money.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let assets = Assets::builder()
        .prices(datasets::asset_class_prices())
        .build()?;

    let plan = assets
        .plan()
        .payments(4_000.0, "2046-11", "2076-10")
        .name("Retirement income")
        .chance(0.85)
        .start_value(250_000.0)
        .saving(1_500.0, Frequency::Month)
        .weights([("US Equity", 0.60), ("Bonds", 0.40)])
        .mix_name("Balanced 60/40")
        .alternative("Cautious 30/70", [("US Equity", 0.30), ("Bonds", 0.70)])
        .alternative("Growth 80/20", [("US Equity", 0.80), ("Bonds", 0.20)])
        .inflation(0.025)
        .fee(0.0025)
        .currency("USD")
        .build()?;

    plan.show()?;
    Ok(())
}
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/planning.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/planning-light.png" alt="The planning report: the plan, its chance of meeting the goal, and the money over time in simulated markets" width="760"></a>

The report gives the chance of meeting the goal against the chance wanted, how
the money could grow, and the smallest change that would reach that chance:
saving more, starting with more, starting the payments later or taking a
little less. It tests the plan when returns are lower and when markets crash as
the payments start, and compares other mixes on the same simulations.
`plan.results` holds the figures behind it, with the value over time.

For an amount by a date, give `.target(150_000.0, "2030-06")`; for a single
payment out along the way, `.spending_on("Car", 25_000.0, "2028-03")`; and for
inflation month by month, a price index of one column, such as a consumer price
index, `.inflation_index(index)`.

## Screening

Screen many securities at once, each measured on its own history: returns over
calendar windows, risk, measures against a benchmark, technical indicators,
income and volume. Rules test the measures and any fields given with the
securities, such as a sector or a price-to-earnings ratio.

```rust
//! The 55 S&P 500 companies of tests/sp500 screened on return, risk, trend and
//! sector. Run it at the root of the repository.

use finalytics::prelude::*;

fn main() -> Result<()> {
    let universe = Universe::builder()
        .prices_file("tests/sp500/prices.csv")
        .fields_file("tests/sp500/companies.csv")
        .risk_free_rate(0.04)
        .build()?;

    let screen = universe
        .screen()
        .above("return_1y", 0.0)
        .below("volatility", 0.30)
        .above("price_to_sma_200", 1.0)
        .between("rsi_14", 30.0, 70.0)
        .above("max_drawdown_3y", -0.30)
        .none_of("sector", ["Utilities"])
        .sort("sharpe_ratio_3y", Order::Descending)
        .limit(10)
        .build()?;

    screen.show()?;
    Ok(())
}
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/screening.html"><img src="https://sigmafieldlabs.github.io/finalytics/images/screening-light.png" alt="The screening report: how the rules narrow the list, the matches in order and the near misses" width="760"></a>

The report shows how the rules narrow the list, the matches in order, the
near misses and where each rule's threshold falls across the universe.

## What is included

**Optimisation**

- Equal weight, minimum variance, mean–variance and robust mean–variance,
  maximum Sharpe, target return or volatility, maximum diversification, minimum
  CVaR, risk parity, hierarchical risk parity
- Limits on each asset and on groups, turnover, tracking error and factor
  exposure
- Black–Litterman and resampled estimates; Ledoit–Wolf, OAS and EWMA covariance
- Validation in sample, on a holdout, or walk-forward; calendar or drift
  rebalancing; trading costs

**Analytics**

- Books from transactions: lots, corporate actions, several accounts and
  currencies
- Time- and money-weighted returns, contribution, Brinson–Fachler and currency
  attribution
- Volatility, drawdowns, value at risk, risk decomposition, factor models and
  scenarios
- Rebalancing trades with fees and tax-aware lot selection, across accounts

**Planning**

- The chance of a goal, an amount by a date or payments over a period, from
  simulated markets: block bootstrap, normal or historical
- Saving and spending once or regularly, fixed or glide-path mixes, fees and
  inflation, in today's money
- The changes that reach the chance wanted, lower returns and a crash, and
  other mixes on the same simulations

**Screening**

- Thousands of securities, each on its own dates, with volumes, dividends and
  fields such as a sector or a valuation
- Returns over calendar windows, volatility, drawdowns, Sharpe and Sortino
  ratios, value at risk, beta and relative returns, moving averages, RSI and
  MACD, dividend yield and volume
- Rules on the measures and on the fields, sorted and limited

Every report prints as text, 80 columns wide, and shows as one self-contained
HTML page, in light and dark. Every model is tested against an independent
implementation; the [methods pages](docs/methods) define each one.

## Documentation

[API reference](https://docs.rs/finalytics) ·
[Python package](bindings/python#readme) ·
[JavaScript package](bindings/wasm#readme) ·
[Data guide](docs/data.md) ·
[Methods](docs/methods) ·
[Performance](docs/performance.md) ·
[Architecture](ARCHITECTURE.md) ·
[Migrating from 0.9](docs/migrating-from-0.9.md)

The programs in this README are the crate's
[examples](crates/finalytics/examples), and the tests run them.

## Contributing and licence

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the
[Code of Conduct](CODE_OF_CONDUCT.md) first. To report a security issue, see
[SECURITY.md](SECURITY.md).

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or
the [MIT licence](LICENSE-MIT), at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this
project, as defined in the Apache-2.0 licence, shall be dual licensed as above,
without any additional terms or conditions.

Finalytics provides analysis tools only. It is not investment advice.
