---
description: Carry screened identifiers into asset selection, optimisation, historical testing and account-level rebalancing.
---

# Go from screen to portfolio

Screening and portfolio construction are separate steps. A screen supplies candidate identifiers; you then choose their price history, an allocation rule and a validation method before turning targets into trades.

## Screen, then optimise the matches

Here the household's funds form a small universe. The matching identifiers select aligned price columns, and equal weight gives a transparent allocation without fitting another model.

[screen-and-optimise.py](../../examples/from-screen-to-portfolio/screen-and-optimise.py)

For a broader universe, align the selected histories deliberately and decide how to treat different start dates and missing prices. Screening on adjusted closes gives total-return measures.

## Backtest the selected allocation

An optimisation of today's matches is not itself a backtest. Test the resulting weights over history with a benchmark, a rebalance schedule and costs.

[backtest-selection.py](../../examples/from-screen-to-portfolio/backtest-selection.py)

This tests the selected allocation, not a historical recreation of the screen. A genuine screen backtest must repeat the screen using only information available at each past date.

## Turn the matches into account trades

Equal target weights can be passed directly to the household's ledger. Untargeted holdings are sold, and fees, tax and whole-unit rounding determine the executable proposal.

[rebalance-household.py](../../examples/from-screen-to-portfolio/rebalance-household.py)

> [!WARNING]
> Screening the latest data and then backtesting over earlier years introduces look-ahead bias. Use dated constituent lists and point-in-time fields for a historical screening strategy.

The same boundary applies in production: `Universe` finds candidates, `Assets` estimates and tests allocations, and `Portfolio` keeps the books and executes the proposed trades.
