# Portfolio optimisation

An optimisation problem holds the assets' covariance matrix $\Sigma$ or a
[factor model](factor-models.md), their expected returns $\mu$ (in the same units, for example
both annualised), scenario returns for the downside objectives (minimum CVaR, semivariance and
CDaR), and constraints. Finalytics formulates each objective as a convex conic programme and
solves it with
Clarabel, an interior-point solver, or, for risk budgeting and hierarchical risk parity, with
methods of their own.

## Constraints

| Constraint | Form | Default |
| --- | --- | --- |
| Budget | $\sum_i w_i = B$ | $B = 1$ |
| Bounds | $l_i \le w_i \le u_i$, for every asset or per asset | $0 \le w_i \le 1$ (long-only) |
| Groups | $l_g \le \sum_{i \in g} w_i \le u_g$ | none |
| Leverage | $\sum_i \lvert w_i \rvert \le L$ | none |
| Turnover | $\tfrac12 \sum_i \lvert w_i - w^0_i \rvert \le T$ (one-way: the share bought, or sold) from current weights $w^0$ | none |
| Trading costs | $c^\top \lvert w - w^0 \rvert$ subtracted from the expected return | none |
| Quadratic costs | $\sum_i k_i (w_i - w^0_i)^2$, such as market impact, subtracted from the expected return of mean–variance, robust mean–variance and target volatility | none |
| Trade limits | Each $\lvert w_i - w^0_i \rvert$ either 0 or at least $m$; at most $K$ assets traded; a heuristic, below | none |
| Tracking error | $\sqrt{(w - b)^\top \Sigma (w - b)} \le \text{TE}$ against benchmark weights $b$ | none |
| Factor exposures | $m_k \le (B^\top w)_k \le M_k$, or on active exposures $(B^\top (w - b))_k$; needs a factor model | none |
| Expected return | $\mu^\top w - c^\top \lvert w - w^0 \rvert \ge R^*$, a floor for objectives that do not use $\mu$, such as the downside ones (`min_return`) | none |
| Effective number of assets | $1 / \sum_i w_i^2 \ge k$, as $\lVert w \rVert_2 \le 1/\sqrt{k}$ (`min_effective_assets`) | none |

The effective number of assets, $1/\sum_i w_i^2$, is $n$ for equal weights and 1 for a single
asset; holding it at or above $k$ is the norm constraint of DeMiguel, Garlappi, Nogales and Uppal
(2009), which keeps an optimiser from concentrating on the assets whose estimates happen to look
best. It is one second-order cone, and the same $k$ means the same spread for every objective,
unlike a penalty $\gamma \lVert w \rVert^2$ whose effect depends on the scale of the objective.
`Assets` checks before solving that the bounds leave room for $k$ effective assets: the most
they allow is that of the weights nearest equal, $w_i = \min(u_i, \max(l_i, \theta))$ with
$\theta$ such that the weights sum to one.

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
| `robust_mean_variance` | $\max \min_{\mu \in U} \mu^\top w - \frac{\lambda}{2} w^\top \Sigma w$ over an ellipsoid or a box $U$; see [robust optimisation](robust-optimisation.md) | Second-order cone or quadratic programme |
| `target_return` | $\min w^\top \Sigma w$ subject to $\mu^\top w \ge R^*$ | Quadratic programme |
| `target_volatility` | $\max \mu^\top w$ subject to $\lVert R w \rVert \le \sigma^*$ | Second-order cone programme |
| `maximum_sharpe` | $\max (\mu^\top w - r_f) / \sqrt{w^\top \Sigma w}$ | Quadratic programme after the Charnes–Cooper transformation |
| `maximum_diversification` | $\max \sigma^\top w / \sqrt{w^\top \Sigma w}$, with $\sigma_i = \sqrt{\Sigma_{ii}}$ | As maximum Sharpe |
| `minimum_cvar` | $\min$ CVaR of the scenario returns | Linear programme |
| `minimum_semivariance` | $\min \frac1S \sum_s \min(r_s^\top w, 0)^2$ | Quadratic programme |
| `minimum_cdar` | $\min$ CDaR of the cumulative scenario returns | Linear programme |
| `equal_weight` | $w_i = 1/n$ | Closed form |
| `inverse_volatility` | $w_i \propto 1/\sigma_i$ | Closed form |
| `risk_budgeting` | Each asset's share of risk, $w_i (\Sigma w)_i / w^\top \Sigma w$, equals its budget; equal shares by default, as the method `risk_parity` gives | Newton's method |
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

### Minimum semivariance and CDaR

The semivariance below zero is the mean square of the scenario returns that fall below zero, over
every scenario, the square of the downside deviation that the
[statistics](statistics.md) report (against a target of zero). With $d_s \ge \max(-r_s^\top w, 0)$
it is the quadratic programme $\min \frac1S \sum_s d_s^2$.

