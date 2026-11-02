# Factor models

A factor model explains the returns of $N$ assets with a few common factors:

$$r_t = \alpha + B f_t + e_t$$

where $B$ is the $N \times K$ matrix of exposures, $f_t$ the $K$ factor returns, and $e_t$ the
specific returns, uncorrelated with the factors and with each other. With $F$ the covariance of
the factor returns and $d$ the specific variances, the covariance of the assets is

$$\Sigma = B F B^\top + \operatorname{diag}(d)$$

Finalytics estimates such models three ways, splits a portfolio's risk by factor, and lets the
optimiser use a model in place of a covariance matrix, with limits on factor exposures.

## Estimation

Each estimate reports the model, the fit for each asset (its intercept and $R^2$), the factor
returns over the period, and the number of periods. The model is per period of the returns;
`scaled` multiplies $F$ and $d$ by the periods in a year to annualise it.

### Time-series regression

Given factor returns, such as a market index or the Fama–French factors, each asset's returns are
regressed on them with an intercept, by ordinary least squares (`regression_model`):

- the exposures are the slopes and $\alpha_i$ the intercept;
- $F$ is the sample covariance of the factor returns (divisor $T - 1$);
- $d_i$ is the residual variance with $T - K - 1$ degrees of freedom;
- $R^2_i = 1 - \text{SSR}_i / \text{SST}_i$.

There must be more periods than factors plus one, and no factor may be constant or a combination
of the others.

### Statistical (principal components)

With no factors given, the first $K$ eigenvectors of the sample covariance matrix $S$ serve as
the factors (`statistical_model`):

- the exposures are the eigenvectors $v_k$, named `pc1`, `pc2` and so on, each with its largest
  loading positive (the convention of scikit-learn);
- $F = \operatorname{diag}(\lambda_1, \dots, \lambda_K)$, the eigenvalues;
- $d_i = S_{ii} - \sum_k \lambda_k v_{ik}^2$, so the model reproduces each asset's sample variance
  exactly;
- the factor returns are the component scores $(x_t - \bar x)^\top v_k$, and each component's
  share of the total variance, $\lambda_k / \operatorname{tr} S$, is reported.

The components are those of the covariance matrix, not the correlation matrix, so assets with
higher volatility weigh more. $K$ is at most the smaller of $N$ and $T - 1$.

### Fundamental (cross-sectional regression)

Given exposures from elsewhere, such as industry memberships, countries or style scores, each
period's asset returns are regressed on the exposures, without an intercept, to estimate that
period's factor returns (`fundamental_model`):

- $f_t = (B^\top B)^{-1} B^\top r_t$, by least squares, and $e_t = r_t - B f_t$;
- $F$ is the sample covariance of the $f_t$ and $d_i$ the sample variance of asset $i$'s specific
  returns (divisor $T - 1$ for both);
- $R^2_i = 1 - d_i / \operatorname{Var}(r_i)$.

The exposures are taken as constant over the period. Include a column of ones for a market
factor. There must be more assets than factors, and no factor's exposures may be a combination of
the others'.

A model whose parts all come from elsewhere, such as a vendor's risk model, is built directly
from its exposures, factor covariance and specific variances (`factor_model`). Its factor
covariance must be symmetric and positive semidefinite, and its specific variances must not be
negative.

## Risk by factor

For a portfolio with weights $w$, the factor exposures are $x = B^\top w$ and the variance splits
into a systematic and a specific part:

$$\sigma^2 = x^\top F x + \sum_i d_i w_i^2$$

`factor_risk` allocates the volatility by Euler's rule:

- factor $k$ contributes $x_k (F x)_k / \sigma$, with marginal risk $(F x)_k / \sigma$;
- the specific part contributes $\sum_i d_i w_i^2 / \sigma$;
- these components add up to $\sigma$, and their shares to one;
- each asset's contribution under $\Sigma$ is reported as well, as in
  [risk decomposition](risk-decomposition.md).

`factor_tracking_error` decomposes the active weights $w - b$ against a benchmark $b$ in the same
way, so the exposures it reports are active exposures and the volatility is the ex-ante tracking
error. When the portfolio has no risk, the marginal risks and shares are not defined and are
reported as none.

## Optimisation with a factor model

A problem made with `Problem.from_factor_model` uses the model in factor form: the exposures
$y = B^\top w$ become $K$ extra variables, and the variance is $y^\top F y + \sum_i d_i w_i^2$.
Every objective and constraint that uses a covariance matrix works the same way, and gives the
same solution as the implied covariance matrix $\Sigma$, to the solver's tolerance. Because the
solver's linear systems stay sparse, large problems solve much faster: a 500-asset
minimum-variance problem with a ten-factor model takes about 10 ms, against about 400 ms with the
full matrix (see [performance](../performance.md)).

