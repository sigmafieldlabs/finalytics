# Finalytics for Python

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. The engine is written in Rust; this package is a typed Python layer over it.

> **Status:** 1.0 is in active development. Until 1.0.0 is released, `pip install finalytics`
> gives 0.9; install a 1.0 pre-release with `pip install --pre finalytics`.

```sh
pip install finalytics            # with NumPy
pip install "finalytics[pandas]"  # with pandas conversions
```

## Quick start

A portfolio is its instruments and transactions; with prices, the engine keeps its books and
measures it. Here, a deposit buys two funds at the end of January, and their prices at the end of
each month value the portfolio. CSV can be given as text, as here, or as a path to a file.

```python
>>> import finalytics as fl
>>> portfolio = fl.Portfolio.from_csv(
...     instruments="""\
... id,kind,currency,class.asset_class
... SPY,fund,USD,Equity
... AGG,fund,USD,Bonds
... """,
...     transactions="""\
... id,account,trade_date,type,instrument,quantity,price,amount,currency
... t1,main,2026-01-30,deposit,,,,10000,USD
... t2,main,2026-01-30,buy,SPY,12,500,,
... t3,main,2026-01-30,buy,AGG,40,100,,
... """,
...     base_currency="USD",
... )
>>> market = fl.MarketData.from_csv(prices="""\
... date,SPY,AGG
... 2026-01-30,500,100
... 2026-02-27,520,100.8
... 2026-03-31,490,101.5
... 2026-04-30,530,101
... """)
>>> books = portfolio.books(market)

```

The value at the end of April, in the base currency:

```python
>>> books.valuation("2026-04-30").total_base
Decimal('10400')

```

The time-weighted return since the purchase, each holding's contribution to it, and statistics of
the monthly returns:

```python
>>> performance = books.performance()
>>> round(performance.period("ITD").cumulative, 4)
0.04
>>> [(c.holding.id, round(c.contribution, 4)) for c in performance.contributions()]
[('AGG', 0.004), ('SPY', 0.036), ('USD', 0.0)]
>>> stats = fl.risk.statistics(performance.returns, periods_per_year=12)
>>> round(stats.volatility, 4), round(stats.max_drawdown, 4)
(0.142, -0.0323)

```

The same example is in the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#quick-start) and the
[npm package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#quick-start).

## Books, performance and risk from files

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

- Source and issues: https://github.com/sigmafieldlabs/finalytics
- Licence: MIT OR Apache-2.0
