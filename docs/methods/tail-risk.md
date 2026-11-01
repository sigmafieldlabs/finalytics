# Value at risk and expected shortfall

At confidence level $c$ (for example 95%), with tail probability $\alpha = 1 - c$, the **value at
risk** is the loss exceeded with probability $\alpha$ over one period of the returns, and the
**expected shortfall** (also called conditional value at risk) is the expected loss when that
happens. Both are reported as **positive losses**: a value at risk of 0.021 means a loss of 2.1%
or more in one period in twenty, at 95%.

## Methods

### Historical

From the $n$ observed returns sorted in ascending order, $r_{(1)} \le \dots \le r_{(n)}$:

- value at risk is minus the $\alpha$ quantile, interpolated linearly between order statistics
  at position $h = (n - 1)\alpha$ (counting from 0): $-\big(r_{(\lfloor h \rfloor)} + (h - \lfloor h \rfloor)(r_{(\lfloor h \rfloor + 1)} - r_{(\lfloor h \rfloor)})\big)$;
- expected shortfall is minus the mean of the worst $\lfloor (n - 1)\alpha \rfloor + 1$ returns.

### Parametric (normal)

With the sample mean $\mu$ and standard deviation $\sigma$, and $z = \Phi^{-1}(\alpha)$:

$$\text{VaR} = -(\mu + \sigma z) \qquad \text{ES} = -\Big(\mu - \sigma \frac{\varphi(z)}{\alpha}\Big)$$

### Cornish–Fisher

The normal quantile is adjusted for the skewness $S$ and excess kurtosis $K$ of the returns
(population moments):

$$h = z + \frac{(z^2 - 1) S}{6} + \frac{(z^3 - 3z) K}{24} - \frac{(2z^3 - 5z) S^2}{36} \qquad \text{VaR} = -(\mu + \sigma h)$$

Expected shortfall integrates the Edgeworth expansion of the density,
$e(x) = \varphi(x)\big(1 + \tfrac{S}{6}\mathrm{He}_3(x) + \tfrac{K}{24}\mathrm{He}_4(x) + \tfrac{S^2}{72}\mathrm{He}_6(x)\big)$,
up to $h$, following Boudt, Peterson and Croux (2008):

$$E = \frac{1}{\alpha}\int_{-\infty}^{h} x\, e(x)\,dx \qquad \text{ES} = -\big(\mu + \sigma \min(E, h)\big)$$

The integral has a closed form through $I_k = \int_{-\infty}^{h} x^k \varphi(x)\,dx$, with
$I_0 = \Phi(h)$, $I_1 = -\varphi(h)$ and $I_k = -h^{k-1}\varphi(h) + (k - 1) I_{k-2}$. Taking the
minimum with $h$ keeps the expected shortfall at least as large as the value at risk, which the
expansion alone does not guarantee when the skewness or kurtosis is large.

### The normal distribution

$\Phi$ is computed from the complementary error function, and $\Phi^{-1}$ by Acklam's rational
approximation refined by two Halley steps against $\Phi$, which brings it to within a few units
in the last place.

## Conventions

- Results are for one period of the returns supplied; scale or aggregate the returns for longer
  horizons.
- The parametric and Cornish–Fisher methods use the sample standard deviation (divisor $n - 1$)
  with population skewness and kurtosis.
- At least two returns are needed, and the confidence level must be strictly between 0 and 1.

## Example

```python
>>> import finalytics as fl
>>> monthly = [0.021, -0.013, 0.034, 0.008, -0.042, 0.017, 0.026, -0.009, 0.012, 0.031, -0.024, 0.015]
>>> for method in ["historical", "parametric", "cornish_fisher"]:
...     tail = fl.risk.tail_risk(monthly, confidence=0.9, method=method)
...     print(method, round(tail.value_at_risk, 6), round(tail.expected_shortfall, 6))
historical 0.0229 0.033
parametric 0.023758 0.034875
cornish_fisher 0.02731 0.038172

```

The returns are negatively skewed, so the Cornish–Fisher estimates are larger than the normal
ones.

## Validation

`scripts/reference/risk.py` computes the historical estimates with empyrical
(`value_at_risk` and `conditional_value_at_risk`), the parametric ones with SciPy, and the
Cornish–Fisher expected shortfall by numerical integration of the Edgeworth density with
`scipy.integrate.quad`, independently of the closed form, for three series at 95% and 99%.
The Rust tests require agreement to $10^{-12}$ for value at risk and $10^{-10}$ for expected
shortfall, and the normal quantile to agree with `scipy.stats.norm.ppf` to $10^{-14}$ from
$p = 10^{-300}$ to $p = 0.999$; the Python and JavaScript tests check the same fixture.

## References

- Cornish, E. A. and Fisher, R. A., "Moments and cumulants in the specification of
  distributions", *Revue de l'Institut International de Statistique* 5(4), 1938.
- Favre, L. and Galeano, J.-A., "Mean-modified value-at-risk optimization with hedge funds",
  *Journal of Alternative Investments* 5(2), 2002.
- Boudt, K., Peterson, B. G. and Croux, C., "Estimation and decomposition of downside risk for
  portfolios with non-normal returns", *Journal of Risk* 11(2), 2008.
- Acklam, P. J., "An algorithm for computing the inverse normal cumulative distribution
  function", 2003.
