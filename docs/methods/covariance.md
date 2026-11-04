# Covariance and mean returns

From $T$ periods of returns on $N$ assets, $x_{t,i}$, Finalytics estimates the covariance matrix
$\Sigma$ that risk decomposition and optimisation use, and the mean returns $\mu$ that
optimisation expects. The estimates are per period of the returns; `Assets` annualises them with
the number of periods in a year.

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

### Shrinkage towards constant correlation

Ledoit and Wolf's "Honey, I shrunk the sample covariance matrix" (2004) shrinks $S$ towards the
matrix $F$ with the sample variances and every correlation equal to the average sample
correlation $\bar r$, $F_{ij} = \bar r \sqrt{S_{ii} S_{jj}}$ for $i \ne j$:

$$\hat\Sigma = (1 - \delta)\, S + \delta F, \qquad \delta = \max\!\big(0, \min(1, \kappa / T)\big), \quad \kappa = (\pi - \rho)/\gamma$$

with $\pi$, $\rho$ and $\gamma$ the estimates of the paper, computed as in the authors' MATLAB
`covCor`. The target keeps each asset's own variance, so it suits assets whose variances differ
widely, where the identity target of Ledoit–Wolf pulls them together.

### Statistical factor model

The first $k$ principal components of the sample covariance (divisor $T - 1$), $\lambda_j$ and
$v_j$, give the common covariance $\sum_{j \le k} \lambda_j v_j v_j^\top$; each asset's specific
variance is what the components leave of its sample variance, so the variances are kept. This is
the covariance the [statistical factor model](factor-models.md) implies; `factors` is $k$, from
1 to the smaller of $N$ and $T - 1$.

### Denoised

Eigenvalues of a correlation matrix of $N$ assets estimated from $T$ independent returns spread
up to the Marchenko–Pastur edge $\lambda_+ = (1 + \sqrt{N/T})^2$ by chance alone (Laloux,
Cizeau, Bouchaud and Potters, 2000). The sample correlation's eigenvalues below $\lambda_+$ are
replaced by their average, which keeps the trace, the matrix is rescaled to a unit diagonal, and
the sample standard deviations are applied again (the constant residual eigenvalue method of
López de Prado, 2020). The variances are the sample's.

### Nonlinear shrinkage

The analytical nonlinear shrinkage of Ledoit and Wolf (2020) keeps the sample eigenvectors and
shrinks each eigenvalue $\lambda_i$ of the sample covariance (divisor $T - 1$) by a kernel
estimate of the spectral density $\tilde f$ and its Hilbert transform $\tilde H$, with the
Epanechnikov kernel and bandwidth $(T - 1)^{-1/3}$:

$$\tilde d_i = \frac{\lambda_i}{\big(\pi c \lambda_i \tilde f(\lambda_i)\big)^2 + \big(1 - c - \pi c \lambda_i \tilde H(\lambda_i)\big)^2}, \qquad c = \frac{N}{T - 1}$$

With more assets than returns, the $N - T + 1$ null eigenvalues share one value from the
density's transform at zero, as in the paper. It needs at least 13 returns, for the bandwidth to
be below $1/\sqrt5$. The formula follows the authors' MATLAB `analytical_shrinkage`.

### Correlation

$\rho_{ij} = \Sigma_{ij} / \sqrt{\Sigma_{ii}\Sigma_{jj}}$, which needs every variance to be
greater than zero: an asset with no variance, such as cash in the base currency, has no
correlations, and is reported as an error by name.

## Mean returns

