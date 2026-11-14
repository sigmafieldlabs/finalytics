---
description: Turn target weights or an optimisation into account-level trades with lots, tax, fees, whole units and preserved cash.
learn: portfolio/1.10
---

# Rebalance a real portfolio

`Portfolio.rebalance` works from the ledger, not just from a return series. It proposes sales, purchases and currency conversions for each account while respecting lots, fees and available cash.

## Rebalance to given weights

Name target weights for the combined portfolio. Holdings left out have a zero target and are sold; rounding whole units can leave small differences from the targets.

[target-weights.py](../../examples/rebalance-a-real-portfolio/target-weights.py)

The report gives weights before, at target and after trading, together with turnover, realised gains, fees, tax and warnings.

## Use an optimisation as the target

Pass the optimisation result, its raw results, or its weights. The example adds tax-aware lot selection, proportional fees and a minimum fee.

[optimised-target.py](../../examples/rebalance-a-real-portfolio/optimised-target.py)

Taxable sales choose lots by estimated tax cost. Non-taxable accounts report no tax and follow their own lot method. This is an estimate, not an implementation of any country's complete tax code.

## Control lots and small trades

Rebalance selected accounts when the others should remain untouched. Set the lot method, a fixed charge per trade, and a minimum base-currency trade value.

[lots-and-fees.py](../../examples/rebalance-a-real-portfolio/lots-and-fees.py)

Small omitted trades appear in warnings. Purchases are resized when fees and available cash require it; rebalancing never moves cash between accounts.

## Keep cash and apply the proposal

`keep_cash` preserves every currency's balance, or pass a list of currencies to preserve only some. The other targets share the remaining value.

[keep-cash-and-apply.py](../../examples/rebalance-a-real-portfolio/keep-cash-and-apply.py)

`with_rebalance` applies the proposal as dated ledger transactions. Reporting on the returned portfolio verifies the books after the trades rather than treating the proposal as already executed.

> [!WARNING]
> Rebalancing targets are long only. Review estimated tax, carried-price warnings and residual cash before applying trades to real accounts.
