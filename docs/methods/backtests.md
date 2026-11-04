# Backtests and comparisons

A backtest holds a strategy's weights through the history of the assets' returns, trades back to
them on a schedule, pays for its trades, and scores the returns it earns. `assets.backtest(...)`
tests one strategy, `assets.compare(...)` tests several side by side on the same periods, and
the history in an optimisation report is a backtest of the weights found. A strategy is a method
of [optimisation](optimisation.md), fitted to returns, or fixed weights.

## Holding the weights

The returns are those of the assets' periods, $r_{t,i}$, with cash earning $c_t$: the risk-free
asset's return, or the annual risk-free rate compounded to the period, $(1 + r_f)^{1/P} - 1$ for
$P$ periods a year. Weights $w_i$ on the assets leave $1 - \sum_i w_i$ in cash.

- **Return.** Over period $t$ the portfolio returns $g_t = \sum_i w_i r_{t,i} + w_{\text{cash}}\, c_t$,
  less the cost of any trade at the end of the period before:
  $R_t = (1 + g_t)(1 - k_{t-1}) - 1$.
- **Drift.** Between trades each weight moves with its asset, $w_i \leftarrow w_i (1 + r_{t,i}) /
  (1 + g_t)$, and cash with its return.
- **Trades.** At the end of a period on the schedule, the weights trade back to the target. A
  trade's turnover is one-way, the share of the portfolio bought (or, equally, sold):
  $\tfrac12 \big(\sum_i \lvert w^*_i - w_i \rvert + \lvert w^*_{\text{cash}} - w_{\text{cash}} \rvert\big)$.
  It costs `trading_cost_bps` of the value traded, bought and sold, $k_t = b \sum_i \lvert w^*_i -
  w_i \rvert$, paid out of the next period's return. Cash is not charged.
- **Start.** Each run starts holding its weights on the date it enters, without a trade: the
  close before its first period. The growth of 100 starts at 100, and the drawdown at 0, on that
  date.

## Rebalancing

| Schedule | Trades at the end of |
| --- | --- |
| `monthly`, `quarterly` (the default), `yearly` | Each calendar month, quarter or year |
| `Drift(band=0.05)` | Any period in which a weight, or cash, is more than the band from its target |
| `Every(periods=21)` | Every 21 periods, counted from the first period of the run |
| `never` | No period: the weights are bought once and drift (buy and hold) |

There is no trade after the last period.

## Validation

How a strategy fitted to returns is scored (fixed weights are held the same way, without
fitting):

- **In sample** (the default) fits the weights to the whole history and scores them on it. The
  method has seen the returns it is scored on, so the result flatters it.
- **Holdout** (`Holdout(start="2016-11-01")`) fits the weights to the history before `start`
  and holds them over the rest. The strategy enters at the last close before the test, so the
  test's first period is the first after `start`. To enter at the first close of the test
  instead, compare fixed weights over the test's own prices.
- **Walk-forward** (`WalkForward(years=5)`, or `periods=60`) fits to the first window, then
  refits at each scheduled rebalance to the window of history ending at that period, and trades
  to the new weights there; `expanding=True` refits to all the history so far. Every return
  scored comes after the weights were chosen. It needs a schedule fixed in advance (calendar or
  every so many periods). For comparison, each method is also fitted to the whole history and
  held over the same periods: that is its run in sample.

A fit that finds no weights keeps the weights held, without trading, and the reports list it;
a method that never found weights holds cash.

## The reports

The comparison and backtest reports score each run over the periods it was tested on:

- the annualised return and volatility, the Sharpe and Sortino ratios against the risk-free
  return over those periods, the maximum drawdown and the conditional drawdown at risk, the
  historical CVaR at 95% of a period's returns, and, against a benchmark, the tracking error,
  information ratio, beta and capture ratios, all as the [statistics](statistics.md) define them;
- turnover and costs a year, the number of trades and the largest trade;
- the growth of 100 and the drawdown, by month for daily or weekly returns, and the Sharpe ratio
  over the last three years, each window against its own risk-free return;
- the return of each calendar year and month, with a year only partly in the run marked, and a
  holdout's year that straddles the start of the test marked †;
- in sample against out of sample, for the methods fitted to returns, and the weights at each
  refit.

Figures shown side by side are differences of the figures as shown, so that they add up.

## Example

```python
>>> import finalytics as fl
>>> assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
>>> result = assets.compare(
...     methods=["minimum_variance", "maximum_sharpe"],
...     weights={"60/40": {"US Equity": 0.6, "Bonds": 0.4}},
...     validation=fl.Holdout(start="2016-11-01"),
...     rebalance="quarterly",
...     trading_cost_bps=10,
...     max_weight=0.6,
... )
>>> for s in result.report.strategies:
...     fit = None if s.in_sample is None else round(s.in_sample.statistics.sharpe_ratio, 2)
...     print(s.name, fit, round(s.tested.statistics.sharpe_ratio, 2))
Minimum variance 0.63 0.38
Maximum Sharpe 0.69 0.29
60/40 None 0.37
>>> run = result.results.strategies[0].out_of_sample
>>> run.dates[0], run.trades[0].date
(datetime.date(2016, 11, 30), datetime.date(2016, 12, 30))
>>> trade = run.trades[0]
>>> round(trade.turnover, 4), round(trade.cost, 6)
(0.022, 4.4e-05)

```

The methods were fitted to the twenty years before November 2016 and held for the ten after;
their Sharpe ratios fall out of sample, as estimated portfolios' do. The 60/40 weights were not
fitted, so they have no run in sample. The first trade, at the end of the first quarter of the
test, buys 2.2% of the portfolio and sells as much, and costs 10 bp of the 4.4% traded.

## Validation

`scripts/reference/backtest.py` simulates five backtests afresh from the definitions above, on
the asset-class dataset with T-bills as cash: in sample, rebalanced quarterly with 10 bp costs;
a holdout from November 2016 with a drift band of 5 points and 25 bp, against a benchmark;
walk-forward on five years, quarterly; walk-forward on all the history after ten years, yearly;
and fixed weights never rebalanced. The minimum-variance refits are solved in CVXPY on
scikit-learn's Ledoit–Wolf covariance. The dates, every return, and each trade's date, weights,
turnover and cost agree to $10^{-12}$ for fixed weights, equal weight and inverse volatility, and
to $10^{-5}$ for minimum variance, where the solvers' tolerances leave a weight that belongs at a
bound a few millionths from it. Rebalancing every so many periods is checked by the dates of
its trades and refits, in Rust, Python and JavaScript.