`Assets(mean_returns=...)` chooses the estimator of the expected returns that optimisation uses
(the risk-free asset's mean is always its own average, the risk-free rate):

- **Historical** (the default): each asset's average return, $\bar x_i$.
- **James–Stein**: the averages shrunk towards their grand mean $g = \frac1N \sum_i \bar x_i$,
  $\hat\mu = (1 - c)\,\bar x + c\, g$, with the intensity of Efron and Morris (1973) for
  correlated returns,
  $c = \min\!\Big(1, \max\!\Big(0, \dfrac{N - 3}{T\, (\bar x - g)^\top S^{-1} (\bar x - g)}\Big)\Big)$,
  where $S$ is the sample covariance (divisor $T - 1$). It needs more returns than assets, so that
  $S$ can be inverted, and four or more assets to shrink at all. Means far apart relative to
  their estimation error are shrunk little; means within noise of each other, much.
- **Exponentially weighted** (`ewma`): $\hat\mu_i = \sum_t w_t x_{t,i}$ with the return $k$
  periods before the latest weighted in proportion to $\lambda^k$ (0.94 by default), as pandas'
  `ewm(alpha=1 - λ, adjust=True)` gives.

Mean returns are far harder to estimate than covariances: the standard error of a mean is
$\sigma / \sqrt{T}$ whatever the frequency of the returns, so a shrinkage estimator or a
[prior](black-litterman.md) usually does better out of sample than the historical mean.

## Conventions

- Ledoit–Wolf, OAS and shrinkage towards constant correlation use the divisor $T$ for $S$, like
  scikit-learn and the authors' code; the sample, statistical, denoised and nonlinear estimators
  use $T - 1$.
- In JSON a method is its name, such as `"denoised"`, or an object with a `type` and its
  parameters, such as `{"type": "statistical", "factors": 3}`.
- `Assets` checks the estimators chosen on the risky assets' returns when it is built, so that a
  choice the returns cannot support (nonlinear shrinkage with fewer than 13 returns, James–Stein
  with no more returns than assets) is reported at `covariance` or `mean_returns` at once.
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

The estimators of mean returns, on the asset-class dataset:

```python
>>> prices = fl.datasets.asset_class_prices()
>>> for method in ["historical", "james_stein", fl.risk.EwmaMean(decay=0.97)]:
...     assets = fl.Assets(prices, risk_free_asset="T-Bills", mean_returns=method)
...     print({k: round(v, 3) for k, v in assets.mean_returns().items()})
{'US Equity': 0.1, 'Intl Equity': 0.071, 'Bonds': 0.04, 'T-Bills': 0.023, 'Gold': 0.074, 'Real Estate': 0.091}
{'US Equity': 0.097, 'Intl Equity': 0.071, 'Bonds': 0.044, 'T-Bills': 0.023, 'Gold': 0.074, 'Real Estate': 0.09}
{'US Equity': 0.009, 'Intl Equity': 0.002, 'Bonds': -0.005, 'T-Bills': 0.023, 'Gold': 0.113, 'Real Estate': 0.01}

```

James–Stein moves each risky asset's mean a little towards their average of 7.5%; the
exponentially weighted mean, with an effective window of $1/(1 - 0.97) \approx 33$ months,
reflects the most recent returns.

## Validation

`scripts/reference/risk.py` computes each estimator on three tables of factor-model returns: 6
assets over 250 periods, 30 assets over 40 periods (where shrinkage matters most), and 3 assets
over 10 periods. The references are NumPy (`np.cov`) for the sample covariance, pandas
(`ewm(adjust=True).cov(bias=True)`) for the exponentially weighted one, and scikit-learn
(`LedoitWolf` and `OAS`) for the shrinkage estimators and their intensities. Every entry agrees to
$10^{-12}$ of the largest entry in Rust, Python and JavaScript.

`scripts/reference/estimators.py` translates the authors' MATLAB `covCor` and
`analytical_shrinkage` to NumPy, and writes the statistical, denoised, James–Stein and
exponentially weighted estimates from `numpy.linalg` and pandas, on 8 assets over 120 returns
and 30 assets over 20 returns. Shrinkage towards constant correlation agrees to $10^{-12}$ of the
largest entry, with its intensity; the statistical and denoised estimates to $10^{-10}$; the
means to $10^{-12}$. Nonlinear shrinkage agrees to $10^{-10}$ with more returns than assets and to
$10^{-7}$ with fewer, where the estimator itself is ill-conditioned: changing the returns by one
part in $10^{14}$ moves the reference by about $3 \times 10^{-7}$ of its largest entry.

## References

- J.P. Morgan and Reuters, *RiskMetrics: Technical Document*, 4th edition, 1996.
- Ledoit, O. and Wolf, M., "A well-conditioned estimator for large-dimensional covariance
  matrices", *Journal of Multivariate Analysis* 88(2), 2004.
- Ledoit, O. and Wolf, M., "Honey, I shrunk the sample covariance matrix", *Journal of
  Portfolio Management* 30(4), 2004.
- Ledoit, O. and Wolf, M., "Analytical nonlinear shrinkage of large-dimensional covariance
  matrices", *Annals of Statistics* 48(5), 2020.
- Chen, Y., Wiesel, A., Eldar, Y. C. and Hero, A. O., "Shrinkage algorithms for MMSE covariance
  estimation", *IEEE Transactions on Signal Processing* 58(10), 2010.
- Laloux, L., Cizeau, P., Bouchaud, J.-P. and Potters, M., "Random matrix theory and financial
  correlations", *International Journal of Theoretical and Applied Finance* 3(3), 2000.
- López de Prado, M., *Machine Learning for Asset Managers*, Cambridge University Press, 2020.
- Efron, B. and Morris, C., "Stein's estimation rule and its competitors: an empirical Bayes
  approach", *Journal of the American Statistical Association* 68(341), 1973.
- Pedregosa, F. et al., "Scikit-learn: machine learning in Python", *Journal of Machine Learning
  Research* 12, 2011.
