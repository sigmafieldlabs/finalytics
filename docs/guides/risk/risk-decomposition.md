---
description: Split volatility, tracking error and expected shortfall into asset and group contributions, including hedges and stressed covariance.
---

# Decompose portfolio risk

A weight says how much money an asset receives; a risk contribution says how much of the portfolio's risk it explains. Euler decomposition makes the components add exactly to volatility, tracking error or expected shortfall.

## Allocate volatility

`risk_decomposition` reports each asset's marginal risk, component and share. Component is weight times marginal risk; share is component divided by total volatility.

[volatility.py](../../examples/risk-decomposition/volatility.py)

A negative component is a hedge: at the margin, that asset reduces total risk. Shares can therefore be negative or greater than one. Long–short and levered weights are valid here because the calculation does not require weights to sum to one.

## Attribute tracking error

Tracking-error decomposition replaces portfolio weights with active weights against a benchmark. The resulting volatility is ex-ante tracking error under the supplied covariance.

[tracking-error.py](../../examples/risk-decomposition/tracking-error.py)

Use benchmark weights in the same asset order and units as the portfolio. An asset with no active weight has no component under this decomposition.

## Add contributions by group

Python's `group_risk` and Rust's `group_risk` sum weights, components and shares by any label mapping, such as asset class, region or currency. Groups are returned in name order.

[groups.py](../../examples/risk-decomposition/groups.py)

> [!NOTE]
> JavaScript does not currently export a group-risk helper. Aggregate `contributions` in application code, preserving the same mapping and summing components and shares.

## Allocate expected shortfall

`shortfall_decomposition` uses historical scenarios and the Rockafellar–Uryasev tail mean. Each asset's marginal value is its mean loss in the tail; multiplying by its weight gives the component.

[shortfall.py](../../examples/risk-decomposition/shortfall.py)

This is a one-period loss, not annualised volatility. The standalone historical tail estimator averages whole observations, so its expected shortfall can differ slightly when the tail cuts through an observation.

## Stress risk and correlations

`stressed_covariance` scales every volatility and moves correlations towards one. Repeating the decomposition under that matrix shows how diversification can weaken in a crisis.

[stressed.py](../../examples/risk-decomposition/stressed.py)

The matrix remains positive semidefinite when the input is. For a portfolio valued holding by holding, use [Scenarios and stress tests](scenarios-and-stress-tests.md); see [Risk decomposition](../../methods/risk-decomposition.md) for the mathematical definitions.
