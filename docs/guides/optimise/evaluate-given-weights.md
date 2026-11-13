---
description: Value current, optimised or externally chosen weights on the same expected-return and covariance estimates, including negative and levered exposures.
learn: portfolio/1.4
---

# Evaluate given weights

`evaluate` answers a different question from `optimise`: given weights chosen elsewhere, what expected return, volatility, Sharpe ratio and risk shares do the current estimates imply? It does not change the weights.

## Value the current portfolio

Passing a portfolio's own weights to its assets reproduces the current point in the overview report. This includes instruments and cash, with base-currency cash treated as riskless and foreign-currency cash carrying exchange-rate risk.

[current.py](../../examples/evaluate-given-weights/current.py)

The result uses the same estimates as optimisation, making current and proposed allocations directly comparable. Its report is titled “Given weights” and has no solver section because no solver ran.

## Compare optimised weights consistently

Evaluating an optimisation's weights reproduces its expected return and volatility up to solver tolerance. The same route works for weights from another model, a policy portfolio or a manual allocation.

[optimised.py](../../examples/evaluate-given-weights/optimised.py)

Add `frontier=True` when the report should also show an efficient frontier. Limits supplied with `evaluate` shape that frontier; they do not reject or modify the given weights themselves.

## Include short and levered exposures

Evaluation accepts negative weights and weights with gross exposure above one, provided the net weights sum to one. Risk contributions can consequently be negative or greater than the total.

[long-short.py](../../examples/evaluate-given-weights/long-short.py)

> [!IMPORTANT]
> This analytical support does not make the transaction ledger short-aware. High-level optimisation and rebalancing remain long only, and selling more than an account holds is still invalid. Use `evaluate` to analyse externally specified exposures, not to generate executable short trades.

See [Beneath the front door](beneath-the-front-door.md) for configured long–short optimisation and [Risk decomposition](../risk/risk-decomposition.md) for interpreting hedging contributions.
