# Robust and resampled optimisation

Optimal portfolios are sensitive to errors in their estimates, above all in expected returns:
small changes in the inputs can move the weights a long way. Robust optimisation and resampling
are two ways to make portfolios less sensitive to those errors. Black–Litterman, the third, is on
[its own page](black-litterman.md).

## Robust mean–variance

Instead of trusting the estimates $m$ of the expected returns, robust optimisation lets the true
expected returns $\mu$ lie anywhere in an uncertainty set $U$ around them, and maximises the worst
case:

$$\max_w \; \min_{\mu \in U} \mu^\top w - \frac{\lambda}{2} w^\top \Sigma w$$

(`RobustMeanVariance`). Both sets below have the worst case in closed form, so the problem stays
convex:

| Set | Definition | Worst case of $\mu^\top w$ | Formulation |
| --- | --- | --- | --- |
| Ellipsoid | $(\mu - m)^\top S^{-1} (\mu - m) \le \kappa^2$ | $m^\top w - \kappa \sqrt{w^\top S w}$ | Second-order cone |
| Box | $\lvert \mu_i - m_i \rvert \le h_i$ | $m^\top w - \sum_i h_i \lvert w_i \rvert$ | Linear, with $u_i \ge \lvert w_i \rvert$ |

For an ellipsoid, $S$ is the covariance of the estimation error. When the estimates are sample
means of $T$ periods, $S = \Sigma / T$ (or its diagonal) and $\kappa^2$ is a quantile of the
chi-squared distribution with $N$ degrees of freedom, so that the set holds the true means with
that probability. Without its own $S$, the ellipsoid uses the problem's covariance matrix or factor
model, and $\kappa$ absorbs the $1/\sqrt{T}$. With $\kappa = 0$ the problem is mean–variance; as
$\kappa$ grows, the return term counts for less and the portfolio approaches minimum variance.

The reported objective value is the worst-case utility,
$m^\top w - \text{shortfall} - \frac{\lambda}{2} w^\top \Sigma w$, and the reported expected return
is the nominal $m^\top w$.

## Resampled optimisation

Resampling (Michaud, 1998) treats the estimates as one draw among many that the same history could
have produced (`Problem.resampled`):

1. Simulate $S$ histories of $T$ periods, $r_t = m + L z_t$, with $L$ the Cholesky factor of
   $\Sigma$ ($L L^\top = \Sigma$) and $z_t$ standard normal.
2. From each history, estimate the sample means and the sample covariance (divisor $T - 1$).
3. Solve the problem for each set of estimates, under the same constraints.
4. Average the optimal weights.

$T$ should be the length of the history behind the estimates, so that the simulated estimates are
as uncertain as the real ones. When the estimates are annual figures from returns with $P$
periods a year, `periods_per_year` makes each simulated period one period of the returns: the
histories are drawn from $m / P$ and $\Sigma / P$ and their estimates multiplied by $P$ again, so
that a mean estimated from $T$ monthly returns has the standard error $\sigma / \sqrt{T / 12}$
of the real one, rather than that of $T$ years.

`Assets.optimise(resample=200)` resamples the assets' own estimates in this way, with $T$ the
number of their returns, $P$ their periods per year and a fixed seed, for every method that uses
estimates other than equal weight and the downside ones. The optimisation report says how many
histories were simulated and solved; `results.resampled` holds each weight's standard deviation
across them, and `results.solution` the optimum for the estimates themselves. The average portfolio is more diversified and changes less from one
set of estimates to the next than the optimum for the estimates themselves. Each weight's standard
deviation across the simulations is reported with it, as are the number of simulated problems
solved and any the solver could not solve, which are left out of the average.

The average of portfolios that satisfy a convex constraint satisfies it too, so the average meets
every constraint that does not depend on the estimates (budget, bounds, groups, leverage,
turnover). A tracking-error limit, or a floor on expected return, holds for each simulation's
own estimates. Scenario-based objectives (minimum CVaR, semivariance and CDaR) and factor
exposure limits are not supported.

The random numbers come from SplitMix64 (Steele, Lea and Flood, 2014) through the Box–Muller
transform, computed with portable mathematical functions, so that the same seed gives the same
result in Rust, Python and JavaScript on every platform. A singular covariance matrix is simulated
with a pivoted factorisation instead of $L$.

## Example

```python
>>> import finalytics as fl
>>> labels = ["us equity", "intl equity", "bonds"]
>>> covariance = fl.risk.matrix(
...     [[0.0256, 0.02304, 0.0008], [0.02304, 0.0324, 0.0009], [0.0008, 0.0009, 0.0025]], labels
... )
>>> problem = fl.opt.Problem.from_covariance(covariance, expected_returns=[0.07, 0.08, 0.03])
>>> [round(w, 3) for w in problem.solve(fl.opt.MeanVariance(risk_aversion=4.0)).weights]
[0.166, 0.307, 0.528]
>>> robust = fl.opt.RobustMeanVariance(risk_aversion=4.0, uncertainty=fl.opt.Ellipsoid(size=0.5))
>>> [round(w, 3) for w in problem.solve(robust).weights]
[0.091, 0.096, 0.814]
>>> resampled = problem.resampled(fl.opt.MeanVariance(risk_aversion=4.0), observations=60, samples=200, seed=7)
>>> [round(w, 3) for w in resampled.weights], resampled.solved
([0.21, 0.28, 0.51], 200)

```

With an ellipsoid of size 0.5 the expected returns are discounted by half a standard deviation of
the portfolio, which favours bonds. Resampling, with histories of 60 periods, stays close to the
mean–variance portfolio but spreads the equity weight more evenly between the two markets, whose
expected returns are hard to tell apart with that much data.

## Validation

`scripts/reference/optimisation.py` formulates three robust problems in CVXPY with the worst case
written out (an ellipsoid of the covariance, an ellipsoid of the estimation error with a group
limit, and a box) and solves them to $10^{-10}$; objectives agree to $10^{-7}$ and weights to
$10^{-4}$. For resampling, the script reimplements the random numbers and the simulation, solves
each of 30 simulated problems with CVXPY, and averages them; the average weights and their
standard deviations agree with the engine's to $10^{-4}$, which also confirms that the simulated
histories are the same. The tests check that a robust problem of size 0 gives the mean–variance
portfolio and a large one the minimum-variance portfolio, and that a resampled result depends only
on its seed. The Python and JavaScript tests repeat the reference checks.

## References

- Michaud, R. O., *Efficient Asset Management*, Harvard Business School Press, 1998.
- Ben-Tal, A. and Nemirovski, A., "Robust convex optimization", *Mathematics of Operations
  Research* 23(4), 1998.
- Goldfarb, D. and Iyengar, G., "Robust portfolio selection problems", *Mathematics of Operations
  Research* 28(1), 2003.
- Fabozzi, F. J., Kolm, P. N., Pachamanova, D. A. and Focardi, S. M., *Robust Portfolio
  Optimization and Management*, Wiley, 2007.
- Steele, G. L., Lea, D. and Flood, C. H., "Fast splittable pseudorandom number generators",
  *OOPSLA*, 2014.
