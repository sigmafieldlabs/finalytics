# Covariance estimation

From $T$ periods of returns on $N$ assets, $x_{t,i}$, Finalytics estimates the covariance matrix
$\Sigma$ that risk decomposition and optimisation use. The estimate is per period of the returns;
multiply it by the periods in a year to annualise.

## Estimators

With $\bar x_i$ the mean of asset $i$'s returns and $y_{t,i} = x_{t,i} - \bar x_i$:

### Sample

$$\hat\Sigma_{ij} = \frac{1}{T - 1} \sum_t y_{t,i}\, y_{t,j}$$

### Exponentially weighted

The return $k$ periods before the latest has weight $w_t \propto \lambda^k$, with the weights
summing to one; deviations are taken from the weighted mean $\bar x^w_i = \sum_t w_t x_{t,i}$:

$$\hat\Sigma_{ij} = \sum_t w_t \,(x_{t,i} - \bar x^w_i)(x_{t,j} - \bar x^w_j)$$

A decay of $\lambda = 0.94$ for daily returns follows RiskMetrics. The estimate adapts quickly to
changes in volatility, at the cost of using fewer effective observations.

### Shrinkage: Ledoit–Wolf and OAS

Both shrink the empirical covariance $S = \frac1T \sum_t y_t y_t^\top$ towards a multiple of the
identity, $\mu I$ with $\mu = \operatorname{tr}(S)/N$ the mean variance:

$$\hat\Sigma = (1 - \delta)\, S + \delta\, \mu I$$

which is better conditioned than $S$, and invertible even when there are fewer observations than
assets. The intensity $\delta \in [0, 1]$ is reported with the estimate.

- **Ledoit–Wolf** (2004) chooses $\delta$ to minimise the expected squared Frobenius distance to
  the true covariance:
  $\delta = \min(\beta, \gamma)/\gamma$, with
  $\gamma = \frac1N \lVert S - \mu I \rVert_F^2$ and
  $\beta = \frac{1}{N T^2} \sum_t \lVert y_t y_t^\top - S \rVert_F^2$, computed as in
  scikit-learn.
- **OAS**, the oracle approximating shrinkage of Chen, Wiesel, Eldar and Hero (2010), assumes
  normal returns:
  $\delta = \min\!\Big(1, \dfrac{\bar a + \mu^2}{(T + 1)(\bar a - \mu^2 / N)}\Big)$, with
  $\bar a$ the mean of the squared entries of $S$, as in scikit-learn.

With one asset there is nothing to shrink, and $\delta = 0$.

### Correlation

$\rho_{ij} = \Sigma_{ij} / \sqrt{\Sigma_{ii}\Sigma_{jj}}$, which needs every variance to be
greater than zero: an asset with no variance, such as cash in the base currency, has no
correlations, and is reported as an error by name.

## Conventions

- The shrinkage estimators use the divisor $T$ for $S$, like scikit-learn; the sample estimator
  uses $T - 1$.
- Returns are a table with one column per asset and the same number of returns in each, at least
  two; each value must be finite and no lower than −100%.

## Example

```python
>>> import numpy as np
>>> import finalytics as fl
>>> returns = np.array([
...     [0.012, 0.004, -0.001],
...     [-0.008, -0.002, 0.003],
...     [0.015, 0.006, 0.000],
...     [-0.004, 0.001, 0.002],
...     [0.009, 0.003, -0.002],
...     [-0.011, -0.005, 0.004],
... ])
>>> labels = ["equity", "credit", "bonds"]
>>> for method in ["sample", "ewma", "ledoit_wolf", "oas"]:
...     estimate = fl.risk.covariance(returns, labels, method=method, decay=0.9)
...     shrinkage = None if estimate.shrinkage is None else round(estimate.shrinkage, 4)
...     print(method, shrinkage, round(estimate.matrix.values[0] * 1e6, 3))
sample None 124.567
ewma None 104.26
ledoit_wolf 0.082 98.633
oas 0.4428 75.891

```

The first entry is the equity variance, in units of $10^{-6}$. Shrinkage pulls it towards the
average variance of the three assets.

## Validation

`scripts/reference/risk.py` computes each estimator on three tables of factor-model returns: 6
assets over 250 periods, 30 assets over 40 periods (where shrinkage matters most), and 3 assets
over 10 periods. The references are NumPy (`np.cov`) for the sample covariance, pandas
(`ewm(adjust=True).cov(bias=True)`) for the exponentially weighted one, and scikit-learn
(`LedoitWolf` and `OAS`) for the shrinkage estimators and their intensities. Every entry agrees to
$10^{-12}$ of the largest entry in Rust, Python and JavaScript.

## References

- J.P. Morgan and Reuters, *RiskMetrics: Technical Document*, 4th edition, 1996.
- Ledoit, O. and Wolf, M., "A well-conditioned estimator for large-dimensional covariance
  matrices", *Journal of Multivariate Analysis* 88(2), 2004.
- Chen, Y., Wiesel, A., Eldar, Y. C. and Hero, A. O., "Shrinkage algorithms for MMSE covariance
  estimation", *IEEE Transactions on Signal Processing* 58(10), 2010.
- Pedregosa, F. et al., "Scikit-learn: machine learning in Python", *Journal of Machine Learning
  Research* 12, 2011.
