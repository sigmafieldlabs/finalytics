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
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. The engine is written in Rust; this package is a typed Python layer over it.

> **Status:** 1.0 is in active development. Until 1.0.0 is released, `pip install finalytics`
> gives 0.9; install a 1.0 pre-release with `pip install --pre finalytics`. Coming from 0.9? See
> the [migration guide](https://github.com/sigmafieldlabs/finalytics/blob/main/docs/migrating-from-0.9.md).

```sh
pip install finalytics            # with NumPy
pip install "finalytics[pandas]"  # with pandas conversions
```

## Quick start

Estimate risk and return from a year of monthly returns, then find the portfolio with the highest
Sharpe ratio that holds between 10% and 60% of each asset: This example uses pandas
(`pip install "finalytics[pandas]"`); NumPy arrays work as well.

```python
>>> import pandas as pd
>>> import finalytics as fl
>>> from finalytics import opt
>>> returns = pd.DataFrame({  # twelve months of returns for three assets
...     "Equities": [-0.028, 0.021, -0.044, 0.043, -0.021, 0.020, 0.055, -0.028, -0.009, 0.017, 0.044, 0.014],
...     "Bonds":    [-0.009, 0.020, 0.002, 0.021, -0.013, 0.005, 0.014, 0.002, -0.001, -0.011, -0.028, 0.032],
...     "Gold":     [-0.013, -0.029, 0.004, 0.069, -0.009, 0.000, 0.017, 0.005, -0.030, 0.003, 0.057, -0.001],
... })
>>> estimate = fl.risk.covariance(returns, method="ledoit_wolf")
>>> problem = opt.Problem.from_covariance(
...     fl.risk.scaled(estimate.matrix, 12),  # annualised
...     expected_returns=returns.mean() * 12,
...     constraints=opt.Constraints(bounds=opt.Bound(min=0.1, max=0.6)),
... )
>>> solution = problem.solve(opt.MaximumSharpe(risk_free_rate=0.02))
>>> dict(zip(solution.labels, [round(w, 3) for w in solution.weights]))
{'Equities': 0.444, 'Bonds': 0.18, 'Gold': 0.375}
>>> round(solution.expected_return, 4), round(solution.volatility, 4)
(0.0709, 0.067)

```

The same example is in the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#quick-start) and the
[npm package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#quick-start), and it runs as a test, so the
results shown are the engine's.

## Books, performance and risk

```python
import finalytics as fl

portfolio = fl.Portfolio.from_csv(
    instruments="instruments.csv", transactions="transactions.csv", base_currency="USD"
)
market = fl.MarketData.from_csv(prices="prices.csv", fx="fx.csv")
books = portfolio.books(market)

books.valuation("2026-10-30")          # positions and cash, valued in local and base currency
performance = books.performance()      # daily time-weighted returns
performance.period("YTD")              # cumulative and, for a year or more, annualised
performance.contributions()            # by holding; they add up to the return

fl.risk.statistics(performance.returns, risk_free_rate=0.03)
fl.risk.tail_risk(performance.returns, confidence=0.99, method="cornish_fisher")
fl.reports.performance(books)          # everything a performance screen shows, ready for charts
```

Results are frozen dataclasses with documented fields (see `finalytics.types`). Invalid input
raises `finalytics.ValidationError`, whose `issues` list every problem with where it is.

Wheels are published for Linux, macOS and Windows and work with Python 3.10 and later.

- Source and issues: https://github.com/sigmafieldlabs/finalytics
- Licence: MIT OR Apache-2.0
