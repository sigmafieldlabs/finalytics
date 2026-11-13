---
description: Measure return, volatility, downside risk, drawdowns and benchmark-relative behaviour from returns or asset histories.
---

# Measure risk

Risk starts with a return series. `statistics` measures its compound return, dispersion, downside and drawdowns; `relative_statistics` compares it with a benchmark over the same periods. Set the return frequency and annual rates explicitly rather than relying on daily defaults.

## Summarise a return series

This monthly series uses an annual risk-free rate for Sharpe and an annual minimum acceptable return for downside deviation and Sortino. Drawdowns and returns are fractions, so negative values are losses.

[summary.py](../../examples/measure-risk/summary.py)

Volatility uses the sample standard deviation. Downside deviation averages shortfalls over every observation, including those above the target. Skewness and excess kurtosis use population moments.

## Compare with a benchmark

The benchmark must have one return for every portfolio return, in the same order. Tracking error is the volatility of their difference; beta, alpha, correlation and conditional capture measures describe different parts of the relationship.

[relative.py](../../examples/measure-risk/relative.py)

Downside and upside beta use only periods in which the benchmark fell or rose. Capture ratios compound over those periods. They are absent when there are too few applicable observations.

## Trace drawdowns and recovery

`drawdowns` takes levels rather than returns: portfolio values, an index or a growth series all work. Each episode records its peak, trough, depth and first recovery. An unrecovered episode has no recovery date or recovery duration.

[drawdowns.py](../../examples/measure-risk/drawdowns.py)

The maximum drawdown in `statistics` is measured from an initial value before the first return. Use dated levels when you also need the dates and calendar durations of each episode.

## Measure the shipped assets

`Assets.statistics()` applies the same calculations to every asset and adds historical tail risk. It detects the asset-class dataset's monthly frequency and uses its T-bill return as the risk-free rate.

[asset-statistics.py](../../examples/measure-risk/asset-statistics.py)

These are historical estimates, not forecasts. Supply total-return prices when income should contribute to performance and risk.

## Choose the conventions deliberately

Changing the target changes downside deviation and Sortino even though volatility is unchanged. Ratios that require a non-zero dispersion are `None`, `null` or `None` when it is absent.

[conventions.py](../../examples/measure-risk/conventions.py)

See [Return statistics and drawdowns](../../methods/statistics.md) for every definition, annualisation rule and validation check. Use [Tail risk](tail-risk.md) to focus on losses beyond a confidence level and [Rolling analysis](rolling-analysis.md) to see estimates change through time.
