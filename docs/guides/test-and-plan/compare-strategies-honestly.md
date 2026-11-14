---
description: Compare fitted methods and fixed allocations on identical holdout or walk-forward periods, with the same limits and costs.
---

# Compare strategies honestly

`Assets.compare` puts methods and fixed allocations through the same test. The important choice is not the number of methods; it is whether the test returns were available when each allocation was chosen.

## Start with a holdout

Fit on the history before a date, then hold and rebalance over the period that follows. Fixed weights join the same test but have no fitted in-sample run.

[holdout.py](../../examples/compare-strategies-honestly/holdout.py)

The shared dates and settings make the comparison meaningful. A holdout is easy to explain, but its verdict can depend on the single split you choose.

## Refit on a rolling window

Walk-forward validation fits on a trailing window, tests what follows, and repeats at each scheduled rebalance. Every tested return comes after the weights were fitted.

[walk-forward.py](../../examples/compare-strategies-honestly/walk-forward.py)

The report also shows each method fitted over the whole history for context. Those in-sample figures are not the evidence for choosing it.

## Or expand the estimation window

An expanding walk-forward starts with the same minimum history but retains all observations as time passes. It suits methods whose estimates should stabilize with more data.

[expanding.py](../../examples/compare-strategies-honestly/expanding.py)

Rolling windows adapt faster to a changed market; expanding windows use more history but can retain regimes that no longer apply.

## Keep in-sample results in their place

Omitting `validation` gives an in-sample comparison. That is useful for inspecting objectives and constraints, not for claiming that an optimised allocation has predictive power.

[in-sample.py](../../examples/compare-strategies-honestly/in-sample.py)

> [!TIP]
> Choose the method, constraints, window and rebalance rule before looking at the test result. Repeatedly tuning them on the same holdout turns it into training data.

All methods use the options they support. The report notes options a method ignores, and rejects an option that no supplied method accepts.
