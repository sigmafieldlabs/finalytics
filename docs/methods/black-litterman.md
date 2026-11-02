# Black–Litterman

Mean–variance optimisation is very sensitive to its expected returns, and historical means are
poor estimates of them. The Black–Litterman model (Black and Litterman, 1992) starts instead from
the returns that make the market portfolio optimal, and moves them towards the investor's views
in proportion to how confident the views are. The result is a set of expected returns, and a
covariance, to optimise with.

## The model

With $\Sigma$ the covariance of the assets' returns:

- **Prior.** The equilibrium returns implied by market weights $w_m$ (scaled to sum to one), a
  risk aversion $\delta$ and a risk-free rate $r_f$ are $\pi = \delta \Sigma w_m + r_f$: the
  returns at which an investor with that risk aversion would hold the market portfolio. A prior
  can also be given directly.
- **Views.** View $k$ states that portfolio $p_k$ (a row of the matrix $P$) has expected return
  $q_k$, with variance $\omega_k$. An absolute view on one asset has a single weight of 1; a
  relative view, such as "A outperforms B by 2%", has weights $+1$ and $-1$.
- **Posterior.** With $\Omega = \operatorname{diag}(\omega)$ and $\tau$ the uncertainty of the
  prior relative to $\Sigma$ (0.05 by default):

$$\mu_{BL} = \pi + \tau \Sigma P^\top (P \tau \Sigma P^\top + \Omega)^{-1} (q - P \pi)$$

$$\Sigma_{BL} = \Sigma + \tau \Sigma - \tau \Sigma P^\top (P \tau \Sigma P^\top + \Omega)^{-1} P \tau \Sigma$$

$\Sigma_{BL}$ adds the remaining uncertainty of the expected returns to $\Sigma$ (He and
Litterman, 2002). Without views, $\mu_{BL} = \pi$ and $\Sigma_{BL} = (1 + \tau)\Sigma$.

### The uncertainty of a view

| Uncertainty | $\omega_k$ |
| --- | --- |
| Proportional (the default) | $\tau\, p_k^\top \Sigma p_k$, the variance of the view portfolio scaled like the prior (He and Litterman, 1999) |
| Confidence $c \in (0, 1]$ | $\frac{1 - c}{c}\, \tau\, p_k^\top \Sigma p_k$ (Idzorek, 2005, in the closed form of Walters, 2014) |
| Variance | $\omega_k$ as given |

A confidence of 1 holds the view exactly, 0.5 gives the proportional uncertainty, and lower
confidences move the posterior less. Views held with certainty must be independent of one another:
two certain views on the same portfolio with different returns cannot both hold, and are reported
as an error, as are views so extreme that the posterior cannot be computed in floating point.

### Implied risk aversion

The market's risk aversion can be estimated from its returns as
$\delta = (\bar r P - r_f) / (s^2 P)$, with $\bar r$ and $s^2$ the mean and sample variance of the
market's returns and $P$ the periods in a year (`implied_risk_aversion`). Values between 2 and 4
are common.

## Example

```python
>>> import finalytics as fl
>>> labels = ["us equity", "intl equity", "bonds"]
>>> covariance = fl.risk.matrix(
...     [[0.0256, 0.02304, 0.0008], [0.02304, 0.0324, 0.0009], [0.0008, 0.0009, 0.0025]], labels
... )
>>> view = fl.opt.View(
...     portfolio={"intl equity": 1.0, "us equity": -1.0},
...     expected_return=0.02,
...     uncertainty=fl.opt.Confidence(confidence=0.6),
... )
>>> estimate = fl.opt.black_litterman(covariance, [view], market_weights=[0.55, 0.25, 0.20], risk_aversion=2.5)
>>> [round(r, 4) for r in estimate.prior_returns]
[0.05, 0.0524, 0.0029]
>>> [round(r, 4) for r in estimate.posterior_returns]
[0.0477, 0.0607, 0.003]
>>> solution = fl.opt.Problem.from_black_litterman(estimate).solve(fl.opt.MeanVariance(risk_aversion=2.5))
>>> [round(w, 3) for w in solution.weights]
[0.177, 0.588, 0.235]

```

At equilibrium, international equities return only 0.24% more than US equities. The view that
they outperform by 2% moves the posterior gap to 1.3%, about 60% of the way, as the confidence of
0.6 implies. The portfolio tilts towards international equities, but keeps the market's bond weight
roughly where it was, as no view concerns bonds.

## Validation

`scripts/reference/black_litterman.py` compares five cases on six assets with PyPortfolioOpt's
`BlackLittermanModel`: absolute views with proportional uncertainty, relative and portfolio views,
Idzorek confidences, given view variances with given prior returns, and a view held with
certainty. Prior and posterior returns, the posterior covariance and the view variances agree to
$10^{-13}$ of the largest value; the implied risk aversion agrees with
`market_implied_risk_aversion` to $10^{-12}$. The Python and JavaScript tests repeat the checks.

## References

- Black, F. and Litterman, R., "Global portfolio optimization", *Financial Analysts Journal*
  48(5), 1992.
- He, G. and Litterman, R., "The intuition behind Black–Litterman model portfolios", Goldman Sachs
  Investment Management Research, 1999.
- Idzorek, T., "A step-by-step guide to the Black–Litterman model", 2005.
- Walters, J., "The Black–Litterman model in detail", 2014.
- Martin, R. A., "PyPortfolioOpt: portfolio optimization in Python", *Journal of Open Source
  Software* 6(61), 2021.
