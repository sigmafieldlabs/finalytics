---
description: Optimise conditional value at risk, semivariance and conditional drawdown at risk directly from return scenarios.
learn: portfolio/1.8
---

# Optimise downside risk

Variance treats gains and losses alike. The downside objectives instead optimise the empirical distribution of scenario returns, which makes them useful when tail loss or drawdown is the investment concern.

## Minimise expected shortfall

`minimum_cvar` minimises the mean loss in the worst tail at `confidence`, using the Rockafellar–Uryasev linear programme. CVaR here is expected shortfall, reported as a positive loss per scenario period.

[cvar.py](../../examples/tail-risk-objectives/cvar.py)

The estimate is only as representative as the scenarios supplied by the asset history. Several portfolios can share the same minimum, so small implementation differences need not imply an error.

## Minimise downside variance

`minimum_semivariance` minimises the mean square of negative scenario returns over every scenario. It penalises larger below-zero returns more than smaller ones.

[semivariance.py](../../examples/tail-risk-objectives/semivariance.py)

Semivariance uses a target of zero and is measured per scenario period. It differs from variance even when the distribution is symmetric because only losses enter the objective.

## Minimise conditional drawdown

`minimum_cdar` minimises the tail mean of drawdowns in the cumulative sum of scenario returns. It responds to the sequence of returns, unlike CVaR and semivariance, which respond to one-period outcomes.

[cdar.py](../../examples/tail-risk-objectives/cdar.py)

This optimisation uses cumulative arithmetic returns from a start of zero. The performance statistics measure drawdown on compounded value, so the two definitions can differ.

## Require an expected return

Downside objectives alone can choose a portfolio with very low expected return. `target` adds an annual expected-return floor; the examples retain a cap on each asset's weight.

[return-floor.py](../../examples/tail-risk-objectives/return-floor.py)

Keep expected returns and the floor in the same annual units. If no allocation meets both the floor and the other constraints, inspect the solver status and relax an input. See [Portfolio optimisation](../../methods/optimisation.md) for the formulations and validation.
