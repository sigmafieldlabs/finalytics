---
description: Guard against expected-return estimation error with ellipsoid or box uncertainty and Michaud resampling.
learn: portfolio/1.9
---

# Make optimisation less sensitive to estimates

Optimised weights can react strongly to small errors in expected returns. Robust optimisation plans for a set of possible returns; resampling averages optima across simulated histories.

## Use ellipsoidal uncertainty

An ellipsoid places the true expected returns within a scaled covariance-shaped region around the estimates. The optimiser maximises worst-case utility over that region.

[ellipsoid.py](../../examples/robust-and-resampled/ellipsoid.py)

With no separate estimation-error covariance, the problem covariance is used and `size` controls the uncertainty. A size of zero reduces to ordinary mean–variance; larger sizes move towards minimum variance.

## Use box uncertainty

A box gives each asset's expected return its own half-width. The worst case subtracts the sum of `half_width × absolute weight` from nominal expected return.

[box.py](../../examples/robust-and-resampled/box.py)

Ellipsoid and box sets have different geometry and scales, so equal-looking size inputs are not equivalent. Both preserve a convex optimisation problem.

## Average resampled optima

`resample` simulates histories from the assets' estimates, re-estimates mean and covariance for each, solves the same problem and averages the weights. The fixed implementation makes results reproducible across languages and platforms.

[resampled.py](../../examples/robust-and-resampled/resampled.py)

The report includes weight deviations across simulations and counts solved and failed problems. `results.solution` retains the optimum for the original estimates. Scenario-based objectives and factor-exposure limits do not support resampling.

Robust and resampled optimisation address estimation error differently; neither creates out-of-sample evidence. Follow with [Compare strategies honestly](../test/compare-strategies-honestly.md), and see [Robust and resampled optimisation](../../methods/robust-optimisation.md) for the algorithms.
