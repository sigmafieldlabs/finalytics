---
description: Estimate regression, statistical and fundamental factor models, supply a vendor model and decompose portfolio or active risk by factor.
---

# Model risk with factors

A factor model explains asset returns through common exposures plus asset-specific residuals. It can describe where risk comes from, supply a covariance matrix to optimisation and translate factor shocks into holding returns.

## Regress assets on observed factors

Use `regression_model` when factor returns are observed, such as a market index or macroeconomic series. Each asset has an intercept and slope exposures; the model also reports its fit and residual variance.

[regression.py](../../examples/factor-models/regression.py)

Asset and factor tables must cover the same periods. Factors cannot be constant or collinear, and there must be enough observations to estimate the regressions. Models are per period until scaled.

## Extract statistical factors

Use `statistical_model` when no external factors are available. Principal components of the sample covariance become the factors, with loadings, component returns and explained-variance shares.

[statistical.py](../../examples/factor-models/statistical.py)

These components are statistical rather than necessarily interpretable economic factors. They come from covariance, not correlation, so higher-variance assets have more influence.

## Use known exposures

Use `fundamental_model` for exposures supplied from elsewhere, such as industry membership or style scores. It estimates each period's factor returns by cross-sectional regression without an intercept.

[fundamental.py](../../examples/factor-models/fundamental.py)

Include a column of ones if a market factor is required. Exposures stay constant over the estimation window; there must be more assets than factors and no collinear exposure columns.

## Supply a complete model

`factor_model` accepts exposures, factor covariance and specific variances directly. This is the route for a vendor risk model or parts estimated in another system. `implied_covariance` reconstructs the asset covariance.

[supplied.py](../../examples/factor-models/supplied.py)

The engine checks matrix symmetry and positive semidefiniteness, dimensions and non-negative specific variances. Keep asset and factor labels aligned with the source model.

## Split total and active factor risk

`factor_risk` reports portfolio exposures and Euler contributions by factor, plus the specific component and asset-level contributions. `factor_tracking_error` applies the same calculation to active weights against a benchmark.

[risk.py](../../examples/factor-models/risk.py)

Scale the model's factor covariance and specific variances together when changing horizon; exposures do not change. Factor contributions and specific risk add to total volatility.

See [Factor models](../../methods/factor-models.md) for estimation details and validation. Factor shocks are applied to real holdings in [Scenarios and stress tests](scenarios-and-stress-tests.md), and [Beneath the front door](../optimise/beneath-the-front-door.md) covers factor exposure constraints.
