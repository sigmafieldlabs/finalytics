---
description: Test a method or fixed allocation with realistic rebalancing, trading costs, benchmarks and cash flows.
---

# Backtest a strategy

`Assets.backtest` replays one method or one set of fixed weights. The result contains the text and HTML report, plus every return, trade and weight in `results`.

## Test a fitted method on a holdout

A fitted method needs validation. Here, maximum Sharpe is fitted before the holdout and then tested against a fixed benchmark, with quarterly trading and costs.

[holdout.py](../../examples/backtest-a-strategy/holdout.py)

The initial allocation enters without an initial trading charge. Later trades pay the stated basis points on both purchases and sales; cash is not charged.

## Trade when weights drift

Calendar schedules are predictable, but a drift band trades only when an asset or cash moves too far from its target. Fixed weights do not need an in-sample fit.

[drift-and-costs.py](../../examples/backtest-a-strategy/drift-and-costs.py)

Use `monthly`, `quarterly`, `yearly`, `fl.Drift`, or `fl.Every` in Python. JavaScript takes the equivalent strings and tagged objects; Rust uses `Rebalance`.

## Follow money in and out

Returns alone do not show an investor's cash experience. Add a starting value and dated positive or negative flows to report ending value and money-weighted return.

[money-flows.py](../../examples/backtest-a-strategy/money-flows.py)

Flows arrive at the end of their return period. They change the money report, not the underlying return series or trades. Withdrawals cannot take more than the portfolio holds.

## Let the allocation drift

`never` buys the starting allocation once and leaves it alone. Comparing this with a rebalanced run separates the effect of rebalancing from the effect of the chosen assets.

[buy-and-hold.py](../../examples/backtest-a-strategy/buy-and-hold.py)

> [!WARNING]
> An in-sample backtest fits and scores a method on the same returns. Use a holdout or walk-forward comparison before treating the result as evidence about future performance.

See [Backtests and comparisons](../../methods/backtests.md) for timing, cost and return conventions.
