# Finalytics for Python

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. The engine is written in Rust; this package is a typed Python layer over it.

> **Status:** 1.0 is in active development (`1.0.0a0`).

```sh
pip install finalytics            # with NumPy
pip install "finalytics[pandas]"  # with pandas conversions
```

## A portfolio's books, performance and risk

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
```

## Returns only

Every risk function takes returns as a list, a NumPy array or a pandas Series:

```python
fl.risk.statistics(returns, periods_per_year=12)
fl.risk.drawdowns(prices)              # a pandas Series indexed by date
estimate = fl.risk.covariance(asset_returns, method="ledoit_wolf")   # one column per asset
```

## Optimisation

```python
from finalytics import opt

problem = opt.Problem.from_covariance(
    fl.risk.scaled(estimate.matrix, 252),
    expected_returns=[0.07, 0.04, 0.05],
    constraints=opt.Constraints(bounds=opt.Bound(min=0.05, max=0.6)),
)
solution = problem.solve(opt.MaximumSharpe(risk_free_rate=0.03))
solution.status, solution.weights, solution.max_violation
```

Results are frozen dataclasses with documented fields (see `finalytics.types`). Invalid input
raises `finalytics.ValidationError`, whose `issues` list every problem with where it is.

Wheels are published for Linux, macOS and Windows and work with Python 3.10 and later.

- Source and issues: https://github.com/Nnamdi-sys/finalytics
- Licence: MIT OR Apache-2.0