The conditional drawdown at risk of Chekhlov, Uryasev and Zabarankin (2005) is the
Rockafellar–Uryasev tail mean of the drawdowns after each scenario, at confidence $1 - \alpha$.
The drawdowns are of the cumulative sum of the returns, $c_s = \sum_{t \le s} r_t^\top w$, from a
start of zero, which keeps the programme linear:

$$\min \; \zeta + \frac{1}{\alpha S}\sum_s u_s \quad\text{subject to}\quad u_s \ge p_s - c_s - \zeta,\; u_s \ge 0,\; p_s \ge c_s,\; p_s \ge p_{s-1},\; p_1 \ge 0$$

where $p_s$ is the running peak. The [statistics](statistics.md) measure drawdowns on the
compounded value instead; for returns of a few percent a period the two are close, but not equal.

With `target`, each of the three downside objectives keeps an annual expected return of at least
`target`, as `target_return` does for variance. As with minimum CVaR, the linear programme of CDaR
can have several optimal portfolios.

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

### Trade limits

A minimum trade size and a maximum number of trades are not convex, so they are met by a
heuristic, reported as the method `trade_elimination`. The problem is solved without them; then
every trade below the minimum and, beyond the largest number allowed, the smallest of the
remaining trades are left out by fixing those assets at their current weights, and the rest is
re-optimised. This repeats until the limits hold. The result satisfies the limits, but it is not
guaranteed to be the best portfolio that does, and fixing assets can make the other constraints
infeasible. A change in weight below $10^{-7}$ counts as no trade.

`Assets.optimise` and the frontier give the solver's weights exactly where the solver is only
close: each within its bounds, one within $10^{-7}$ of the weight held now (with
`current_weights`) at that weight, another within $10^{-7}$ of zero at zero, and the others
scaled, within their bounds, to add up to one. The expected return and volatility are those of
the weights given. So the engine's weights are always valid weights: a target for rebalancing,
the current weights of another optimisation, or fixed weights.

### Group limits by class

A group limit names its assets, or a classification of them: the group is then the assets
classed as its name, such as `Equity` under `asset_class`, at most 60% together. Assets take
their classes from a portfolio's instruments (`Portfolio.assets()`), or as given (`classes`, such
as `{"asset_class": {"US Equity": "Equity", "Bonds": "Fixed income"}}`). A classification or class
that no asset has is refused, naming those there are.

### Trade and tracking-error limits on assets

`Assets.optimise` takes these limits with plain inputs: `current_weights`, the weights held now
by asset, adding up to one with the cash (below); `max_turnover`, the one-way
turnover from them; `min_trade` and `max_trades`, the trade limits above; and
`benchmark_weights` with `max_tracking_error`, an annual limit on the tracking error against a
benchmark. The turnover counts the cash: an optimisation invests the cash it does not choose
(all of it, without limits on cash), so a weight $c$ held in that cash takes $c/2$ of the
turnover, and the assets' trades have $T - c/2$; a limit below $c/2$ is refused. With limits on
cash (below), the risk-free asset is one of the choices and its change counts with the others'.
The tracking error counts the benchmark's cash in other currencies, with its risk. The report lists the trades from the current weights with the turnover, and gives the
tracking error against the benchmark weights whenever they are given, and the frontier is drawn
under the same limits. The trade limits need a unique optimum, so they are not offered for the
methods measured on the periods' returns.

In a backtest, the tracking-error limit applies at every fit. The turnover and trade limits apply
at each refit of a walk-forward validation, from the weights held at that refit, which have
drifted with the returns since the last; the first fit enters from cash without them. So these
refits are fitted in order as the run reaches them, rather than all at once beforehand. A backtest
takes no current weights, and refuses the turnover and trade limits under a validation that fits
only once, in sample or on a holdout.

```python
>>> import finalytics as fl
>>> assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
>>> current = {"US Equity": 0.5, "Intl Equity": 0.1, "Bonds": 0.3, "Gold": 0.05, "T-Bills": 0.05}
>>> result = assets.optimise(method="maximum_sharpe", current_weights=current, max_turnover=0.1)
>>> round(result.report.trades.turnover, 4)
0.1

```

### Cash

`Assets.optimise` invests in the risky assets, all but the cash, unless `cash` limits allow some:
`cash=(0, 0.2)` lets the weights hold up to 20% in cash, riskless (its variance and covariances
are zero). The cash is the risk-free asset, earning its own mean return, or, for assets without
one, an asset named `Cash` earning the risk-free rate. The frontier then includes the cash, from
the least risky mix the limits allow. The Sharpe and diversification ratios are the same with any
cash at the risk-free rate, so those methods take no limits on it. The methods measured on the
periods' returns hold the cash with its return in each period: the risk-free asset's own, or the
risk-free rate; for minimum CVaR, the report's shares of risk are of the expected shortfall of
every weight, the cash's included. In a backtest, the cash is held between refits as the weights
leave it, earning the risk-free return.

