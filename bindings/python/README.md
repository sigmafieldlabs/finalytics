<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![PyPI](https://img.shields.io/pypi/v/finalytics)](https://pypi.org/project/finalytics/)
[![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-blue)](https://pypi.org/project/finalytics/)
[![Downloads](https://static.pepy.tech/badge/finalytics)](https://pepy.tech/projects/finalytics)
[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#contributing-and-licence)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine written in Rust, with this typed
Python package over it. It compares portfolio optimisation methods on returns
they have not seen, and keeps the books of a real portfolio to report its
performance, risk and attribution.

```sh
pip install finalytics            # with NumPy
pip install "finalytics[pandas]"  # with pandas
```

Until 1.0.0 is released, `pip install finalytics` gives 0.9; install a 1.0
pre-release with `pip install --pre finalytics`. The
[migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md) explains what
replaces each part of 0.9.

**Sample reports**, built by the programs below from the bundled datasets:
[comparison](https://sigmafieldlabs.github.io/finalytics/reports/comparison.html) ·
[backtest](https://sigmafieldlabs.github.io/finalytics/reports/backtest.html) ·
[optimisation](https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html) ·
[portfolio](https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html) ·
[rebalance](https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html)

## Portfolio optimisation

Compare optimisation methods on the same assets. Each method is refitted every
quarter on the five years before, and judged on the quarter that follows.

```python
import finalytics as fl

assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")

result = assets.compare(
    methods=[
        "equal_weight",
        "minimum_variance",
        "maximum_sharpe",
        "maximum_diversification",
        "risk_parity",
        "hierarchical_risk_parity",
    ],
    max_weight=0.40,
    rebalance="quarterly",
    validation=fl.WalkForward(years=5),
    trading_cost_bps=10,
)

result.show()
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/comparison.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/comparison-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/comparison-light.png" alt="The comparison report: each method's return, volatility, Sharpe ratio, drawdown and turnover out of sample" width="760">
</picture></a>

Every result has the same shape: `result.report` is the report, which
`print(result)` prints as text and `result.show()` opens as a page (inline in a
Jupyter notebook), and `result.results` holds every return and trade behind it.

To test one allocation, give a method or your own weights. Here, the maximum
Sharpe ratio fitted to the years before November 2016, held from then on against
a 60/40 benchmark:

```python
result = assets.backtest(
    method="maximum_sharpe",
    max_weight=0.40,
    benchmark={"US Equity": 0.60, "Bonds": 0.40},
    rebalance="quarterly",
    validation=fl.Holdout(start="2016-11-01"),
)

result.show()
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/backtest.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/backtest-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/backtest-light.png" alt="The backtest report: the allocation, its statistics in and out of sample against the benchmark, and its growth" width="760">
</picture></a>

To see the weights a method chooses today, where their risk comes from and
where they sit on the efficient frontier:

```python
result = assets.optimise(
    method="maximum_sharpe", max_weight=0.40, frontier=True
)

result.show()
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/optimisation.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/optimisation-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/optimisation-light.png" alt="The optimisation report: the weights, what they are expected to deliver, and each asset's share of the risk" width="760">
</picture></a>

## Portfolio analytics

Load a portfolio from its transactions and prices, and report on it: holdings,
performance, risk, attribution and scenarios.

```python
import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    transactions="transactions.csv",
    prices="prices.csv",
    fx="fx.csv",
    instruments="instruments.csv",
    accounts="accounts.csv",
)

report = portfolio.report(benchmark="BENCH", group_by=["asset_class", "region"])

report.show()
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/portfolio.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/portfolio-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/portfolio-light.png" alt="The portfolio report: value, returns, volatility, drawdown and value at risk, and growth against the benchmark" width="760">
</picture></a>

The files are the portfolio dataset: a household's brokerage and retirement
accounts, holding five funds in dollars and euros for three years.
`fl.datasets.file("portfolio/prices.csv")` and the like return each of them, and
`fl.datasets.portfolio()` loads the same portfolio without them. `BENCH` is a
column of `prices.csv`, the benchmark's levels; `asset_class` and `region` are
columns of `instruments.csv` that classify each holding. The sample report adds
the attribution and scenarios of `fl.datasets.report_options()`.

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

The [data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) describes every file, column by
column.

</details>

To move the same funds to their least risky mix, 5% to 40% each, optimise the
holdings and rebalance the accounts together, with fees and tax rates:

```python
target = portfolio.assets().optimise(
    method="minimum_variance", bounds=(0.05, 0.40)
)

result = portfolio.rebalance(
    target, whole_units=True, fee_bps=5, minimum_fee=1, tax=(0.37, 0.20)
)

result.show()
```

<a href="https://sigmafieldlabs.github.io/finalytics/reports/rebalance.html"><picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://sigmafieldlabs.github.io/finalytics/images/rebalance-dark.png">
  <img src="https://sigmafieldlabs.github.io/finalytics/images/rebalance-light.png" alt="The rebalance report: the trades, how each holding's weight moves, and what the trades change and cost" width="760">
</picture></a>

Sales in the brokerage account take the lots that cost the least tax; the
retirement account is not taxable (`taxable` is false in `accounts.csv`), so its
sales pay none. `portfolio.with_rebalance(result)` makes the trades.

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

Every report prints as text, 80 columns wide, and shows as one self-contained
HTML page, in light and dark. Every model is tested against an independent
implementation; the [methods pages](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) define each
one.

Beneath the front doors, `fl.risk`, `fl.opt` and `fl.performance` give each
method on its own, and results are frozen dataclasses with documented fields
(see `finalytics.types`). Wheels are published for Linux, macOS and Windows.

## Documentation

[Data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) ·
[Methods](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) ·
[Performance](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/performance.md) ·
[Migrating from 0.9](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md) ·
[Source and issues](https://github.com/sigmafieldlabs/finalytics)

## Contributing and licence

Contributions are welcome: see
[CONTRIBUTING.md](https://github.com/sigmafieldlabs/finalytics/blob/main/CONTRIBUTING.md). Finalytics is licensed under
[MIT](https://github.com/sigmafieldlabs/finalytics/blob/main/LICENSE-MIT) or [Apache-2.0](https://github.com/sigmafieldlabs/finalytics/blob/main/LICENSE-APACHE),
at your option. It provides analysis tools only and is not investment advice.
