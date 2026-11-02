# Portfolio optimisation

An optimisation problem holds the assets' covariance matrix $\Sigma$ or a
[factor model](factor-models.md), their expected returns $\mu$ (in the same units, for example
both annualised), scenario returns for minimum CVaR, and constraints. Finalytics formulates each objective as a convex conic programme and solves it with
Clarabel, an interior-point solver, or, for risk budgeting and hierarchical risk parity, with
methods of their own.

## Constraints

| Constraint | Form | Default |
| --- | --- | --- |
| Budget | $\sum_i w_i = B$ | $B = 1$ |
| Bounds | $l_i \le w_i \le u_i$, for every asset or per asset | $0 \le w_i \le 1$ (long-only) |
| Groups | $l_g \le \sum_{i \in g} w_i \le u_g$ | none |
| Leverage | $\sum_i \lvert w_i \rvert \le L$ | none |
| Turnover | $\sum_i \lvert w_i - w^0_i \rvert \le T$ from current weights $w^0$ | none |
| Trading costs | $c^\top \lvert w - w^0 \rvert$ subtracted from the expected return | none |
| Tracking error | $\sqrt{(w - b)^\top \Sigma (w - b)} \le \text{TE}$ against benchmark weights $b$ | none |
| Factor exposures | $m_k \le (B^\top w)_k \le M_k$, or on active exposures $(B^\top (w - b))_k$; needs a factor model | none |

Absolute values are linearised with auxiliary variables $d_i \ge \pm(w_i - w^0_i)$, and the
tracking-error limit is a second-order cone on a factor $R$ of the covariance ($R^\top R =
\Sigma$, by Cholesky factorisation with pivoting, which also handles riskless assets). With a
factor model, the variance is written in factor form, $y^\top F y + \sum_i d_i w_i^2$ with
$y = B^\top w$, and the cone is built from a factor of $F$ and the specific volatilities; see
[factor models](factor-models.md).

## Objectives

| Objective | Problem | Formulation |
| --- | --- | --- |
| `minimum_variance` | $\min w^\top \Sigma w$ | Quadratic programme |
| `mean_variance` | $\max \mu^\top w - \frac{\lambda}{2} w^\top \Sigma w$ | Quadratic programme |
| `target_return` | $\min w^\top \Sigma w$ subject to $\mu^\top w \ge R^*$ | Quadratic programme |
| `target_volatility` | $\max \mu^\top w$ subject to $\lVert R w \rVert \le \sigma^*$ | Second-order cone programme |
| `maximum_sharpe` | $\max (\mu^\top w - r_f) / \sqrt{w^\top \Sigma w}$ | Quadratic programme after the Charnes–Cooper transformation |
| `maximum_diversification` | $\max \sigma^\top w / \sqrt{w^\top \Sigma w}$, with $\sigma_i = \sqrt{\Sigma_{ii}}$ | As maximum Sharpe |
| `minimum_cvar` | $\min$ CVaR of the scenario returns | Linear programme |
| `risk_budgeting` | Each asset's share of risk equals its budget | Newton's method |
| `hierarchical_risk_parity` | Clustering and recursive bisection | No optimisation |

### Maximum Sharpe ratio and diversification

The ratio is not concave, but the Charnes–Cooper transformation makes it so. With $y = \kappa w$
and $\kappa > 0$ chosen so that $(\mu - r_f)^\top y = 1$ (or $\sigma^\top y = 1$), the problem
becomes $\min y^\top \Sigma y$, with every constraint's constant multiplied by $\kappa$; then
$w = y / \kappa$. This is exact for linear and second-order cone constraints. When no feasible
portfolio earns more than $r_f$, the problem is infeasible, and the solution says so.

### Minimum CVaR

Following Rockafellar and Uryasev (2000), over $S$ scenarios $r_s$ at tail probability $\alpha$:

$$\min_{w,\,\zeta,\,u} \; \zeta + \frac{1}{\alpha S}\sum_s u_s \quad\text{subject to}\quad u_s \ge -r_s^\top w - \zeta,\; u_s \ge 0$$

At the optimum $\zeta$ is the value at risk and the objective the CVaR (expected shortfall) of the
portfolio's scenario losses. Several portfolios can share the minimum.

### Risk budgeting and risk parity

Following Spinu (2013), the weights whose risk shares equal budgets $b$ (summing to one) come from
the strictly convex problem

$$\min_{x > 0}\; \tfrac12 x^\top \Sigma x - \sum_i b_i \ln x_i \qquad w = x / \textstyle\sum_i x_i$$

whose optimum satisfies $x_i (\Sigma x)_i = b_i$. It is solved by Newton's method with a
backtracking line search, from inverse-volatility weights scaled so that $x^\top \Sigma x = 1$,
until the Newton decrement is below $10^{-24}$; if Newton's method fails, the same problem is
solved as an exponential-cone programme. Equal budgets give risk parity. The solution reports the
largest difference between an asset's share of risk and its budget.

### Hierarchical risk parity

Following López de Prado (2016):

1. the distance between assets is $d_{ij} = \sqrt{(1 - \rho_{ij})/2}$;
2. assets are clustered agglomeratively, by single (default), complete, average or Ward
   linkage, and ordered by the leaves of the tree, left branch first, with the earlier-formed
   cluster on the left (as SciPy numbers clusters);
