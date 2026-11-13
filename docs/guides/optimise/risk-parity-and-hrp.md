---
description: Allocate by inverse volatility, equal or custom risk budgets, and hierarchical risk parity with different cluster linkages.
learn: portfolio/1.7
---

# Allocate by risk

Risk-based methods do not need expected-return forecasts. They offer useful alternatives and baselines when estimates of returns are especially uncertain.

## Start with inverse volatility

Inverse volatility gives each asset weight proportional to `1 / volatility`, then normalises the weights to one. It is risk parity only when assets are uncorrelated.

[inverse-volatility.py](../../examples/risk-parity-and-hrp/inverse-volatility.py)

The calculation uses variances but not correlations. It is fully invested and long only, and does not accept the general weight constraints.

## Equalise risk contributions

`risk_parity` chooses positive weights at which every asset has the same share of portfolio volatility. Unlike inverse volatility, it accounts for the full covariance matrix.

[risk-parity.py](../../examples/risk-parity-and-hrp/risk-parity.py)

The report's risk shares are the diagnostic to inspect. The solver targets equality through Spinu's convex risk-budgeting formulation.

## Set custom risk budgets

`risk_budgeting` generalises risk parity. Positive budgets are scaled to sum to one, so they express relative risk allocations rather than capital weights.

[risk-budgets.py](../../examples/risk-parity-and-hrp/risk-budgets.py)

A larger budget means a larger targeted contribution to volatility, not necessarily a larger allocation of money. Assets with zero variance must be removed first.

## Cluster before allocating

Hierarchical risk parity clusters assets by correlation distance, orders the tree and splits weight recursively in inverse proportion to cluster variance. `linkage` controls how clusters are joined: `single`, `complete`, `average` or `ward`.

[hrp.py](../../examples/risk-parity-and-hrp/hrp.py)

HRP uses no expected returns and no convex solver. Like risk budgeting, it is fully invested and long only and takes no general constraints. See [Portfolio optimisation](../../methods/optimisation.md) for the algorithms and validation.
