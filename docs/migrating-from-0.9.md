# Migrating from 0.9

Finalytics 1.0 is a redesign. It keeps the purpose of 0.9, portfolio analytics and optimisation,
and changes where the data comes from and what the library returns. This guide lists what changed,
what replaces each part of 0.9, and works one 0.9 example through in 1.0.

0.9 remains available: see [Staying on 0.9](#staying-on-09).

## What changed

- **You supply the data.** 0.9 downloaded prices, financial statements, options and news from
  Yahoo Finance and other websites. 1.0 performs no input or output: you pass in prices, and FX
  rates for other currencies, from any source you are entitled to use, as CSV, JSON, mappings, a
  pandas DataFrame or Series, NumPy arrays or JavaScript arrays. This was the "custom data" path
  of 0.9 (`KLINE`); in 1.0 it is the only one. One price per instrument and date is enough, such
  as the close or the adjusted close.
- **Results, not charts.** 0.9 drew charts and HTML reports (`report(...).show()`). 1.0 returns
  data: the reports in `finalytics.reports` (Rust: `finalytics::reports`) carry the series,
  breakdowns, grids and tables a screen draws, and you draw them with the plotting library you
  prefer.
- **Portfolios are kept from transactions.** A 1.0 portfolio is its instruments, accounts and
  transactions, kept as books with tax lots and several currencies; performance is measured from
  the books. The starting weights and scheduled cash flows of 0.9 become transactions.
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
| Downloading prices (`Ticker`, `Tickers` or `Portfolio` with dates and an interval) | Load prices yourself: `MarketData` from CSV, mappings or pandas (Python), `formats::prices_from_csv` (Rust), `MarketData.fromCsv` (JavaScript) |
| `KLINE` (open, high, low, close, volume) | One price series per instrument: a column per instrument in a CSV file, or a pandas Series |
| `performance_stats()`; the performance report of `Ticker` | `risk.statistics` and `risk.relative_statistics` on returns; `books.performance()` for a portfolio kept from transactions |
| `objective_function="max_sharpe"` | `opt.MaximumSharpe(risk_free_rate=...)` |
| `"min_vol"`, `"min_var"` | `opt.MinimumVariance()`; both have the same optimum |
| `"min_cvar"` | `opt.MinimumCvar(confidence=...)`, with the historical returns as scenarios |
| `"risk_parity"` | `opt.RiskBudgeting()`; equal budgets unless you give others |
| `"max_diversification"` | `opt.MaximumDiversification()` |
| `"hierarchical_risk_parity"` | `opt.HierarchicalRiskParity()` |
| `"max_return"`, `"max_sortino"`, `"min_drawdown"` | No direct equivalent. `opt.TargetVolatility` gives the largest expected return within a volatility limit, and `opt.MeanVariance` trades return against variance |
| `asset_constraints`: (lower, upper) per symbol, in order | `opt.Constraints(bounds=...)` for every asset, `asset_bounds=[opt.AssetBound(asset=..., min=..., max=...)]` for particular ones |
| `categorical_constraints`: a category per symbol and limits per category | `opt.Constraints(groups=[...])`: one `opt.GroupLimit(name=..., assets=[...], min=..., max=...)` per category |
| `weights`, `rebalance_strategy`, `scheduled_cash_flows` (a simulated allocation with rebalancing and regular contributions) | Transactions (deposits, purchases, sales) in a `Portfolio`, kept as books. `books.rebalance(...)` proposes the trades back to target weights, with fees and tax-aware lot selection. 1.0 does not simulate a schedule: generate the transactions for the schedule you want |
| `update_dates(...)` for out-of-sample evaluation | Compute the portfolio's returns over the later period from its weights and the assets' returns, then `risk.statistics` |
| `report("optimization")`, `report("performance")` | `finalytics.reports`: holdings, overview, performance, risk, attribution and scenarios, as data |

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

In 1.0, the prices come in as data, the returns and their covariance are estimated explicitly, and
the solution is returned as numbers. Here the prices are month-end closes, and the defensive
assets must make up 60% to 90% of the portfolio:

```python
>>> import io
>>> import pandas as pd
>>> import finalytics as fl
>>> from finalytics import opt
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
>>> returns = prices.pct_change().dropna()
>>> estimate = fl.risk.covariance(returns, method="ledoit_wolf")
>>> problem = opt.Problem.from_covariance(
...     fl.risk.scaled(estimate.matrix, 12),  # annualised
...     expected_returns=returns.mean() * 12,
...     constraints=opt.Constraints(
...         bounds=opt.Bound(min=0.1, max=0.6),
...         groups=[opt.GroupLimit(name="Defensive", assets=["AGG", "GLD"], min=0.6, max=0.9)],
...     ),
... )
>>> solution = problem.solve(opt.MaximumSharpe(risk_free_rate=0.02))
>>> solution.status
'optimal'
>>> dict(zip(solution.labels, [round(w, 3) for w in solution.weights]))
{'SPY': 0.4, 'AGG': 0.207, 'GLD': 0.393}
>>> round(solution.expected_return, 4), round(solution.volatility, 4)
(0.0693, 0.0651)

```

The defensive group's limit binds: without it, the same returns give 44% in SPY, as in the
README's quick start.

The same steps work in Rust and JavaScript; the READMEs of the
[Rust crate](https://github.com/sigmafieldlabs/finalytics#quick-start) and the
[npm package](https://github.com/sigmafieldlabs/finalytics/tree/main/bindings/wasm#quick-start)
show them.

## Staying on 0.9

0.9 stays installable, and its source is on the
[`0.9.x` branch](https://github.com/sigmafieldlabs/finalytics/tree/0.9.x). It receives no new
features. To keep using it, pin it:

```sh
cargo add finalytics@0.9
pip install "finalytics<1"
npm install finalytics@0.2
```