3. the ordered list is bisected recursively; at each split the two halves share their parent's
   weight in inverse proportion to their variances under inverse-variance weights within each
   half.

Risk budgeting and hierarchical risk parity give fully invested, long-only portfolios and take no
other constraints; assets with no variance must be removed first.

### Efficient frontier

`frontier(points)` solves `target_return` for targets evenly spaced from the minimum-variance
portfolio's expected return to the largest attainable under the constraints (found by
maximising the expected return), and returns each portfolio's expected return, volatility and weights.

## Reporting

Every solution reports its **status** (`optimal`, `nearly_optimal`, `infeasible`, `unbounded`,
`iteration_limit` or `numerical_difficulty`), the weights when there is a solution, the
objective's value, the expected return and volatility, the **largest constraint violation** of
the weights returned, the method, the iterations and the time taken. A problem whose constraints
cannot all hold is not an error: its solution has the status `infeasible` and no weights.

Clarabel runs at its default tolerances ($10^{-8}$ on the duality gap and feasibility), so
interior-point weights are accurate to about $10^{-5}$.

## Example

```python
>>> import finalytics as fl
>>> from finalytics import opt
>>> covariance = [[0.0400, 0.0060, 0.0020], [0.0060, 0.0144, 0.0015], [0.0020, 0.0015, 0.0036]]
>>> problem = opt.Problem.from_covariance(
...     covariance, labels=["equity", "credit", "bonds"], expected_returns=[0.07, 0.045, 0.03]
... )
>>> sharpe = problem.solve(opt.MaximumSharpe(risk_free_rate=0.02))
>>> sharpe.status, [round(w, 3) for w in sharpe.weights], round(sharpe.objective_value, 4)
('optimal', [0.255, 0.294, 0.451], 0.3092)
>>> sharpe.max_violation < 1e-7
True
>>> parity = problem.solve(opt.RiskBudgeting())
>>> parity.method, [round(w, 4) for w in parity.weights], parity.objective_value < 1e-12
('newton', [0.1659, 0.2711, 0.563], True)
>>> hrp = problem.solve(opt.HierarchicalRiskParity())
>>> [round(w, 4) for w in hrp.weights]
[0.0577, 0.1602, 0.7821]

```

With constraints, and when they cannot all hold:

```python
>>> constrained = opt.Problem.from_covariance(
...     covariance,
...     labels=["equity", "credit", "bonds"],
...     expected_returns=[0.07, 0.045, 0.03],
...     constraints=opt.Constraints(
...         bounds=opt.Bound(min=0.1, max=0.6),
...         groups=[opt.GroupLimit(name="credit risk", assets=["equity", "credit"], min=0.0, max=0.5)],
...     ),
... )
>>> [round(w, 3) for w in constrained.solve(opt.MaximumSharpe(risk_free_rate=0.02)).weights]
[0.238, 0.262, 0.5]
>>> tight = opt.Problem.from_covariance(
...     covariance, labels=["equity", "credit", "bonds"], constraints=opt.Constraints(bounds=opt.Bound(min=0, max=0.3))
... )
>>> found = tight.solve(opt.MinimumVariance())
>>> found.status, found.weights
('infeasible', None)

```

## Validation

`scripts/reference/optimisation.py` produces nine reference solutions on an 8-asset problem.
Seven are formulated independently in CVXPY and solved to tolerances of $10^{-10}$: minimum
variance; mean–variance with a group and an asset bound; long-short target return with a
leverage limit; target volatility with a turnover limit; mean–variance with trading costs and a
tracking-error limit; maximum diversification; and minimum CVaR (as a linear programme solved by
HiGHS). The other two, maximum Sharpe portfolios with and without bounds and a group, come from
PyPortfolioOpt, as do the hierarchical risk parity weights for all four linkages on an 8-asset
and a 20-asset matrix. The tests require objective values to agree
to $10^{-7}$, weights to $2 \times 10^{-5}$ (CVaR portfolios are compared by objective only), and
hierarchical risk parity weights to $10^{-13}$, in Rust, Python and JavaScript. Risk budgeting is
checked by its defining property, risk shares equal to the budgets to $10^{-12}$, and against
the exponential-cone formulation.

## References

- Markowitz, H., "Portfolio selection", *Journal of Finance* 7(1), 1952.
- Charnes, A. and Cooper, W. W., "Programming with linear fractional functionals", *Naval
  Research Logistics Quarterly* 9(3–4), 1962.
- Choueifaty, Y. and Coignard, Y., "Toward maximum diversification", *Journal of Portfolio
  Management* 35(1), 2008.
- Rockafellar, R. T. and Uryasev, S., "Optimization of conditional value-at-risk", *Journal of
  Risk* 2(3), 2000.
- Spinu, F., "An algorithm for computing risk parity weights", SSRN 2297383, 2013.
- López de Prado, M., "Building diversified portfolios that outperform out of sample", *Journal
  of Portfolio Management* 42(4), 2016.
- Goulart, P. J. and Chen, Y., "Clarabel: an interior-point solver for conic programs with
  quadratic objectives", arXiv:2405.12762, 2024.
- Diamond, S. and Boyd, S., "CVXPY: a Python-embedded modeling language for convex optimization",
  *Journal of Machine Learning Research* 17(83), 2016.
- Martin, R. A., "PyPortfolioOpt: portfolio optimization in Python", *Journal of Open Source
  Software* 6(61), 2021.