`Portfolio.assets()` includes the portfolio's cash: base-currency cash, such as `USD cash`, as
the risk-free asset, riskless and earning nothing, and cash in other currencies, such as `EUR
cash`, which moves with its exchange rate and is estimated with the instruments. The estimates
are those of the overview report, made over every holding but the base-currency cash on the
same dates, so the portfolio's own weights valued on them give the overview's current point.
Optimisation chooses among the instruments (`risky_assets`); the cash (`cash_assets`) is there
to value the portfolio, and only the risk-free asset can be chosen, with `cash` limits.

```python
>>> import finalytics as fl
>>> assets = fl.Assets(fl.datasets.asset_class_prices(), risk_free_asset="T-Bills")
>>> result = assets.optimise(method="minimum_variance", cash=(0, 0.2))
>>> round(result.results.weights["T-Bills"], 4)
0.2
>>> fl.datasets.portfolio().assets().cash_assets
['USD cash', 'EUR cash']

```

### Weights by asset

Every input that takes weights by asset follows one rule: the current and benchmark weights of
an optimisation and its frontier, the weights `evaluate` values, fixed weights and benchmark
weights in backtests and comparisons, the mixes of a plan, and rebalancing targets. Each may
name any of the assets, cash included: the risk-free asset, cash in other currencies such as a
portfolio's `EUR cash`, and, where there is no risk-free asset (and no asset of that name),
`Cash` at the risk-free rate. They add up to one with the cash, so a portfolio's own
`portfolio.weights()` are valid everywhere. A label that is none of these is refused as
`unknown asset Nope`, at the path of the weight. The weights of an optimisation and of a
rebalancing are zero or more; those valued or backtested may be negative. Limits on weights
(`bounds`, `asset_bounds`, `groups`) and risk budgets are for the assets optimisation chooses,
so they refuse the cash, in favour of `cash` limits. A backtest holds cash in other currencies
that fixed weights name as the assets it is, moving with its exchange rate, and the risk-free
asset, or `Cash`, as the cash the weights leave.

### Given weights

`Assets.evaluate(weights)` values weights chosen some other way on the estimates optimisation
uses, the same covariance and expected returns, so that they compare with an optimisation's:
their expected return $\mu^\top w$, volatility $\sqrt{w^\top \Sigma w}$, Sharpe ratio and each
asset's share of the variance, $w_i (\Sigma w)_i / w^\top \Sigma w$. The weights add up to one
and may hold the risk-free asset and other cash. The report is the optimisation report's, titled
"Given weights", without a solver; with `frontier=True` it draws the efficient frontier under
the limits given, which apply to nothing else. A portfolio's own weights on its assets give the
overview report's current point.

```python
>>> import finalytics as fl
>>> portfolio = fl.datasets.portfolio()
>>> valued = portfolio.assets().evaluate(portfolio.weights())
>>> current = portfolio.report().overview.current
>>> abs(valued.results.volatility - current.volatility) < 1e-12
True

```

### Resampling and Black–Litterman

`Problem.resampled` solves any of the covariance-based objectives above for estimates re-made from
simulated histories and averages the weights, and `Assets.optimise(resample=200)` does the same
for the assets' own estimates; see [robust and resampled optimisation](robust-optimisation.md).
[Black–Litterman](black-litterman.md) supplies expected returns and a covariance to optimise with.

### Efficient frontier

`frontier(points)` solves `target_return` for targets evenly spaced from the minimum-variance
portfolio's expected return to the largest attainable under the constraints (found by
maximising the expected return), and returns each portfolio's expected return, volatility and weights.
`Assets.frontier` takes the limits of an optimisation (on weights, cash, trades and tracking
error), the expected returns, and the current and benchmark weights the limits start from; each
point is the least variance for its return, so it takes no method and refuses a method's
parameters, as `options.target: the frontier does not use target`. The risk-free rate is the
return of `Cash` where cash limits hold it, and is refused otherwise.

## What each method takes

`Assets.optimise` names each method's inputs: the parameters some methods need (a target for
target return and target volatility, a risk aversion for the mean–variance methods, a confidence
for minimum CVaR and CDaR), those others may take (risk budgets, a linkage, resampling), the
options every method takes (a risk-free rate and expected returns), and the limits on weights,
which equal weight, inverse volatility, risk parity, risk budgeting and hierarchical risk parity
do not take, being fully invested and long only by construction. `methods()` (`fl.methods()` in
Python, `methods()` in JavaScript) describes every method this way: each input's name, kind,
default, and the range of values allowed, with whether either end of it is allowed. The checks on
the options read the same descriptions.

