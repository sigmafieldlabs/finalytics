---
description: Estimate value at risk and expected shortfall with historical, parametric and Cornish–Fisher methods, then decompose expected shortfall by asset.
---

# Estimate tail risk

Value at risk is the loss threshold at a chosen confidence level; expected shortfall is the mean loss in the tail beyond it. Finalytics reports both as positive losses over one period of the returns supplied.

## Compare the estimators

Historical estimates use observed returns. Parametric estimates assume a normal distribution. Cornish–Fisher adjusts the normal estimate for skewness and excess kurtosis and integrates the expanded density for expected shortfall.

[methods.py](../../examples/tail-risk/methods.py)

The methods answer different modelling questions; none makes a small sample's tail certain. Cornish–Fisher can be unstable when higher moments are extreme, and Finalytics ensures expected shortfall is not below value at risk.

## Change the confidence level

A higher confidence asks about a more remote tail. With few observations, historical estimates may reuse the same worst returns or interpolate between only a few points.

[confidence.py](../../examples/tail-risk/confidence.py)

Use a return history that contains the market conditions the estimate should reflect. Confidence does not specify the horizon: monthly returns produce monthly losses.

## Attribute expected shortfall

`shortfall_decomposition` allocates the historical expected shortfall to assets by Euler's rule. A negative component means the asset offsets losses in the selected tail. Components sum to the portfolio's expected shortfall.

[shortfall.py](../../examples/tail-risk/shortfall.py)

This uses the Rockafellar–Uryasev tail mean, including a fractional boundary scenario when needed. The standalone historical `tail_risk` estimator averages whole observations, so the two conventions can differ slightly.

## Treat horizon scaling as an assumption

Tail risk is not automatically annualised. A square-root-of-time rescaling is shown here only as a modelling assumption for independent, identically distributed normal risk; it is not a general conversion for skewed, autocorrelated or regime-dependent returns.

[horizon.py](../../examples/tail-risk/horizon.py)

For a different horizon, prefer returns or scenarios measured at that horizon and state the assumptions behind any scaling.

## Apply it to asset histories

Use an asset's return column directly when you need another method or confidence than `Assets.statistics()` provides. The flag records whether Cornish–Fisher expected shortfall had to be raised to value at risk.

[assets.py](../../examples/tail-risk/assets.py)

See [Value at risk and expected shortfall](../../methods/tail-risk.md) for the exact estimators and [Risk decomposition](risk-decomposition.md) for volatility and tracking-error allocations.
