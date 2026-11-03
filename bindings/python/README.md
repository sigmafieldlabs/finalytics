<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/finalytics-logo.png" alt="Finalytics" width="420">
</picture>

[![PyPI](https://img.shields.io/pypi/v/finalytics)](https://pypi.org/project/finalytics/)
[![Python 3.10+](https://img.shields.io/badge/python-3.10%2B-blue)](https://pypi.org/project/finalytics/)
[![Downloads](https://static.pepy.tech/badge/finalytics)](https://pepy.tech/projects/finalytics)
[![CI](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sigmafieldlabs/finalytics/actions/workflows/ci.yml)
[![Licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue)](https://github.com/sigmafieldlabs/finalytics#licence)
[![Homepage](https://img.shields.io/badge/homepage-sigmafield.app-0b7a75)](https://sigmafield.app/libraries/finalytics)

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, explains returns against a benchmark, and
optimises portfolios under real-world constraints. The engine is written in Rust; this package is
a typed Python layer over it.

> **Status:** 1.0 is in active development. Until 1.0.0 is released, `pip install finalytics`
> gives 0.9; install a 1.0 pre-release with `pip install --pre finalytics`. Coming from 0.9? See
> the [migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md).

```sh
pip install finalytics            # with NumPy
pip install "finalytics[pandas]"  # with pandas
```

## A portfolio in four steps

Review a portfolio, find where its risk comes from, choose a better mix of the same funds, and
get the trades that reach it, in each account. The portfolio is one of the datasets that come with
the package: three years of a brokerage and a retirement account, in dollars and euros.

```python
import finalytics as fl

# Three years of a two-account portfolio, in dollars and euros.
portfolio = fl.datasets.portfolio()

# 1. Review: every report, from one set of options.
report = portfolio.report(
    benchmark="BENCH",
    group_by=["asset_class", "region"],
    risk_free_rate=0.03,
)
report.save_html("report.html", name="Morgan family portfolio")
since = report.performance.periods[-1]
print(f"Return {since.cumulative:+.1%}, benchmark {since.benchmark:+.1%}")

# 2. Diagnose: which holdings carry the risk?
for part in report.risk.holdings.decomposition.contributions:
    if part.share > 0.10:
        print(f"{part.label}: {part.weight:.0%} of value, "
              f"{part.share:.0%} of risk")

# 3. Decide: the least risky mix of the same funds, 5% to 40% each.
target = portfolio.assets().optimise(
    method="minimum_variance",
    bounds=(0.05, 0.40),
)
for name, mix in [("Now", report.overview.current), ("After", target)]:
    print(f"{name}: volatility {mix.volatility:.1%}, "
          f"expected return {mix.expected_return:.1%}")

# 4. Trade: what each account buys and sells, in whole units.
proposal = portfolio.rebalance(target.weights, whole_units=True)
for account in proposal.accounts:
    for trade in account.trades:
        print(account.account, trade.side, trade.quantity, trade.instrument)
```

It prints:

```text
Return +56.1%, benchmark +55.2%
VTI: 51% of value, 79% of risk
Now: volatility 9.5%, expected return 14.3%
After: volatility 7.5%, expected return 11.2%
brokerage sell 394 VTI
brokerage buy 898 BND
brokerage buy 1024 EXSA
brokerage buy 30 GLD
brokerage buy 200 VNQ
ira sell 259 VTI
ira buy 571 BND
ira buy 651 EXSA
ira buy 19 GLD
ira buy 127 VNQ
```

`report.html` is one self-contained page: every report, with charts, in light and dark, ready to
print. `print(report)` shows the same reports as text, 80 columns wide.

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/report-overview-dark.png">
  <img src="https://raw.githubusercontent.com/sigmafieldlabs/finalytics/main/docs/assets/report-overview.png" alt="The overview of the HTML report: value, returns, volatility, drawdown and value at risk, and the growth of 100 against the benchmark" width="760">
</picture>

Give the report options the benchmark's segments and some scenarios, as
`fl.datasets.report_options()` does, and the report adds attribution against the benchmark and
the effect of each scenario.

## Your own portfolio

A portfolio needs its transactions and the prices that value them:

```python
portfolio = fl.Portfolio.from_csv(
    transactions="transactions.csv",
    prices="prices.csv",
)
```

```csv
date,type,instrument,quantity,price,amount,currency
2026-01-02,deposit,,,,20000,USD
2026-01-02,buy,VOO,20,500,,
2026-01-02,buy,AGG,100,98,,
```

Add `fx=` for other currencies, `instruments=` to classify holdings by asset class or region,
and `accounts=` to name accounts. The
[data guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/data.md) describes every
file, column by column, and `fl.datasets.file("portfolio/transactions.csv")` returns a complete
example of each. Invalid input raises `fl.ValidationError`, which lists every problem with its
file, line and column.

## From prices to a portfolio

`Assets` takes prices, from a pandas DataFrame or a CSV file, for statistics, covariance and
optimisation. Here, the mix of six asset classes with the highest Sharpe ratio, with at most 40%
in any one and 15% in real assets, against the return on T-bills:

```python
import finalytics as fl

# Thirty years of month-end levels of six asset-class indices.
prices = fl.datasets.asset_class_prices()

assets = fl.Assets(prices, risk_free_asset="T-Bills")
result = assets.optimise(
    method="maximum_sharpe",
    bounds=(0.0, 0.4),
    groups={"Real assets": (["Gold", "Real Estate"], 0.0, 0.15)},
)
for asset, weight in result.weights.items():
    print(f"{asset:12} {weight:4.0%}")
print(f"Sharpe ratio {result.sharpe_ratio:.2f}")
```

```text
US Equity     40%
Intl Equity    5%
Bonds         40%
Gold          15%
Real Estate    0%
Sharpe ratio 0.55
```

The other objectives are `minimum_variance`, `mean_variance`, `robust_mean_variance`,
`target_return`, `target_volatility`, `maximum_diversification`, `minimum_cvar`,
`risk_budgeting` and `hierarchical_risk_parity`; `assets.frontier(points=20)` gives the
efficient frontier.

## Beneath the front doors

`fl.risk`, `fl.opt` and `fl.performance` give each method on its own: statistics, covariance
estimators, factor models, risk decomposition, scenarios, Black–Litterman, resampled
optimisation, attribution and more. The
[methods pages](https://github.com/sigmafieldlabs/finalytics/tree/main/docs/methods) define every
model, with examples. Results are frozen dataclasses with documented fields (see
`finalytics.types`).

Wheels are published for Linux, macOS and Windows and work with Python 3.10 and later.

- Source and issues: https://github.com/sigmafieldlabs/finalytics
- Licence: MIT OR Apache-2.0
