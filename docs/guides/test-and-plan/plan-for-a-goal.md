---
description: Simulate saving, spending and investment mixes for a target, an income stream or a projection without a goal.
---

# Plan for a goal

`Assets.plan` simulates monthly markets from the assets' history. A plan tests one target or payment goal and reports its chance, value over time, possible changes and behaviour in weaker markets.

## Set a target and regular saving

Give an amount and destination month, a starting value, recurring saving and any one-off spending. Inflation and fees are deducted in the simulation so amounts remain in today's money.

[target.py](../../examples/plan-for-a-goal/target.py)

The report estimates the chance of reaching the target and shows changes that could reach the desired chance: save more, start with more, adjust the target or change its date.

## Plan a stream of payments

`Payments` succeeds when every payment is made in full. Add alternatives to compare investment mixes on exactly the same simulated markets.

[payments-and-alternatives.py](../../examples/plan-for-a-goal/payments-and-alternatives.py)

The payment report also shows the share paid and when money runs short in weak, average and strong markets. Spending outside the goal is paid as far as available money allows but is not itself scored.

## Change the mix along a glide path

A glide path interpolates weights between dated points, holds the first mix before its first point and the last mix after its final point. This example uses normally distributed monthly returns.

[glide-path.py](../../examples/plan-for-a-goal/glide-path.py)

The default stationary block bootstrap replays runs of historical months. `historical` instead evaluates every historical stretch, while `normal` draws from estimated means and covariance.

## Project without a success probability

Omit the goal and give an end month for a projection. A custom bootstrap block controls the average length of replayed historical runs.

[projection.py](../../examples/plan-for-a-goal/projection.py)

Without a goal there is no chance or status, only the projected values and market scenarios. Set a seed when random simulations need to be reproduced.

> [!WARNING]
> Plans rebalance monthly without trading costs, omit taxes and market-responsive spending rules, and model one goal at a time. Bootstrap and historical simulations cannot invent a month worse than the history contains.

See [Goal planning](../../methods/planning.md) for the simulation and disclosure conventions.
