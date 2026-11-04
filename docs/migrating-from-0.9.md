# Migrating from 0.9

Finalytics 1.0 is a redesign. It keeps the purpose of 0.9, portfolio analytics and optimisation,
and changes where the data comes from and what the library returns. This guide lists what changed,
what replaces each part of 0.9, and works one 0.9 example through in 1.0.

0.9 remains available: see [Staying on 0.9](#staying-on-09).

## What changed

- **You supply the data.** 0.9 downloaded prices, financial statements, options and news from
  Yahoo Finance and other websites. 1.0 downloads nothing: you pass in prices, and FX rates for
  other currencies, from any source you are entitled to use, as CSV text or files, JSON,
  mappings, a pandas DataFrame or Series, NumPy arrays or JavaScript arrays. This was the "custom
  data" path of 0.9 (`KLINE`); in 1.0 it is the only one. One price per instrument and date is
  enough, such as the close or the adjusted close. The files it touches are the ones you name: it
  reads CSV and portfolio files you point it to, and `save_html` and `show` write a report's page.
- **A report and the numbers behind it.** 0.9 drew charts and HTML reports
  (`report(...).show()`). In 1.0 every optimisation, comparison, backtest, rebalance and
  portfolio report prints as text, 80 columns wide, and lays out as one self-contained HTML page
  with its charts: `print(result)`, `result.show()`, `result.save_html(path)`. The report is also
  data, in `result.report`, with the series, breakdowns, grids and tables a screen draws, and the
  raw results are in `result.results`, for charts of your own.
- **Two front doors.** `Assets` takes the prices or returns of things one might hold, for
  statistics and optimisation, much as 0.9's `Portfolio` did. A 1.0 `Portfolio` is kept from
  transactions, as books with tax lots and several currencies, and measures performance, risk and
  attribution from them. The starting weights and scheduled cash flows of 0.9 become transactions.
- **Packages.**
  - Rust: a workspace of crates behind the `finalytics` crate, with no async runtime and no
    Polars; the minimum supported Rust version is 1.88.
  - Python: NumPy is the only required dependency (0.9 required Polars, PyArrow, Plotly and
    IPython), pandas is optional (`pip install "finalytics[pandas]"`), and Python 3.10 or later is
    needed.
  - JavaScript: the npm package is the engine compiled to WebAssembly, for browsers and Node.js;
    the Node.js native binding of 0.2 is gone.
  - Go and the C interface are no longer provided.
- **Licence:** MIT OR Apache-2.0, at your option (0.9: MIT).

## What replaces what

| 0.9 | 1.0 |
| --- | --- |
| `Screener`; the financials, options and news reports of `Ticker`; technical indicators | Removed, with no equivalent in the library |
| Downloading prices (`Ticker`, `Tickers` or `Portfolio` with dates and an interval) | Load prices yourself and pass them in: `Assets(prices)` from a pandas DataFrame or CSV, `Portfolio.from_csv(transactions=..., prices=...)` |
| `KLINE` (open, high, low, close, volume) | One price series per instrument: a column per instrument in a CSV file, or a pandas Series |
| `performance_stats()`; the performance report of `Ticker` | `Assets(prices).statistics()`; `risk.statistics` and `risk.relative_statistics` on returns; `portfolio.performance()` for a portfolio kept from transactions |
| `objective_function="max_sharpe"` | `assets.optimise(method="maximum_sharpe")`, with `risk_free_rate` given to `Assets` |
| `"min_vol"`, `"min_var"` | `method="minimum_variance"`; both have the same optimum |
| `"min_cvar"` | `method="minimum_cvar"`, with the historical returns as scenarios |
| `"risk_parity"` | `method="risk_parity"`; `"risk_budgeting"` with `budgets` for unequal shares |
| `"max_diversification"` | `method="maximum_diversification"` |
| `"hierarchical_risk_parity"` | `method="hierarchical_risk_parity"` |
| `"max_return"`, `"max_sortino"`, `"min_drawdown"` | No direct equivalent. `method="target_volatility"` gives the largest expected return within a volatility limit, and `"mean_variance"` trades return against variance |
| `asset_constraints`: (lower, upper) per symbol, in order | `bounds=(min, max)` for every asset, `asset_bounds={"SPY": (min, max)}` for particular ones |
| `categorical_constraints`: a category per symbol and limits per category | `groups={"Defensive": (["AGG", "GLD"], min, max)}`: one entry per category |
| `weights`, `rebalance_strategy` (an allocation held and rebalanced over the prices' history) | `assets.backtest(weights={...}, rebalance="quarterly")`, or `fl.Drift(band=0.05)`, `fl.Every(periods=21)` or `"never"`, with `trading_cost_bps`; `assets.compare(...)` sets several side by side. See [backtests](methods/backtests.md) |
| `scheduled_cash_flows` (regular contributions) | Deposits and purchases as transactions in a `Portfolio`, whose returns are time- and money-weighted so that the flows do not count as return. `portfolio.rebalance(...)` proposes the trades back to target weights, with fees and tax-aware lot selection |
| `update_dates(...)` for out-of-sample evaluation | `assets.backtest(method=..., validation=fl.Holdout(start="2026-01-01"))` fits on the history before the date and tests on the rest; `fl.WalkForward(years=5)` refits at each rebalance |
| `report("optimization")`, `report("performance")` | `result.show()` on an optimisation or backtest; `portfolio.report()`: overview, holdings, performance, risk, attribution and scenarios, as text, as a page and as data |

The methods pages in [docs/methods](methods) define each calculation and its conventions.

## A worked example

This 0.9 example optimises a portfolio with per-asset and categorical constraints, from prices
it downloads:

```python
from finalytics import Portfolio

portfolio = Portfolio(
    ticker_symbols=["SPY", "AGG", "GLD"],
    benchmark_symbol="^GSPC",
    start_date="2025-10-31",
    end_date="2026-10-31",
    interval="1mo",
    confidence_level=0.95,
    risk_free_rate=0.02,
    objective_function="max_sharpe",
    asset_constraints=[(0.1, 0.6), (0.1, 0.6), (0.1, 0.6)],
    categorical_constraints=[
        ("Style", ["Growth", "Defensive", "Defensive"], [("Growth", 0.1, 0.4), ("Defensive", 0.6, 0.9)]),
    ],
)
portfolio.report("optimization")
```

In 1.0, the prices come in as data and the solution is returned as numbers. Here the prices are
month-end closes, and the defensive assets must make up 60% to 90% of the portfolio:

```python
>>> import io
>>> import pandas as pd
>>> import finalytics as fl
>>> prices = pd.read_csv(io.StringIO("""\
... date,SPY,AGG,GLD
... 2025-10-31,100.00,100.00,100.00
... 2025-11-30,97.20,99.10,98.70
... 2025-12-31,99.24,101.08,95.84
... 2026-01-31,94.87,101.28,96.22
... 2026-02-28,98.95,103.41,102.86
... 2026-03-31,96.88,102.07,101.93
... 2026-04-30,98.81,102.58,101.93
... 2026-05-31,104.25,104.01,103.67
... 2026-06-30,101.33,104.22,104.19
... 2026-07-31,100.42,104.12,101.06
... 2026-08-31,102.12,102.97,101.36
... 2026-09-30,106.62,100.09,107.14
... 2026-10-31,108.11,103.29,107.03
... """), index_col="date", parse_dates=True)
>>> assets = fl.Assets(prices, risk_free_rate=0.02)
>>> result = assets.optimise(
...     method="maximum_sharpe",
...     bounds=(0.1, 0.6),
...     groups={"Defensive": (["AGG", "GLD"], 0.6, 0.9)},
... )
>>> result.results.status
'optimal'
>>> {asset: round(w, 3) for asset, w in result.results.weights.items()}
{'SPY': 0.4, 'AGG': 0.207, 'GLD': 0.393}
>>> round(result.results.expected_return, 4), round(result.results.volatility, 4)
(0.0693, 0.0651)

```

`Assets` detects the monthly frequency from the dates, annualises the mean returns and the
Ledoit–Wolf covariance by 12, and optimises over them. The defensive group's limit binds; without
it, more goes to SPY:

```python
>>> without = assets.optimise(method="maximum_sharpe", bounds=(0.1, 0.6))
>>> round(without.results.weights["SPY"], 3)
0.445

```

`print(result)` prints the optimisation report and `result.show()` shows it as a page. The same
steps work in Rust and JavaScript, as "Portfolio optimisation" in the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#portfolio-optimisation) and the
[npm package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#portfolio-optimisation)
shows.

## Staying on 0.9

0.9 stays installable, and its source is on the
[`0.9.x` branch](https://github.com/sigmafieldlabs/finalytics/tree/0.9.x). It receives no new
features. To keep using it, pin it:

```sh
cargo add finalytics@0.9
pip install "finalytics<1"
npm install finalytics@0.2
```
