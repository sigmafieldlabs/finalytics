---
description: Use the low-level Problem API for configured long–short weights, gross leverage, linear and quadratic costs, factor exposures and solver status.
learn: portfolio/1.6
---

# Go beneath the front door

`Problem` exposes the optimisation model directly: covariance or factor risk, expected returns or scenarios, constraints and an objective. It returns a raw `Solution`, not the high-level optimisation report.

> [!IMPORTANT]
> `Problem` defaults to long-only weights. Negative positions arise only when negative bounds are explicitly configured. This lower-level analytical capability does not extend to `Assets.optimise`, rebalancing or the transaction ledger, which remain long only.

## Configure long–short weights and leverage

Set `bounds` with a negative minimum and add `leverage` to cap gross exposure, the sum of absolute weights. The budget constraint still controls net exposure.

[long-short.py](../../examples/beneath-the-front-door/long-short.py)

Without finite bounds or a leverage limit, some objectives can be unbounded. The status distinguishes that case from an infeasible constraint set. A mathematically valid short allocation still needs an external execution and accounting route.

## Include trading costs

`Turnover` supplies current weights, linear costs per unit traded and optional quadratic costs. Linear costs can shape objectives that use expected return; quadratic costs support mean–variance, robust mean–variance and target volatility.

[costs.py](../../examples/beneath-the-front-door/costs.py)

Costs influence the weights. The solution's reported expected return and objective retain the library's reporting conventions rather than serving as a complete after-cost implementation record.

## Constrain factor exposures

Build the problem from a factor model and set minimum and maximum exposure by factor name. Supplying benchmark weights in the limit instead constrains active exposure.

[factor-exposures.py](../../examples/beneath-the-front-door/factor-exposures.py)

The factor model's asset labels and order must match the problem. Factor-form variance can be more efficient than a full covariance matrix for large models.

## Inspect solver status

A problem whose constraints cannot all hold returns `infeasible` with no weights; it is not an input-validation exception. Other statuses include `nearly_optimal`, `unbounded`, `iteration_limit` and `numerical_difficulty`.

[status.py](../../examples/beneath-the-front-door/status.py)

Check status before using weights, then inspect `max_violation`, method and iteration count. Invalid dimensions or option values still raise validation errors. See [Portfolio optimisation](../../methods/optimisation.md) for supported objectives, constraints and solver tolerances.
