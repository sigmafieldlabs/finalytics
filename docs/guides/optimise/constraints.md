---
description: Constrain optimised weights by asset, group, cash, turnover, trade count, tracking error and diversification.
learn: portfolio/1.5
---

# Constrain an optimisation

Constraints make an optimisation investable. The high-level `Assets.optimise` front door is fully invested and long only by default; the limits below shape those non-negative weights.

## Bound assets and groups

Use `bounds` for every asset, `asset_bounds` for exceptions and `groups` for a total across named assets. Group limits can also use instrument classifications when assets come from a portfolio.

[bounds-and-groups.py](../../examples/constraints/bounds-and-groups.py)

The budget remains one. Bounds and group ranges must leave a feasible portfolio; for example, five assets capped at one tenth each cannot hold the whole portfolio.

## Allow cash

Without cash limits, optimisation invests fully in the risky assets. `cash` permits a position in the risk-free asset, or in `Cash` earning the risk-free rate when no risk-free asset is named.

[cash.py](../../examples/constraints/cash.py)

Maximum Sharpe and maximum diversification do not accept cash limits because risk-free cash leaves their ratios unchanged. Methods based directly on return scenarios include cash's return in each period.

## Limit turnover and trades

`current_weights` supplies the starting allocation. `max_turnover` is one-way turnover: half the sum of absolute weight changes, including cash. `min_trade` and `max_trades` then suppress small or excess trades.

[turnover-and-trades.py](../../examples/constraints/turnover-and-trades.py)

Minimum trade sizes and trade counts are not convex. Finalytics uses trade elimination and re-optimisation, so the result satisfies the limits but is not guaranteed to be the best possible discrete solution.

## Limit benchmark risk

`benchmark_weights` with `max_tracking_error` constrain ex-ante annual tracking error. The report also shows tracking error whenever benchmark weights are supplied without a limit.

[tracking-error.py](../../examples/constraints/tracking-error.py)

Keep the covariance and tracking-error limit in the same time units. The benchmark's cash participates in the calculation, including foreign-currency cash risk for portfolio assets.

## Require diversification

`min_effective_assets` bounds `1 / sum(w²)` from below. Equal weights across `n` assets have an effective count of `n`; one asset has a count of one.

[effective-assets.py](../../examples/constraints/effective-assets.py)

This is a convex norm constraint, not a requirement to hold a particular number of non-zero positions. The engine checks that the other bounds leave enough room before solving.

## Keep existing cash

`keep_cash=True` fixes every currency's cash at its weight in `current_weights`; a list of currency codes keeps only selected currencies. The remaining allocation is optimised under limits measured on the whole portfolio.

[keep-cash.py](../../examples/constraints/keep-cash.py)

Keeping cash is different from `cash` limits. Use the same setting when rebalancing so the proposed trades preserve it. See [Portfolio optimisation](../../methods/optimisation.md) for the exact conventions and method support.