Factor exposure limits, $m_k \le (B^\top w)_k \le M_k$, keep a portfolio's exposure to a factor
within bounds, for example a beta to the market between 0.9 and 1.1 or a neutral exposure to an
industry. With benchmark weights $b$, the limits apply to the active exposure
$(B^\top (w - b))_k$. Both are linear constraints on $y$. They need a factor model.

## Example

```python
>>> import numpy as np
>>> import finalytics as fl
>>> market = np.array([0.012, -0.008, 0.015, -0.004, 0.009, -0.011, 0.006, 0.002, -0.006, 0.010])
>>> returns = np.array([
...     [0.0176, 0.0110, 0.0084, 0.0034],
...     [-0.0114, -0.0060, -0.0076, -0.0016],
...     [0.0205, 0.0150, 0.0125, 0.0020],
...     [-0.0052, -0.0060, -0.0018, 0.0012],
...     [0.0097, 0.0100, 0.0063, 0.0008],
...     [-0.0133, -0.0110, -0.0087, -0.0012],
...     [0.0078, 0.0070, 0.0062, -0.0008],
...     [0.0016, 0.0010, 0.0014, 0.0014],
...     [-0.0058, -0.0060, -0.0052, -0.0012],
...     [0.0120, 0.0110, 0.0080, 0.0010],
... ])
>>> labels = ["tech", "industrials", "utilities", "bonds"]
>>> estimate = fl.risk.regression_model(returns, market[:, None], labels=labels, factor_labels=["market"])
>>> for fit, row in zip(estimate.fit, estimate.model.exposures):
...     print(fit.asset, round(row[0], 3), round(fit.r_squared, 3))
tech 1.289 0.987
industrials 1.001 0.983
utilities 0.799 0.984
bonds 0.133 0.562
>>> model = fl.risk.scaled(estimate.model, 252)
>>> risk = fl.risk.factor_risk([0.25, 0.25, 0.25, 0.25], model)
>>> round(risk.volatility, 4), round(risk.factors[0].share, 3), round(risk.specific_share, 3)
(0.1183, 0.993, 0.007)
>>> beta = fl.opt.FactorExposureLimit(factor="market", min=0.8, max=0.8)
>>> problem = fl.opt.Problem.from_factor_model(model, constraints=fl.opt.Constraints(factor_exposures=[beta]))
>>> solution = problem.solve(fl.opt.MinimumVariance())
>>> [round(w, 3) for w in solution.weights]
[0.196, 0.235, 0.355, 0.214]

```

The market explains 99% of the equal-weighted portfolio's risk. The minimum-variance portfolio
with a beta of exactly 0.8 holds less of the high-beta assets than equal weights do, and more of
utilities.

## Validation

`scripts/reference/factors.py` simulates 250 periods of returns for 12 assets from a three-factor
model with correlated factors, and compares:

- the regression model with statsmodels (`OLS` with a constant): exposures and intercepts to
  $10^{-11}$, $R^2$ and specific variances to $10^{-12}$ of the largest value;
- the statistical model with scikit-learn (`PCA` with the full solver): eigenvalues, explained
  variance, loadings and scores to $10^{-11}$;
- the fundamental model with NumPy's `lstsq` on given exposures: factor returns, factor
  covariance, specific variances and $R^2$ to $10^{-12}$;
- the risk decomposition and the implied covariance with their definitions computed in NumPy, to
  $10^{-13}$.

`scripts/reference/optimisation.py` solves four problems on a two-factor model in CVXPY,
formulated independently in factor form: minimum variance with an absolute and an active exposure
limit, mean–variance with a tracking-error limit, maximum Sharpe ratio, and target volatility.
Objectives agree to $10^{-7}$ and weights to $10^{-4}$; where a binding cone leaves the objective
nearly flat, the weights are less tightly determined than the objective. Every objective is also
solved both with the factor model and with its implied covariance matrix, and the two agree to the
same tolerances. The Python and JavaScript tests repeat these checks.

## References

- Sharpe, W. F., "A simplified model for portfolio analysis", *Management Science* 9(2), 1963.
- Fama, E. F. and French, K. R., "Common risk factors in the returns on stocks and bonds",
  *Journal of Financial Economics* 33(1), 1993.
- Connor, G., "The three types of factor models: a comparison of their explanatory power",
  *Financial Analysts Journal* 51(3), 1995.
- Grinold, R. C. and Kahn, R. N., *Active Portfolio Management*, 2nd edition, McGraw-Hill, 2000.
- Golub, G. H. and Van Loan, C. F., *Matrix Computations*, 4th edition, Johns Hopkins University
  Press, 2013 (Householder least squares, and the symmetric QL algorithm).