One rule applies to an option a method does not use, in every verb. An optimisation, and a
backtest of one method, refuse it, naming the option and the method:
`options.target: minimum variance does not use target`. A comparison gives its options to every
method it compares, so it applies each option to the methods that take it and notes, for each
other method, the options that do not apply to it (`The limits on cash do not apply.`); it
refuses an option that none of its methods takes, naming them all, in the words an
optimisation of each would use (`maximum Sharpe does not take cash: …`). Fixed weights take no
limits or parameters.

```python
>>> import finalytics as fl
>>> cvar = next(m for m in fl.methods() if m.method == "minimum_cvar")
>>> [(p.name, p.required, p.default) for p in cvar.parameters[:2]]
[('confidence', False, 0.95), ('target', False, None)]
>>> [p.name for p in cvar.limits]
['max_weight', 'bounds', 'asset_bounds', 'groups', 'min_effective_assets', 'max_turnover', 'max_tracking_error', 'cash']
>>> confidence = cvar.parameters[0]
>>> confidence.min, confidence.min_exclusive, confidence.max, confidence.max_exclusive
(0.0, True, 1.0, True)

```

## Reporting

Every solution reports its **status** (`optimal`, `nearly_optimal`, `infeasible`, `unbounded`,
`iteration_limit` or `numerical_difficulty`), the weights when there is a solution, the
objective's value, the expected return and volatility, the **largest constraint violation** of
the weights returned, the method, the iterations and the time taken. A problem whose constraints
cannot all hold is not an error: its solution has the status `infeasible` and no weights. The
objective's value and the expected return are before trading costs, linear or quadratic; the
costs shape the weights but are not subtracted from the reported figures.

Clarabel runs at tolerances of $10^{-10}$ on the duality gap and feasibility, tighter than its
defaults of $10^{-8}$, so that a weight that belongs at a bound ends within about $10^{-6}$ of it.
Where a constraint binds on the boundary of a second-order cone, as the limit on the effective
number of assets can, the solver's iterates can lose feasibility before the gap closes to
$10^{-10}$, and it stops short; such a solution is `optimal` when its gap is within $10^{-8}$ and
its weights meet every constraint to $10^{-9}$, and `nearly_optimal` otherwise.

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

`scripts/reference/downside_objectives.py` formulates seven more problems on 6 assets and 400
scenarios in CVXPY: minimum semivariance, with and without a floor on expected return, as the
mean square of `cp.neg` of the returns; minimum CDaR at 95%, and at 90% with an asset bound and a
floor, as the linear programme above, its value checked against the drawdowns computed
directly; minimum CVaR with a floor; minimum variance with at least 4.5 effective assets, as
`cp.norm(w, 2) <= 1 / sqrt(k)`; and the maximum Sharpe ratio with at least 4, found without the
Charnes–Cooper transformation by golden-section search along the constrained frontier. The
tolerances are those above, with the linear programmes compared by objective only, in Rust,
Python and JavaScript.

## References

- Markowitz, H., "Portfolio selection", *Journal of Finance* 7(1), 1952.
- Charnes, A. and Cooper, W. W., "Programming with linear fractional functionals", *Naval
  Research Logistics Quarterly* 9(3–4), 1962.
- Choueifaty, Y. and Coignard, Y., "Toward maximum diversification", *Journal of Portfolio
  Management* 35(1), 2008.
- Rockafellar, R. T. and Uryasev, S., "Optimization of conditional value-at-risk", *Journal of
  Risk* 2(3), 2000.
- Chekhlov, A., Uryasev, S. and Zabarankin, M., "Drawdown measure in portfolio optimization",
  *International Journal of Theoretical and Applied Finance* 8(1), 2005.
- DeMiguel, V., Garlappi, L., Nogales, F. J. and Uppal, R., "A generalized approach to portfolio
  optimization: improving performance by constraining portfolio norms", *Management Science*
  55(5), 2009.
- Spinu, F., "An algorithm for computing risk parity weights", SSRN 2297383, 2013.
- López de Prado, M., "Building diversified portfolios that outperform out of sample", *Journal
  of Portfolio Management* 42(4), 2016.
- Goulart, P. J. and Chen, Y., "Clarabel: an interior-point solver for conic programs with
  quadratic objectives", arXiv:2405.12762, 2024.
- Diamond, S. and Boyd, S., "CVXPY: a Python-embedded modeling language for convex optimization",
  *Journal of Machine Learning Research* 17(83), 2016.
- Martin, R. A., "PyPortfolioOpt: portfolio optimization in Python", *Journal of Open Source
  Software* 6(61), 2021.
