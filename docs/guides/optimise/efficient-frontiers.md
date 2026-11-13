---
description: Compute and display efficient frontiers under asset, group, cash and trading limits.
learn: portfolio/1.3
---

# Trace an efficient frontier

An efficient frontier shows the least-variance portfolio at each attainable expected return. It makes the return-for-risk trade-off and the effect of constraints visible instead of reducing the decision to one optimised point.

## Compute frontier points

`frontier(points=...)` starts at the minimum-variance portfolio and ends at the largest expected return attainable under the limits. Intermediate targets are evenly spaced in expected return.

[frontier.py](../../examples/efficient-frontiers/frontier.py)

Each point contains expected return, volatility and weights. The frontier does not take an optimisation method or method parameters: every point is a target-return problem.

## Apply the same investment limits

Bounds, groups, cash, turnover, trade and tracking-error limits apply across the frontier. Supply the current and benchmark weights that those limits need.

[constrained.py](../../examples/efficient-frontiers/constrained.py)

Changing a limit changes the whole frontier, not just one point. Infeasible intermediate targets are reported through their status rather than being presented as valid portfolios.

## Put a portfolio on the frontier

`evaluate(..., frontier=True)` values given weights and includes a frontier in the report. The frontier uses the supplied limits; it does not alter the given weights being evaluated.

[show.py](../../examples/efficient-frontiers/show.py)

The current portfolio can sit inside the feasible region rather than on its efficient edge. That is information about the estimates and constraints, not by itself evidence that the allocation is wrong. See [Evaluate given weights](evaluate-given-weights.md) for the evaluation result.
