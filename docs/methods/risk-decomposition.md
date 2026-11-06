# Risk decomposition

Risk decomposition splits a portfolio's volatility, or its expected shortfall, into the part each
asset contributes, so that the parts add up to the whole: the Euler allocation.

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

**Expected shortfall** at confidence $c$, over historical scenarios $r_{it}$, is the mean loss in
the worst share $\alpha = 1 - c$ of them. With the scenarios sorted by the portfolio's loss
$L_t = -\sum_i w_i r_{it}$, the worst $\lfloor \alpha T \rfloor$ have weight $p_t = 1/(\alpha T)$ and
the next the rest, $p_t = (\alpha T - \lfloor \alpha T \rfloor)/(\alpha T)$, so that
$\text{ES} = \sum_t p_t L_t$, the Rockafellar–Uryasev tail mean that minimum CVaR minimises. It is
also homogeneous of degree one, and each asset's component is $w_i \sum_t p_t (-r_{it})$: its
weight times its mean loss in the tail. `shortfall_decomposition` returns them; they add up to
the expected shortfall exactly. The minimum-CVaR optimisation report gives each asset's share of
the expected shortfall, rather than of the variance. The historical expected shortfall of
[tail risk](tail-risk.md) averages whole returns, the worst $\lfloor (T - 1)\alpha \rfloor + 1$, so
the two differ slightly unless $\alpha T$ is a whole number or less than one.

Components and shares add up by any grouping, such as asset class, region or currency, with
`group_risk`, or `group_contributions` for either decomposition.

### From a portfolio

`Portfolio.weights(date)` gives each holding's share of the portfolio's value, and
`Portfolio.holding_returns(holdings, dates)` the holdings' returns in the base currency between
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

Each asset's part of the expected shortfall, at 75%, over four scenarios:

```python
>>> scenarios = [[-0.08, 0.01], [0.02, 0.0], [0.03, 0.005], [-0.01, -0.004]]
>>> es = fl.risk.shortfall_decomposition([0.6, 0.4], scenarios, ["equity", "bonds"], confidence=0.75)
>>> round(es.expected_shortfall, 6), [round(c.component, 6) for c in es.contributions]
(0.044, [0.048, -0.004])

```

## Validation

`crates/finalytics-risk/src/decomposition.rs` checks the components and shares against
hand-computed values and their sums against the volatility to $10^{-15}$;
`crates/finalytics-risk/tests/holdings.rs` decomposes a multi-currency portfolio built from its
books, checking that the weights add up across accounts, that the returns of foreign holdings
and cash are in the base currency, and that the decomposition by asset class adds up. The Python
and JavaScript tests repeat the check on the shared scenario. The contributions to expected
shortfall are checked by the Euler property itself: `scripts/reference/downside.py` finds each
asset's weight times the derivative of the expected shortfall, by central differences of its
linear programme solved with SciPy, and `crates/finalytics-risk/tests/downside.rs` requires
agreement to $10^{-8}$.

## References

- Litterman, R., "Hot spots and hedges", *Journal of Portfolio Management*, special issue,
  December 1996.
- Tasche, D., "Capital allocation to business units and sub-portfolios: the Euler principle",
  in *Pillar II in the New Basel Accord*, Risk Books, 2008 (arXiv:0708.2542).
