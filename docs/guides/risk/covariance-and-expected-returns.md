---
description: Estimate covariance and correlation with eight methods, choose expected-return estimators and annualise the results consistently.
---

# Estimate covariance and expected returns

Covariance drives risk decomposition and most optimisation objectives. Expected returns drive return-seeking allocations and are usually much harder to estimate. Finalytics keeps both choices explicit and labels every matrix with its assets.

## Start with the sample matrix

The low-level `covariance` function takes returns and produces a matrix per input period. Python accepts one row per period; JavaScript and Rust tables hold one column per asset. Scale the matrix by periods per year to annualise it.

[sample.py](../../examples/covariance-and-expected-returns/sample.py)

The sample estimator uses the usual unbiased divisor. `Assets.covariance()` performs the corresponding estimate and annualisation together.

## Shrink a noisy estimate

Ledoit–Wolf and OAS shrink towards a scaled identity; constant-correlation shrinkage preserves each asset's variance while moving correlations towards their average. The result reports the chosen intensity where applicable.

[shrinkage.py](../../examples/covariance-and-expected-returns/shrinkage.py)

Shrinkage is especially useful with many assets relative to observations or nearly collinear returns. It is a bias–variance trade-off, not a correction that always increases accuracy.

## Weight recent data or use factors

EWMA gives recent observations more weight through its decay. The statistical estimator uses a chosen number of principal components and keeps each asset's sample variance.

[weighted-statistical.py](../../examples/covariance-and-expected-returns/weighted-statistical.py)

A lower decay reacts faster but uses fewer effective observations. A factor covariance can also make large optimisation problems more efficient when used through the low-level factor-model interface.

## Denoise the spectrum

Denoised covariance replaces correlation eigenvalues below the random-matrix noise edge. Nonlinear shrinkage instead shrinks sample eigenvalues individually through analytical nonlinear shrinkage.

[spectral.py](../../examples/covariance-and-expected-returns/spectral.py)

Nonlinear shrinkage needs enough returns for its bandwidth calculation. `Assets` validates estimator requirements when it is built, before an optimisation starts.

## Choose expected returns

Historical means are the default. James–Stein shrinks asset means towards their grand mean; EWMA emphasises recent returns. A named risk-free asset always uses its own historical mean as the risk-free rate.

[means.py](../../examples/covariance-and-expected-returns/means.py)

Expected-return error can dominate optimisation. Consider shrinkage, explicit forecasts or a Black–Litterman prior, and compare out-of-sample results rather than selecting whichever historical estimate gives the preferred weights.

## Convert covariance to correlation

Correlation divides each covariance by the corresponding standard deviations. Every variance must be positive, so a zero-variance cash series has no correlation estimate.

[correlation.py](../../examples/covariance-and-expected-returns/correlation.py)

See [Covariance and mean returns](../../methods/covariance.md) for formulas and estimator requirements. The matrices feed directly into [Risk decomposition](risk-decomposition.md) and the optimisation guides.
