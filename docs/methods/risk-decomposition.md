# Risk decomposition

Risk decomposition splits a portfolio's volatility into the part each asset contributes, so that
the parts add up to the whole: the Euler allocation of volatility.

## Definitions

For weights $w$ and covariance $\Sigma$, the portfolio's volatility is
$\sigma = \sqrt{w^\top \Sigma w}$. Because $\sigma$ is homogeneous of degree one in $w$, Euler's
theorem gives $\sigma = \sum_i w_i\, \partial\sigma/\partial w_i$, and for each asset:

| Measure | Definition |
| --- | --- |
| Marginal risk | $\partial \sigma / \partial w_i = (\Sigma w)_i / \sigma$ |
| Component risk | $w_i (\Sigma w)_i / \sigma$; the components add up to $\sigma$ |
| Share | Component over $\sigma$; the shares add up to one |

An asset with a negative share reduces the portfolio's risk: it hedges the others at the margin.

**Ex-ante tracking error** against benchmark weights $b$ is the volatility of the active weights,
$\sqrt{(w - b)^\top \Sigma (w - b)}$, and decomposes the same way with $w - b$ in place of $w$.

Components and shares add up by any grouping, such as asset class, region or currency, with
`group_risk`.

### From a portfolio's books

`Books.weights(date)` gives each holding's share of the portfolio's value, and
`Books.holding_returns(holdings, dates)` the holdings' returns in the base currency between
consecutive dates: an instrument's price return converted at each date's rate, and a foreign cash
balance's change in its exchange rate (zero for base-currency cash). A covariance matrix estimated
from those returns decomposes the portfolio's risk by holding. Supply total-return prices to
include income in the returns.

## Conventions

- Volatility is in the units of the covariance matrix: annualise the matrix to annualise the
  decomposition.
- When the portfolio has no volatility (for example, all in base-currency cash), marginal risks
  and shares are None and components are zero.
- Weights need not sum to one: active weights, long-short portfolios and leverage are handled the
  same way.

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
>>> estimate = fl.risk.covariance(returns, ["equity", "credit", "bonds"], method="ledoit_wolf")
>>> annual = fl.risk.scaled(estimate.matrix, 252)
>>> risk = fl.risk.risk_decomposition([0.5, 0.3, 0.2], annual)
>>> round(risk.volatility, 6)
0.089748
>>> for c in risk.contributions:
...     print(c.label, round(c.marginal, 6), round(c.component, 6), round(c.share, 6))
equity 0.156194 0.078097 0.870182
credit 0.056543 0.016963 0.189007
bonds -0.026561 -0.005312 -0.059189
>>> te = fl.risk.tracking_error_decomposition([0.5, 0.3, 0.2], [0.4, 0.3, 0.3], annual)
>>> round(te.volatility, 6)
0.018961

```

Half the portfolio is in equity but equity carries 87% of its risk; the bonds, negatively
correlated with the rest in these returns, take risk away.

## Validation

`crates/finalytics-risk/src/decomposition.rs` checks the components and shares against
hand-computed values and their sums against the volatility to $10^{-15}$;
`crates/finalytics-risk/tests/holdings.rs` decomposes a multi-currency portfolio built from its
books, checking that the weights add up across accounts, that the returns of foreign holdings
and cash are in the base currency, and that the decomposition by asset class adds up. The Python
and JavaScript tests repeat the check on the shared scenario.

## References

- Litterman, R., "Hot spots and hedges", *Journal of Portfolio Management*, special issue,
  December 1996.
- Tasche, D., "Capital allocation to business units and sub-portfolios: the Euler principle",
  in *Pillar II in the New Basel Accord*, Risk Books, 2008 (arXiv:0708.2542).
