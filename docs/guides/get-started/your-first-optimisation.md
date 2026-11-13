---
description: Build Assets from monthly prices, optimise with three objectives, inspect weights and compare an existing allocation.
learn: portfolio/1.4
---

# Your first optimisation

This guide uses the asset-class dataset included with Finalytics. Its total-return prices need no download, and every example is a complete program.

## Maximise Sharpe ratio

Name `T-Bills` as the risk-free asset and cap any other weight at 40%. The printed report explains the result; `show()` adds the formatted report view.

[maximum-sharpe.py](../../examples/your-first-optimisation/maximum-sharpe.py)

## Change the objective

Minimum variance seeks the least volatile mix. Target return and target volatility instead minimise risk subject to the requested target. Expected return and volatility are annualised using the monthly frequency detected from the dates.

[objectives.py](../../examples/your-first-optimisation/objectives.py)

## Read the weights

`results.weights` maps asset names to fractions. Check `results.status` before using them; an infeasible request is not a portfolio you should trade.

[weights.py](../../examples/your-first-optimisation/weights.py)

## Trace an efficient frontier

A frontier evaluates a sequence of minimum-risk portfolios at different returns. The same asset and weight limits apply at every point.

[frontier.py](../../examples/your-first-optimisation/frontier.py)

## Evaluate weights you already hold

Optimisation is not required: `evaluate` measures any complete set of weights on the same return and covariance estimates. This example evaluates a 60/40 stock-and-bond mix.

[evaluate.py](../../examples/your-first-optimisation/evaluate.py)

> [!TIP]
> Use **Optimise a portfolio** for estimator choices, cash limits and infeasible targets. Later guides cover constraints, backtests and rebalancing real accounts.
